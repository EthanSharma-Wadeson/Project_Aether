//! Individual Apply execution pipeline steps (ordered, fail-closed).

use super::errors::{ExecutionPipelineError, ExecutionPipelineErrorCode};
use super::model::{
    allowed_execution_transition, ApplyExecutionContext, ApplyExecutionPhase, ExecuteApplyRequest,
};
use crate::apply::approval::ApprovalStore;
use crate::apply::attestation::AttestationStore;
use crate::apply::errors::ApplyErrorCode;
use crate::apply::policy_mapping::EnterpriseIssuerRef;
use crate::apply::replay::{ReplayError, ReplayStore};
use crate::apply::resimulation::{
    resimulate_apply_intent, ResimulationContext, ResimulationRequest,
};
use crate::apply::signature::SignatureStore;
use crate::apply::validation::{validate_apply_request, ApplyValidationRequest, ValidationContext};
use crate::db::policies as policy_db;
use crate::execution::types::ProtocolOperationKind;
use crate::protocol::proto0_write::{self, ApplyContext, Proto0WriteRequest};
use crate::protocol::state::ProtocolState;
use crate::signer::SigningGateway;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Shared handles for the guarded execution pipeline.
pub struct ExecutionPipelineContext<'a> {
    pub pool: &'a SqlitePool,
    pub protocol: &'a ProtocolState,
    pub approvals: &'a ApprovalStore,
    pub attestations: &'a AttestationStore,
    pub signatures: &'a SignatureStore,
    pub signing: &'a SigningGateway,
    pub replay: &'a ReplayStore,
}

pub(crate) fn advance(
    ctx: &mut ApplyExecutionContext,
    to: ApplyExecutionPhase,
) -> Result<(), ExecutionPipelineError> {
    if !allowed_execution_transition(ctx.phase, to) {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::InvalidTransition,
            format!(
                "invalid execution transition {} -> {}",
                ctx.phase.as_str(),
                to.as_str()
            ),
            ctx.phase,
            Some(ctx.clone()),
        ));
    }
    ctx.phase = to;
    Ok(())
}

pub(crate) fn new_context(request: &ExecuteApplyRequest) -> ApplyExecutionContext {
    ApplyExecutionContext {
        operation_id: request.operation_id.clone(),
        approval_id: request.approval_id.clone(),
        dry_run_id: request.dry_run_id.clone(),
        execution_hash: request.execution_hash.clone(),
        policy_id: String::new(),
        policy_version: 0,
        operation_intent: String::new(),
        actor_id: request.actor.operator_id.clone(),
        actor_username: request.actor.username.clone(),
        request_id: request.request_id.clone(),
        audit_correlation_id: Uuid::new_v4().to_string(),
        phase: ApplyExecutionPhase::Created,
    }
}

/// Step: run validation gate chain (must precede re-simulation and replay).
pub(crate) async fn step_validate(
    pipe: &ExecutionPipelineContext<'_>,
    request: &ExecuteApplyRequest,
    exec: &mut ApplyExecutionContext,
) -> Result<(), ExecutionPipelineError> {
    assert_phase(exec, ApplyExecutionPhase::Created)?;

    let vctx = ValidationContext {
        pool: pipe.pool,
        approvals: pipe.approvals,
        attestations: pipe.attestations,
        signatures: pipe.signatures,
        signing: pipe.signing,
    };
    let vreq = ApplyValidationRequest {
        request_id: request.request_id.clone(),
        operation_id: request.operation_id.clone(),
        approval_id: request.approval_id.clone(),
        dry_run_id: request.dry_run_id.clone(),
        execution_hash: request.execution_hash.clone(),
        authenticated: request.authenticated,
        jwt_valid: request.jwt_valid,
        csrf_valid: request.csrf_valid,
        actor: request.actor.clone(),
    };

    match validate_apply_request(&vctx, vreq).await {
        Ok(result) if result.approved => {
            // Enrich context from signed operation.
            let signed = pipe
                .signatures
                .get(&request.operation_id)
                .await
                .map_err(|e| {
                    ExecutionPipelineError::new(
                        ExecutionPipelineErrorCode::Database,
                        e.to_string(),
                        exec.phase,
                        Some(exec.clone()),
                    )
                })?
                .ok_or_else(|| {
                    ExecutionPipelineError::new(
                        ExecutionPipelineErrorCode::SignedOperationMissing,
                        "signed operation missing after validation",
                        exec.phase,
                        Some(exec.clone()),
                    )
                })?;
            exec.policy_id = signed.policy_id;
            exec.policy_version = signed.policy_version;
            exec.operation_intent = signed.operation_intent;
            exec.execution_hash = signed.execution_hash;
            advance(exec, ApplyExecutionPhase::Validated)
        }
        Ok(result) => Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::ValidationFailed,
            format!(
                "validation not approved: {}",
                result.decision_code.unwrap_or_default()
            ),
            exec.phase,
            Some(exec.clone()),
        )),
        Err(err) => Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::ValidationFailed,
            err.to_string(),
            exec.phase,
            Some(exec.clone()),
        )),
    }
}

