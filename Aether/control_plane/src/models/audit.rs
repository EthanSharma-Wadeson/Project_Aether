use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub id: String,
    pub operator_id: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub source: String,
    pub event_type: String,
    pub agent_id: Option<String>,
    pub summary: String,
    pub timestamp: u64,
    pub metadata: Option<Value>,
}
