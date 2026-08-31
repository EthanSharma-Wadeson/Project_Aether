use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SystemHealth {
    pub status: String,
    pub protocol_agents_indexed: usize,
    pub protocol_capabilities_indexed: usize,
    pub protocol_escrows_indexed: usize,
    pub protocol_settlements_indexed: usize,
    pub reputation_events: usize,
    pub read_only_mode: bool,
    pub loaded_at: String,
}
