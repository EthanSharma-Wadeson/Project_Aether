use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct ReputationMetricsView {
    pub agent_id: String,
    pub escrow_completed: u64,
    pub escrow_refunded: u64,
    pub escrow_expired: u64,
    pub dispute_initiated: u64,
    pub dispute_lost: u64,
    pub settlement_finalized: u64,
    pub settlement_failed: u64,
    pub completion_rate: Option<f64>,
    pub settlement_success_rate: Option<f64>,
    pub dispute_rate: Option<f64>,
    pub counterparty_count: u64,
    pub first_event_time: Option<u64>,
    pub last_event_time: Option<u64>,
    pub total_settled_value: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
pub struct ReputationEventView {
    pub event_id: String,
    pub event_type: String,
    pub subject_agent_id: String,
    pub counterparty_agent_id: Option<String>,
    pub logical_time: u64,
    pub attribution: String,
}

#[derive(Debug, Serialize)]
pub struct EvidenceRefView {
    pub ref_type: String,
    pub commitment: String,
    pub source_proto: String,
    pub locator: Option<String>,
}
