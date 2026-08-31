//! Phase 30 — Lab tool registry & decision gateway HTTP (no execution).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::{Extension, Json};
use serde_json::{json, Value};

use crate::auth::middleware::AuthContext;
use crate::auth::roles::require_operator;
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::routes::AppState;
use crate::security::write_guard::guard_mutation;
use crate::tools::runtime::models::{CreateToolRequest, ToolEvaluateHttpRequest};
use crate::tools::runtime::ToolGatewayService;

fn org(state: &AppState) -> &str {
    state.treasury.organisation_id()
}

/// POST /api/tools — register tool (lab).
pub async fn create_tool(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(req): Json<CreateToolRequest>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let tool = ToolGatewayService::create_tool(&state.db, org(&state), &ctx, req)
        .await
        .map_err(|e| e.into_cp())?;
    Ok(Json(json!({ "tool": tool, "request_id": request_id })))
}

/// GET /api/tools
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
) -> Result<Json<Value>> {
    let tools = ToolGatewayService::list_tools(&state.db, org(&state))
        .await
        .map_err(|e| e.into_cp())?;
    Ok(Json(json!({
        "organisation_id": org(&state),
        "tools": tools,
        "execution_enabled": false,
    })))
}

/// POST /api/tools/:tool_id/disable
pub async fn disable_tool(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(tool_id): Path<String>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let tool = ToolGatewayService::disable_tool(&state.db, org(&state), &tool_id)
        .await
        .map_err(|e| e.into_cp())?;
    Ok(Json(json!({ "tool": tool, "request_id": request_id })))
}

/// POST /api/tools/evaluate — decision only.
pub async fn evaluate_tool(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(req): Json<ToolEvaluateHttpRequest>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let decision = ToolGatewayService::evaluate(
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
    Ok(Json(json!({
        "decision": decision.decision,
        "reason": decision.reason,
        "request_id": decision.request_id,
        "agent_id": decision.agent_id,
        "tool_id": decision.tool_id,
        "decision_code": decision.decision_code,
        "evidence": decision.evidence,
        "tools_executed": decision.tools_executed,
        "apply_invoked": decision.apply_invoked,
        "proto0_mutated": decision.proto0_mutated,
        "treasury_mutated": decision.treasury_mutated,
    })))
}
