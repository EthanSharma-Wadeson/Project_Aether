//! Mandatory re-simulation — final pre-execution safety barrier (Phase 7).
//!
//! Proves dry-run assumptions still hold against current policy + protocol state.
//! Does **not** mutate protocol state, reserve replay, consume approvals, or
//! enable Apply.

pub mod compare;
pub mod engine;
pub mod errors;
pub mod model;

pub use engine::{resimulate_apply_intent, ResimulationContext};
pub use errors::{ResimulationError, ResimulationErrorCode};
pub use model::{
    ComparisonSnapshot, ResimulationDecisionCode, ResimulationRequest, ResimulationResult,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::apply_enabled;
    use crate::apply::approval::{ApprovalStatus, ApprovalStore, CreateApprovalRequest};
    use crate::apply::attestation::{AttestationStore, CreateAttestationRequest};
    use crate::apply::replay::{self, ReplayStore};
    use crate::apply::signature::{prepare_signature, PrepareSignatureRequest, SignatureStore};
    use crate::config::SignerConfig;
    use crate::db::operators::{self, OperatorRole};
    use crate::db::policies as policy_db;
    use crate::db::Db;
    use crate::execution::hash::execution_hash_for_policy;
    use crate::execution::simulate::{protocol_observation_fingerprint, simulate_policy_apply};
    use crate::execution::types::{ProtocolOperationKind, SimulationOutcome};
    use crate::models::policies::PolicyStatus;
    use crate::policies::service::compute_policy_hash;
    use crate::protocol::state::ProtocolState;
    use crate::signer::build_signing_gateway_with;
    use serde_json::{json, Value};
    use sqlx::SqlitePool;
    use std::sync::Arc;

    struct Fixture {
        _dir: tempfile::TempDir,
        pool: SqlitePool,
        protocol: Arc<ProtocolState>,
        approvals: ApprovalStore,
        attestations: AttestationStore,
        signatures: SignatureStore,
        replay: ReplayStore,
        operation_id: String,
        approval_id: String,
        dry_run_id: String,
        execution_hash: String,
        policy_id: String,
        policy_hash: String,
        target_agent: String,
        actor_id: String,
        fingerprint: String,
    }

    async fn setup(suffix: &str) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("resim.db");
        let db = Db::connect(path.to_str().unwrap()).await.unwrap();
        db.migrate().await.unwrap();
        let pool = db.pool().clone();

        let protocol = ProtocolState::bootstrap().unwrap();
        let fingerprint = protocol_observation_fingerprint(&protocol);
        let target_agent = protocol.index.agent_ids[0].clone();

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
        let policy_hash = compute_policy_hash(
            "grant-demo",
            "d",
            Some(&target_agent),
            "capability_grant",
            &policy_data,
            1,
        );
        policy_db::insert_policy(
            &pool,
            &policy_id,
            "grant-demo",
            "d",
            Some(&target_agent),
            "capability_grant",
            &policy_data,
            &op.id,
            1,
            &policy_hash,
        )
        .await
        .unwrap();

        let now = chrono::Utc::now().to_rfc3339();
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

        let sim = simulate_policy_apply(&protocol, &policy);
        assert!(
            sim.executable,
            "fixture policy must simulate cleanly: {:?}",
            sim.reason
        );

        let execution_hash =
            execution_hash_for_policy(&policy, ProtocolOperationKind::CapabilityGrant);

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
        let predicted_changes = json!({
            "operation_id": format!("op-{suffix}"),
            "policy_id": policy_id,
            "policy_version": 1,
            "policy_hash": policy_hash,
            "predicted_protocol_operation": "CapabilityGrant",
            "operation_intent": "CapabilityGrant",
            "target_agent": target_agent,
            "capability_changes": {
                "actions": ["pay"],
                "constraints": null,
                "limits": null,
                "capability_id": null,
            },
            "blocking_errors": [],
            "warnings": [],
        });

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
                predicted_changes,
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
                target_agent: Some(target_agent.clone()),
                purpose: "apply".into(),
                ttl: None,
            },
        )
        .await
        .unwrap();

        Fixture {
            _dir: dir,
            pool,
            protocol,
            approvals,
            attestations,
            signatures,
            replay,
            operation_id: signed.operation_id,
            approval_id: approval.approval_id,
            dry_run_id,
            execution_hash,
            policy_id,
            policy_hash,
            target_agent,
            actor_id: op.id,
            fingerprint,
        }
    }

    fn ctx_from(fx: &Fixture) -> ResimulationContext<'_> {
        ResimulationContext {
            pool: &fx.pool,
            protocol: &fx.protocol,
            approvals: &fx.approvals,
            attestations: &fx.attestations,
            signatures: &fx.signatures,
        }
    }

    fn req(fx: &Fixture) -> ResimulationRequest {
        ResimulationRequest {
            request_id: "req-resim".into(),
            operation_id: fx.operation_id.clone(),
            actor_id: fx.actor_id.clone(),
        }
    }

    #[tokio::test]
    async fn unchanged_policy_and_state_passes() {
        let fx = setup("ok").await;
        assert!(!apply_enabled());

        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();

        assert!(result.approved);
        assert!(result.changes_match);
        assert_eq!(
            result.decision_code,
            ResimulationDecisionCode::Approved.as_str()
        );
        assert_eq!(result.execution_hash, fx.execution_hash);
        assert_eq!(result.comparison_hash.len(), 64);
        assert_eq!(
            protocol_observation_fingerprint(&fx.protocol),
            fx.fingerprint
        );
        let appr = fx
            .approvals
            .get_approval(&fx.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(appr.status, ApprovalStatus::Active);
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn matching_hash_passes() {
        let fx = setup("hashok").await;
        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert_eq!(result.execution_hash, fx.execution_hash);
        assert!(result.approved);
    }

    #[tokio::test]
    async fn determinism_same_state_same_comparison() {
        let fx = setup("det").await;
        let a = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        let b = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert_eq!(a.approved, b.approved);
        assert_eq!(a.decision_code, b.decision_code);
        assert_eq!(a.comparison_hash, b.comparison_hash);
        assert_eq!(a.execution_hash, b.execution_hash);
        assert_eq!(a.changes_match, b.changes_match);
    }

    #[tokio::test]
    async fn policy_version_changed_rejected() {
        let fx = setup("polver").await;
        sqlx::query("UPDATE policy_templates SET version = 2 WHERE id = ?")
            .bind(&fx.policy_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(!result.approved);
        assert_eq!(
            result.decision_code,
            ResimulationDecisionCode::PolicyChanged.as_str()
        );
    }

    #[tokio::test]
    async fn policy_hash_changed_rejected() {
        let fx = setup("polhash").await;
        let new_data = json!({"actions": ["pay", "refund"], "scope": "write"});
        let new_hash = compute_policy_hash(
            "grant-demo",
            "d",
            Some(&fx.target_agent),
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
        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(!result.approved);
        assert_eq!(
            result.decision_code,
            ResimulationDecisionCode::PolicyChanged.as_str()
        );
        assert_ne!(new_hash, fx.policy_hash);
    }

    #[tokio::test]
    async fn execution_hash_changed_rejected() {
        let fx = setup("exhash").await;
        let bogus = format!("{:0>64}", "abad1dea");
        sqlx::query("UPDATE dry_run_attestations SET execution_hash = ? WHERE dry_run_id = ?")
            .bind(&bogus)
            .bind(&fx.dry_run_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE apply_approvals SET execution_hash = ? WHERE approval_id = ?")
            .bind(&bogus)
            .bind(&fx.approval_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE signed_operations SET execution_hash = ? WHERE operation_id = ?")
            .bind(&bogus)
            .bind(&fx.operation_id)
            .execute(&fx.pool)
            .await
            .unwrap();

        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(!result.approved);
        assert_eq!(
            result.decision_code,
            ResimulationDecisionCode::HashMismatch.as_str()
        );
    }

    #[tokio::test]
    async fn predicted_changes_changed_rejected() {
        let fx = setup("pred").await;
        let mut row: (String,) = sqlx::query_as(
            "SELECT predicted_changes FROM dry_run_attestations WHERE dry_run_id = ?",
        )
        .bind(&fx.dry_run_id)
        .fetch_one(&fx.pool)
        .await
        .unwrap();
        let mut predicted: Value = serde_json::from_str(&row.0).unwrap();
        predicted["capability_changes"]["actions"] = json!(["pay", "escalate"]);
        row.0 = predicted.to_string();
        sqlx::query("UPDATE dry_run_attestations SET predicted_changes = ? WHERE dry_run_id = ?")
            .bind(&row.0)
            .bind(&fx.dry_run_id)
            .execute(&fx.pool)
            .await
            .unwrap();

        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(!result.approved);
        assert_eq!(
            result.decision_code,
            ResimulationDecisionCode::StateChanged.as_str()
        );
    }

    #[tokio::test]
    async fn target_identity_changed_rejected() {
        let fx = setup("tgt").await;
        let other = fx
            .protocol
            .index
            .agent_ids
            .get(1)
            .cloned()
            .unwrap_or_else(|| "other-agent".into());
        let mut predicted: Value = sqlx::query_as::<_, (String,)>(
            "SELECT predicted_changes FROM dry_run_attestations WHERE dry_run_id = ?",
        )
        .bind(&fx.dry_run_id)
        .fetch_one(&fx.pool)
        .await
        .map(|(s,)| serde_json::from_str(&s).unwrap())
        .unwrap();
        predicted["target_agent"] = json!(other);
        sqlx::query("UPDATE dry_run_attestations SET predicted_changes = ? WHERE dry_run_id = ?")
            .bind(predicted.to_string())
            .bind(&fx.dry_run_id)
            .execute(&fx.pool)
            .await
            .unwrap();

        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(!result.approved);
        assert_eq!(
            result.decision_code,
            ResimulationDecisionCode::StateChanged.as_str()
        );
    }

    #[tokio::test]
    async fn protocol_operation_changed_rejected() {
        let fx = setup("kind").await;
        let mut predicted: Value = sqlx::query_as::<_, (String,)>(
            "SELECT predicted_changes FROM dry_run_attestations WHERE dry_run_id = ?",
        )
        .bind(&fx.dry_run_id)
        .fetch_one(&fx.pool)
        .await
        .map(|(s,)| serde_json::from_str(&s).unwrap())
        .unwrap();
        predicted["predicted_protocol_operation"] = json!("FreezeIdentity");
        predicted["operation_intent"] = json!("FreezeIdentity");
        sqlx::query(
            r#"
            UPDATE dry_run_attestations SET
                operation_intent = ?,
                protocol_operation_kind = ?,
                predicted_changes = ?
            WHERE dry_run_id = ?
            "#,
        )
        .bind("FreezeIdentity")
        .bind("FreezeIdentity")
        .bind(predicted.to_string())
        .bind(&fx.dry_run_id)
        .execute(&fx.pool)
        .await
        .unwrap();

        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(!result.approved);
        assert_eq!(
            result.decision_code,
            ResimulationDecisionCode::StateChanged.as_str()
        );
    }

    #[tokio::test]
    async fn failed_resimulation_does_not_consume_approval() {
        let fx = setup("noconsume").await;
        sqlx::query("UPDATE policy_templates SET version = 9 WHERE id = ?")
            .bind(&fx.policy_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(!result.approved);
        let appr = fx
            .approvals
            .get_approval(&fx.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(appr.status, ApprovalStatus::Active);
    }

    #[tokio::test]
    async fn failed_resimulation_does_not_reserve_replay() {
        let fx = setup("noreplay").await;
        sqlx::query("UPDATE policy_templates SET version = 9 WHERE id = ?")
            .bind(&fx.policy_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let _ = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn no_proto0_write_and_apply_still_disabled() {
        let fx = setup("noproto").await;
        let before = fx.fingerprint.clone();
        let result = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        assert!(result.approved);
        assert!(!apply_enabled());
        assert_eq!(protocol_observation_fingerprint(&fx.protocol), before);
        // Structural: ResimulationContext has no proto0_write / SigningGateway.
    }

    #[tokio::test]
    async fn attestation_remains_immutable() {
        let fx = setup("immut").await;
        let before: String = sqlx::query_scalar(
            "SELECT predicted_changes FROM dry_run_attestations WHERE dry_run_id = ?",
        )
        .bind(&fx.dry_run_id)
        .fetch_one(&fx.pool)
        .await
        .unwrap();
        let _ = resimulate_apply_intent(&ctx_from(&fx), req(&fx))
            .await
            .unwrap();
        let after: String = sqlx::query_scalar(
            "SELECT predicted_changes FROM dry_run_attestations WHERE dry_run_id = ?",
        )
        .bind(&fx.dry_run_id)
        .fetch_one(&fx.pool)
        .await
        .unwrap();
        assert_eq!(before, after);
    }
}
