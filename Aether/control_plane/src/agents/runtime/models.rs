use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentStatus {
    Active,
    Frozen,
    Disabled,
    Revoked,
}

impl AgentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Frozen => "FROZEN",
            Self::Disabled => "DISABLED",
            Self::Revoked => "REVOKED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ACTIVE" | "active" => Some(Self::Active),
            "FROZEN" | "frozen" => Some(Self::Frozen),
            "DISABLED" | "disabled" => Some(Self::Disabled),
            "REVOKED" | "revoked" => Some(Self::Revoked),
            _ => None,
        }
    }

    pub fn can_create_session(self) -> bool {
        matches!(self, Self::Active)
    }

    pub fn can_act(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionStatus {
    Created,
    Active,
    Expired,
    Revoked,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "CREATED",
            Self::Active => "ACTIVE",
            Self::Expired => "EXPIRED",
            Self::Revoked => "REVOKED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "CREATED" | "created" => Some(Self::Created),
            "ACTIVE" | "active" => Some(Self::Active),
            "EXPIRED" | "expired" => Some(Self::Expired),
            "REVOKED" | "revoked" => Some(Self::Revoked),
            _ => None,
        }
    }

    pub fn is_usable(self) -> bool {
        matches!(self, Self::Created | Self::Active)
    }
}

/// Provider is metadata only — never identity / never API keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentProviderType {
    Anthropic,
    Google,
    OpenAi,
    Local,
    Enterprise,
}

impl AgentProviderType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC",
            Self::Google => "GOOGLE",
            Self::OpenAi => "OPENAI",
            Self::Local => "LOCAL",
            Self::Enterprise => "ENTERPRISE",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "ANTHROPIC" | "CLAUDE" => Some(Self::Anthropic),
            "GOOGLE" | "GEMINI" => Some(Self::Google),
            "OPENAI" => Some(Self::OpenAi),
            "LOCAL" => Some(Self::Local),
            "ENTERPRISE" => Some(Self::Enterprise),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrganisationRuntimeStatus {
    Active,
    Frozen,
}

impl OrganisationRuntimeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Frozen => "FROZEN",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ACTIVE" | "active" => Some(Self::Active),
            "FROZEN" | "frozen" => Some(Self::Frozen),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeAgent {
    pub agent_id: String,
    pub organisation_id: String,
    pub display_name: String,
    pub description: String,
    pub provider_type: AgentProviderType,
    pub model_identifier: Option<String>,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSession {
    pub session_id: String,
    pub agent_id: String,
    pub organisation_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub request_counter: i64,
    pub status: SessionStatus,
    pub credential_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgentRequest {
    pub display_name: String,
    pub description: Option<String>,
    pub provider_type: AgentProviderType,
    pub model_identifier: Option<String>,
    /// Optional stable id; otherwise generated `rt-agent-…`.
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: String,
    /// Session TTL seconds (default 1800, max 86400).
    pub ttl_secs: Option<u64>,
}

/// Placeholder L3 credential — binds agent + org + expiry + credential_id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L3Credential {
    pub credential_id: String,
    pub agent_id: String,
    pub organisation_id: String,
    pub session_id: String,
    pub expires_at: DateTime<Utc>,
    /// One-shot replay id (jti).
    pub jti: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeEvaluateRequest {
    pub credential: L3Credential,
    /// Action for E2 (no tool execution).
    pub action: String,
    pub asset_id: Option<String>,
    pub amount_minor: Option<i64>,
    pub policy_id: Option<String>,
    #[serde(default)]
    pub force_review: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEvaluateResponse {
    pub identity_valid: bool,
    pub agent_id: String,
    pub organisation_id: String,
    pub session_id: String,
    pub enforcement: serde_json::Value,
    pub tools_executed: bool,
    pub apply_invoked: bool,
    pub proto0_mutated: bool,
    pub treasury_mutated: bool,
    pub request_id: String,
}
