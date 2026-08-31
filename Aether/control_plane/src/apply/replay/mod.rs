//! Signed operation replay store — one-shot Apply execution guard.
//!
//! Phase 2: persistence and state machine only. No PROTO-0 mutations.

pub mod cleanup;
pub mod model;
pub mod reconcile;
pub mod store;

pub use cleanup::{sweep_executing_timeouts, sweep_reserved_timeouts};
pub use model::{
    allowed_transition, AtomicPrepareRequest, ReplayError, ReplayErrorCode, ReplayRecord,
    ReplayStatus, ReserveRequest,
};
pub use reconcile::{
    admin_abort, get, list_executing, list_reserved, list_stuck, list_terminal, startup_scan,
    ReconciliationReport,
};
pub use store::{ReplayStore, DEFAULT_RESERVE_TIMEOUT};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use chrono::{Duration as ChronoDuration, Utc};
    use uuid::Uuid;

    async fn test_store() -> (ReplayStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("replay.db");
        let db = Db::connect(path.to_str().unwrap()).await.expect("db");
        db.migrate().await.expect("migrate");
        (ReplayStore::new(db.pool().clone()), dir)
    }

    fn reserve_req(op_suffix: &str) -> ReserveRequest {
        ReserveRequest {
            operation_id: format!("op-{op_suffix}"),
            execution_hash: format!("hash-{op_suffix}"),
            dry_run_id: format!("dry-{op_suffix}"),
        }
    }

    #[test]
    fn allowed_transition_matrix_matches_spec() {
        assert!(allowed_transition(
            ReplayStatus::Reserved,
            ReplayStatus::Executing
        ));
        assert!(allowed_transition(
            ReplayStatus::Reserved,
            ReplayStatus::Stuck
        ));
        assert!(allowed_transition(
            ReplayStatus::Executing,
            ReplayStatus::Executed
        ));
        assert!(!allowed_transition(
            ReplayStatus::Executed,
            ReplayStatus::Executing
        ));
        assert!(allowed_transition(
            ReplayStatus::Stuck,
            ReplayStatus::Aborted
        ));
        assert!(allowed_transition(
            ReplayStatus::Executing,
            ReplayStatus::Stuck
        ));
    }

    #[tokio::test]
    async fn valid_lifecycle() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("lifecycle");
        let reserved = store.reserve(req.clone()).await.expect("reserve");
        assert_eq!(reserved.status, ReplayStatus::Reserved);

        let executing = store
            .begin_execution(&req.operation_id)
            .await
            .expect("begin");
        assert_eq!(executing.status, ReplayStatus::Executing);

        let executed = store
            .mark_executed(&req.operation_id, Some("audit-1".into()))
            .await
            .expect("executed");
        assert_eq!(executed.status, ReplayStatus::Executed);
        assert_eq!(executed.audit_reference.as_deref(), Some("audit-1"));
        assert!(executed.finalised_at.is_some());
    }

    #[tokio::test]
    async fn duplicate_reserve_rejected() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("dup");
        store.reserve(req.clone()).await.expect("first");

        let err = store.reserve(req).await.unwrap_err();
        assert!(matches!(err, ReplayError::InProgress { .. }));
        assert_eq!(err.code(), ReplayErrorCode::ReplayInProgress);
    }

    #[tokio::test]
    async fn duplicate_reserve_after_terminal() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("terminal-dup");
        store.reserve(req.clone()).await.unwrap();
        store.begin_execution(&req.operation_id).await.unwrap();
        store
            .mark_rejected(&req.operation_id, Some("sim fail".into()), None)
            .await
            .unwrap();

        let err = store.reserve(req).await.unwrap_err();
        assert!(matches!(err, ReplayError::Duplicate { .. }));
    }

    #[tokio::test]
    async fn invalid_transition_rejected() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("invalid");
        store.reserve(req.clone()).await.unwrap();

        let err = store
            .mark_executed(&req.operation_id, None)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ReplayError::InvalidTransition {
                from: ReplayStatus::Reserved,
                to: ReplayStatus::Executed
            }
        ));
    }

    #[tokio::test]
    async fn executed_immutable() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("exec-immutable");
        store.reserve(req.clone()).await.unwrap();
        store.begin_execution(&req.operation_id).await.unwrap();
        store.mark_executed(&req.operation_id, None).await.unwrap();

        assert!(store.begin_execution(&req.operation_id).await.is_err());
        assert!(store
            .mark_rejected(&req.operation_id, None, None)
            .await
            .is_err());
        assert!(store
            .mark_aborted(&req.operation_id, None, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn rejected_immutable() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("rej-immutable");
        store.reserve(req.clone()).await.unwrap();
        store.begin_execution(&req.operation_id).await.unwrap();
        store
            .mark_rejected(&req.operation_id, Some("proto".into()), None)
            .await
            .unwrap();

        assert!(store.begin_execution(&req.operation_id).await.is_err());
        assert!(store.mark_executed(&req.operation_id, None).await.is_err());
    }

    #[tokio::test]
    async fn aborted_immutable() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("abort-immutable");
        store.reserve(req.clone()).await.unwrap();
        store
            .mark_aborted(&req.operation_id, Some("admin".into()), None)
            .await
            .unwrap();

        assert!(store.begin_execution(&req.operation_id).await.is_err());
        assert!(store.reserve(req).await.is_err());
    }

    #[tokio::test]
    async fn reserve_timeout_becomes_stuck() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("replay-timeout.db");
        let db = Db::connect(path.to_str().unwrap()).await.unwrap();
        db.migrate().await.unwrap();
        let store = ReplayStore::with_reserve_timeout(
            db.pool().clone(),
            std::time::Duration::from_secs(60),
        );

        let req = reserve_req("timeout");
        store.reserve(req.clone()).await.unwrap();

        let past = (Utc::now() - ChronoDuration::minutes(10)).to_rfc3339();
        sqlx::query("UPDATE signed_operation_replay SET reserved_at = ? WHERE operation_id = ?")
            .bind(&past)
            .bind(&req.operation_id)
            .execute(store.pool())
            .await
            .unwrap();

        let count = sweep_reserved_timeouts(&store).await.unwrap();
        assert_eq!(count, 1);

        let row = get(&store, &req.operation_id).await.unwrap().unwrap();
        assert_eq!(row.status, ReplayStatus::Stuck);
        assert_eq!(row.terminal_reason.as_deref(), Some("reserve_timeout"));
    }

    #[tokio::test]
    async fn concurrent_reserve_exactly_one_wins() {
        let (store, _dir) = test_store().await;
        let op_id = Uuid::new_v4().to_string();
        let req = ReserveRequest {
            operation_id: op_id.clone(),
            execution_hash: "hash".into(),
            dry_run_id: "dry".into(),
        };

        let store2 = store.clone();
        let req2 = req.clone();
        let (a, b) = tokio::join!(store.reserve(req), store2.reserve(req2));

        let successes = [a, b].into_iter().filter(|r| r.is_ok()).count();
        assert_eq!(successes, 1);

        let row = get(&store, &op_id).await.unwrap().unwrap();
        assert_eq!(row.status, ReplayStatus::Reserved);
    }

    #[tokio::test]
    async fn persistence_round_trip_and_lookup() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("persist");
        store.reserve(req.clone()).await.unwrap();

        let loaded = get(&store, &req.operation_id).await.unwrap().unwrap();
        assert_eq!(loaded.execution_hash, req.execution_hash);
        assert_eq!(loaded.dry_run_id, req.dry_run_id);
        assert_eq!(loaded.status, ReplayStatus::Reserved);
    }

    #[tokio::test]
    async fn crash_after_reserve_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crash-reserve.db");
        let db_path = path.to_str().unwrap().to_string();

        {
            let db = Db::connect(&db_path).await.unwrap();
            db.migrate().await.unwrap();
            let store = ReplayStore::new(db.pool().clone());
            store.reserve(reserve_req("crash-r")).await.unwrap();
        }

        let db = Db::connect(&db_path).await.unwrap();
        let store = ReplayStore::new(db.pool().clone());
        let row = get(&store, "op-crash-r").await.unwrap().unwrap();
        assert_eq!(row.status, ReplayStatus::Reserved);
    }

    #[tokio::test]
    async fn crash_after_executing_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crash-exec.db");
        let db_path = path.to_str().unwrap().to_string();

        {
            let db = Db::connect(&db_path).await.unwrap();
            db.migrate().await.unwrap();
            let store = ReplayStore::new(db.pool().clone());
            let req = reserve_req("crash-e");
            store.reserve(req.clone()).await.unwrap();
            store.begin_execution(&req.operation_id).await.unwrap();
        }

        let db = Db::connect(&db_path).await.unwrap();
        let store = ReplayStore::new(db.pool().clone());
        let row = get(&store, "op-crash-e").await.unwrap().unwrap();
        assert_eq!(row.status, ReplayStatus::Executing);
    }

    #[tokio::test]
    async fn list_terminal_and_stuck_helpers() {
        let (store, _dir) = test_store().await;

        let r1 = reserve_req("t1");
        store.reserve(r1.clone()).await.unwrap();
        store.begin_execution(&r1.operation_id).await.unwrap();
        store.mark_executed(&r1.operation_id, None).await.unwrap();

        let r2 = reserve_req("t2");
        store.reserve(r2.clone()).await.unwrap();
        store
            .mark_stuck(&r2.operation_id, Some("reserve_timeout".into()))
            .await
            .unwrap();

        let terminal = list_terminal(&store).await.unwrap();
        assert_eq!(terminal.len(), 2);
        let stuck = list_stuck(&store).await.unwrap();
        assert_eq!(stuck.len(), 1);
        assert_eq!(stuck[0].operation_id, r2.operation_id);
    }

    #[tokio::test]
    async fn atomic_reserve_and_consume_rolls_back_on_missing_approval() {
        let (store, _dir) = test_store().await;
        let err = store
            .reserve_and_consume_approval(AtomicPrepareRequest {
                operation_id: "op-atomic-miss".into(),
                execution_hash: "h".into(),
                dry_run_id: "d".into(),
                approval_id: "missing-approval".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ReplayError::ApprovalConsumeFailed { .. }));
        assert!(get(&store, "op-atomic-miss").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn reconciliation_startup_scan_and_admin_abort() {
        let (store, _dir) = test_store().await;
        let req = reserve_req("recon");
        store.reserve(req.clone()).await.unwrap();
        store
            .mark_stuck(&req.operation_id, Some("reserve_timeout".into()))
            .await
            .unwrap();

        let report = startup_scan(&store).await.unwrap();
        assert_eq!(report.stuck.len(), 1);

        let aborted = admin_abort(&store, &req.operation_id, "admin-1", "manual")
            .await
            .unwrap();
        assert_eq!(aborted.status, ReplayStatus::Aborted);
    }

    #[tokio::test]
    async fn executing_timeout_becomes_stuck() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("exec-timeout.db");
        let db = Db::connect(path.to_str().unwrap()).await.unwrap();
        db.migrate().await.unwrap();
        let store = ReplayStore::with_reserve_timeout(
            db.pool().clone(),
            std::time::Duration::from_secs(60),
        );
        let req = reserve_req("exto");
        store.reserve(req.clone()).await.unwrap();
        store.begin_execution(&req.operation_id).await.unwrap();

        let past = (Utc::now() - ChronoDuration::minutes(10)).to_rfc3339();
        sqlx::query("UPDATE signed_operation_replay SET updated_at = ? WHERE operation_id = ?")
            .bind(&past)
            .bind(&req.operation_id)
            .execute(store.pool())
            .await
            .unwrap();

        let count = sweep_executing_timeouts(&store).await.unwrap();
        assert_eq!(count, 1);
        let row = get(&store, &req.operation_id).await.unwrap().unwrap();
        assert_eq!(row.status, ReplayStatus::Stuck);
        assert_eq!(row.terminal_reason.as_deref(), Some("execution_timeout"));
    }
}
