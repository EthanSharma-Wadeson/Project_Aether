//! Apply pre-execution validation gates (Phase 6).
//!
//! Deterministic authorisation chain. Does **not** mutate protocol state,
//! consume approvals, reserve replay, or enable Apply.

pub mod errors;
pub mod gates;
pub mod model;
pub mod pipeline;

pub use errors::{ValidationError, ValidationErrorCode};
pub use gates::ValidationContext;
pub use model::{ApplyValidationRequest, GateId, GateResult, ValidationResult};
pub use pipeline::validate_apply_request;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::apply_enabled;
    use crate::apply::approval::{ApprovalStatus, ApprovalStore, CreateApprovalRequest};
    use crate::apply::attestation::{AttestationStore, CreateAttestationRequest};
    use crate::apply::replay::{self, ReplayStore};
    use crate::apply::signature::{prepare_signature, PrepareSignatureRequest, SignatureStore};
    use crate::auth::middleware::AuthContext;
    use crate::config::SignerConfig;
    use crate::db::operators::{self, OperatorRole};
    use crate::db::policies as policy_db;
    use crate::db::Db;
    use crate::execution::hash::execution_hash_for_policy;
    use crate::execution::types::{ProtocolOperationKind, SimulationOutcome};
    use crate::models::policies::PolicyStatus;
    use crate::policies::service::compute_policy_hash;
    use crate::signer::build_signing_gateway_with;
    use chrono::{Duration, Utc};
    use serde_json::json;
    use sqlx::SqlitePool;

    struct Fixture {
        _dir: tempfile::TempDir,
        pool: SqlitePool,
        approvals: ApprovalStore,
        attestations: AttestationStore,
        signatures: SignatureStore,
        signing: crate::signer::SharedSigningGateway,
        replay: ReplayStore,
        operation_id: String,
        approval_id: String,
        dry_run_id: String,
        execution_hash: String,
        policy_id: String,
        actor: AuthContext,
    }

    async fn setup(suffix: &str) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("val.db");
        let db = Db::connect(path.to_str().unwrap()).await.unwrap();
        db.migrate().await.unwrap();
        let pool = db.pool().clone();

        let op = operators::create_operator(
            &pool,
            &format!("op-{suffix}"),
            "password-ok-12",
            OperatorRole::Operator,
        )
        .await
        .unwrap();
        let admin = operators::create_operator(
            &pool,
            &format!("adm-{suffix}"),
            "password-ok-12",
            OperatorRole::Admin,
        )
        .await
        .unwrap();

        let policy_id = format!("pol-{suffix}");
        let policy_data = json!({"actions": ["pay"], "scope": "read"});
        let hash = compute_policy_hash(
            "grant-demo",
            "d",
            Some("agent-1"),
            "capability_grant",
            &policy_data,
            1,
        );
        policy_db::insert_policy(
            &pool,
            &policy_id,
            "grant-demo",
            "d",
            Some("agent-1"),
            "capability_grant",
            &policy_data,
            &op.id,
            1,
            &hash,
        )
        .await
        .unwrap();

        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE policy_templates SET
                status = 'approved',
                approved_by = ?,
                approved_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&admin.id)
        .bind(&now)
        .bind(&now)
        .bind(&policy_id)
        .execute(&pool)
        .await
        .unwrap();

        let policy = policy_db::get_policy(&pool, &policy_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(policy.status, PolicyStatus::Approved);

        let execution_hash =
            execution_hash_for_policy(&policy, ProtocolOperationKind::CapabilityGrant);
        assert_eq!(execution_hash.len(), 64);

        let signing = build_signing_gateway_with(
            &SignerConfig {
                mode: "enterprise".into(),
                identity: "enterprise-default".into(),
                seed_hex: Some(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
                ),
                ephemeral: false,
            },
            &db,
        )
        .unwrap();

        let attestations = AttestationStore::new(pool.clone());
        let approvals = ApprovalStore::new(pool.clone());
        let signatures = SignatureStore::new(pool.clone());
        let replay = ReplayStore::new(pool.clone());

        let dry_run_id = format!("dry-{suffix}");
        attestations
            .create_attestation(CreateAttestationRequest {
                dry_run_id: dry_run_id.clone(),
                operation_intent: "CapabilityGrant".into(),
                policy_id: policy_id.clone(),
                policy_version: 1,
                execution_hash: execution_hash.clone(),
                simulation: SimulationOutcome {
                    executable: true,
                    reason: None,
                },
                protocol_operation_kind: ProtocolOperationKind::CapabilityGrant,
                predicted_changes: json!({}),
                created_by: op.id.clone(),
                audit_reference: None,
                ttl: None,
            })
            .await
            .unwrap();

        let approval = approvals
            .create_approval(CreateApprovalRequest {
                approval_id: None,
                dry_run_id: dry_run_id.clone(),
                execution_hash: execution_hash.clone(),
                policy_id: policy_id.clone(),
                policy_version: 1,
                operation_intent: "CapabilityGrant".into(),
                approved_by: admin.id.clone(),
                approver_role: "admin".into(),
                request_id: format!("req-appr-{suffix}"),
                audit_reference: None,
                ttl: None,
            })
            .await
            .unwrap();

        let signed = prepare_signature(
            &signatures,
            &approvals,
            &signing,
            PrepareSignatureRequest {
                operation_id: Some(format!("op-{suffix}")),
                approval_id: approval.approval_id.clone(),
                dry_run_id: dry_run_id.clone(),
                execution_hash: execution_hash.clone(),
                policy_id: policy_id.clone(),
                policy_version: 1,
                operation_intent: "CapabilityGrant".into(),
                request_id: format!("req-sign-{suffix}"),
                signer_id: op.id.clone(),
                signer_role: "operator".into(),
                target_agent: Some("agent-1".into()),
                purpose: "apply".into(),
                ttl: None,
            },
        )
        .await
        .unwrap();

        Fixture {
            _dir: dir,
            pool,
            approvals,
            attestations,
            signatures,
            signing,
            replay,
            operation_id: signed.operation_id,
            approval_id: approval.approval_id,
            dry_run_id,
            execution_hash,
            policy_id,
            actor: AuthContext {
                operator_id: op.id,
                username: format!("op-{suffix}"),
                role: OperatorRole::Operator,
            },
        }
    }

    fn ctx_from(fx: &Fixture) -> ValidationContext<'_> {
        ValidationContext {
            pool: &fx.pool,
            approvals: &fx.approvals,
            attestations: &fx.attestations,
            signatures: &fx.signatures,
            signing: &fx.signing,
        }
    }

    fn good_request(fx: &Fixture) -> ApplyValidationRequest {
        ApplyValidationRequest {
            request_id: "req-val".into(),
            operation_id: fx.operation_id.clone(),
            approval_id: fx.approval_id.clone(),
            dry_run_id: fx.dry_run_id.clone(),
            execution_hash: fx.execution_hash.clone(),
            authenticated: true,
            jwt_valid: true,
            csrf_valid: true,
            actor: fx.actor.clone(),
        }
    }

    #[tokio::test]
    async fn full_validation_success_requires_resimulation() {
        let fx = setup("ok").await;
        assert!(!apply_enabled());

        let result = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap();

        assert!(result.approved);
        assert_eq!(
            result.decision_code.as_deref(),
            Some(ValidationErrorCode::RequiresResimulation.as_str())
        );
        assert_eq!(result.gate_results.len(), 14);
        assert!(result.gate_results.iter().all(|g| g.passed));
        assert_eq!(
            result
                .gate_results
                .iter()
                .map(|g| g.gate_id.as_str())
                .collect::<Vec<_>>(),
            GateId::ORDER.iter().map(|g| g.as_str()).collect::<Vec<_>>()
        );
        // INV-V3: approval still active
        let appr = fx
            .approvals
            .get_approval(&fx.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(appr.status, ApprovalStatus::Active);
        // No replay reservation
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn gate_order_stable_on_early_failure() {
        let fx = setup("order").await;
        let mut req = good_request(&fx);
        req.authenticated = false;
        let err = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G1);
        assert_eq!(err.partial.gate_results.len(), 1);
        assert_eq!(err.partial.gate_results[0].gate_id, "G1");
        assert!(!err.partial.gate_results[0].passed);
    }

    #[tokio::test]
    async fn determinism_same_input_same_decision() {
        let fx = setup("det").await;
        let a = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap();
        let b = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap();
        assert_eq!(a.approved, b.approved);
        assert_eq!(a.decision_code, b.decision_code);
        assert_eq!(
            a.gate_results
                .iter()
                .map(|g| (g.gate_id.clone(), g.passed, g.failure_code.clone()))
                .collect::<Vec<_>>(),
            b.gate_results
                .iter()
                .map(|g| (g.gate_id.clone(), g.passed, g.failure_code.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn g1_unauthenticated_rejected() {
        let fx = setup("g1").await;
        let mut req = good_request(&fx);
        req.authenticated = false;
        let err = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        assert_eq!(err.code, ValidationErrorCode::Unauthenticated);
        assert_eq!(err.gate, GateId::G1);
    }

    #[tokio::test]
    async fn g2_jwt_invalid_rejected() {
        let fx = setup("g2").await;
        let mut req = good_request(&fx);
        req.jwt_valid = false;
        let err = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        assert_eq!(err.code, ValidationErrorCode::JwtInvalid);
        assert_eq!(err.gate, GateId::G2);
    }

    #[tokio::test]
    async fn g3_viewer_rejected() {
        let fx = setup("g3").await;
        let mut req = good_request(&fx);
        req.actor.role = OperatorRole::Viewer;
        let err = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        assert_eq!(err.code, ValidationErrorCode::RbacDenied);
        assert_eq!(err.gate, GateId::G3);
    }

    #[tokio::test]
    async fn g4_csrf_rejected() {
        let fx = setup("g4").await;
        let mut req = good_request(&fx);
        req.csrf_valid = false;
        let err = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        assert_eq!(err.code, ValidationErrorCode::CsrfInvalid);
        assert_eq!(err.gate, GateId::G4);
    }

    #[tokio::test]
    async fn g5_passes_while_apply_disabled() {
        let fx = setup("g5").await;
        assert!(!apply_enabled());
        let result = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap();
        assert!(result
            .gate_results
            .iter()
            .any(|g| g.gate_id == "G5" && g.passed));
        assert!(result.approved);
    }

    #[tokio::test]
    async fn g6_missing_approval_rejected() {
        let fx = setup("g6").await;
        let mut req = good_request(&fx);
        req.approval_id = "missing-appr".into();
        let err = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        assert_eq!(err.code, ValidationErrorCode::ApprovalMissing);
        assert_eq!(err.gate, GateId::G6);
    }

    #[tokio::test]
    async fn expired_approval_rejected() {
        let fx = setup("exp-appr").await;
        sqlx::query("UPDATE apply_approvals SET expires_at = ? WHERE approval_id = ?")
            .bind((Utc::now() - Duration::hours(1)).to_rfc3339())
            .bind(&fx.approval_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G7);
        assert_eq!(err.code, ValidationErrorCode::ApprovalExpired);
        let appr = fx
            .approvals
            .get_approval(&fx.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(appr.status, ApprovalStatus::Consumed);
    }

    #[tokio::test]
    async fn consumed_approval_rejected_without_reconsume() {
        let fx = setup("cons").await;
        fx.approvals
            .consume_approval(&fx.approval_id, Some(&fx.operation_id))
            .await
            .unwrap();
        let err = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G7);
        assert_eq!(err.code, ValidationErrorCode::ApprovalConsumed);
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn expired_attestation_rejected() {
        let fx = setup("exp-att").await;
        sqlx::query("UPDATE dry_run_attestations SET expires_at = ? WHERE dry_run_id = ?")
            .bind((Utc::now() - Duration::hours(1)).to_rfc3339())
            .bind(&fx.dry_run_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G8);
        assert_eq!(err.code, ValidationErrorCode::AttestationExpired);
    }

    #[tokio::test]
    async fn hash_mismatch_rejected() {
        let fx = setup("hash").await;
        let mut req = good_request(&fx);
        req.execution_hash = format!("{:0>64}", "deadbeef");
        let err = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        // G7 binding check or G8/G9 depending on validate_approval hash check
        assert!(matches!(err.gate, GateId::G7 | GateId::G8 | GateId::G9));
        let appr = fx
            .approvals
            .get_approval(&fx.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(appr.status, ApprovalStatus::Active);
    }

    #[tokio::test]
    async fn missing_signed_operation_rejected() {
        let fx = setup("nosig").await;
        let mut req = good_request(&fx);
        req.operation_id = "missing-op".into();
        let err = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G10);
        assert_eq!(err.code, ValidationErrorCode::SignedOperationMissing);
    }

    #[tokio::test]
    async fn invalid_signature_rejected() {
        let fx = setup("badsig").await;
        sqlx::query("UPDATE signed_operations SET signature = ? WHERE operation_id = ?")
            .bind("00".repeat(64))
            .bind(&fx.operation_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G11);
        assert_eq!(err.code, ValidationErrorCode::SignatureInvalid);
    }

    #[tokio::test]
    async fn expired_signature_rejected() {
        let fx = setup("expsig").await;
        sqlx::query("UPDATE signed_operations SET expires_at = ? WHERE operation_id = ?")
            .bind((Utc::now() - Duration::minutes(1)).to_rfc3339())
            .bind(&fx.operation_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        // Lazy expiry on get may surface at G9/G10/G12
        assert!(matches!(err.gate, GateId::G9 | GateId::G10 | GateId::G12));
    }

    #[tokio::test]
    async fn policy_version_change_rejected() {
        let fx = setup("polver").await;
        sqlx::query("UPDATE policy_templates SET version = 2 WHERE id = ?")
            .bind(&fx.policy_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G13);
        assert_eq!(err.code, ValidationErrorCode::PolicyVersionMismatch);
    }

    #[tokio::test]
    async fn policy_content_change_rejected() {
        let fx = setup("polhash").await;
        let new_data = json!({"actions": ["pay", "refund"], "scope": "write"});
        let new_hash = compute_policy_hash(
            "grant-demo",
            "d",
            Some("agent-1"),
            "capability_grant",
            &new_data,
            1,
        );
        sqlx::query("UPDATE policy_templates SET policy_data = ?, hash = ? WHERE id = ?")
            .bind(new_data.to_string())
            .bind(&new_hash)
            .bind(&fx.policy_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G13);
        assert!(matches!(
            err.code,
            ValidationErrorCode::PolicyHashChanged | ValidationErrorCode::StaleExecutionHash
        ));
    }

    #[tokio::test]
    async fn archived_policy_rejected() {
        let fx = setup("arch").await;
        policy_db::set_archived(&fx.pool, &fx.policy_id)
            .await
            .unwrap();
        let err = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert_eq!(err.gate, GateId::G13);
        assert_eq!(err.code, ValidationErrorCode::PolicyArchived);
    }

    #[tokio::test]
    async fn failed_validation_does_not_reserve_replay() {
        let fx = setup("noreplay").await;
        let mut req = good_request(&fx);
        req.csrf_valid = false;
        let _ = validate_apply_request(&ctx_from(&fx), req)
            .await
            .unwrap_err();
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn validation_does_not_call_proto0_write() {
        // Structural: ValidationContext has no protocol engine / proto0 handle.
        // Success path still leaves apply disabled and no replay row.
        let fx = setup("noproto").await;
        let result = validate_apply_request(&ctx_from(&fx), good_request(&fx))
            .await
            .unwrap();
        assert!(result.approved);
        assert!(!apply_enabled());
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }
}
