//! Apply validation pipeline — deterministic G1→G14 gate chain.
//!
//! Produces an execution decision. Does **not** execute, reserve replay,
//! consume approvals, or call PROTO-0.

use chrono::Utc;
use serde_json::json;

use super::errors::{ValidationError, ValidationErrorCode};
use super::gates::{
    empty_result, gate_apply_enabled, gate_approval_active, gate_approval_exists,
    gate_attestation_valid, gate_authentication, gate_csrf, gate_execution_hash_match, gate_jwt,
    gate_policy_version_unchanged, gate_rbac, gate_resimulation_required, gate_signature_ttl,
    gate_signature_valid, gate_signed_operation_exists, ValidationContext,
};
use super::model::{ApplyValidationRequest, ValidationResult};
use crate::db::audit;

/// Run the fixed Apply validation gate chain (fail-closed).
///
/// On success (G1–G13 pass): returns `ValidationResult` with `approved = true`
/// and `decision_code = VALIDATION_REQUIRES_RESIMULATION` (G14). Nothing executes.
pub async fn validate_apply_request(
    ctx: &ValidationContext<'_>,
    request: ApplyValidationRequest,
) -> Result<ValidationResult, ValidationError> {
    let mut partial = empty_result(&request);

    audit_started(ctx, &request).await;

    let outcome = run_gates(ctx, &request, &mut partial).await;

    match &outcome {
        Ok(result) => {
            audit_complete(ctx, &request, result, None).await;
        }
        Err(err) => {
            audit_gate_failed(ctx, &request, err).await;
            audit_complete(ctx, &request, &err.partial, Some(err)).await;
        }
    }

    outcome
}

async fn run_gates(
    ctx: &ValidationContext<'_>,
    request: &ApplyValidationRequest,
    partial: &mut ValidationResult,
) -> Result<ValidationResult, ValidationError> {
    gate_authentication(request, partial)?;
    audit_gate_passed(ctx, request, "G1").await;

    gate_jwt(request, partial)?;
    audit_gate_passed(ctx, request, "G2").await;

    gate_rbac(request, partial)?;
    audit_gate_passed(ctx, request, "G3").await;

    gate_csrf(request, partial)?;
    audit_gate_passed(ctx, request, "G4").await;

    gate_apply_enabled(partial)?;
    audit_gate_passed(ctx, request, "G5").await;

    let approval = gate_approval_exists(ctx, request, partial).await?;
    audit_gate_passed(ctx, request, "G6").await;

    let approval = gate_approval_active(ctx, request, &approval, partial).await?;
    audit_gate_passed(ctx, request, "G7").await;

    let attestation = gate_attestation_valid(ctx, request, &approval, partial).await?;
    audit_gate_passed(ctx, request, "G8").await;

    gate_execution_hash_match(ctx, request, &approval, &attestation, partial).await?;
    audit_gate_passed(ctx, request, "G9").await;

    let signed = gate_signed_operation_exists(ctx, request, partial).await?;
    audit_gate_passed(ctx, request, "G10").await;

    gate_signature_valid(ctx, &signed, partial)?;
    audit_gate_passed(ctx, request, "G11").await;

    gate_signature_ttl(&signed, partial)?;
    audit_gate_passed(ctx, request, "G12").await;

    let _policy = gate_policy_version_unchanged(ctx, &signed, &attestation, partial).await?;
    audit_gate_passed(ctx, request, "G13").await;

    gate_resimulation_required(partial)?;
    audit_gate_passed(ctx, request, "G14").await;

    partial.approved = true;
    partial.validated_at = Utc::now();
    if partial.decision_code.is_none() {
        partial.decision_code = Some(ValidationErrorCode::RequiresResimulation.as_str().into());
    }

    Ok(partial.clone())
}

async fn audit_started(ctx: &ValidationContext<'_>, request: &ApplyValidationRequest) {
    let _ = audit::append(
        ctx.pool,
        Some(&request.actor.operator_id),
        "APPLY_VALIDATION_STARTED",
        Some(&request.operation_id),
        Some(json!({
            "request_id": request.request_id,
            "operation_id": request.operation_id,
            "approval_id": request.approval_id,
            "dry_run_id": request.dry_run_id,
            "execution_hash": request.execution_hash,
            "actor": request.actor.operator_id,
        })),
    )
    .await;
}

async fn audit_gate_passed(
    ctx: &ValidationContext<'_>,
    request: &ApplyValidationRequest,
    gate: &str,
) {
    let _ = audit::append(
        ctx.pool,
        Some(&request.actor.operator_id),
        "APPLY_GATE_PASSED",
        Some(&request.operation_id),
        Some(json!({
            "request_id": request.request_id,
            "operation_id": request.operation_id,
            "approval_id": request.approval_id,
            "dry_run_id": request.dry_run_id,
            "execution_hash": request.execution_hash,
            "gate": gate,
            "actor": request.actor.operator_id,
        })),
    )
    .await;
}

async fn audit_gate_failed(
    ctx: &ValidationContext<'_>,
    request: &ApplyValidationRequest,
    err: &ValidationError,
) {
    let _ = audit::append(
        ctx.pool,
        Some(&request.actor.operator_id),
        "APPLY_GATE_FAILED",
        Some(&request.operation_id),
        Some(json!({
            "request_id": request.request_id,
            "operation_id": request.operation_id,
            "approval_id": request.approval_id,
            "dry_run_id": request.dry_run_id,
            "execution_hash": request.execution_hash,
            "failed_gate": err.gate.as_str(),
            "failure_code": err.code.as_str(),
            "actor": request.actor.operator_id,
        })),
    )
    .await;
}

async fn audit_complete(
    ctx: &ValidationContext<'_>,
    request: &ApplyValidationRequest,
    result: &ValidationResult,
    err: Option<&ValidationError>,
) {
    let _ = audit::append(
        ctx.pool,
        Some(&request.actor.operator_id),
        "APPLY_VALIDATION_COMPLETE",
        Some(&request.operation_id),
        Some(json!({
            "request_id": request.request_id,
            "operation_id": request.operation_id,
            "approval_id": request.approval_id,
            "dry_run_id": request.dry_run_id,
            "execution_hash": request.execution_hash,
            "approved": result.approved,
            "decision_code": result.decision_code,
            "failed_gate": err.map(|e| e.gate.as_str()),
            "failure_code": err.map(|e| e.code.as_str()),
            "actor": request.actor.operator_id,
            "gates_passed": result.gate_results.iter().filter(|g| g.passed).count(),
        })),
    )
    .await;
}
