//! Apply HTTP surface — prepare, execute, status, reconcile (Phase 12).
//!
//! All mutating routes use existing middleware (`guard_mutation`) and call into
//! the existing Apply engine. `apply_enabled()` remains `false`; PROTO-0 is not
//! mutated via these routes.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::apply::apply_enabled;
use crate::apply::approval::ApprovalStore;
use crate::apply::attestation::AttestationStore;
use crate::apply::execution::{
    execute_apply, ExecuteApplyRequest, ExecutionPipelineContext, ExecutionPipelineError,
    ExecutionPipelineErrorCode,
};
use crate::apply::replay::{self, ReplayError, ReplayStore};
use crate::apply::signature::{
    prepare_signature, PrepareSignatureRequest, SignatureError, SignatureErrorCode, SignatureStore,
    PURPOSE_APPLY,
};
use crate::auth::middleware::{require_admin, AuthContext};
use crate::auth::roles::require_operator;
use crate::db::audit;
use crate::error::{Error, Result};
use crate::middleware::request_id::RequestId;
use crate::routes::AppState;
use crate::security::write_guard::guard_mutation;

// ── Request bodies ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PrepareApplyBody {
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub operation_intent: String,
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub target_agent: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteApplyBody {
    pub operation_id: String,
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    /// Mandatory explicit confirmation (C9 / Phase 12).
    pub confirm: bool,
}

#[derive(Debug, Deserialize)]
pub struct AbortReconcileBody {
    #[serde(default)]
    pub reason: Option<String>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn pipe_from(state: &AppState) -> (ApprovalStore, AttestationStore, SignatureStore, ReplayStore) {
    let pool = state.db.pool().clone();
    (
        ApprovalStore::new(pool.clone()),
        AttestationStore::new(pool.clone()),
        SignatureStore::new(pool.clone()),
        ReplayStore::new(pool),
    )
}

fn map_signature_err(err: SignatureError) -> Error {
    match err.code() {
        SignatureErrorCode::NotFound | SignatureErrorCode::MissingApproval => {
            Error::NotFound(err.to_string())
        }
        SignatureErrorCode::InvalidSignerRole => Error::Forbidden(err.to_string()),
        SignatureErrorCode::DuplicateOperationId => Error::BadRequest(err.to_string()),
        _ => Error::BadRequest(format!("{}: {err}", err.code().as_str())),
    }
}

fn map_exec_err(err: ExecutionPipelineError) -> Error {
    match err.code {
        ExecutionPipelineErrorCode::ConfirmRequired => {
            Error::BadRequest(format!("{}: {}", err.code.as_str(), err.message))
        }
        ExecutionPipelineErrorCode::ReplayDuplicate
        | ExecutionPipelineErrorCode::ReplayInProgress => {
            Error::BadRequest(format!("{}: {}", err.code.as_str(), err.message))
        }
        ExecutionPipelineErrorCode::ValidationFailed
        | ExecutionPipelineErrorCode::ResimulationFailed
        | ExecutionPipelineErrorCode::ResimulationNotApproved
        | ExecutionPipelineErrorCode::StaleApproval
        | ExecutionPipelineErrorCode::StaleSignature
        | ExecutionPipelineErrorCode::StaleAttestation
        | ExecutionPipelineErrorCode::PolicyConsistencyFailed
        | ExecutionPipelineErrorCode::ApprovalConsumeFailed
        | ExecutionPipelineErrorCode::UnsupportedOperation
        | ExecutionPipelineErrorCode::SignedOperationMissing
        | ExecutionPipelineErrorCode::PolicyMissing => {
            Error::BadRequest(format!("{}: {}", err.code.as_str(), err.message))
        }
        ExecutionPipelineErrorCode::ExecutionDisabled => {
            // Should surface as success with disabled outcome; treat as bad request if raised.
            Error::BadRequest(format!("{}: {}", err.code.as_str(), err.message))
        }
        _ => Error::BadRequest(format!("{}: {}", err.code.as_str(), err.message)),
    }
}

fn map_replay_err(err: ReplayError) -> Error {
    match &err {
        ReplayError::NotFound { .. } => Error::NotFound(err.to_string()),
        ReplayError::InvalidTransition { .. } => Error::BadRequest(err.to_string()),
        _ => Error::BadRequest(err.to_string()),
    }
}

fn replay_json(record: &crate::apply::replay::ReplayRecord) -> Value {
    json!({
        "operation_id": record.operation_id,
        "execution_hash": record.execution_hash,
        "dry_run_id": record.dry_run_id,
        "status": record.status.as_str(),
        "created_at": record.created_at.to_rfc3339(),
        "updated_at": record.updated_at.to_rfc3339(),
        "reserved_at": record.reserved_at.to_rfc3339(),
        "finalised_at": record.finalised_at.map(|t| t.to_rfc3339()),
        "terminal_reason": record.terminal_reason,
        "audit_reference": record.audit_reference,
        "terminal": record.status.is_terminal(),
    })
}

// ── POST /api/apply/prepare ──────────────────────────────────────────────────

/// Prepare an Apply signed operation (no execution, no PROTO-0).
pub async fn prepare_apply(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
    Json(body): Json<PrepareApplyBody>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        false,
    )?;

