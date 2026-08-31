use aether_core::reputation::model::Attribution;

use crate::error::{Error, Result};
use crate::models::reputation::{EvidenceRefView, ReputationEventView, ReputationMetricsView};
use crate::protocol::state::ProtocolState;

pub fn get_metrics(state: &ProtocolState, agent_id: &str) -> Result<ReputationMetricsView> {
    let metrics = state
        .reputation
        .get_metrics(&agent_id.into())
        .ok_or_else(|| Error::NotFound(format!("reputation metrics for {agent_id}")))?;
    Ok(ReputationMetricsView {
        agent_id: agent_id.into(),
        escrow_completed: metrics.escrow_completed,
        escrow_refunded: metrics.escrow_refunded,
        escrow_expired: metrics.escrow_expired,
        dispute_initiated: metrics.dispute_initiated,
        dispute_lost: metrics.dispute_lost,
        settlement_finalized: metrics.settlement_finalized,
        settlement_failed: metrics.settlement_failed,
        completion_rate: metrics.completion_rate(),
        settlement_success_rate: metrics.settlement_success_rate(),
        dispute_rate: metrics.dispute_rate(),
        counterparty_count: metrics.counterparty_count,
        first_event_time: metrics.first_event_time,
        last_event_time: metrics.last_event_time,
        total_settled_value: metrics.total_settled_value.clone(),
    })
}

pub fn list_events(state: &ProtocolState, agent_id: &str) -> Vec<ReputationEventView> {
    state
        .reputation
        .agent_events(&agent_id.into())
        .into_iter()
        .map(|e| ReputationEventView {
            event_id: hex::encode(e.event_id),
            event_type: e.event_type.as_str().into(),
            subject_agent_id: e.subject_agent_id.clone(),
            counterparty_agent_id: e.counterparty_agent_id.clone(),
            logical_time: e.logical_time,
            attribution: attribution_str(e.attribution),
        })
        .collect()
}

pub fn list_evidence(state: &ProtocolState, agent_id: &str) -> Vec<EvidenceRefView> {
    state
        .reputation
        .agent_events(&agent_id.into())
        .into_iter()
        .flat_map(|e| {
            e.evidence_refs.iter().map(|r| EvidenceRefView {
                ref_type: r.ref_type.as_str().into(),
                commitment: hex::encode(r.commitment),
                source_proto: r.source_proto.as_str().into(),
                locator: r.locator.map(hex::encode),
            })
        })
        .collect()
}

fn attribution_str(a: Attribution) -> String {
    match a {
        Attribution::Credit => "credit".into(),
        Attribution::Neutral => "neutral".into(),
        Attribution::Fault => "fault".into(),
        Attribution::Debit => "debit".into(),
    }
}