/// Step: mandatory re-simulation (must precede replay reservation).
pub(crate) async fn step_resimulate(
    pipe: &ExecutionPipelineContext<'_>,
    exec: &mut ApplyExecutionContext,
) -> Result<(), ExecutionPipelineError> {
    assert_phase(exec, ApplyExecutionPhase::Validated)?;

    let rctx = ResimulationContext {
        pool: pipe.pool,
        protocol: pipe.protocol,
        approvals: pipe.approvals,
        attestations: pipe.attestations,
        signatures: pipe.signatures,
    };
    let rreq = ResimulationRequest {
        request_id: exec.request_id.clone(),
        operation_id: exec.operation_id.clone(),
        actor_id: exec.actor_id.clone(),
    };

    match resimulate_apply_intent(&rctx, rreq).await {
        Ok(result) if result.approved => advance(exec, ApplyExecutionPhase::ResimulationApproved),
        Ok(result) => Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::ResimulationNotApproved,
            format!("re-simulation decision: {}", result.decision_code),
            exec.phase,
            Some(exec.clone()),
        )),
        Err(err) => Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::ResimulationFailed,
            err.to_string(),
            exec.phase,
            Some(exec.clone()),
        )),
    }
}

/// Step: atomically reserve replay slot + consume approval (C2; must precede PROTO-0).
pub(crate) async fn step_reserve(
    pipe: &ExecutionPipelineContext<'_>,
    exec: &mut ApplyExecutionContext,
) -> Result<(), ExecutionPipelineError> {
    assert_phase(exec, ApplyExecutionPhase::ResimulationApproved)?;

    match pipe
        .replay
        .reserve_and_consume_approval(crate::apply::replay::AtomicPrepareRequest {
            operation_id: exec.operation_id.clone(),
            execution_hash: exec.execution_hash.clone(),
            dry_run_id: exec.dry_run_id.clone(),
            approval_id: exec.approval_id.clone(),
        })
        .await
    {
        Ok(_) => {}
        Err(ReplayError::Duplicate { .. }) => {
            return Err(ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::ReplayDuplicate,
                "operation already finalised in replay store",
                exec.phase,
                Some(exec.clone()),
            ));
        }
        Err(ReplayError::InProgress { .. }) => {
            return Err(ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::ReplayInProgress,
                "operation already reserved or executing",
                exec.phase,
                Some(exec.clone()),
            ));
        }
        Err(ReplayError::ApprovalConsumeFailed { message }) => {
            // Transaction rolled back — no reserved row, approval unchanged.
            return Err(ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::ApprovalConsumeFailed,
                message,
                exec.phase,
                Some(exec.clone()),
            ));
        }
        Err(e) => {
            return Err(ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::ReplayFailed,
                e.to_string(),
                exec.phase,
                Some(exec.clone()),
            ));
        }
    }

    advance(exec, ApplyExecutionPhase::ReplayReserved)
}

