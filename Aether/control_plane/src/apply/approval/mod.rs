//! Apply approval persistence — authorisation binding for future Apply.
//!
//! Phase 4: approvals only. No PROTO-0, no Apply execution, `apply_enabled` stays false.

pub mod errors;
pub mod model;
pub mod store;

pub use errors::{ApprovalError, ApprovalErrorCode};
pub use model::{
    ApplyApproval, ApprovalBinding, ApprovalStatus, CreateApprovalRequest, DEFAULT_APPROVAL_TTL,
};
pub use store::ApprovalStore;

use serde_json::json;
use sqlx::SqlitePool;

use crate::auth::middleware::AuthContext;
use crate::auth::roles::require_admin;
use crate::db::audit;
use crate::db::operators::OperatorRole;

/// Service wrapper: RBAC + SoD + audit around [`ApprovalStore`].
pub struct ApprovalService<'a> {
    store: ApprovalStore,
    pool: &'a SqlitePool,
}

impl<'a> ApprovalService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            store: ApprovalStore::new(pool.clone()),
            pool,
        }
    }

    pub fn store(&self) -> &ApprovalStore {
        &self.store
    }

    /// Admin-only approval grant with audit.
    pub async fn create_approval(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        mut request: CreateApprovalRequest,
    ) -> Result<ApplyApproval, ApprovalError> {
        require_admin(ctx).map_err(|_| ApprovalError::ForbiddenRole {
            role: ctx.role.as_str().to_string(),
        })?;
        if ctx.role == OperatorRole::Viewer {
            return Err(ApprovalError::ForbiddenRole {
                role: "viewer".into(),
            });
        }

        request.approved_by = ctx.operator_id.clone();
        request.approver_role = ctx.role.as_str().to_string();
        request.request_id = request_id.to_string();

        let approval = self.store.create_approval(request).await?;

        let event = audit::append(
            self.pool,
            Some(&ctx.operator_id),
            "APPLY_APPROVAL_CREATED",
            Some(&approval.policy_id),
            Some(json!({
                "request_id": request_id,
                "approval_id": approval.approval_id,
                "dry_run_id": approval.dry_run_id,
                "execution_hash": approval.execution_hash,
                "policy_version": approval.policy_version,
                "operation_intent": approval.operation_intent,
                "actor": ctx.operator_id,
                "expires_at": approval.expires_at.to_rfc3339(),
            })),
        )
        .await
        .map_err(|e| ApprovalError::Database(e.to_string()))?;

        // Persist audit reference if not already set.
        if approval.audit_reference.is_none() {
            let _ =
                sqlx::query("UPDATE apply_approvals SET audit_reference = ? WHERE approval_id = ?")
                    .bind(&event.id)
                    .bind(&approval.approval_id)
                    .execute(self.pool)
                    .await;
        }

        let mut out = approval;
        if out.audit_reference.is_none() {
            out.audit_reference = Some(event.id);
        }
        Ok(out)
    }

    pub async fn get_approval(
        &self,
        approval_id: &str,
    ) -> Result<Option<ApplyApproval>, ApprovalError> {
        let approval = self.store.get_approval(approval_id).await?;
        if let Some(ref a) = approval {
            if a.status == ApprovalStatus::Expired {
                let _ = audit::append(
                    self.pool,
                    None,
                    "APPLY_APPROVAL_EXPIRED",
                    Some(&a.policy_id),
                    Some(json!({
                        "approval_id": a.approval_id,
                        "dry_run_id": a.dry_run_id,
                        "execution_hash": a.execution_hash,
                    })),
                )
                .await;
            }
        }
        Ok(approval)
    }

    pub async fn validate_approval(
        &self,
        binding: &ApprovalBinding,
    ) -> Result<ApplyApproval, ApprovalError> {
        self.store.validate_approval(binding).await
    }

    pub async fn consume_approval(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        approval_id: &str,
        operation_id: Option<&str>,
    ) -> Result<ApplyApproval, ApprovalError> {
        require_admin(ctx).map_err(|_| ApprovalError::ForbiddenRole {
            role: ctx.role.as_str().to_string(),
        })?;

        let approval = self
            .store
            .consume_approval(approval_id, operation_id)
            .await?;

        let _ = audit::append(
            self.pool,
            Some(&ctx.operator_id),
            "APPLY_APPROVAL_CONSUMED",
            Some(&approval.policy_id),
            Some(json!({
                "request_id": request_id,
                "approval_id": approval.approval_id,
                "dry_run_id": approval.dry_run_id,
                "execution_hash": approval.execution_hash,
                "operation_id": operation_id,
                "actor": ctx.operator_id,
            })),
        )
        .await;

        Ok(approval)
    }

    pub async fn cancel_approval(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        approval_id: &str,
    ) -> Result<ApplyApproval, ApprovalError> {
        require_admin(ctx).map_err(|_| ApprovalError::ForbiddenRole {
            role: ctx.role.as_str().to_string(),
        })?;

        let approval = self
            .store
            .cancel_approval(approval_id, &ctx.operator_id)
            .await?;

        let _ = audit::append(
            self.pool,
            Some(&ctx.operator_id),
            "APPLY_APPROVAL_CANCELLED",
            Some(&approval.policy_id),
            Some(json!({
                "request_id": request_id,
                "approval_id": approval.approval_id,
                "dry_run_id": approval.dry_run_id,
                "execution_hash": approval.execution_hash,
                "actor": ctx.operator_id,
            })),
        )
        .await;

        Ok(approval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::attestation::{AttestationStore, CreateAttestationRequest};
    use crate::db::Db;
    use crate::execution::types::{ProtocolOperationKind, SimulationOutcome};
    use chrono::{Duration, Utc};
    use serde_json::json;
    use uuid::Uuid;

    async fn setup() -> (ApprovalStore, AttestationStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("approval.db");
        let db = Db::connect(path.to_str().unwrap()).await.unwrap();
        db.migrate().await.unwrap();
        (
            ApprovalStore::new(db.pool().clone()),
            AttestationStore::new(db.pool().clone()),
            dir,
        )
    }

    fn hash_for(label: &str) -> String {
        format!("{:0<64}", hex::encode(label.as_bytes()))
            .chars()
            .take(64)
            .collect()
    }

    async fn seed_attestation(
        attest: &AttestationStore,
        suffix: &str,
        created_by: &str,
    ) -> CreateAttestationRequest {
        let req = CreateAttestationRequest {
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
            predicted_changes: json!({}),
            created_by: created_by.into(),
            audit_reference: Some(format!("audit-{suffix}")),
            ttl: None,
        };
        attest.create_attestation(req.clone()).await.unwrap();
        req
    }

    fn approval_from_attest(
        attest: &CreateAttestationRequest,
        approved_by: &str,
    ) -> CreateApprovalRequest {
        CreateApprovalRequest {
            approval_id: None,
            dry_run_id: attest.dry_run_id.clone(),
            execution_hash: attest.execution_hash.clone(),
            policy_id: attest.policy_id.clone(),
            policy_version: attest.policy_version,
            operation_intent: attest.operation_intent.clone(),
            approved_by: approved_by.into(),
            approver_role: "admin".into(),
            request_id: "req-1".into(),
            audit_reference: None,
            ttl: None,
        }
    }

    #[tokio::test]
    async fn create_and_retrieve_approval() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "ok", "operator-1").await;
        let approval = store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        assert_eq!(approval.status, ApprovalStatus::Active);

        let loaded = store
            .get_approval(&approval.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.execution_hash, a.execution_hash);
        assert_eq!(loaded.dry_run_id, a.dry_run_id);
    }

    #[tokio::test]
    async fn consume_approval() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "cons", "operator-1").await;
        let approval = store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        let consumed = store
            .consume_approval(&approval.approval_id, Some("op-xyz"))
            .await
            .unwrap();
        assert_eq!(consumed.status, ApprovalStatus::Consumed);
        assert_eq!(consumed.consumed_by_operation_id.as_deref(), Some("op-xyz"));
        assert!(store
            .consume_approval(&approval.approval_id, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn cancel_approval() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "can", "operator-1").await;
        let approval = store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        let cancelled = store
            .cancel_approval(&approval.approval_id, "admin-1")
            .await
            .unwrap();
        assert_eq!(cancelled.status, ApprovalStatus::Cancelled);
    }

    #[tokio::test]
    async fn expire_approval() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "exp", "operator-1").await;
        let mut req = approval_from_attest(&a, "admin-1");
        req.ttl = Some(Duration::seconds(1));
        let approval = store.create_approval(req).await.unwrap();

        let past = (Utc::now() - Duration::minutes(5)).to_rfc3339();
        sqlx::query("UPDATE apply_approvals SET expires_at = ? WHERE approval_id = ?")
            .bind(&past)
            .bind(&approval.approval_id)
            .execute(store.pool())
            .await
            .unwrap();

        let loaded = store
            .get_approval(&approval.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.status, ApprovalStatus::Expired);
    }

    #[tokio::test]
    async fn wrong_execution_hash_rejected() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "hash", "operator-1").await;
        let mut req = approval_from_attest(&a, "admin-1");
        req.execution_hash = hash_for("wrong");
        let err = store.create_approval(req).await.unwrap_err();
        assert_eq!(err.code(), ApprovalErrorCode::ExecutionHashMismatch);
    }

    #[tokio::test]
    async fn expired_attestation_rejected() {
        let (store, attest, _dir) = setup().await;
        let a_req = CreateAttestationRequest {
            dry_run_id: "dry-att-exp".into(),
            operation_intent: "CapabilityGrant".into(),
            policy_id: "pol-att-exp".into(),
            policy_version: 1,
            execution_hash: hash_for("att-exp"),
            simulation: SimulationOutcome {
                executable: true,
                reason: None,
            },
            protocol_operation_kind: ProtocolOperationKind::CapabilityGrant,
            predicted_changes: json!({}),
            created_by: "operator-1".into(),
            audit_reference: None,
            ttl: Some(Duration::seconds(1)),
        };
        attest.create_attestation(a_req.clone()).await.unwrap();
        let past = (Utc::now() - Duration::minutes(5)).to_rfc3339();
        sqlx::query("UPDATE dry_run_attestations SET expires_at = ? WHERE dry_run_id = ?")
            .bind(&past)
            .bind(&a_req.dry_run_id)
            .execute(attest.pool())
            .await
            .unwrap();

        let err = store
            .create_approval(approval_from_attest(&a_req, "admin-1"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), ApprovalErrorCode::AttestationExpired);
    }

    #[tokio::test]
    async fn duplicate_approval_id_rejected() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "dup", "operator-1").await;
        let id = Uuid::new_v4().to_string();
        let mut req = approval_from_attest(&a, "admin-1");
        req.approval_id = Some(id.clone());
        store.create_approval(req.clone()).await.unwrap();

        // Different dry-run needed for second create attempt with same approval_id —
        // first create consumes the active binding; use same approval_id on another attest.
        let a2 = seed_attestation(&attest, "dup2", "operator-1").await;
        let mut req2 = approval_from_attest(&a2, "admin-1");
        req2.approval_id = Some(id);
        let err = store.create_approval(req2).await.unwrap_err();
        assert_eq!(err.code(), ApprovalErrorCode::DuplicateApprovalId);
    }

    #[tokio::test]
    async fn self_approval_rejected() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "sod", "same-op").await;
        let err = store
            .create_approval(approval_from_attest(&a, "same-op"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), ApprovalErrorCode::SeparationOfDuties);
    }

    #[tokio::test]
    async fn service_viewer_cannot_approve() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("viewer.db");
        let db = Db::connect(path.to_str().unwrap()).await.unwrap();
        db.migrate().await.unwrap();
        let attest = AttestationStore::new(db.pool().clone());
        let a = seed_attestation(&attest, "view", "operator-1").await;
        let service = ApprovalService::new(db.pool());
        let ctx = AuthContext {
            operator_id: "viewer-1".into(),
            username: "viewer".into(),
            role: OperatorRole::Viewer,
        };
        let err = service
            .create_approval(&ctx, "req", approval_from_attest(&a, "viewer-1"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), ApprovalErrorCode::ForbiddenRole);
    }

    #[tokio::test]
    async fn consumed_cannot_be_reused() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "reuse", "operator-1").await;
        let approval = store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        store
            .consume_approval(&approval.approval_id, None)
            .await
            .unwrap();
        let binding = ApprovalBinding {
            approval_id: approval.approval_id.clone(),
            dry_run_id: a.dry_run_id,
            execution_hash: a.execution_hash,
            policy_version: 1,
        };
        let err = store.validate_approval(&binding).await.unwrap_err();
        assert_eq!(err.code(), ApprovalErrorCode::Consumed);
    }

    #[tokio::test]
    async fn two_active_approvals_same_binding_rejected() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "two", "operator-1").await;
        store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        let err = store
            .create_approval(approval_from_attest(&a, "admin-2"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), ApprovalErrorCode::ActiveBindingExists);
    }

    #[tokio::test]
    async fn concurrent_consume_exactly_one_wins() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "cconc", "operator-1").await;
        let approval = store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        let store2 = store.clone();
        let id = approval.approval_id.clone();
        let id2 = approval.approval_id.clone();
        let (r1, r2) = tokio::join!(
            store.consume_approval(&id, Some("op-a")),
            store2.consume_approval(&id2, Some("op-b"))
        );
        let successes = [r1, r2].into_iter().filter(|r| r.is_ok()).count();
        assert_eq!(successes, 1);
    }

    #[tokio::test]
    async fn concurrent_cancel_exactly_one_wins() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "ccan", "operator-1").await;
        let approval = store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        let store2 = store.clone();
        let id = approval.approval_id.clone();
        let id2 = approval.approval_id.clone();
        let (r1, r2) = tokio::join!(
            store.cancel_approval(&id, "admin-a"),
            store2.cancel_approval(&id2, "admin-b")
        );
        // First cancel succeeds; second may succeed as idempotent cancel of already-cancelled
        // or fail with invalid transition — at least one must report Cancelled.
        let cancelled = [r1, r2]
            .into_iter()
            .filter_map(|r| r.ok())
            .filter(|a| a.status == ApprovalStatus::Cancelled)
            .count();
        assert!(cancelled >= 1);
        let final_row = store
            .get_approval(&approval.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(final_row.status, ApprovalStatus::Cancelled);
    }

    #[tokio::test]
    async fn persistence_round_trip_and_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("restart.db");
        let db_path = path.to_str().unwrap().to_string();
        let approval_id;
        let hash;
        {
            let db = Db::connect(&db_path).await.unwrap();
            db.migrate().await.unwrap();
            let attest = AttestationStore::new(db.pool().clone());
            let store = ApprovalStore::new(db.pool().clone());
            let a = seed_attestation(&attest, "rest", "operator-1").await;
            let approval = store
                .create_approval(approval_from_attest(&a, "admin-1"))
                .await
                .unwrap();
            approval_id = approval.approval_id;
            hash = a.execution_hash;
        }
        let db = Db::connect(&db_path).await.unwrap();
        let store = ApprovalStore::new(db.pool().clone());
        let loaded = store.get_approval(&approval_id).await.unwrap().unwrap();
        assert_eq!(loaded.execution_hash, hash);
        assert_eq!(loaded.status, ApprovalStatus::Active);
    }

    #[tokio::test]
    async fn invalid_transitions_rejected() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "inv", "operator-1").await;
        let approval = store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        store
            .consume_approval(&approval.approval_id, None)
            .await
            .unwrap();
        let err = store
            .cancel_approval(&approval.approval_id, "admin-1")
            .await
            .unwrap_err();
        assert_eq!(err.code(), ApprovalErrorCode::InvalidTransition);
    }

    #[tokio::test]
    async fn validate_active_approval() {
        let (store, attest, _dir) = setup().await;
        let a = seed_attestation(&attest, "val", "operator-1").await;
        let approval = store
            .create_approval(approval_from_attest(&a, "admin-1"))
            .await
            .unwrap();
        let binding = ApprovalBinding {
            approval_id: approval.approval_id,
            dry_run_id: a.dry_run_id,
            execution_hash: a.execution_hash,
            policy_version: 1,
        };
        let ok = store.validate_approval(&binding).await.unwrap();
        assert_eq!(ok.status, ApprovalStatus::Active);
    }
}
