use aether_core::capability::model::CapabilityRecord;
use aether_core::types::AgentStatus;

use crate::error::{Error, Result};
use crate::models::agents::{AgentActivitySummary, AgentDetail, AgentSummary};
use crate::models::capabilities::CapabilityView;
use crate::protocol::state::ProtocolState;

pub fn list_agents(state: &ProtocolState) -> Vec<AgentSummary> {
    state
        .index
        .agent_ids
        .iter()
        .filter_map(|id| agent_summary(state, id).ok())
        .collect()
}

pub fn get_agent(state: &ProtocolState, agent_id: &str) -> Result<AgentDetail> {
    let entry = state
        .registry
        .get(agent_id)
        .ok_or_else(|| Error::NotFound(format!("agent {agent_id}")))?;

    let caps = capabilities_for_agent(state, agent_id);
    let escrows = escrows_for_agent(state, agent_id);
    let settlements = settlements_for_agent(state, agent_id);

    Ok(AgentDetail {
        agent_id: agent_id.into(),
        status: status_str(entry.status),
        registered_at: entry.registered_at,
        operational_public_key: hex::encode(&entry.identity.operational_public_key),
        root_version: entry.permission_root_material.root_version,
        activity_summary: AgentActivitySummary {
            active_capabilities: caps.len(),
            escrows: escrows.len(),
            settlements: settlements.len(),
            reputation_events: state.reputation.agent_events(&agent_id.into()).len(),
        },
        read_at: state.loaded_at.to_rfc3339(),
    })
}

fn agent_summary(state: &ProtocolState, agent_id: &str) -> Result<AgentSummary> {
    let entry = state
        .registry
        .get(agent_id)
        .ok_or_else(|| Error::NotFound(format!("agent {agent_id}")))?;
    let caps = capabilities_for_agent(state, agent_id);
    let escrows = escrows_for_agent(state, agent_id);
    let settlements = settlements_for_agent(state, agent_id);
    Ok(AgentSummary {
        agent_id: agent_id.into(),
        status: status_str(entry.status),
        registered_at: entry.registered_at,
        active_capabilities: caps.len(),
        escrow_count: escrows.len(),
        settlement_count: settlements.len(),
        has_reputation_metrics: state.reputation.get_metrics(&agent_id.into()).is_some(),
    })
}

pub fn list_capabilities(state: &ProtocolState) -> Vec<CapabilityView> {
    state
        .index
        .capability_ids
        .iter()
        .filter_map(|id| capability_view(state, id).ok())
        .collect()
}

pub fn capabilities_for_agent(state: &ProtocolState, agent_id: &str) -> Vec<CapabilityView> {
    list_capabilities(state)
        .into_iter()
        .filter(|c| c.subject_agent_id == agent_id)
        .collect()
}

pub fn get_capability(state: &ProtocolState, capability_id: &[u8; 32]) -> Result<CapabilityView> {
    capability_view(state, capability_id)
}

fn capability_view(state: &ProtocolState, capability_id: &[u8; 32]) -> Result<CapabilityView> {
    let record = state
        .caps
        .get(capability_id)
        .ok_or_else(|| Error::NotFound("capability".into()))?;
    Ok(to_capability_view(
        record,
        state.caps.is_revoked(capability_id),
    ))
}

fn to_capability_view(record: &CapabilityRecord, revoked: bool) -> CapabilityView {
    let subject_agent_id = match &record.body.subject {
        aether_core::types::SubjectRef::AgentId(id) => id.clone(),
        aether_core::types::SubjectRef::PublicKey(pk) => format!("pubkey:{}", hex::encode(pk)),
    };
    CapabilityView {
        capability_id: hex::encode(record.capability_id),
        issuer: record.body.issuer.clone(),
        subject_agent_id,
        actions: record.body.actions.clone(),
        max_spend: record.body.constraints.max_spend,
        asset: record.body.constraints.asset.clone(),
        valid_after: record.body.constraints.valid_after,
        valid_before: record.body.constraints.valid_before,
        delegation_depth: record.body.delegation_depth,
        revoked,
        granted_under_root_version: record.granted_under_root_version,
    }
}

pub fn escrows_for_agent(state: &ProtocolState, agent_id: &str) -> Vec<String> {
    state
        .index
        .escrow_ids
        .iter()
        .filter(|id| {
            state
                .escrows
                .get(id)
                .map(|r| r.escrow.terms.payer == agent_id || r.escrow.terms.provider == agent_id)
                .unwrap_or(false)
        })
        .map(hex::encode)
        .collect()
}

pub fn settlements_for_agent(state: &ProtocolState, agent_id: &str) -> Vec<String> {
    state
        .index
        .settlement_binding_ids
        .iter()
        .filter(|id| {
            state
                .settlements
                .get_settlement(id)
                .map(|b| b.payer_agent_id == agent_id || b.provider_agent_id == agent_id)
                .unwrap_or(false)
        })
        .map(hex::encode)
        .collect()
}

fn status_str(status: AgentStatus) -> String {
    match status {
        AgentStatus::Active => "active".into(),
        AgentStatus::Frozen => "frozen".into(),
        AgentStatus::Revoked => "revoked".into(),
    }
}
