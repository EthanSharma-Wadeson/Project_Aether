//! Dry-run attestation domain types.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::execution::types::{ProtocolOperationKind, SimulationOutcome};

/// Dry-run attestation TTL — `DRY_RUN_BINDING_MAX_AGE` (60 minutes).
pub const DEFAULT_ATTESTATION_TTL: Duration = Duration::minutes(60);

/// Attestation lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttestationStatus {
    Executable,
    Expired,
    Invalidated,
}

impl AttestationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::Expired => "expired",
            Self::Invalidated => "invalidated",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "executable" => Some(Self::Executable),
            "expired" => Some(Self::Expired),
            "invalidated" => Some(Self::Invalidated),
            _ => None,
        }
    }
}

/// Immutable dry-run attestation — evidence for future Apply binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunAttestation {
    pub dry_run_id: String,
    pub operation_intent: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub execution_hash: String,
    pub simulation_result: Value,
    pub protocol_operation_kind: ProtocolOperationKind,
    pub predicted_changes: Value,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub created_by: String,
    pub audit_reference: Option<String>,
    pub status: AttestationStatus,
}

/// Inputs to create a new attestation (append-only).
#[derive(Debug, Clone)]
pub struct CreateAttestationRequest {
    pub dry_run_id: String,
    pub operation_intent: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub execution_hash: String,
    pub simulation: SimulationOutcome,
    pub protocol_operation_kind: ProtocolOperationKind,
    pub predicted_changes: Value,
    pub created_by: String,
    pub audit_reference: Option<String>,
    pub ttl: Option<Duration>,
}

/// Binding fields checked by `get_valid_attestation`.
#[derive(Debug, Clone)]
pub struct AttestationBinding {
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_version: i64,
    pub operation_intent: String,
}
