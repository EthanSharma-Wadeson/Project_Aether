//! Apply approval domain types.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Apply approval TTL — `APPLY_APPROVAL_TTL` (60 minutes).
pub const DEFAULT_APPROVAL_TTL: Duration = Duration::minutes(60);

/// Approval lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Active,
    Expired,
    Consumed,
    Cancelled,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Consumed => "consumed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "expired" => Some(Self::Expired),
            "consumed" => Some(Self::Consumed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Active)
    }
}

/// Durable Apply approval — authorisation binding only (no protocol mutation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyApproval {
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub operation_intent: String,
    pub approved_by: String,
    pub approved_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: ApprovalStatus,
    pub consumed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub audit_reference: Option<String>,
    /// Request that granted the approval.
    pub request_id: String,
    /// Approver RBAC role at grant time.
    pub approver_role: String,
    /// Set when consumed by replay reserve (Phase 2+).
    pub consumed_by_operation_id: Option<String>,
    pub cancelled_by: Option<String>,
}

/// Inputs to create an approval.
#[derive(Debug, Clone)]
pub struct CreateApprovalRequest {
    pub approval_id: Option<String>,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub operation_intent: String,
    pub approved_by: String,
    pub approver_role: String,
    pub request_id: String,
    pub audit_reference: Option<String>,
    pub ttl: Option<Duration>,
}

/// Binding checked by `validate_approval`.
#[derive(Debug, Clone)]
pub struct ApprovalBinding {
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_version: i64,
}
