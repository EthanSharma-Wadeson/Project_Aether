//! Execution types — dry-run governance pipeline (Phase 4A).
//!
//! No protocol mutations. `execution_mode` is always `DryRun` in this phase.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::signer::operation::GovernanceOperation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    DryRun,
    /// Reserved for Phase 4B — not enabled.
    Live,
}

impl ExecutionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry_run",
            Self::Live => "live",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProtocolOperationKind {
    CapabilityGrant,
    CapabilityRevoke,
    FreezeIdentity,
    PolicyApply,
    Unknown,
}

impl ProtocolOperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityGrant => "CapabilityGrant",
            Self::CapabilityRevoke => "CapabilityRevoke",
            Self::FreezeIdentity => "FreezeIdentity",
            Self::PolicyApply => "PolicyApply",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStep {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionPlan {
    pub operation_id: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub target: Option<String>,
    pub action: String,
    pub signer_required: bool,
    pub protocol_operation: ProtocolOperationKind,
    pub validation_steps: Vec<String>,
    pub execution_mode: ExecutionMode,
    pub governance_operation: GovernanceOperation,
    pub planned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutcome {
    pub executable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunReport {
    pub executable: bool,
    pub validation_results: Vec<ValidationStep>,
    pub predicted_protocol_operation: ProtocolOperationKind,
    pub warnings: Vec<String>,
    pub blocking_errors: Vec<String>,
    pub execution_mode: ExecutionMode,
    /// Durable id for this dry-run attempt (Apply must bind to it).
    pub dry_run_id: String,
    pub operation_id: String,
    pub policy_id: String,
    pub policy_version: i64,
    /// Content-addressed binding of the intended mutation (not count-based).
    pub execution_hash: String,
    pub signer_identity: Option<String>,
    pub signature_hex: Option<String>,
    pub simulation: SimulationOutcome,
    pub note: String,
}
