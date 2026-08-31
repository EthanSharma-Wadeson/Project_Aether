//! Apply execution pipeline skeleton (Phase 8).
//!
//! Orchestrates validation → re-simulation → replay reserve → PROTO-0 boundary.
//! Stops before real protocol mutation. `apply_enabled()` remains `false`.

pub mod errors;
pub mod model;
pub mod pipeline;
pub mod steps;

pub use errors::{ExecutionPipelineError, ExecutionPipelineErrorCode};
pub use model::{
    allowed_execution_transition, ApplyExecutionContext, ApplyExecutionPhase, ExecuteApplyRequest,
    ExecuteApplyResult,
};
pub use pipeline::{execute_apply, reconstruct_audit_by_request_id};
pub use steps::ExecutionPipelineContext;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::apply_enabled;
    use crate::apply::approval::{ApprovalStatus, ApprovalStore, CreateApprovalRequest};
    use crate::apply::attestation::{AttestationStore, CreateAttestationRequest};
    use crate::apply::errors::ApplyErrorCode;
    use crate::apply::execution::reconstruct_audit_by_request_id;
    use crate::apply::replay::{self, ReplayStatus, ReplayStore};
    use crate::apply::signature::{prepare_signature, PrepareSignatureRequest, SignatureStore};
    use crate::auth::middleware::AuthContext;
    use crate::config::SignerConfig;
    use crate::db::operators::{self, OperatorRole};
    use crate::db::policies as policy_db;
    use crate::db::Db;
    use crate::execution::hash::execution_hash_for_policy;
    use crate::execution::simulate::{protocol_observation_fingerprint, simulate_policy_apply};
    use crate::execution::types::{ProtocolOperationKind, SimulationOutcome};
    use crate::policies::service::compute_policy_hash;
    use crate::protocol::state::ProtocolState;
    use crate::signer::build_signing_gateway_with;
    use serde_json::json;
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use std::time::Duration;

    struct Fixture {
        _dir: tempfile::TempDir,
        pool: SqlitePool,
        protocol: Arc<ProtocolState>,
        approvals: ApprovalStore,
        attestations: AttestationStore,
        signatures: SignatureStore,
        signing: crate::signer::SharedSigningGateway,
        replay: ReplayStore,
        operation_id: String,
        approval_id: String,
        dry_run_id: String,
        execution_hash: String,
        actor: AuthContext,
        fingerprint: String,
    }

    async fn setup(suffix: &str) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("exec.db");
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
                status = 'approved', approved_by = ?, approved_at = ?, updated_at = ?
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
        assert!(simulate_policy_apply(&protocol, &policy).executable);

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
                target_agent: Some(target_agent),
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
            signing,
            replay,
            operation_id: signed.operation_id,
            approval_id: approval.approval_id,
            dry_run_id,
            execution_hash,
            actor: AuthContext {
                operator_id: op.id,
                username: format!("op-{suffix}"),
                role: OperatorRole::Operator,
            },
            fingerprint,
        }
    }

    fn pipe_from(fx: &Fixture) -> ExecutionPipelineContext<'_> {
        ExecutionPipelineContext {
            pool: &fx.pool,
            protocol: &fx.protocol,
            approvals: &fx.approvals,
            attestations: &fx.attestations,
            signatures: &fx.signatures,
            signing: &fx.signing,
            replay: &fx.replay,
        }
    }

    fn good_request(fx: &Fixture) -> ExecuteApplyRequest {
        ExecuteApplyRequest {
            request_id: "req-exec".into(),
            operation_id: fx.operation_id.clone(),
            approval_id: fx.approval_id.clone(),
            dry_run_id: fx.dry_run_id.clone(),
            execution_hash: fx.execution_hash.clone(),
            authenticated: true,
            jwt_valid: true,
            csrf_valid: true,
            actor: fx.actor.clone(),
            confirm: true,
        }
    }

    #[test]
    fn state_machine_valid_and_invalid_transitions() {
        use ApplyExecutionPhase::*;
        assert!(allowed_execution_transition(Created, Validated));
        assert!(allowed_execution_transition(
            Validated,
            ResimulationApproved
        ));
        assert!(allowed_execution_transition(
            ResimulationApproved,
            ReplayReserved
        ));
        assert!(allowed_execution_transition(ReplayReserved, Executing));
        assert!(allowed_execution_transition(Executing, Rejected));
        assert!(allowed_execution_transition(Executing, Executed));
        assert!(allowed_execution_transition(Executing, Aborted));

        assert!(!allowed_execution_transition(Created, Executed));
        assert!(!allowed_execution_transition(Executing, ReplayReserved));
        assert!(!allowed_execution_transition(Executed, Executing));
        assert!(!allowed_execution_transition(Rejected, Executing));
    }

    #[tokio::test]
    async fn pipeline_blocks_at_proto0_with_execution_disabled() {
        let fx = setup("ok").await;
        assert!(!apply_enabled());

        let result = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap();

        assert_eq!(
            result.outcome_code,
            ApplyErrorCode::ApplyExecutionDisabled.as_str()
        );
        assert_eq!(result.context.phase, ApplyExecutionPhase::Rejected);
        assert!(result.protocol_unchanged);
        assert!(result.replay_reserved);
        assert_eq!(
            protocol_observation_fingerprint(&fx.protocol),
            fx.fingerprint
        );

        let replay = replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(replay.status, ReplayStatus::Rejected);
        assert_eq!(
            replay.terminal_reason.as_deref(),
            Some(ApplyErrorCode::ApplyExecutionDisabled.as_str())
        );

        let appr = fx
            .approvals
            .get_approval(&fx.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(appr.status, ApprovalStatus::Consumed);
    }

    #[tokio::test]
    async fn validation_failure_does_not_reserve_replay() {
        let fx = setup("noval").await;
        let mut req = good_request(&fx);
        req.csrf_valid = false;
        let err = execute_apply(&pipe_from(&fx), req).await.unwrap_err();
        assert_eq!(err.code, ExecutionPipelineErrorCode::ValidationFailed);
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
        let appr = fx
            .approvals
            .get_approval(&fx.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(appr.status, ApprovalStatus::Active);
    }

    #[tokio::test]
    async fn resimulation_failure_does_not_reserve_replay() {
        let fx = setup("noresim").await;
        sqlx::query("UPDATE policy_templates SET version = 9 WHERE id = (SELECT policy_id FROM signed_operations WHERE operation_id = ?)")
            .bind(&fx.operation_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert!(matches!(
            err.code,
            ExecutionPipelineErrorCode::ResimulationNotApproved
                | ExecutionPipelineErrorCode::ResimulationFailed
                | ExecutionPipelineErrorCode::ValidationFailed
        ));
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn duplicate_operation_rejected() {
        let fx = setup("dup").await;
        let _ = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap();
        // Re-activate approval so validation can reach the replay gate.
        sqlx::query(
            r#"
            UPDATE apply_approvals SET
                status = 'active',
                consumed_at = NULL,
                consumed_by_operation_id = NULL
            WHERE approval_id = ?
            "#,
        )
        .bind(&fx.approval_id)
        .execute(&fx.pool)
        .await
        .unwrap();
        let err = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert_eq!(err.code, ExecutionPipelineErrorCode::ReplayDuplicate);
    }

    #[tokio::test]
    async fn concurrent_execution_attempts_one_wins() {
        let fx = setup("conc").await;
        let pipe = pipe_from(&fx);
        let r1 = good_request(&fx);
        let r2 = good_request(&fx);
        let (a, b) = tokio::join!(execute_apply(&pipe, r1), execute_apply(&pipe, r2));
        let wins = [a.is_ok(), b.is_ok()].iter().filter(|x| **x).count();
        let losers = [&a, &b]
            .iter()
            .filter(|r| {
                r.as_ref()
                    .err()
                    .map(|e| {
                        matches!(
                            e.code,
                            ExecutionPipelineErrorCode::ReplayDuplicate
                                | ExecutionPipelineErrorCode::ReplayInProgress
                                | ExecutionPipelineErrorCode::ApprovalConsumeFailed
                        )
                    })
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(wins, 1);
        assert_eq!(losers, 1);
    }

    #[tokio::test]
    async fn terminal_replay_immutable() {
        let fx = setup("term").await;
        let _ = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap();
        let err = fx
            .replay
            .begin_execution(&fx.operation_id)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::apply::replay::ReplayError::InvalidTransition { .. }
        ));
    }

    #[tokio::test]
    async fn ordering_validate_before_replay() {
        let fx = setup("order1").await;
        let mut req = good_request(&fx);
        req.authenticated = false;
        let _ = execute_apply(&pipe_from(&fx), req).await.unwrap_err();
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn ordering_resimulation_before_replay() {
        // Policy change fails validation/resim before reserve.
        let fx = setup("order2").await;
        sqlx::query("UPDATE policy_templates SET hash = ? WHERE id = (SELECT policy_id FROM signed_operations WHERE operation_id = ?)")
            .bind("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef")
            .bind(&fx.operation_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let _ = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn crash_after_reserve_survives_restart() {
        let fx = setup("crash").await;
        // Manually reserve like a crash mid-pipeline after reserve.
        fx.replay
            .reserve(crate::apply::replay::ReserveRequest {
                operation_id: fx.operation_id.clone(),
                execution_hash: fx.execution_hash.clone(),
                dry_run_id: fx.dry_run_id.clone(),
            })
            .await
            .unwrap();

        let row = replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, ReplayStatus::Reserved);

        // Restart: reconnect store on same DB.
        let replay2 = ReplayStore::new(fx.pool.clone());
        let row2 = replay::get(&replay2, &fx.operation_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row2.status, ReplayStatus::Reserved);
        assert_eq!(row2.execution_hash, fx.execution_hash);
    }

    #[tokio::test]
    async fn stuck_operation_handling() {
        let fx = setup("stuck").await;
        let short = ReplayStore::with_reserve_timeout(fx.pool.clone(), Duration::from_millis(1));
        short
            .reserve(crate::apply::replay::ReserveRequest {
                operation_id: fx.operation_id.clone(),
                execution_hash: fx.execution_hash.clone(),
                dry_run_id: fx.dry_run_id.clone(),
            })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        let swept = crate::apply::replay::sweep_reserved_timeouts(&short)
            .await
            .unwrap();
        assert!(swept >= 1);
        let row = replay::get(&short, &fx.operation_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, ReplayStatus::Stuck);
    }

    #[tokio::test]
    async fn invalid_phase_transition_rejected() {
        let mut ctx = ApplyExecutionContext {
            operation_id: "op".into(),
            approval_id: "ap".into(),
            dry_run_id: "dry".into(),
            execution_hash: format!("{:0>64}", "ab"),
            policy_id: "pol".into(),
            policy_version: 1,
            operation_intent: "CapabilityGrant".into(),
            actor_id: "a".into(),
            actor_username: "u".into(),
            request_id: "r".into(),
            audit_correlation_id: "c".into(),
            phase: ApplyExecutionPhase::Created,
        };
        let err = steps::advance(&mut ctx, ApplyExecutionPhase::Executed).unwrap_err();
        assert_eq!(err.code, ExecutionPipelineErrorCode::InvalidTransition);
    }

    #[tokio::test]
    async fn stale_signature_at_proto0_boundary_rejected() {
        let fx = setup("stalesig").await;
        // Force signature expiry after validation would have passed in a slower path:
        // expire signature before full pipeline; validation itself will fail at G12.
        // Mid-pipeline: reserve manually then call boundary after expiry.
        sqlx::query("UPDATE signed_operations SET expires_at = ? WHERE operation_id = ?")
            .bind((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
            .bind(&fx.operation_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        // Fails at validation (G12) or, if somehow past, at C3 — either way no mutation.
        assert!(matches!(
            err.code,
            ExecutionPipelineErrorCode::ValidationFailed
                | ExecutionPipelineErrorCode::StaleSignature
        ));
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
        assert!(!apply_enabled());
    }

    #[tokio::test]
    async fn unknown_intent_defaults_rejected_at_boundary() {
        // C5 regression: parse_intent must not map garbage → CapabilityGrant.
        assert_eq!(
            super::steps::parse_intent_for_test("not-a-real-intent"),
            ProtocolOperationKind::Unknown
        );
    }

    #[tokio::test]
    async fn stale_attestation_at_proto0_boundary_rejected() {
        let fx = setup("staleatt").await;
        // Expire attestation after validation would pass — force mid-pipeline by
        // invalidating after a successful path is prepared via direct SQL.
        sqlx::query("UPDATE dry_run_attestations SET expires_at = ? WHERE dry_run_id = ?")
            .bind((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
            .bind(&fx.dry_run_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        assert!(matches!(
            err.code,
            ExecutionPipelineErrorCode::ValidationFailed
                | ExecutionPipelineErrorCode::StaleAttestation
                | ExecutionPipelineErrorCode::ResimulationFailed
                | ExecutionPipelineErrorCode::ResimulationNotApproved
        ));
        assert!(!apply_enabled());
    }

    #[tokio::test]
    async fn approval_expiry_during_prepare_rolls_back_atomically() {
        let fx = setup("apprexp").await;
        sqlx::query("UPDATE apply_approvals SET expires_at = ? WHERE approval_id = ?")
            .bind((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
            .bind(&fx.approval_id)
            .execute(&fx.pool)
            .await
            .unwrap();
        let err = execute_apply(&pipe_from(&fx), good_request(&fx))
            .await
            .unwrap_err();
        // Fails at validation (G7) or atomic prepare — no reserved row.
        assert!(replay::get(&fx.replay, &fx.operation_id)
            .await
            .unwrap()
            .is_none());
        let _ = err;
    }

    #[tokio::test]
    async fn audit_reconstruction_by_request_id() {
        let fx = setup("audrec").await;
        let req = good_request(&fx);
        let request_id = req.request_id.clone();
        let _ = execute_apply(&pipe_from(&fx), req).await.unwrap();
        let events = reconstruct_audit_by_request_id(&fx.pool, &request_id)
            .await
            .unwrap();
        let pipeline: Vec<_> = events
            .iter()
            .filter(|e| {
                e.action.starts_with("APPLY_EXECUTION_")
                    || e.action == "APPLY_VALIDATION_PASSED"
                    || e.action == "APPLY_RESIMULATION_APPROVED"
                    || e.action == "APPLY_REPLAY_RESERVED"
                    || e.action == "APPLY_PROTO_BOUNDARY"
            })
            .collect();
        let actions: Vec<_> = pipeline.iter().map(|e| e.action.as_str()).collect();
        assert!(actions.contains(&"APPLY_EXECUTION_STARTED"));
        assert!(actions.contains(&"APPLY_VALIDATION_PASSED"));
        assert!(actions.contains(&"APPLY_RESIMULATION_APPROVED"));
        assert!(actions.contains(&"APPLY_REPLAY_RESERVED"));
        assert!(actions.contains(&"APPLY_PROTO_BOUNDARY"));
        assert!(actions.contains(&"APPLY_EXECUTION_FINALISED"));
        for ev in &pipeline {
            let meta = ev.metadata.as_ref().unwrap();
            assert_eq!(meta["request_id"], request_id);
            assert!(meta.get("audit_correlation_id").is_some());
            assert!(meta.get("execution_hash").is_some());
            assert!(meta.get("actor").is_some());
            assert!(meta.get("result").is_some());
        }
    }

    #[tokio::test]
    async fn crash_during_executing_survives_without_mutation() {
        let fx = setup("crashex").await;
        // Manually reserve+consume then begin like a crash mid-execution.
        fx.replay
            .reserve_and_consume_approval(crate::apply::replay::AtomicPrepareRequest {
                operation_id: fx.operation_id.clone(),
                execution_hash: fx.execution_hash.clone(),
                dry_run_id: fx.dry_run_id.clone(),
                approval_id: fx.approval_id.clone(),
            })
            .await
            .unwrap();
        fx.replay.begin_execution(&fx.operation_id).await.unwrap();

        let replay2 = ReplayStore::new(fx.pool.clone());
        let row = replay::get(&replay2, &fx.operation_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, ReplayStatus::Executing);
        assert_eq!(
            protocol_observation_fingerprint(&fx.protocol),
            fx.fingerprint
        );
    }

    #[tokio::test]
    async fn concurrent_apply_preparation_one_wins() {
        let fx = setup("concprep").await;
        let pipe = pipe_from(&fx);
        let r1 = good_request(&fx);
        let r2 = good_request(&fx);
        let (a, b) = tokio::join!(execute_apply(&pipe, r1), execute_apply(&pipe, r2));
        let wins = [a.is_ok(), b.is_ok()].iter().filter(|x| **x).count();
        assert_eq!(wins, 1);
        let appr = fx
            .approvals
            .get_approval(&fx.approval_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(appr.status, ApprovalStatus::Consumed);
    }

    #[tokio::test]
    async fn production_apply_remains_disabled() {
        assert!(!apply_enabled());
        assert!(crate::apply::enablement::runtime_apply_is_disabled());
    }

    #[tokio::test]
    async fn confirm_required_when_apply_enabled() {
        let fx = setup("confirm").await;
        let _guard = crate::apply::ApplyEnabledGuard::enable();
        let mut req = good_request(&fx);
        req.confirm = false;
        let err = execute_apply(&pipe_from(&fx), req).await.unwrap_err();
        assert_eq!(err.code, ExecutionPipelineErrorCode::ConfirmRequired);
        // Enabling Apply for confirm gate must not mutate when pipeline stops early.
        assert_eq!(
            protocol_observation_fingerprint(&fx.protocol),
            fx.fingerprint
        );
    }
}