    let approvals = ApprovalStore::new(state.db.pool().clone());
    let signatures = SignatureStore::new(state.db.pool().clone());

    let signed = prepare_signature(
        &signatures,
        &approvals,
        state.signing.as_ref(),
        PrepareSignatureRequest {
            operation_id: body.operation_id,
            approval_id: body.approval_id.clone(),
            dry_run_id: body.dry_run_id,
            execution_hash: body.execution_hash,
            policy_id: body.policy_id,
            policy_version: body.policy_version,
            operation_intent: body.operation_intent,
            request_id: request_id.clone(),
            signer_id: ctx.operator_id.clone(),
            signer_role: ctx.role.as_str().to_string(),
            target_agent: body.target_agent,
            purpose: PURPOSE_APPLY.to_string(),
            ttl: None,
        },
    )
    .await
    .map_err(map_signature_err)?;

    let _ = audit::append(
        state.db.pool(),
        Some(&ctx.operator_id),
        "APPLY_HTTP_PREPARE",
        Some(&signed.operation_id),
        Some(json!({
            "request_id": request_id,
            "operation_id": signed.operation_id,
            "approval_id": signed.approval_id,
            "execution_hash": signed.execution_hash,
            "payload_hash": signed.payload_hash,
            "actor": ctx.operator_id,
            "result": "prepared",
            "apply_enabled": apply_enabled(),
        })),
    )
    .await;

    Ok(Json(json!({
        "operation_id": signed.operation_id,
        "payload_hash": signed.payload_hash,
        "execution_hash": signed.execution_hash,
        "approval_id": signed.approval_id,
        "dry_run_id": signed.dry_run_id,
        "policy_id": signed.policy_id,
        "policy_version": signed.policy_version,
        "operation_intent": signed.operation_intent,
        "signature": {
            "algorithm": signed.signature_algorithm,
            "value": signed.signature,
            "status": signed.status.as_str(),
            "signer_identity": signed.signer_identity,
            "signer_id": signed.signer_id,
            "signer_role": signed.signer_role,
            "signed_at": signed.signed_at,
            "expires_at": signed.expires_at.to_rfc3339(),
            "purpose": signed.purpose,
        },
        "request_id": request_id,
        "apply_enabled": false,
        "protocol_mutated": false,
        "note": "Signature prepared only. Apply execution remains disabled.",
    })))
}

// ── POST /api/apply ──────────────────────────────────────────────────────────

/// Execute Apply via the existing pipeline. Returns disabled outcome while
/// `apply_enabled()==false`. Never bypasses validation / resim / replay.
pub async fn execute_apply_http(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
    Json(body): Json<ExecuteApplyBody>,
) -> Result<Json<Value>> {
    require_admin(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        true, // consume CSRF for execute
    )?;

    // Phase 12: confirm is mandatory on the HTTP surface even while disabled.
    if !body.confirm {
        return Err(Error::BadRequest(
            "APPLY_EXECUTION_CONFIRM_REQUIRED: confirm: true is required".into(),
        ));
    }

    let (approvals, attestations, signatures, replay) = pipe_from(&state);
    let pipe = ExecutionPipelineContext {
        pool: state.db.pool(),
        protocol: state.protocol.as_ref(),
        approvals: &approvals,
        attestations: &attestations,
        signatures: &signatures,
        signing: state.signing.as_ref(),
        replay: &replay,
    };

    let result = execute_apply(
        &pipe,
        ExecuteApplyRequest {
            request_id: request_id.clone(),
            operation_id: body.operation_id.clone(),
            approval_id: body.approval_id.clone(),
            dry_run_id: body.dry_run_id.clone(),
            execution_hash: body.execution_hash.clone(),
            authenticated: true,
            jwt_valid: true,
            csrf_valid: true,
            actor: ctx.clone(),
            confirm: body.confirm,
        },
    )
    .await
    .map_err(map_exec_err)?;

    let replay_row = replay::get(&replay, &body.operation_id)
        .await
        .ok()
        .flatten();

    let outcome = if apply_enabled() {
        result.outcome_code.clone()
    } else {
        // Stable disabled code for clients while Apply is off.
        crate::apply::errors::ApplyErrorCode::ApplyExecutionDisabled
            .as_str()
            .to_string()
    };

    let _ = audit::append(
        state.db.pool(),
        Some(&ctx.operator_id),
        "APPLY_HTTP_EXECUTE",
        Some(&body.operation_id),
        Some(json!({
            "request_id": request_id,
            "operation_id": body.operation_id,
            "approval_id": body.approval_id,
            "execution_hash": body.execution_hash,
            "actor": ctx.operator_id,
            "result": outcome,
            "apply_enabled": apply_enabled(),
            "protocol_mutated": !result.protocol_unchanged,
        })),
    )
    .await;

    Ok(Json(json!({
        "outcome_code": outcome,
        "code": outcome,
        "apply_enabled": false,
        "protocol_mutated": !result.protocol_unchanged,
        "protocol_unchanged": result.protocol_unchanged,
        "operation_id": result.context.operation_id,
        "approval_id": result.context.approval_id,
        "dry_run_id": result.context.dry_run_id,
        "execution_hash": result.context.execution_hash,
        "policy_id": result.context.policy_id,
        "policy_version": result.context.policy_version,
        "phase": result.context.phase.as_str(),
        "request_id": request_id,
        "audit_correlation_id": result.context.audit_correlation_id,
        "replay": replay_row.as_ref().map(replay_json),
        "replay_reserved": result.replay_reserved,
        "note": "Apply execution pipeline completed without protocol mutation (apply_enabled=false).",
    })))
}

