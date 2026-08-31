use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::error::Result;
use crate::protocol::proto3;
use crate::routes::AppState;

pub async fn get_reputation(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let metrics = proto3::get_metrics(&state.protocol, &agent_id)?;
    let events = proto3::list_events(&state.protocol, &agent_id);
    let evidence = proto3::list_evidence(&state.protocol, &agent_id);
    Ok(Json(json!({
        "agent_id": agent_id,
        "metrics": metrics,
        "events": events,
        "evidence": evidence,
    })))
}
