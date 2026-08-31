use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "LOW" => Some(Self::Low),
            "MEDIUM" => Some(Self::Medium),
            "HIGH" => Some(Self::High),
            "CRITICAL" => Some(Self::Critical),
            _ => None,
        }
    }

    pub fn forces_review(self) -> bool {
        matches!(self, Self::Critical)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolStatus {
    Active,
    Disabled,
    Restricted,
}

impl ToolStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Disabled => "DISABLED",
            Self::Restricted => "RESTRICTED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "ACTIVE" => Some(Self::Active),
            "DISABLED" => Some(Self::Disabled),
            "RESTRICTED" => Some(Self::Restricted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayOutcome {
    Allow,
    Deny,
    RequiresReview,
}

impl GatewayOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "ALLOW",
            Self::Deny => "DENY",
            Self::RequiresReview => "REQUIRES_REVIEW",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolRecord {
    pub tool_id: String,
    pub organisation_id: String,
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub required_capability: String,
    pub status: ToolStatus,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateToolRequest {
    pub tool_id: String,
    pub name: String,
    pub description: Option<String>,
    pub risk_level: RiskLevel,
    pub required_capability: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolRequest {
    pub request_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub tool_id: String,
    pub parameters_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolEvaluateHttpRequest {
    pub agent_id: String,
    pub session_id: String,
    pub tool_id: String,
    #[serde(default)]
    pub parameters: Value,
    /// Optional client-supplied id for idempotency/replay.
    pub request_id: Option<String>,
    pub amount_minor: Option<i64>,
    pub asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEvidence {
    pub agent_status: Option<String>,
    pub session_status: Option<String>,
    pub tool_status: Option<String>,
    pub risk_level: Option<String>,
    pub required_capability: Option<String>,
    pub parameters_hash: String,
    pub e2_decision: Option<String>,
    pub e2_deny_reason: Option<String>,
    pub organisation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayDecision {
    pub decision: GatewayOutcome,
    pub decision_code: String,
    pub reason: String,
    pub request_id: String,
    pub agent_id: String,
    pub tool_id: String,
    pub session_id: String,
    pub evidence: GatewayEvidence,
    pub tools_executed: bool,
    pub apply_invoked: bool,
    pub proto0_mutated: bool,
    pub treasury_mutated: bool,
}