// ── GET /api/apply/operations/:operation_id ──────────────────────────────────

/// Read-only operation / replay status.
pub async fn get_operation(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(operation_id): Path<String>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;

    let signatures = SignatureStore::new(state.db.pool().clone());
    let replay = ReplayStore::new(state.db.pool().clone());

    let signed = signatures
        .get(&operation_id)
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?;

    let replay_row = replay::get(&replay, &operation_id)
        .await
        .map_err(map_replay_err)?;

    if signed.is_none() && replay_row.is_none() {
        return Err(Error::NotFound(format!(
            "apply operation {operation_id} not found"
        )));
    }

    Ok(Json(json!({
        "operation_id": operation_id,
        "signed_operation": signed.as_ref().map(|s| json!({
            "approval_id": s.approval_id,
            "dry_run_id": s.dry_run_id,
            "execution_hash": s.execution_hash,
            "policy_id": s.policy_id,
            "policy_version": s.policy_version,
            "operation_intent": s.operation_intent,
            "status": s.status.as_str(),
            "expires_at": s.expires_at.to_rfc3339(),
            "created_at": s.created_at.to_rfc3339(),
            "payload_hash": s.payload_hash,
            "request_id": s.request_id,
            "signer_id": s.signer_id,
        })),
        "replay": replay_row.as_ref().map(replay_json),
        "status": replay_row
            .as_ref()
            .map(|r| r.status.as_str())
            .or_else(|| signed.as_ref().map(|s| s.status.as_str()))
            .unwrap_or("unknown"),
        "terminal": replay_row
            .as_ref()
            .map(|r| r.status.is_terminal())
            .unwrap_or(false),
        "failure_reason": replay_row.as_ref().and_then(|r| r.terminal_reason.clone()),
        "apply_enabled": false,
        "protocol_mutated": false,
    })))
}

// ── Reconcile (admin only) ───────────────────────────────────────────────────

/// GET /api/apply/reconcile — scan stuck / reserved / executing (no PROTO-0 retry).
pub async fn reconcile_list(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
) -> Result<Json<Value>> {
    require_admin(&ctx)?;
    let replay = ReplayStore::new(state.db.pool().clone());
    let report = replay::startup_scan(&replay)
        .await
        .map_err(map_replay_err)?;

    Ok(Json(json!({
        "scanned_at": report.scanned_at.to_rfc3339(),
        "reserved": report.reserved.iter().map(replay_json).collect::<Vec<_>>(),
        "executing": report.executing.iter().map(replay_json).collect::<Vec<_>>(),
        "stuck": report.stuck.iter().map(replay_json).collect::<Vec<_>>(),
        "reserved_timed_out": report.reserved_timed_out,
        "executing_timed_out": report.executing_timed_out,
        "apply_enabled": false,
        "protocol_mutated": false,
        "note": "Reconciliation never retries PROTO-0 and never assumes success.",
    })))
}

/// GET /api/apply/reconcile/:operation_id
pub async fn reconcile_get(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(operation_id): Path<String>,
) -> Result<Json<Value>> {
    require_admin(&ctx)?;
    let replay = ReplayStore::new(state.db.pool().clone());
    let row = replay::get(&replay, &operation_id)
        .await
        .map_err(map_replay_err)?
        .ok_or_else(|| Error::NotFound(format!("replay operation {operation_id}")))?;

    Ok(Json(json!({
        "operation": replay_json(&row),
        "apply_enabled": false,
        "protocol_mutated": false,
        "note": "Status only — no automatic retry.",
    })))
}

/// POST /api/apply/reconcile/:operation_id/abort — admin abort only.
pub async fn reconcile_abort(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(operation_id): Path<String>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
    Json(body): Json<AbortReconcileBody>,
) -> Result<Json<Value>> {
    require_admin(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        true,
    )?;

    let replay = ReplayStore::new(state.db.pool().clone());
    let reason = body
        .reason
        .unwrap_or_else(|| "admin_http_abort".to_string());
    let aborted = replay::admin_abort(&replay, &operation_id, &ctx.operator_id, &reason)
        .await
        .map_err(map_replay_err)?;

    Ok(Json(json!({
        "operation": replay_json(&aborted),
        "request_id": request_id,
        "apply_enabled": false,
        "protocol_mutated": false,
        "note": "Aborted without PROTO-0 retry or assumed success.",
    })))
}
