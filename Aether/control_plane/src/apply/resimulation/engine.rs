//! Mandatory re-simulation engine — final safety barrier before execution.
//!
//! Produces evidence. Does **not** execute, reserve replay, consume approvals,
//! call the signer, or call `proto0_write`.

use chrono::Utc;
use serde_json::json;

use super::compare::{attested_snapshot, compare_snapshots, comparison_hash, fresh_snapshot};
use super::errors::{ResimulationError, ResimulationErrorCode};
use super::model::{ResimulationDecisionCode, ResimulationRequest, ResimulationResult};
use crate::apply::approval::ApprovalStore;
use crate::apply::attestation::AttestationStore;
use crate::apply::signature::SignatureStore;
use crate::db::audit;
use crate::db::policies as policy_db;
use crate::execution::hash::execution_hash_for_policy;
use crate::execution::simulate::{
    predict_protocol_operation, protocol_observation_fingerprint, simulate_policy_apply,
};
use crate::protocol::state::ProtocolState;
use sqlx::SqlitePool;

/// Stores required for re-simulation (no signer, no replay, no PROTO-0 write).
pub struct ResimulationContext<'a> {
    pub pool: &'a SqlitePool,
    pub protocol: &'a ProtocolState,
    pub approvals: &'a ApprovalStore,
    pub attestations: &'a AttestationStore,
    pub signatures: &'a SignatureStore,
}

/// Run mandatory re-simulation for a signed Apply operation (R1–R9).
///
/// Comparison mismatches return `Ok(ResimulationResult { approved: false, ... })`.
/// Missing artefacts / infrastructure failures return `Err`.
pub async fn resimulate_apply_intent(
    ctx: &ResimulationContext<'_>,
    request: ResimulationRequest,
) -> Result<ResimulationResult, ResimulationError> {
    audit_started(ctx, &request).await;

    let fingerprint_before = protocol_observation_fingerprint(ctx.protocol);

    let outcome = run_pipeline(ctx, &request).await;

    let fingerprint_after = protocol_observation_fingerprint(ctx.protocol);
    if fingerprint_before != fingerprint_after {
        let err = ResimulationError::new(
            ResimulationErrorCode::Failed,
            "protocol observation fingerprint changed during re-simulation (INV-R1)",
        );
        audit_failed(ctx, &request, None, None, &err.message, err.decision_code).await;
        return Err(err);
    }

    match &outcome {
        Ok(result) if result.approved => {
            audit_passed(ctx, &request, result).await;
        }
        Ok(result) => {
            audit_failed(
                ctx,
                &request,
                Some(result.execution_hash.as_str()),
                Some(result.comparison_hash.as_str()),
                result.decision_code.as_str(),
                ResimulationDecisionCode::from_str_code(&result.decision_code),
            )
            .await;
        }
        Err(err) => {
            audit_failed(ctx, &request, None, None, &err.message, err.decision_code).await;
        }
    }

    outcome
}

