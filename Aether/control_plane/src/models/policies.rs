//! Policy template models — Control Plane records only (no protocol effect).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyStatus {
    Draft,
    PendingReview,
    Approved,
    Rejected,
    Archived,
}

impl PolicyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> crate::error::Result<Self> {
        match s {
            "draft" => Ok(Self::Draft),
            "pending_review" => Ok(Self::PendingReview),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "archived" => Ok(Self::Archived),
            _ => Err(crate::error::Error::BadRequest(format!(
                "unknown policy status: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub target_agent_id: Option<String>,
    pub policy_type: String,
    pub policy_data: Value,
    pub status: PolicyStatus,
    pub created_by: String,
    pub created_by_username: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitted_by: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by: Option<String>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub version: i64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyTemplateVersion {
    pub id: String,
    pub policy_id: String,
    pub version: i64,
    pub name: String,
    pub description: String,
    pub target_agent_id: Option<String>,
    pub policy_type: String,
    pub policy_data: Value,
    pub status: PolicyStatus,
    pub hash: String,
    pub changed_by: String,
    pub changed_at: DateTime<Utc>,
    pub change_action: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub target_agent_id: Option<String>,
    pub policy_type: String,
    pub policy_data: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub target_agent_id: Option<String>,
    pub policy_type: Option<String>,
    pub policy_data: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RejectPolicyRequest {
    pub reason: Option<String>,
}
