//! Dry-run attestation errors.

use crate::apply::attestation::model::AttestationStatus;

/// Stable attestation error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttestationErrorCode {
    MissingExecutionHash,
    SimulationFailed,
    UnknownOperation,
    DuplicateDryRunId,
    NotFound,
    Expired,
    Invalidated,
    PolicyVersionMismatch,
    ExecutionHashMismatch,
    IntentMismatch,
    Database,
}

impl AttestationErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingExecutionHash => "ATTESTATION_MISSING_HASH",
            Self::SimulationFailed => "ATTESTATION_SIMULATION_FAILED",
            Self::UnknownOperation => "ATTESTATION_UNKNOWN_OPERATION",
            Self::DuplicateDryRunId => "ATTESTATION_DUPLICATE",
            Self::NotFound => "ATTESTATION_NOT_FOUND",
            Self::Expired => "ATTESTATION_EXPIRED",
            Self::Invalidated => "ATTESTATION_INVALIDATED",
            Self::PolicyVersionMismatch => "ATTESTATION_POLICY_VERSION_MISMATCH",
            Self::ExecutionHashMismatch => "ATTESTATION_EXECUTION_HASH_MISMATCH",
            Self::IntentMismatch => "ATTESTATION_INTENT_MISMATCH",
            Self::Database => "ATTESTATION_DATABASE",
        }
    }
}

/// Structured attestation store errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationError {
    MissingExecutionHash,
    SimulationFailed { reason: String },
    UnknownOperation,
    DuplicateDryRunId { dry_run_id: String },
    NotFound { dry_run_id: String },
    Expired { dry_run_id: String },
    Invalidated { dry_run_id: String },
    PolicyVersionMismatch { expected: i64, actual: i64 },
    ExecutionHashMismatch,
    IntentMismatch,
    InvalidStatus { status: AttestationStatus },
    Database(String),
}

impl AttestationError {
    pub fn code(&self) -> AttestationErrorCode {
        match self {
            Self::MissingExecutionHash => AttestationErrorCode::MissingExecutionHash,
            Self::SimulationFailed { .. } => AttestationErrorCode::SimulationFailed,
            Self::UnknownOperation => AttestationErrorCode::UnknownOperation,
            Self::DuplicateDryRunId { .. } => AttestationErrorCode::DuplicateDryRunId,
            Self::NotFound { .. } => AttestationErrorCode::NotFound,
            Self::Expired { .. } => AttestationErrorCode::Expired,
            Self::Invalidated { .. } => AttestationErrorCode::Invalidated,
            Self::PolicyVersionMismatch { .. } => AttestationErrorCode::PolicyVersionMismatch,
            Self::ExecutionHashMismatch => AttestationErrorCode::ExecutionHashMismatch,
            Self::IntentMismatch => AttestationErrorCode::IntentMismatch,
            Self::InvalidStatus { .. } => AttestationErrorCode::Invalidated,
            Self::Database(_) => AttestationErrorCode::Database,
        }
    }
}

impl std::fmt::Display for AttestationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", self.code().as_str())?;
        match self {
            Self::MissingExecutionHash => write!(f, "execution_hash is required"),
            Self::SimulationFailed { reason } => write!(f, "simulation failed: {reason}"),
            Self::UnknownOperation => write!(f, "protocol operation kind is unknown"),
            Self::DuplicateDryRunId { dry_run_id } => {
                write!(f, "dry_run_id already attested: {dry_run_id}")
            }
            Self::NotFound { dry_run_id } => write!(f, "attestation not found: {dry_run_id}"),
            Self::Expired { dry_run_id } => write!(f, "attestation expired: {dry_run_id}"),
            Self::Invalidated { dry_run_id } => write!(f, "attestation invalidated: {dry_run_id}"),
            Self::PolicyVersionMismatch { expected, actual } => {
                write!(
                    f,
                    "policy version mismatch: expected {expected}, have {actual}"
                )
            }
            Self::ExecutionHashMismatch => write!(f, "execution_hash does not match attestation"),
            Self::IntentMismatch => write!(f, "operation intent does not match attestation"),
            Self::InvalidStatus { status } => {
                write!(f, "attestation not usable (status={})", status.as_str())
            }
            Self::Database(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AttestationError {}
