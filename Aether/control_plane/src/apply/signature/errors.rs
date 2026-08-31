//! Apply signature errors.

use crate::apply::signature::model::SignatureStatus;

/// Stable signature error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignatureErrorCode {
    MissingApproval,
    ExpiredApproval,
    ConsumedApproval,
    CancelledApproval,
    ExecutionHashMismatch,
    DryRunMismatch,
    PolicyVersionMismatch,
    PolicyIdMismatch,
    IntentMismatch,
    InvalidPurpose,
    ExpiredSignature,
    InvalidSignature,
    InvalidSignerRole,
    SignerMismatch,
    DuplicateOperationId,
    NotFound,
    InvalidTransition,
    ApprovalBindingFailed,
    Database,
    Canonicalize,
    SignFailed,
}

impl SignatureErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingApproval => "SIGNATURE_MISSING_APPROVAL",
            Self::ExpiredApproval => "SIGNATURE_EXPIRED_APPROVAL",
            Self::ConsumedApproval => "SIGNATURE_CONSUMED_APPROVAL",
            Self::CancelledApproval => "SIGNATURE_CANCELLED_APPROVAL",
            Self::ExecutionHashMismatch => "SIGNATURE_EXECUTION_HASH_MISMATCH",
            Self::DryRunMismatch => "SIGNATURE_DRY_RUN_MISMATCH",
            Self::PolicyVersionMismatch => "SIGNATURE_POLICY_VERSION_MISMATCH",
            Self::PolicyIdMismatch => "SIGNATURE_POLICY_ID_MISMATCH",
            Self::IntentMismatch => "SIGNATURE_INTENT_MISMATCH",
            Self::InvalidPurpose => "APPLY_INVALID_SIGNATURE_PURPOSE",
            Self::ExpiredSignature => "SIGNATURE_EXPIRED",
            Self::InvalidSignature => "SIGNATURE_INVALID",
            Self::InvalidSignerRole => "SIGNATURE_INVALID_SIGNER_ROLE",
            Self::SignerMismatch => "SIGNATURE_SIGNER_MISMATCH",
            Self::DuplicateOperationId => "SIGNATURE_DUPLICATE_OPERATION",
            Self::NotFound => "SIGNATURE_NOT_FOUND",
            Self::InvalidTransition => "SIGNATURE_INVALID_TRANSITION",
            Self::ApprovalBindingFailed => "SIGNATURE_APPROVAL_BINDING_FAILED",
            Self::Database => "SIGNATURE_DATABASE",
            Self::Canonicalize => "SIGNATURE_CANONICALIZE",
            Self::SignFailed => "SIGNATURE_SIGN_FAILED",
        }
    }
}

/// Structured Apply signature errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    MissingApproval {
        approval_id: String,
    },
    ExpiredApproval {
        approval_id: String,
    },
    ConsumedApproval {
        approval_id: String,
    },
    CancelledApproval {
        approval_id: String,
    },
    ExecutionHashMismatch,
    DryRunMismatch,
    PolicyVersionMismatch {
        expected: i64,
        actual: i64,
    },
    PolicyIdMismatch,
    IntentMismatch,
    InvalidPurpose {
        purpose: String,
    },
    ExpiredSignature {
        operation_id: String,
    },
    InvalidSignature {
        reason: String,
    },
    InvalidSignerRole {
        role: String,
    },
    SignerMismatch,
    DuplicateOperationId {
        operation_id: String,
    },
    NotFound {
        operation_id: String,
    },
    InvalidTransition {
        from: SignatureStatus,
        to: SignatureStatus,
    },
    ApprovalBindingFailed(String),
    Database(String),
    Canonicalize(String),
    SignFailed(String),
}

impl SignatureError {
    pub fn code(&self) -> SignatureErrorCode {
        match self {
            Self::MissingApproval { .. } => SignatureErrorCode::MissingApproval,
            Self::ExpiredApproval { .. } => SignatureErrorCode::ExpiredApproval,
            Self::ConsumedApproval { .. } => SignatureErrorCode::ConsumedApproval,
            Self::CancelledApproval { .. } => SignatureErrorCode::CancelledApproval,
            Self::ExecutionHashMismatch => SignatureErrorCode::ExecutionHashMismatch,
            Self::DryRunMismatch => SignatureErrorCode::DryRunMismatch,
            Self::PolicyVersionMismatch { .. } => SignatureErrorCode::PolicyVersionMismatch,
            Self::PolicyIdMismatch => SignatureErrorCode::PolicyIdMismatch,
            Self::IntentMismatch => SignatureErrorCode::IntentMismatch,
            Self::InvalidPurpose { .. } => SignatureErrorCode::InvalidPurpose,
            Self::ExpiredSignature { .. } => SignatureErrorCode::ExpiredSignature,
            Self::InvalidSignature { .. } => SignatureErrorCode::InvalidSignature,
            Self::InvalidSignerRole { .. } => SignatureErrorCode::InvalidSignerRole,
            Self::SignerMismatch => SignatureErrorCode::SignerMismatch,
            Self::DuplicateOperationId { .. } => SignatureErrorCode::DuplicateOperationId,
            Self::NotFound { .. } => SignatureErrorCode::NotFound,
            Self::InvalidTransition { .. } => SignatureErrorCode::InvalidTransition,
            Self::ApprovalBindingFailed(_) => SignatureErrorCode::ApprovalBindingFailed,
            Self::Database(_) => SignatureErrorCode::Database,
            Self::Canonicalize(_) => SignatureErrorCode::Canonicalize,
            Self::SignFailed(_) => SignatureErrorCode::SignFailed,
        }
    }
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", self.code().as_str())?;
        match self {
            Self::MissingApproval { approval_id } => {
                write!(f, "approval not found: {approval_id}")
            }
            Self::ExpiredApproval { approval_id } => {
                write!(f, "approval expired: {approval_id}")
            }
            Self::ConsumedApproval { approval_id } => {
                write!(f, "approval consumed: {approval_id}")
            }
            Self::CancelledApproval { approval_id } => {
                write!(f, "approval cancelled: {approval_id}")
            }
            Self::ExecutionHashMismatch => write!(f, "execution_hash mismatch"),
            Self::DryRunMismatch => write!(f, "dry_run_id mismatch"),
            Self::PolicyVersionMismatch { expected, actual } => {
                write!(
                    f,
                    "policy version mismatch: expected {expected}, have {actual}"
                )
            }
            Self::PolicyIdMismatch => write!(f, "policy_id mismatch"),
            Self::IntentMismatch => write!(f, "operation intent mismatch"),
            Self::InvalidPurpose { purpose } => {
                write!(f, "purpose '{purpose}' is not valid for Apply")
            }
            Self::ExpiredSignature { operation_id } => {
                write!(f, "signature expired: {operation_id}")
            }
            Self::InvalidSignature { reason } => write!(f, "{reason}"),
            Self::InvalidSignerRole { role } => {
                write!(f, "role '{role}' cannot prepare Apply signatures")
            }
            Self::SignerMismatch => write!(f, "signer identity does not match configured signer"),
            Self::DuplicateOperationId { operation_id } => {
                write!(f, "operation_id already signed: {operation_id}")
            }
            Self::NotFound { operation_id } => {
                write!(f, "signed operation not found: {operation_id}")
            }
            Self::InvalidTransition { from, to } => {
                write!(f, "cannot transition {} -> {}", from.as_str(), to.as_str())
            }
            Self::ApprovalBindingFailed(msg)
            | Self::Database(msg)
            | Self::Canonicalize(msg)
            | Self::SignFailed(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SignatureError {}
