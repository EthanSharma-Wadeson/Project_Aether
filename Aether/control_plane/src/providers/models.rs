use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agents::runtime::models::AgentProviderType;

/// Lab provider family — metadata only, never identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderKind {
    Anthropic,
    Google,
    OpenAi,
    Local,
    /// Deterministic mock used in Phase 32 lab (not a live network provider).
    Mock,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC",
            Self::Google => "GOOGLE",
            Self::OpenAi => "OPENAI",
            Self::Local => "LOCAL",
            Self::Mock => "MOCK",
        }
    }

    pub fn from_agent_provider(p: AgentProviderType) -> Self {
        match p {
            AgentProviderType::Anthropic => Self::Anthropic,
            AgentProviderType::Google => Self::Google,
            AgentProviderType::OpenAi => Self::OpenAi,
            AgentProviderType::Local => Self::Local,
            AgentProviderType::Enterprise => Self::Local,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub provider_kind: ProviderKind,
    pub model_id: String,
    /// Observability only — never used as agent_id.
    pub provider_call_id: String,
}

/// Trusted caller context — NOT taken from model output.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderReasonHttpRequest {
    pub agent_id: String,
    pub session_id: String,
    /// Which lab mock to invoke: `research` | `finance` | `hostile` (tests).
    pub mock_profile: String,
    pub prompt: String,
    /// Optional correlation id from client; otherwise generated.
    pub correlation_id: Option<String>,
    /// Optional secret reference handle (never a raw API key).
    pub secret_ref: Option<String>,
    pub amount_minor: Option<i64>,
    pub asset_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderReasonRequest {
    pub prompt: String,
    pub correlation_id: String,
    pub secret_ref: Option<crate::providers::secrets::SecretRef>,
    pub provider_call_id: String,
}

/// Untrusted model output — proposals only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedIntent {
    pub tool_id: String,
    #[serde(default)]
    pub parameters: Value,
    /// Ignored if present — models cannot set identity.
    #[serde(default)]
    pub claimed_agent_id: Option<String>,
    /// Ignored — models cannot set session.
    #[serde(default)]
    pub claimed_session_id: Option<String>,
    /// Ignored — models cannot create authority.
    #[serde(default)]
    pub claimed_capability: Option<String>,
    /// Ignored — models cannot bypass E2.
    #[serde(default)]
    pub bypass_e2: bool,
    /// Ignored — models cannot assert decision.
    #[serde(default)]
    pub claimed_decision: Option<String>,
    /// Ignored — models cannot access treasury/protocol.
    #[serde(default)]
    pub access_treasury: bool,
    #[serde(default)]
    pub access_protocol: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderReasonResponse {
    pub metadata: ProviderMetadata,
    pub reasoning_summary: String,
    pub proposed_intents: Vec<ProposedIntent>,
    /// Always false — providers never execute.
    pub tools_executed: bool,
    pub raw_redacted: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GovernedProviderResult {
    pub correlation_id: String,
    pub provider_call_id: String,
    pub provider_kind: String,
    pub model_id: String,
    pub agent_id: String,
    pub organisation_id: String,
    pub session_id: String,
    pub tool_id: Option<String>,
    pub decision: String,
    pub reason: String,
    pub simulated_result: Option<String>,
    pub review_event_id: Option<String>,
    pub governor: GovernorSnapshot,
    pub evidence: Value,
    pub external_execution: bool,
    pub apply_invoked: bool,
    pub proto0_mutated: bool,
    pub treasury_mutated: bool,
    pub tools_executed: bool,
    pub provider_has_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorSnapshot {
    pub action_count: i64,
    pub max_actions: i64,
    pub cancelled: bool,
    pub circuit_breaker: String,
    pub expires_at: DateTime<Utc>,
}
