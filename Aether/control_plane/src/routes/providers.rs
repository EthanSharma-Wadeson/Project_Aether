//! Phase 32/34 — Lab provider adapter HTTP (mock + external Claude lab).

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use serde_json::{json, Value};

use crate::auth::middleware::AuthContext;
use crate::auth::roles::require_operator;
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::providers::external::run_agent_a_external_demo;
use crate::providers::governors;
use crate::providers::models::ProviderReasonHttpRequest;
use crate::providers::ProviderAdapterService;
use crate::routes::AppState;
use crate::security::write_guard::guard_mutation;

fn org(state: &AppState) -> &str {
    state.treasury.organisation_id()
}

/// POST /api/providers/lab/reason
/// Mock or external lab provider proposes intents; Aether governs → sandbox only.
pub async fn lab_reason(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(body): Json<ProviderReasonHttpRequest>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let result = ProviderAdapterService::reason_and_govern(
        &state.protocol,
        &state.treasury,
        &state.db,
        &ctx,
        body,
        request_id,
    )
    .await
    .map_err(|e| e.into_cp())?;
    Ok(Json(serde_json::to_value(result)?))
}

/// POST /api/providers/lab/demo/agent-a-research
/// Agent A + Anthropic lab (live if AETHER_LAB_* key set). No real tools/money.
pub async fn demo_agent_a_research(
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
    let live = std::env::var("AETHER_LAB_ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("AETHER_LAB_PROVIDER_API_KEY"))
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if !live {
        return Err(crate::error::Error::BadRequest(
            "set AETHER_LAB_ANTHROPIC_API_KEY (or AETHER_LAB_PROVIDER_API_KEY) for live demo; use tests with ScriptedLabTransport otherwise".into(),
        ));
    }
    let report = run_agent_a_external_demo(
        &state.protocol,
        &state.treasury,
        &state.db,
        &ctx,
        None,
        true,
    )
    .await
    .map_err(|e| e.into_cp())?;
    Ok(Json(json!({
        "request_id": request_id,
        "report": report,
        "production_mode": false,
        "real_tool_execution": false,
        "real_money": false,
    })))
}

/// POST /api/providers/lab/sessions/:session_id/cancel
pub async fn cancel_session_governor(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let g = governors::cancel(state.db.pool(), org(&state), &session_id)
        .await
        .map_err(|e| e.into_cp())?;
    Ok(Json(json!({
        "request_id": request_id,
        "governor": g.snapshot(),
        "cancelled": true,
    })))
}
