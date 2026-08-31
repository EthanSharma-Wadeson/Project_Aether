//! Apply approval HTTP routes — authorisation binding only (no PROTO-0 / Apply execute).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::apply::approval::{ApprovalError, ApprovalService, CreateApprovalRequest};
use crate::auth::middleware::AuthContext;
use crate::error::{Error, Result};
use crate::middleware::request_id::RequestId;
use crate::routes::AppState;
use crate::security::write_guard::guard_mutation;

#[derive(Debug, Deserialize)]
pub struct CreateApprovalBody {
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub operation_intent: String,
}

fn map_approval_err(err: ApprovalError) -> Error {
    use crate::apply::approval::ApprovalErrorCode;
    match err.code() {
        ApprovalErrorCode::NotFound | ApprovalErrorCode::AttestationNotFound => {
            Error::NotFound(err.to_string())
        }
        ApprovalErrorCode::ForbiddenRole | ApprovalErrorCode::SeparationOfDuties => {
            Error::Forbidden(err.to_string())
        }
        ApprovalErrorCode::Database => Error::BadRequest(err.to_string()),
        _ => Error::BadRequest(err.to_string()),
    }
}

/// POST /api/apply-approvals — admin grants Apply approval (no protocol mutation).
pub async fn create_approval(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
    Json(body): Json<CreateApprovalBody>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        false,
    )?;

    let service = ApprovalService::new(state.db.pool());
    let approval = service
        .create_approval(
            &ctx,
            &request_id,
            CreateApprovalRequest {
                approval_id: None,
                dry_run_id: body.dry_run_id,
                execution_hash: body.execution_hash,
                policy_id: body.policy_id,
                policy_version: body.policy_version,
                operation_intent: body.operation_intent,
                approved_by: ctx.operator_id.clone(),
                approver_role: ctx.role.as_str().to_string(),
                request_id: request_id.clone(),
                audit_reference: None,
                ttl: None,
            },
        )
        .await
        .map_err(map_approval_err)?;

    Ok(Json(json!({
        "approval": approval,
        "apply_enabled": false,
        "protocol_mutated": false,
        "note": "Apply approval is an authorisation binding only. Apply execution remains disabled.",
    })))
}

/// GET /api/apply-approvals/:id
pub async fn get_approval(
    State(state): State<Arc<AppState>>,
    axum::Extension(_ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let service = ApprovalService::new(state.db.pool());
    let approval = service
        .get_approval(&id)
        .await
        .map_err(map_approval_err)?
        .ok_or_else(|| Error::NotFound(format!("approval {id}")))?;

    Ok(Json(json!({
        "approval": approval,
        "apply_enabled": false,
        "protocol_mutated": false,
    })))
}

/// DELETE /api/apply-approvals/:id — cancel active approval.
pub async fn cancel_approval(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        false,
    )?;

    let service = ApprovalService::new(state.db.pool());
    let approval = service
        .cancel_approval(&ctx, &request_id, &id)
        .await
        .map_err(map_approval_err)?;

    Ok(Json(json!({
        "approval": approval,
        "apply_enabled": false,
        "protocol_mutated": false,
    })))
}
