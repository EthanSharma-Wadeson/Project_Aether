//! Phase 31 — Sandbox HTTP (simulated lifecycle only).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::{Extension, Json};
use serde_json::{json, Value};

use crate::agents::sandbox::models::SandboxTaskRequest;
use crate::agents::sandbox::SandboxRuntime;
use crate::auth::middleware::AuthContext;
use crate::auth::roles::require_operator;
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::routes::AppState;
use crate::security::write_guard::guard_mutation;

fn org(state: &AppState) -> &str {
    state.treasury.organisation_id()
}

/// POST /api/sandbox/bootstrap
pub async fn bootstrap(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let agents =
        SandboxRuntime::bootstrap(&state.protocol, &state.treasury, &state.db, &ctx).await?;
    Ok(Json(json!({
        "agents": agents,
        "request_id": request_id,
        "external_execution": false,
        "note": "£1 simulated_budget_minor is metadata only — not a wallet"
    })))
}

/// POST /api/sandbox/run
pub async fn run_task(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(task): Json<SandboxTaskRequest>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let result = SandboxRuntime::run_task(
        &state.protocol,
        &state.treasury,
        &state.db,
        &ctx,
        task,
        request_id,
    )
    .await?;
    Ok(Json(serde_json::to_value(result)?))
}

/// GET /api/sandbox/reconstruct/:request_id
pub async fn reconstruct(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>> {
    let rec = SandboxRuntime::reconstruct(&state.db, org(&state), &request_id).await?;
    Ok(Json(json!({
        "agent": rec.agent_id,
        "session": rec.session_id,
        "tool": rec.tool_id,
        "decision": rec.decision,
        "reason": rec.reason,
        "request_id": rec.request_id,
        "organisation_id": rec.organisation_id,
        "simulated_result": rec.simulated_result,
        "review_event_id": rec.review_event_id,
        "created_at": rec.created_at,
    })))
}