async fn run_pipeline(
    ctx: &ResimulationContext<'_>,
    request: &ResimulationRequest,
) -> Result<ResimulationResult, ResimulationError> {
    // R1 — Load signed operation
    let signed = ctx
        .signatures
        .get(&request.operation_id)
        .await
        .map_err(|e| ResimulationError::new(ResimulationErrorCode::Database, e.to_string()))?
        .ok_or_else(|| {
            ResimulationError::new(
                ResimulationErrorCode::SignedOperationMissing,
                format!("signed operation {} not found", request.operation_id),
            )
        })?;

    // R2 — Load approval binding
    let approval = ctx
        .approvals
        .get_approval(&signed.approval_id)
        .await
        .map_err(|e| ResimulationError::new(ResimulationErrorCode::Database, e.to_string()))?
        .ok_or_else(|| {
            ResimulationError::new(
                ResimulationErrorCode::ApprovalMissing,
                format!("approval {} not found", signed.approval_id),
            )
        })?;

    // R3 — Load dry-run attestation
    let attestation = ctx
        .attestations
        .get(&signed.dry_run_id)
        .await
        .map_err(|e| ResimulationError::new(ResimulationErrorCode::Database, e.to_string()))?
        .ok_or_else(|| {
            ResimulationError::new(
                ResimulationErrorCode::AttestationMissing,
                format!("attestation {} not found", signed.dry_run_id),
            )
        })?;

    // Binding sanity (stale approval / wrong dry-run)
    if approval.dry_run_id != attestation.dry_run_id
        || approval.execution_hash != attestation.execution_hash
        || signed.execution_hash != attestation.execution_hash
        || signed.dry_run_id != attestation.dry_run_id
    {
        return Ok(reject_result(
            &signed.operation_id,
            &attestation.dry_run_id,
            &attestation.execution_hash,
            "",
            false,
            ResimulationDecisionCode::Failed,
        ));
    }

    // R4 — Load current policy version
    let policy = policy_db::get_policy(ctx.pool, &signed.policy_id)
        .await
        .map_err(|e| ResimulationError::new(ResimulationErrorCode::Database, e.to_string()))?
        .ok_or_else(|| {
            ResimulationError::new(
                ResimulationErrorCode::PolicyMissing,
                format!("policy {} not found", signed.policy_id),
            )
        })?;

    let attested = attested_snapshot(&attestation, &signed, &policy);

    // Early policy identity check (id / version / hash vs attested evidence)
    if policy.id != attestation.policy_id
        || policy.version != attestation.policy_version
        || (!attested.policy_hash.is_empty() && policy.hash != attested.policy_hash)
    {
        let fresh_kind = predict_protocol_operation(&policy);
        let fresh_hash = execution_hash_for_policy(&policy, fresh_kind);
        let fresh = fresh_snapshot(
            &policy,
            &signed,
            fresh_kind,
            &fresh_hash,
            &crate::execution::types::SimulationOutcome {
                executable: false,
                reason: Some("policy identity changed".into()),
            },
        );
        let cmp = comparison_hash(&fresh).unwrap_or_default();
        return Ok(reject_result(
            &signed.operation_id,
            &attestation.dry_run_id,
            &attestation.execution_hash,
            &cmp,
            false,
            ResimulationDecisionCode::PolicyChanged,
        ));
    }

    // R5 — Re-run simulation engine (read-only)
    let simulation = simulate_policy_apply(ctx.protocol, &policy);

    // R6 — Generate new simulation / comparison snapshot
    let kind = predict_protocol_operation(&policy);
    let new_execution_hash = execution_hash_for_policy(&policy, kind);
    let fresh = fresh_snapshot(&policy, &signed, kind, &new_execution_hash, &simulation);

    // R7 + R8 — Compare predicted changes and execution_hash
    let decision = compare_snapshots(&attested, &fresh);

    // R9 — Produce decision
    let cmp_hash = comparison_hash(&fresh).map_err(|e| {
        ResimulationError::new(
            ResimulationErrorCode::Failed,
            format!("comparison hash: {e}"),
        )
    })?;

    let approved = decision == ResimulationDecisionCode::Approved;
    Ok(ResimulationResult {
        approved,
        dry_run_id: attestation.dry_run_id,
        operation_id: signed.operation_id,
        execution_hash: if approved {
            new_execution_hash
        } else {
            attestation.execution_hash
        },
        comparison_hash: cmp_hash,
        changes_match: approved,
        validated_at: Utc::now(),
        decision_code: decision.as_str().into(),
    })
}

fn reject_result(
    operation_id: &str,
    dry_run_id: &str,
    execution_hash: &str,
    comparison_hash: &str,
    changes_match: bool,
    decision: ResimulationDecisionCode,
) -> ResimulationResult {
    ResimulationResult {
        approved: false,
        dry_run_id: dry_run_id.into(),
        operation_id: operation_id.into(),
        execution_hash: execution_hash.into(),
        comparison_hash: comparison_hash.into(),
        changes_match,
        validated_at: Utc::now(),
        decision_code: decision.as_str().into(),
    }
}

impl ResimulationDecisionCode {
    fn from_str_code(s: &str) -> Self {
        match s {
            "RESIMULATION_APPROVED" => Self::Approved,
            "RESIMULATION_HASH_MISMATCH" => Self::HashMismatch,
            "RESIMULATION_STATE_CHANGED" => Self::StateChanged,
            "RESIMULATION_POLICY_CHANGED" => Self::PolicyChanged,
            _ => Self::Failed,
        }
    }
}

async fn audit_started(ctx: &ResimulationContext<'_>, request: &ResimulationRequest) {
    let _ = audit::append(
        ctx.pool,
        Some(&request.actor_id),
        "APPLY_RESIMULATION_STARTED",
        Some(&request.operation_id),
        Some(json!({
            "request_id": request.request_id,
            "operation_id": request.operation_id,
            "actor": request.actor_id,
        })),
    )
    .await;
}

async fn audit_passed(
    ctx: &ResimulationContext<'_>,
    request: &ResimulationRequest,
    result: &ResimulationResult,
) {
    let _ = audit::append(
        ctx.pool,
        Some(&request.actor_id),
        "APPLY_RESIMULATION_PASSED",
        Some(&request.operation_id),
        Some(json!({
            "request_id": request.request_id,
            "operation_id": result.operation_id,
            "dry_run_id": result.dry_run_id,
            "execution_hash": result.execution_hash,
            "comparison_hash": result.comparison_hash,
            "actor": request.actor_id,
        })),
    )
    .await;
}

async fn audit_failed(
    ctx: &ResimulationContext<'_>,
    request: &ResimulationRequest,
    execution_hash: Option<&str>,
    comparison_hash: Option<&str>,
    reason: &str,
    decision: ResimulationDecisionCode,
) {
    let _ = audit::append(
        ctx.pool,
        Some(&request.actor_id),
        "APPLY_RESIMULATION_FAILED",
        Some(&request.operation_id),
        Some(json!({
            "request_id": request.request_id,
            "operation_id": request.operation_id,
            "execution_hash": execution_hash,
            "comparison_hash": comparison_hash,
            "failed_reason": reason,
            "decision_code": decision.as_str(),
            "actor": request.actor_id,
        })),
    )
    .await;
}
