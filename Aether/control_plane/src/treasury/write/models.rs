use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreasuryOperation {
    AllocationCreate,
    AllocationIncrease,
    AllocationDecrease,
    Reserve,
    Release,
    SettlementPost,
    Refund,
    Chargeback,
    Adjustment,
    Freeze,
    Unfreeze,
}

impl TreasuryOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllocationCreate => "allocation_create",
            Self::AllocationIncrease => "allocation_increase",
            Self::AllocationDecrease => "allocation_decrease",
            Self::Reserve => "reserve",
            Self::Release => "release",
            Self::SettlementPost => "settlement_post",
            Self::Refund => "refund",
            Self::Chargeback => "chargeback",
            Self::Adjustment => "adjustment",
            Self::Freeze => "freeze",
            Self::Unfreeze => "unfreeze",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allocation_create" => Some(Self::AllocationCreate),
            "allocation_increase" => Some(Self::AllocationIncrease),
            "allocation_decrease" => Some(Self::AllocationDecrease),
            "reserve" => Some(Self::Reserve),
            "release" => Some(Self::Release),
            "settlement_post" => Some(Self::SettlementPost),
            "refund" => Some(Self::Refund),
            "chargeback" => Some(Self::Chargeback),
            "adjustment" => Some(Self::Adjustment),
            "freeze" => Some(Self::Freeze),
            "unfreeze" => Some(Self::Unfreeze),
            _ => None,
        }
    }

    /// High-risk ops require dual-control (request → approve → execute).
    pub fn requires_dual_control(self) -> bool {
        matches!(
            self,
            Self::AllocationCreate
                | Self::AllocationIncrease
                | Self::AllocationDecrease
                | Self::Refund
                | Self::Chargeback
                | Self::Adjustment
                | Self::Unfreeze
        )
    }

    /// Low-risk single-shot (CSRF + RBAC + idempotency).
    pub fn is_single_shot(self) -> bool {
        matches!(
            self,
            Self::Reserve | Self::Release | Self::SettlementPost | Self::Freeze
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
    Executing,
    Executed,
    Failed,
}

impl MutationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Executing => "executing",
            Self::Executed => "executed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            "cancelled" => Some(Self::Cancelled),
            "executing" => Some(Self::Executing),
            "executed" => Some(Self::Executed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationRequest {
    pub mutation_id: String,
    pub organisation_id: String,
    pub request_id: String,
    pub idempotency_key: String,
    pub operation: TreasuryOperation,
    pub payload: Value,
    pub payload_hash: String,
    pub target: Option<String>,
    pub requested_by: String,
    pub requester_role: String,
    pub status: MutationStatus,
    pub approval_id: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub journal_batch_id: Option<String>,
    pub outcome: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub exec_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMutationBody {
    pub operation: TreasuryOperation,
    pub payload: Value,
    pub idempotency_key: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApproveBody {
    pub decision: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteBody {
    pub confirm: bool,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SingleShotBody {
    pub confirm: bool,
    pub idempotency_key: String,
    #[serde(flatten)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct MutationResponse {
    pub mutation_id: String,
    pub status: MutationStatus,
    pub operation: TreasuryOperation,
    pub payload_hash: String,
    pub requested_by: String,
    pub approved_by: Option<String>,
    pub approval_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub request_id: String,
    pub journal_batch_id: Option<String>,
    pub outcome: Option<String>,
    pub duplicate: bool,
    /// Always present for console copy.
    pub ledger_notice: &'static str,
}

pub const LEDGER_NOTICE: &str =
    "Internal ledger governance only. No external money movement.";

impl From<&MutationRequest> for MutationResponse {
    fn from(m: &MutationRequest) -> Self {
        Self {
            mutation_id: m.mutation_id.clone(),
            status: m.status,
            operation: m.operation,
            payload_hash: m.payload_hash.clone(),
            requested_by: m.requested_by.clone(),
            approved_by: m.approved_by.clone(),
            approval_id: m.approval_id.clone(),
            expires_at: m.expires_at,
            request_id: m.request_id.clone(),
            journal_batch_id: m.journal_batch_id.clone(),
            outcome: m.outcome.clone(),
            duplicate: false,
            ledger_notice: LEDGER_NOTICE,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecuteResult {
    pub request_id: String,
    pub outcome: String,
    pub operation: TreasuryOperation,
    pub journal_batch_id: Option<String>,
    pub mutation_id: Option<String>,
    pub resource: Value,
    pub approval_id: Option<String>,
    pub duplicate: bool,
    pub ledger_notice: &'static str,
}
