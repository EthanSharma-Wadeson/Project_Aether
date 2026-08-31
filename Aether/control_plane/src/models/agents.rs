use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AgentSummary {
    pub agent_id: String,
    pub status: String,
    pub registered_at: u64,
    pub active_capabilities: usize,
    pub escrow_count: usize,
    pub settlement_count: usize,
    pub has_reputation_metrics: bool,
}

#[derive(Debug, Serialize)]
pub struct AgentDetail {
    pub agent_id: String,
    pub status: String,
    pub registered_at: u64,
    pub operational_public_key: String,
    pub root_version: u64,
    pub activity_summary: AgentActivitySummary,
    pub read_at: String,
}

#[derive(Debug, Serialize)]
pub struct AgentActivitySummary {
    pub active_capabilities: usize,
    pub escrows: usize,
    pub settlements: usize,
    pub reputation_events: usize,
}
