use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Explicit enforcement outcomes (E2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnforcementOutcome {
    Allow,
    Deny,
    RequiresReview,
}

impl EnforcementOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "ALLOW",
            Self::Deny => "DENY",
            Self::RequiresReview => "REQUIRES_REVIEW",
        }
    }
}

/// Structured DENY taxonomy — never a generic blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DenyReason {
    CapabilityMissing,
    CapabilityRevoked,
    CapabilityActionMismatch,
    CapabilityLimitExceeded,
    AllocationMissing,
    AllocationExceeded,
    ExpiredAllocation,
    InactiveAllocation,
    PolicyBlocked,
    AgentFrozen,
    AgentRevoked,
    AgentNotFound,
    TreasuryFrozen,
    UnknownAsset,
    UnsupportedAction,
    OrganisationMismatch,
    InvalidRequest,
}

impl DenyReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityMissing => "CAPABILITY_MISSING",
            Self::CapabilityRevoked => "CAPABILITY_REVOKED",
            Self::CapabilityActionMismatch => "CAPABILITY_ACTION_MISMATCH",
            Self::CapabilityLimitExceeded => "CAPABILITY_LIMIT_EXCEEDED",
            Self::AllocationMissing => "ALLOCATION_MISSING",
            Self::AllocationExceeded => "ALLOCATION_EXCEEDED",
            Self::ExpiredAllocation => "EXPIRED_ALLOCATION",
            Self::InactiveAllocation => "INACTIVE_ALLOCATION",
            Self::PolicyBlocked => "POLICY_BLOCKED",
            Self::AgentFrozen => "AGENT_FROZEN",
            Self::AgentRevoked => "AGENT_REVOKED",
            Self::AgentNotFound => "AGENT_NOT_FOUND",
            Self::TreasuryFrozen => "TREASURY_FROZEN",
            Self::UnknownAsset => "UNKNOWN_ASSET",
            Self::UnsupportedAction => "UNSUPPORTED_ACTION",
            Self::OrganisationMismatch => "ORGANISATION_MISMATCH",
            Self::InvalidRequest => "INVALID_REQUEST",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskResult {
    Pass,
    Escalate,
    Block,
}

impl RiskResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Escalate => "ESCALATE",
            Self::Block => "BLOCK",
        }
    }
}

/// Inbound agent action intent (evaluation only — never executes).
#[derive(Debug, Clone, Deserialize)]
pub struct EnforcementRequest {
    pub agent_id: String,
    pub organisation_id: Option<String>,
    /// Action selector (e.g. `settlement.settle`, `spend`).
    pub action: String,
    /// Optional asset for spend-scoped checks.
    pub asset_id: Option<String>,
    /// Requested amount in minor units (spend-scoped).
    pub amount_minor: Option<i64>,
    /// Optional approved policy id to enforce.
    pub policy_id: Option<String>,
    /// Soft flag: force review path even if otherwise ALLOW.
    #[serde(default)]
    pub force_review: bool,
}

/// Evidence / check markers on the decision.
#[derive(Debug, Clone, Serialize)]
pub struct CheckEvidence {
    pub capability_id: Option<String>,
    pub capability_max_spend: Option<u64>,
    pub allocation_id: Option<String>,
    pub allocation_remaining_minor: Option<i64>,
    pub allocation_status: Option<String>,
    pub treasury_id: Option<String>,
    pub treasury_status: Option<String>,
    pub policy_id: Option<String>,
    pub policy_status: Option<String>,
    pub agent_status: Option<String>,
    pub effective_limit_minor: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationHint {
    /// Non-executing hint only (E1/E3 future).
    pub requires_remediation: bool,
    pub proposal_candidate: Option<String>,
    pub note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnforcementDecision {
    pub decision: EnforcementOutcome,
    pub decision_code: String,
    pub reason: Option<String>,
    pub deny_reason: Option<DenyReason>,
    pub agent_id: String,
    pub organisation_id: String,
    pub requested_action: String,
    pub capability_checked: bool,
    pub allocation_checked: bool,
    pub policy_checked: bool,
    pub risk_result: RiskResult,
    pub evidence: CheckEvidence,
    pub remediation: RemediationHint,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    /// Hard guarantees for operators.
    pub observation_only_execution: bool,
    pub apply_invoked: bool,
    pub treasury_mutated: bool,
    pub proto0_mutated: bool,
}

/// Snapshot inputs for pure evaluation (no I/O).
#[derive(Debug, Clone)]
pub struct AgentAuthoritySnapshot {
    pub agent_id: String,
    pub agent_status: String,
    pub capabilities: Vec<CapabilitySnap>,
    pub allocations: Vec<AllocationSnap>,
    pub treasury_status_by_id: std::collections::HashMap<String, String>,
    pub known_assets: Vec<String>,
    pub policy: Option<PolicySnap>,
}

#[derive(Debug, Clone)]
pub struct CapabilitySnap {
    pub capability_id: String,
    pub actions: Vec<String>,
    pub max_spend: Option<u64>,
    pub asset: Option<String>,
    pub revoked: bool,
}

#[derive(Debug, Clone)]
pub struct AllocationSnap {
    pub allocation_id: String,
    pub organisation_id: String,
    pub treasury_id: String,
    pub agent_id: String,
    pub asset_id: String,
    pub ceiling_minor: i64,
    pub remaining_minor: i64,
    pub status: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PolicySnap {
    pub policy_id: String,
    pub status: String,
    pub policy_type: String,
    pub blocked_actions: Vec<String>,
}
