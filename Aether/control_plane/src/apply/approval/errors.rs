//! Apply approval errors.

use crate::apply::approval::model::ApprovalStatus;

/// Stable approval error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApprovalErrorCode {
    MissingDryRunId,
    MissingExecutionHash,
    InvalidPolicyVersion,
    AttestationNotFound,
    AttestationExpired,
    AttestationInvalid,
    ExecutionHashMismatch,
    IntentMismatch,
    PolicyVersionMismatch,
    PolicyIdMismatch,
    DuplicateApprovalId,
    ActiveBindingExists,
    NotFound,
    Expired,
    Consumed,
    Cancelled,
    InvalidTransition,
    ForbiddenRole,
    SeparationOfDuties,
    Database,
}

impl ApprovalErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingDryRunId => "APPROVAL_MISSING_DRY_RUN",
            Self::MissingExecutionHash => "APPROVAL_MISSING_HASH",
            Self::InvalidPolicyVersion => "APPROVAL_INVALID_POLICY_VERSION",
            Self::AttestationNotFound => "APPROVAL_ATTESTATION_NOT_FOUND",
            Self::AttestationExpired => "APPROVAL_ATTESTATION_EXPIRED",
            Self::AttestationInvalid => "APPROVAL_ATTESTATION_INVALID",
            Self::ExecutionHashMismatch => "APPROVAL_EXECUTION_HASH_MISMATCH",
            Self::IntentMismatch => "APPROVAL_INTENT_MISMATCH",
            Self::PolicyVersionMismatch => "APPROVAL_POLICY_VERSION_MISMATCH",
            Self::PolicyIdMismatch => "APPROVAL_POLICY_ID_MISMATCH",
            Self::DuplicateApprovalId => "APPROVAL_DUPLICATE",
            Self::ActiveBindingExists => "APPROVAL_ACTIVE_EXISTS",
            Self::NotFound => "APPROVAL_NOT_FOUND",
            Self::Expired => "APPROVAL_EXPIRED",
            Self::Consumed => "APPROVAL_CONSUMED",
            Self::Cancelled => "APPROVAL_CANCELLED",
            Self::InvalidTransition => "APPROVAL_INVALID_TRANSITION",
            Self::ForbiddenRole => "APPROVAL_FORBIDDEN_ROLE",
            Self::SeparationOfDuties => "APPROVAL_SOD_VIOLATION",
            Self::Database => "APPROVAL_DATABASE",
        }
    }
}

/// Structured approval errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalError {
    MissingDryRunId,
    MissingExecutionHash,
    InvalidPolicyVersion,
    AttestationNotFound {
        dry_run_id: String,
    },
    AttestationExpired {
        dry_run_id: String,
    },
    AttestationInvalid {
        dry_run_id: String,
    },
    ExecutionHashMismatch,
    IntentMismatch,
    PolicyVersionMismatch {
        expected: i64,
        actual: i64,
    },
    PolicyIdMismatch,
    DuplicateApprovalId {
        approval_id: String,
    },
    ActiveBindingExists,
    NotFound {
        approval_id: String,
    },
    Expired {
        approval_id: String,
    },
    Consumed {
        approval_id: String,
    },
    Cancelled {
        approval_id: String,
    },
    InvalidTransition {
        from: ApprovalStatus,
        to: ApprovalStatus,
    },
    ForbiddenRole {
        role: String,
    },
    SeparationOfDuties {
        message: String,
    },
    Database(String),
}

impl ApprovalError {
    pub fn code(&self) -> ApprovalErrorCode {
        match self {
            Self::MissingDryRunId => ApprovalErrorCode::MissingDryRunId,
            Self::MissingExecutionHash => ApprovalErrorCode::MissingExecutionHash,
            Self::InvalidPolicyVersion => ApprovalErrorCode::InvalidPolicyVersion,
            Self::AttestationNotFound { .. } => ApprovalErrorCode::AttestationNotFound,
            Self::AttestationExpired { .. } => ApprovalErrorCode::AttestationExpired,
            Self::AttestationInvalid { .. } => ApprovalErrorCode::AttestationInvalid,
            Self::ExecutionHashMismatch => ApprovalErrorCode::ExecutionHashMismatch,
            Self::IntentMismatch => ApprovalErrorCode::IntentMismatch,
            Self::PolicyVersionMismatch { .. } => ApprovalErrorCode::PolicyVersionMismatch,
            Self::PolicyIdMismatch => ApprovalErrorCode::PolicyIdMismatch,
            Self::DuplicateApprovalId { .. } => ApprovalErrorCode::DuplicateApprovalId,
            Self::ActiveBindingExists => ApprovalErrorCode::ActiveBindingExists,
            Self::NotFound { .. } => ApprovalErrorCode::NotFound,
            Self::Expired { .. } => ApprovalErrorCode::Expired,
            Self::Consumed { .. } => ApprovalErrorCode::Consumed,
            Self::Cancelled { .. } => ApprovalErrorCode::Cancelled,
            Self::InvalidTransition { .. } => ApprovalErrorCode::InvalidTransition,
            Self::ForbiddenRole { .. } => ApprovalErrorCode::ForbiddenRole,
            Self::SeparationOfDuties { .. } => ApprovalErrorCode::SeparationOfDuties,
            Self::Database(_) => ApprovalErrorCode::Database,
        }
    }
}

impl std::fmt::Display for ApprovalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", self.code().as_str())?;
        match self {
            Self::MissingDryRunId => write!(f, "dry_run_id is required"),
            Self::MissingExecutionHash => write!(f, "execution_hash is required"),
            Self::InvalidPolicyVersion => write!(f, "policy_version must be >= 1"),
            Self::AttestationNotFound { dry_run_id } => {
                write!(f, "attestation not found: {dry_run_id}")
            }
            Self::AttestationExpired { dry_run_id } => {
                write!(f, "attestation expired: {dry_run_id}")
            }
            Self::AttestationInvalid { dry_run_id } => {
                write!(f, "attestation not executable: {dry_run_id}")
            }
            Self::ExecutionHashMismatch => write!(f, "execution_hash does not match attestation"),
            Self::IntentMismatch => write!(f, "operation intent does not match attestation"),
            Self::PolicyVersionMismatch { expected, actual } => {
                write!(
                    f,
                    "policy version mismatch: expected {expected}, have {actual}"
                )
            }
            Self::PolicyIdMismatch => write!(f, "policy_id does not match attestation"),
            Self::DuplicateApprovalId { approval_id } => {
                write!(f, "approval_id already exists: {approval_id}")
            }
            Self::ActiveBindingExists => {
                write!(f, "active approval already exists for this binding")
            }
            Self::NotFound { approval_id } => write!(f, "approval not found: {approval_id}"),
            Self::Expired { approval_id } => write!(f, "approval expired: {approval_id}"),
            Self::Consumed { approval_id } => write!(f, "approval already consumed: {approval_id}"),
            Self::Cancelled { approval_id } => write!(f, "approval cancelled: {approval_id}"),
            Self::InvalidTransition { from, to } => {
                write!(f, "cannot transition {} -> {}", from.as_str(), to.as_str())
            }
            Self::ForbiddenRole { role } => write!(f, "role '{role}' cannot grant Apply approval"),
            Self::SeparationOfDuties { message } => write!(f, "{message}"),
            Self::Database(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ApprovalError {}
