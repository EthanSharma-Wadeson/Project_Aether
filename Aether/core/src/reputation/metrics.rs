//! Deterministic metric computation from reputation events.

use std::collections::HashMap;

use crate::types::AgentId;

use super::model::{Attribution, EventType, ReputationEventV0};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMetricsV0 {
    pub agent_id: AgentId,
    pub escrow_completed: u64,
    pub escrow_refunded: u64,
    pub escrow_expired: u64,
    pub dispute_initiated: u64,
    pub dispute_lost: u64,
    pub settlement_finalized: u64,
    pub settlement_failed: u64,
    pub channel_closed_cooperative: u64,
    pub channel_dispute_lost: u64,
    pub replay_reject_count: u64,
    pub identity_revoked: bool,
    /// Per-asset settled value totals.
    pub total_settled_value: HashMap<String, u64>,
    settlement_latency_sum: u64,
    settlement_latency_count: u64,
    pub first_event_time: Option<u64>,
    pub last_event_time: Option<u64>,
    /// Distinct counterparties observed.
    pub counterparty_count: u64,
    /// (counterparty, count) for dyad concentration analysis.
    pub dyad_counts: HashMap<AgentId, u64>,
}

impl AgentMetricsV0 {
    pub fn empty(agent_id: &AgentId) -> Self {
        Self {
            agent_id: agent_id.clone(),
            escrow_completed: 0,
            escrow_refunded: 0,
            escrow_expired: 0,
            dispute_initiated: 0,
            dispute_lost: 0,
            settlement_finalized: 0,
            settlement_failed: 0,
            channel_closed_cooperative: 0,
            channel_dispute_lost: 0,
            replay_reject_count: 0,
            identity_revoked: false,
            total_settled_value: HashMap::new(),
            settlement_latency_sum: 0,
            settlement_latency_count: 0,
            first_event_time: None,
            last_event_time: None,
            counterparty_count: 0,
            dyad_counts: HashMap::new(),
        }
    }

    pub fn compute(agent_id: &AgentId, events: &[&ReputationEventV0]) -> Self {
        let mut m = Self::empty(agent_id);
        let mut counterparties = std::collections::HashSet::new();

        for event in events {
            let is_subject = event.subject_agent_id == *agent_id;

            // Track time bounds.
            match m.first_event_time {
                None => m.first_event_time = Some(event.logical_time),
                Some(t) if event.logical_time < t => m.first_event_time = Some(event.logical_time),
                _ => {}
            }
            match m.last_event_time {
                None => m.last_event_time = Some(event.logical_time),
                Some(t) if event.logical_time > t => m.last_event_time = Some(event.logical_time),
                _ => {}
            }

            // Track counterparties.
            if is_subject {
                if let Some(ref cp) = event.counterparty_agent_id {
                    if counterparties.insert(cp.clone()) {
                        m.counterparty_count += 1;
                    }
                    *m.dyad_counts.entry(cp.clone()).or_default() += 1;
                }
            }

            match event.event_type {
                EventType::EscrowReleased if is_subject => {
                    if matches!(event.attribution, Attribution::Credit) {
                        m.escrow_completed += 1;
                    }
                }
                EventType::EscrowRefunded if is_subject => {
                    if matches!(event.attribution, Attribution::Fault) {
                        m.escrow_refunded += 1;
                    }
                }
                EventType::EscrowExpired if is_subject => {
                    m.escrow_expired += 1;
                }
                EventType::EscrowDisputeRaised if is_subject => {
                    m.dispute_initiated += 1;
                }
                EventType::EscrowDisputeResolved => {
                    if is_subject && matches!(event.attribution, Attribution::Fault) {
                        m.dispute_lost += 1;
                    }
                }
                EventType::SettlementFinalized if is_subject => {
                    m.settlement_finalized += 1;
                    if let Some(w) = event.weight_hint {
                        *m.total_settled_value
                            .entry("default".to_string())
                            .or_default() += w;
                    }
                }
                EventType::SettlementFailed if is_subject => {
                    m.settlement_failed += 1;
                }
                EventType::ChannelClosedCooperative => {
                    m.channel_closed_cooperative += 1;
                }
                EventType::ChannelDisputeLost => {
                    if is_subject && matches!(event.attribution, Attribution::Fault) {
                        m.channel_dispute_lost += 1;
                    }
                }
                EventType::IntegrityReplayRejected if is_subject => {
                    m.replay_reject_count += 1;
                }
                EventType::IdentityRevoked if is_subject => {
                    m.identity_revoked = true;
                }
                _ => {}
            }
        }

        m
    }

    /// `disputes / (completed + disputed)` — returns None if denominator is zero.
    pub fn dispute_rate(&self) -> Option<f64> {
        let denom = self.escrow_completed + self.dispute_initiated;
        if denom == 0 {
            return None;
        }
        Some(self.dispute_initiated as f64 / denom as f64)
    }

    /// `finalized / (finalized + failed)` — returns None if denominator is zero.
    pub fn settlement_success_rate(&self) -> Option<f64> {
        let denom = self.settlement_finalized + self.settlement_failed;
        if denom == 0 {
            return None;
        }
        Some(self.settlement_finalized as f64 / denom as f64)
    }

    /// `escrow_completed / (completed + refunded + expired)` — returns None if zero.
    pub fn completion_rate(&self) -> Option<f64> {
        let denom = self.escrow_completed + self.escrow_refunded + self.escrow_expired;
        if denom == 0 {
            return None;
        }
        Some(self.escrow_completed as f64 / denom as f64)
    }

    /// Maximum fraction of events with any single counterparty.
    pub fn max_dyad_concentration(&self) -> Option<f64> {
        if self.dyad_counts.is_empty() {
            return None;
        }
        let total: u64 = self.dyad_counts.values().sum();
        let max = self.dyad_counts.values().max().copied().unwrap_or(0);
        if total == 0 {
            return None;
        }
        Some(max as f64 / total as f64)
    }
}