/// Step: transition replay reserved → executing and mark orchestration Executing.
pub(crate) async fn step_begin_execution(
    pipe: &ExecutionPipelineContext<'_>,
    exec: &mut ApplyExecutionContext,
) -> Result<(), ExecutionPipelineError> {
    assert_phase(exec, ApplyExecutionPhase::ReplayReserved)?;

    pipe.replay
        .begin_execution(&exec.operation_id)
        .await
        .map_err(|e| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::ReplayFailed,
                e.to_string(),
                exec.phase,
                Some(exec.clone()),
            )
        })?;

    advance(exec, ApplyExecutionPhase::Executing)
}

/// Result of the PROTO-0 adapter boundary (Phase 8: always disabled).
#[derive(Debug, Clone)]
pub(crate) struct Proto0BoundaryOutcome {
    pub pipeline_code: ApplyErrorCode,
    #[allow(dead_code)]
    pub adapter_code: Option<String>,
}

/// Step: invoke PROTO-0 **only** via `proto0_write` — shared-ref path while Apply disabled.
///
/// C3/C4: re-check approval, signature, attestation, and policy consistency
/// immediately before the adapter call.
pub(crate) async fn step_proto0_boundary(
    pipe: &ExecutionPipelineContext<'_>,
    exec: &ApplyExecutionContext,
) -> Result<Proto0BoundaryOutcome, ExecutionPipelineError> {
    assert_phase(exec, ApplyExecutionPhase::Executing)?;

    // C3 + C4 — freshness / consistency re-check immediately before PROTO-0.
    recheck_pre_proto0(pipe, exec).await?;

    let policy = policy_db::get_policy(pipe.pool, &exec.policy_id)
        .await
        .map_err(|e| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::Database,
                e.to_string(),
                exec.phase,
                Some(exec.clone()),
            )
        })?
        .ok_or_else(|| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::PolicyMissing,
                format!("policy {} not found", exec.policy_id),
                exec.phase,
                Some(exec.clone()),
            )
        })?;

    // C4 — policy version must still match execution context.
    if policy.version != exec.policy_version {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::PolicyConsistencyFailed,
            format!(
                "policy version drifted: context={} live={}",
                exec.policy_version, policy.version
            ),
            exec.phase,
            Some(exec.clone()),
        ));
    }

    let intent = parse_intent(&exec.operation_intent);
    if matches!(intent, ProtocolOperationKind::Unknown) {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::UnsupportedOperation,
            format!("unknown operation_intent '{}'", exec.operation_intent),
            exec.phase,
            Some(exec.clone()),
        ));
    }

    let issuer = EnterpriseIssuerRef {
        agent_id: pipe
            .protocol
            .index
            .agent_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "enterprise".into()),
    };

    let write_req = Proto0WriteRequest {
        ctx: ApplyContext {
            operation_id: exec.operation_id.clone(),
            request_id: exec.request_id.clone(),
            execution_hash: exec.execution_hash.clone(),
            operator_id: exec.actor_id.clone(),
            capability_intent: intent,
        },
        policy,
        enterprise_issuer: issuer,
        grant_key: None,
        revoked_at: chrono::Utc::now().timestamp().max(0) as u64,
    };

    // Shared-ref adapter entry — mutations require `execute(&mut …)` when enabled.
    let outcome = proto0_write::execute_shared(pipe.protocol, write_req);

    Ok(Proto0BoundaryOutcome {
        pipeline_code: ApplyErrorCode::ApplyExecutionDisabled,
        adapter_code: outcome.apply_error_code,
    })
}

