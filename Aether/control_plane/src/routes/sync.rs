//! Phase 24 — Apply ↔ Treasury sync observation HTTP (read-only).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use serde_json::Value;

use crate::apply_treasury_sync::SyncObservationService;
use crate::auth::middleware::AuthContext;
use crate::error::Result;
use crate::routes::AppState;

/// GET /api/sync/treasury-capabilities
pub async fn observe_org(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> Result<Json<Value>> {
    let report = SyncObservationService::observe_org(
        &state.protocol,
        &state.treasury,
        &state.db,
        &ctx.operator_id,
    )
    .await?;
    Ok(Json(serde_json::to_value(report)?))
}

/// GET /api/sync/treasury-capabilities/:agent_id
pub async fn observe_agent(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(agent_id): Path<String>,
) -> Result<Json<Value>> {
    let report = SyncObservationService::observe_agent(
        &state.protocol,
        &state.treasury,
        &state.db,
        &ctx.operator_id,
        &agent_id,
    )
    .await?;
    Ok(Json(serde_json::to_value(report)?))
}
