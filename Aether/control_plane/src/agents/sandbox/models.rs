use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SandboxAgentStatus {
    Active,
    Stopped,
}

impl SandboxAgentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Stopped => "STOPPED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ACTIVE" | "active" => Some(Self::Active),
            "STOPPED" | "stopped" => Some(Self::Stopped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SandboxRuntimeType {
    ResearchAssistant,
    FinanceAssistant,
    Generic,
}

impl SandboxRuntimeType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ResearchAssistant => "RESEARCH_ASSISTANT",
            Self::FinanceAssistant => "FINANCE_ASSISTANT",
            Self::Generic => "GENERIC",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "RESEARCH_ASSISTANT" => Some(Self::ResearchAssistant),
            "FINANCE_ASSISTANT" => Some(Self::FinanceAssistant),
            "GENERIC" => Some(Self::Generic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxAgent {
    pub agent_id: String,
    pub organisation_id: String,
    pub session_id: String,
    pub runtime_type: SandboxRuntimeType,
    pub status: SandboxAgentStatus,
    /// Scenario key: `agent_a` | `agent_b`
    pub scenario: String,
    /// £1 simulated = 100 minor — metadata only, not a wallet / treasury write.
    pub simulated_budget_minor: i64,
    pub sandbox_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SimulatedResult {
    Success,
    Stopped,
    ReviewPending,
}

impl SimulatedResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::Stopped => "STOPPED",
            Self::ReviewPending => "REVIEW_PENDING",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "SUCCESS" => Some(Self::Success),
            "STOPPED" => Some(Self::Stopped),
            "REVIEW_PENDING" => Some(Self::ReviewPending),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SandboxTaskRequest {
    /// `agent_a` | `agent_b` | explicit agent_id
    pub scenario: Option<String>,
    pub agent_id: Option<String>,
    pub tool_id: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
    pub request_id: Option<String>,
    pub amount_minor: Option<i64>,
    pub asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SandboxRunResult {
    pub request_id: String,
    pub agent_id: String,
    pub organisation_id: String,
    pub session_id: String,
    pub tool_id: String,
    pub decision: String,
    pub reason: String,
    pub simulated_result: SimulatedResult,
    pub review_event_id: Option<String>,
    pub evidence: serde_json::Value,
    pub external_execution: bool,
    pub apply_invoked: bool,
    pub proto0_mutated: bool,
    pub treasury_mutated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReconstruction {
    pub request_id: String,
    pub agent_id: String,
    pub organisation_id: String,
    pub session_id: String,
    pub tool_id: String,
    pub decision: String,
    pub reason: String,
    pub simulated_result: String,
    pub review_event_id: Option<String>,
    pub created_at: String,
}