/// C3/C4 — reject stale approval / signature / attestation / binding drift before PROTO-0.
async fn recheck_pre_proto0(
    pipe: &ExecutionPipelineContext<'_>,
    exec: &ApplyExecutionContext,
) -> Result<(), ExecutionPipelineError> {
    use crate::apply::approval::ApprovalStatus;
    use crate::apply::attestation::AttestationBinding;
    use crate::apply::signature::SignatureStatus;
    use chrono::Utc;

    let approval = pipe
        .approvals
        .get_approval(&exec.approval_id)
        .await
        .map_err(|e| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::Database,
                e.to_string(),
                exec.phase,
                Some(exec.clone()),
            )
        })?
        .ok_or_else(|| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::StaleApproval,
                "approval missing at PROTO-0 boundary",
                exec.phase,
                Some(exec.clone()),
            )
        })?;

    // Consumed is expected after atomic reserve; reject if expired/cancelled or past TTL.
    if matches!(
        approval.status,
        ApprovalStatus::Expired | ApprovalStatus::Cancelled
    ) || approval.expires_at <= Utc::now()
    {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::StaleApproval,
            "approval expired or inactive at PROTO-0 boundary",
            exec.phase,
            Some(exec.clone()),
        ));
    }

    if approval.execution_hash != exec.execution_hash
        || approval.policy_version != exec.policy_version
        || approval.operation_intent != exec.operation_intent
        || approval.policy_id != exec.policy_id
    {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::PolicyConsistencyFailed,
            "approval binding drifted from execution context",
            exec.phase,
            Some(exec.clone()),
        ));
    }

    let signed = pipe
        .signatures
        .get(&exec.operation_id)
        .await
        .map_err(|e| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::Database,
                e.to_string(),
                exec.phase,
                Some(exec.clone()),
            )
        })?
        .ok_or_else(|| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::StaleSignature,
                "signed operation missing at PROTO-0 boundary",
                exec.phase,
                Some(exec.clone()),
            )
        })?;

    if matches!(signed.status, SignatureStatus::Expired) || signed.expires_at <= Utc::now() {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::StaleSignature,
            "signature expired at PROTO-0 boundary",
            exec.phase,
            Some(exec.clone()),
        ));
    }

    // Attestation must still be executable and bound (C3).
    pipe.attestations
        .get_valid_attestation(&AttestationBinding {
            dry_run_id: exec.dry_run_id.clone(),
            execution_hash: exec.execution_hash.clone(),
            policy_version: exec.policy_version,
            operation_intent: exec.operation_intent.clone(),
        })
        .await
        .map_err(|e| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::StaleAttestation,
                e.to_string(),
                exec.phase,
                Some(exec.clone()),
            )
        })?;

    Ok(())
}

/// Step: finalise replay as rejected (known disabled outcome) or other terminal.
pub(crate) async fn step_finalise_rejected(
    pipe: &ExecutionPipelineContext<'_>,
    exec: &mut ApplyExecutionContext,
    reason: &str,
) -> Result<(), ExecutionPipelineError> {
    assert_phase(exec, ApplyExecutionPhase::Executing)?;

    pipe.replay
        .mark_rejected(
            &exec.operation_id,
            Some(reason.to_string()),
            Some(exec.audit_correlation_id.clone()),
        )
        .await
        .map_err(|e| {
            ExecutionPipelineError::new(
                ExecutionPipelineErrorCode::ReplayFailed,
                e.to_string(),
                exec.phase,
                Some(exec.clone()),
            )
        })?;

    advance(exec, ApplyExecutionPhase::Rejected)
}

pub(crate) fn assert_phase(
    exec: &ApplyExecutionContext,
    expected: ApplyExecutionPhase,
) -> Result<(), ExecutionPipelineError> {
    if exec.phase != expected {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::InvalidTransition,
            format!(
                "expected phase {}, have {}",
                expected.as_str(),
                exec.phase.as_str()
            ),
            exec.phase,
            Some(exec.clone()),
        ));
    }
    Ok(())
}

fn parse_intent(intent: &str) -> ProtocolOperationKind {
    match intent {
        "CapabilityGrant" => ProtocolOperationKind::CapabilityGrant,
        "CapabilityRevoke" => ProtocolOperationKind::CapabilityRevoke,
        "FreezeIdentity" => ProtocolOperationKind::FreezeIdentity,
        "PolicyApply" => ProtocolOperationKind::PolicyApply,
        _ => ProtocolOperationKind::Unknown,
    }
}

#[cfg(test)]
pub(crate) fn parse_intent_for_test(intent: &str) -> ProtocolOperationKind {
    parse_intent(intent)
}
