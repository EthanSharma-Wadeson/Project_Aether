use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use crate::error::Result;
use crate::models::system::SystemHealth;
use crate::routes::AppState;

pub async fn health(State(state): State<Arc<AppState>>) -> Result<Json<SystemHealth>> {
    Ok(Json(SystemHealth {
        status: "ok".into(),
        protocol_agents_indexed: state.protocol.index.agent_ids.len(),
        protocol_capabilities_indexed: state.protocol.index.capability_ids.len(),
        protocol_escrows_indexed: state.protocol.index.escrow_ids.len(),
        protocol_settlements_indexed: state.protocol.index.settlement_binding_ids.len(),
        reputation_events: state.protocol.reputation.event_count(),
        read_only_mode: true,
        loaded_at: state.protocol.loaded_at.to_rfc3339(),
    }))
}
