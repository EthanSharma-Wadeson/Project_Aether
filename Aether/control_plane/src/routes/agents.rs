use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::auth::middleware::AuthContext;
use crate::db::audit;
use crate::error::Result;
use crate::models::audit::TimelineEvent;
use crate::protocol::{proto0, proto2, proto3, proto4};
use crate::routes::AppState;

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
) -> Result<Json<serde_json::Value>> {
    audit::append(
        state.db.pool(),
        Some(&ctx.operator_id),
        "observation.agents.list",
        None,
        None,
    )
    .await?;
    let agents = proto0::list_agents(&state.protocol);
    Ok(Json(
        json!({ "agents": agents, "read_at": state.protocol.loaded_at }),
    ))
}

pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    audit::append(
        state.db.pool(),
        Some(&ctx.operator_id),
        "observation.agent.view",
        Some(&id),
        None,
    )
    .await?;
    let agent = proto0::get_agent(&state.protocol, &id)?;
    Ok(Json(json!(agent)))
}

pub async fn agent_capabilities(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let capabilities = proto0::capabilities_for_agent(&state.protocol, &id);
    Ok(Json(
        json!({ "agent_id": id, "capabilities": capabilities }),
    ))
}

pub async fn agent_escrows(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let escrows = proto2::escrows_for_agent(&state.protocol, &id);
    Ok(Json(json!({ "agent_id": id, "escrows": escrows })))
}

pub async fn agent_settlements(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let settlements = proto4::settlements_for_agent(&state.protocol, &id);
    Ok(Json(json!({ "agent_id": id, "settlements": settlements })))
}

pub async fn agent_reputation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let metrics = proto3::get_metrics(&state.protocol, &id)?;
    let events = proto3::list_events(&state.protocol, &id);
    let evidence = proto3::list_evidence(&state.protocol, &id);
    Ok(Json(json!({
        "agent_id": id,
        "metrics": metrics,
        "events": events,
        "evidence": evidence,
    })))
}

pub async fn agent_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let mut timeline = Vec::new();

    if let Ok(agent) = proto0::get_agent(&state.protocol, &id) {
        timeline.push(TimelineEvent {
            source: "PROTO-0".into(),
            event_type: "identity.registered".into(),
            agent_id: Some(id.clone()),
            summary: format!("Agent registered with status {}", agent.status),
            timestamp: agent.registered_at,
            metadata: None,
        });
    }

    for cap in proto0::capabilities_for_agent(&state.protocol, &id) {
        timeline.push(TimelineEvent {
            source: "PROTO-0".into(),
            event_type: "capability.grant".into(),
            agent_id: Some(id.clone()),
            summary: format!(
                "Capability {} actions={:?} revoked={}",
                cap.capability_id, cap.actions, cap.revoked
            ),
            timestamp: cap.valid_after.unwrap_or(0),
            metadata: Some(json!({ "capability_id": cap.capability_id })),
        });
    }

    for escrow in proto2::escrows_for_agent(&state.protocol, &id) {
        timeline.push(TimelineEvent {
            source: "PROTO-2".into(),
            event_type: format!("escrow.{}", escrow.status),
            agent_id: Some(id.clone()),
            summary: format!(
                "Escrow {} amount={} soft={} hard={}",
                escrow.escrow_id,
                escrow.principal_amount,
                escrow.soft_finality,
                escrow.hard_finality
            ),
            timestamp: escrow.created_at,
            metadata: Some(json!({ "escrow_id": escrow.escrow_id })),
        });
    }

    for settlement in proto4::settlements_for_agent(&state.protocol, &id) {
        timeline.push(TimelineEvent {
            source: "PROTO-4".into(),
            event_type: format!("settlement.{}", settlement.settlement_status),
            agent_id: Some(id.clone()),
            summary: format!(
                "Settlement {} soft={} hard={}",
                settlement.binding_id, settlement.soft_finality, settlement.hard_finality
            ),
            timestamp: settlement.updated_at,
            metadata: Some(json!({
                "binding_id": settlement.binding_id,
                "external_ref": settlement.external_settlement_ref,
            })),
        });
    }

    for event in proto3::list_events(&state.protocol, &id) {
        timeline.push(TimelineEvent {
            source: "PROTO-3".into(),
            event_type: event.event_type.clone(),
            agent_id: Some(id.clone()),
            summary: format!("Reputation event {}", event.event_id),
            timestamp: event.logical_time,
            metadata: Some(json!({ "event_id": event.event_id })),
        });
    }

    timeline.sort_by_key(|e| e.timestamp);

    Ok(Json(json!({ "agent_id": id, "events": timeline })))
}
