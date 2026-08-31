//! Dry-run attestation persistence — immutable evidence for future Apply binding.
//!
//! Phase 3: store + dry-run hook only. No approvals, signatures, or PROTO-0 writes.

pub mod errors;
pub mod model;
pub mod store;

pub use errors::{AttestationError, AttestationErrorCode};
pub use model::{
    AttestationBinding, AttestationStatus, CreateAttestationRequest, DryRunAttestation,
    DEFAULT_ATTESTATION_TTL,
};
pub use store::AttestationStore;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::execution::types::{ProtocolOperationKind, SimulationOutcome};
    use chrono::{Duration, Utc};
    use serde_json::json;
    use uuid::Uuid;

    async fn test_store() -> (AttestationStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("attest.db");
        let db = Db::connect(path.to_str().unwrap()).await.expect("db");
        db.migrate().await.expect("migrate");
        (AttestationStore::new(db.pool().clone()), dir)
    }

    fn hash_for(label: &str) -> String {
        // 64 hex chars
        format!("{:0<64}", hex::encode(label.as_bytes()))
            .chars()
            .take(64)
            .collect()
    }

    fn valid_request(suffix: &str) -> CreateAttestationRequest {
        CreateAttestationRequest {
            dry_run_id: format!("dry-{suffix}"),
            operation_intent: "CapabilityGrant".into(),
            policy_id: format!("pol-{suffix}"),
            policy_version: 1,
            execution_hash: hash_for(suffix),
            simulation: SimulationOutcome {
                executable: true,
                reason: None,
            },
            protocol_operation_kind: ProtocolOperationKind::CapabilityGrant,
            predicted_changes: json!({"actions": ["transfer"]}),
            created_by: "operator-1".into(),
            audit_reference: Some(format!("audit-{suffix}")),
            ttl: None,
        }
    }

    #[tokio::test]
    async fn create_valid_attestation() {
        let (store, _dir) = test_store().await;
        let req = valid_request("ok");
        let row = store.create_attestation(req.clone()).await.unwrap();
        assert_eq!(row.status, AttestationStatus::Executable);
        assert_eq!(row.execution_hash, req.execution_hash);
        assert_eq!(row.audit_reference.as_deref(), Some("audit-ok"));
        assert!(row.expires_at > row.created_at);
    }

    #[tokio::test]
    async fn duplicate_dry_run_id_rejected() {
        let (store, _dir) = test_store().await;
        let req = valid_request("dup");
        store.create_attestation(req.clone()).await.unwrap();
        let err = store.create_attestation(req).await.unwrap_err();
        assert_eq!(err.code(), AttestationErrorCode::DuplicateDryRunId);
    }

    #[tokio::test]
    async fn immutable_execution_hash() {
        let (store, _dir) = test_store().await;
        let req = valid_request("imm");
        store.create_attestation(req.clone()).await.unwrap();

        // Re-insert with different hash, same dry_run_id — must fail as duplicate.
        let mut altered = req.clone();
        altered.execution_hash = hash_for("other");
        let err = store.create_attestation(altered).await.unwrap_err();
        assert_eq!(err.code(), AttestationErrorCode::DuplicateDryRunId);

        let loaded = store.get(&req.dry_run_id).await.unwrap().unwrap();
        assert_eq!(loaded.execution_hash, req.execution_hash);
    }

    #[tokio::test]
    async fn expired_attestation_rejected() {
        let (store, _dir) = test_store().await;
        let mut req = valid_request("exp");
        req.ttl = Some(Duration::seconds(1));
        store.create_attestation(req.clone()).await.unwrap();

        // Force past expiry in DB.
        let past = (Utc::now() - Duration::minutes(5)).to_rfc3339();
        sqlx::query("UPDATE dry_run_attestations SET expires_at = ? WHERE dry_run_id = ?")
            .bind(&past)
            .bind(&req.dry_run_id)
            .execute(store.pool())
            .await
            .unwrap();

        let binding = AttestationBinding {
            dry_run_id: req.dry_run_id.clone(),
            execution_hash: req.execution_hash.clone(),
            policy_version: 1,
            operation_intent: req.operation_intent.clone(),
        };
        let err = store.get_valid_attestation(&binding).await.unwrap_err();
        assert_eq!(err.code(), AttestationErrorCode::Expired);

        let loaded = store.get(&req.dry_run_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, AttestationStatus::Expired);
    }

    #[tokio::test]
    async fn policy_version_mismatch_rejected() {
        let (store, _dir) = test_store().await;
        let req = valid_request("ver");
        store.create_attestation(req.clone()).await.unwrap();

        let binding = AttestationBinding {
            dry_run_id: req.dry_run_id,
            execution_hash: req.execution_hash,
            policy_version: 99,
            operation_intent: req.operation_intent,
        };
        let err = store.get_valid_attestation(&binding).await.unwrap_err();
        assert_eq!(err.code(), AttestationErrorCode::PolicyVersionMismatch);
    }

    #[tokio::test]
    async fn failed_simulation_rejected() {
        let (store, _dir) = test_store().await;
        let mut req = valid_request("sim");
        req.simulation = SimulationOutcome {
            executable: false,
            reason: Some("target frozen".into()),
        };
        let err = store.create_attestation(req).await.unwrap_err();
        assert_eq!(err.code(), AttestationErrorCode::SimulationFailed);
    }

    #[tokio::test]
    async fn unknown_operation_rejected() {
        let (store, _dir) = test_store().await;
        let mut req = valid_request("unk");
        req.protocol_operation_kind = ProtocolOperationKind::Unknown;
        let err = store.create_attestation(req).await.unwrap_err();
        assert_eq!(err.code(), AttestationErrorCode::UnknownOperation);
    }

    #[tokio::test]
    async fn missing_hash_rejected() {
        let (store, _dir) = test_store().await;
        let mut req = valid_request("hash");
        req.execution_hash = String::new();
        let err = store.create_attestation(req).await.unwrap_err();
        assert_eq!(err.code(), AttestationErrorCode::MissingExecutionHash);
    }

    #[tokio::test]
    async fn persistence_round_trip() {
        let (store, _dir) = test_store().await;
        let req = valid_request("persist");
        store.create_attestation(req.clone()).await.unwrap();
        let loaded = store.get(&req.dry_run_id).await.unwrap().unwrap();
        assert_eq!(loaded.policy_id, req.policy_id);
        assert_eq!(loaded.operation_intent, req.operation_intent);
        assert_eq!(loaded.predicted_changes, req.predicted_changes);
    }

    #[tokio::test]
    async fn concurrent_creation_exactly_one_wins() {
        let (store, _dir) = test_store().await;
        let dry_run_id = Uuid::new_v4().to_string();
        let req = CreateAttestationRequest {
            dry_run_id: dry_run_id.clone(),
            ..valid_request("conc")
        };
        let store2 = store.clone();
        let req2 = CreateAttestationRequest {
            dry_run_id: dry_run_id.clone(),
            execution_hash: hash_for("conc2"),
            ..valid_request("conc2")
        };
        // Force same dry_run_id
        let mut r1 = req;
        let mut r2 = req2;
        r1.dry_run_id = dry_run_id.clone();
        r2.dry_run_id = dry_run_id.clone();

        let (a, b) = tokio::join!(store.create_attestation(r1), store2.create_attestation(r2));
        let successes = [a, b].into_iter().filter(|r| r.is_ok()).count();
        assert_eq!(successes, 1);
        assert!(store.get(&dry_run_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn stale_attestation_cannot_validate() {
        let (store, _dir) = test_store().await;
        let req = valid_request("stale");
        store.create_attestation(req.clone()).await.unwrap();
        store
            .invalidate(&req.dry_run_id, Some("policy edited"))
            .await
            .unwrap();

        let binding = AttestationBinding {
            dry_run_id: req.dry_run_id,
            execution_hash: req.execution_hash,
            policy_version: 1,
            operation_intent: req.operation_intent,
        };
        let err = store.get_valid_attestation(&binding).await.unwrap_err();
        assert_eq!(err.code(), AttestationErrorCode::Invalidated);
    }

    #[tokio::test]
    async fn get_valid_attestation_success() {
        let (store, _dir) = test_store().await;
        let req = valid_request("valid");
        store.create_attestation(req.clone()).await.unwrap();
        let binding = AttestationBinding {
            dry_run_id: req.dry_run_id.clone(),
            execution_hash: req.execution_hash.clone(),
            policy_version: 1,
            operation_intent: req.operation_intent.clone(),
        };
        let row = store.get_valid_attestation(&binding).await.unwrap();
        assert_eq!(row.status, AttestationStatus::Executable);
        assert_eq!(row.dry_run_id, req.dry_run_id);
    }
}
