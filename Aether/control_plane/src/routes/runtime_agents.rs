//! Phase 28 — Lab runtime agent registry HTTP (no tools / no provider APIs).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::{Extension, Json};
use serde_json::{json, Value};

use crate::agents::runtime::models::{
    CreateAgentRequest, CreateSessionRequest, RuntimeEvaluateRequest,
};
use crate::agents::runtime::AgentRuntimeService;
use crate::auth::middleware::AuthContext;
use crate::auth::roles::require_operator;
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::routes::AppState;
use crate::security::write_guard::guard_mutation;

fn org(state: &AppState) -> &str {
    state.treasury.organisation_id()
}

/// POST /api/runtime/agents
pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let agent = AgentRuntimeService::create_agent(&state.db, org(&state), &ctx, req, &request_id)
        .await
        .map_err(|e| e.into_cp())?;
    Ok(Json(json!({ "agent": agent, "request_id": request_id })))
}

/// GET /api/runtime/agents
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
) -> Result<Json<Value>> {
    let agents = AgentRuntimeService::list_agents(&state.db, org(&state))
        .await
        .map_err(|e| e.into_cp())?;
    Ok(Json(json!({
        "organisation_id": org(&state),
        "agents": agents,
        "lab_only": true,
        "proto0_mutated": false,
    })))
}

/// GET /api/runtime/agents/:agent_id
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(agent_id): Path<String>,
) -> Result<Json<Value>> {
    let agent = AgentRuntimeService::get_agent(&state.db, org(&state), &agent_id)
        .await
        .map_err(|e| e.into_cp())?;
    Ok(Json(json!({ "agent": agent })))
}

/// POST /api/runtime/agents/:agent_id/freeze
pub async fn freeze_agent(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(agent_id): Path<String>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let agent =
        AgentRuntimeService::freeze_agent(&state.db, org(&state), &ctx, &agent_id, &request_id)
            .await
            .map_err(|e| e.into_cp())?;
    Ok(Json(json!({ "agent": agent, "request_id": request_id })))
}

/// POST /api/runtime/agents/:agent_id/revoke
pub async fn revoke_agent(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(agent_id): Path<String>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let agent =
        AgentRuntimeService::revoke_agent(&state.db, org(&state), &ctx, &agent_id, &request_id)
            .await
            .map_err(|e| e.into_cp())?;
    Ok(Json(json!({ "agent": agent, "request_id": request_id })))
}

/// POST /api/runtime/sessions
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let (session, credential) =
        AgentRuntimeService::create_session(&state.db, org(&state), &ctx, req, &request_id)
            .await
            .map_err(|e| e.into_cp())?;
    Ok(Json(json!({
        "session": session,
        "credential": credential,
        "request_id": request_id,
        "note": "L3 placeholder credential — provider API keys are not identity"
    })))
}

/// POST /api/runtime/sessions/:session_id/revoke
pub async fn revoke_session(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let session = AgentRuntimeService::revoke_session(
        &state.db,
        org(&state),
        &ctx,
        &session_id,
        &request_id,
    )
    .await
    .map_err(|e| e.into_cp())?;
    Ok(Json(json!({ "session": session, "request_id": request_id })))
}

/// POST /api/runtime/sessions/:session_id/expire-check
pub async fn expire_session_check(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let session = AgentRuntimeService::expire_session_check(
        &state.db,
        org(&state),
        &ctx,
        &session_id,
        &request_id,
    )
    .await
    .map_err(|e| e.into_cp())?;
    Ok(Json(json!({ "session": session, "request_id": request_id })))
}

/// POST /api/runtime/evaluate — identity + E2 only (no tools).
pub async fn evaluate_runtime_request(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(req): Json<RuntimeEvaluateRequest>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let resp = AgentRuntimeService::evaluate_request(
        &state.protocol,
        &state.treasury,
        &state.db,
        &ctx,
        org(&state),
        req,
        request_id,
    )
    .await
    .map_err(|e| e.into_cp())?;
    Ok(Json(serde_json::to_value(resp)?))
}
