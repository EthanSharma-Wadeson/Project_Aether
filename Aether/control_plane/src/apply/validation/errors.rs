//! Apply validation error codes (Phase 6).

use super::model::{GateId, ValidationResult};

/// Stable validation / gate failure codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationErrorCode {
    Unauthenticated,
    JwtInvalid,
    RbacDenied,
    CsrfInvalid,
    ApprovalMissing,
    ApprovalInactive,
    ApprovalExpired,
    ApprovalConsumed,
    AttestationInvalid,
    AttestationExpired,
    AttestationNotFound,
    ExecutionHashMismatch,
    SignedOperationMissing,
    SignatureInvalid,
    SignatureExpired,
    SignaturePurposeInvalid,
    SignatureSignerUnauthorised,
    PolicyNotFound,
    PolicyArchived,
    PolicyNotApproved,
    PolicyVersionMismatch,
    PolicyHashChanged,
    StaleExecutionHash,
    RequiresResimulation,
    Database,
}

impl ValidationErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unauthenticated => "VALIDATION_UNAUTHENTICATED",
            Self::JwtInvalid => "VALIDATION_JWT_INVALID",
            Self::RbacDenied => "VALIDATION_RBAC_DENIED",
            Self::CsrfInvalid => "VALIDATION_CSRF_INVALID",
            Self::ApprovalMissing => "VALIDATION_APPROVAL_MISSING",
            Self::ApprovalInactive => "VALIDATION_APPROVAL_INACTIVE",
            Self::ApprovalExpired => "VALIDATION_APPROVAL_EXPIRED",
            Self::ApprovalConsumed => "VALIDATION_APPROVAL_CONSUMED",
            Self::AttestationInvalid => "VALIDATION_ATTESTATION_INVALID",
            Self::AttestationExpired => "VALIDATION_ATTESTATION_EXPIRED",
            Self::AttestationNotFound => "VALIDATION_ATTESTATION_NOT_FOUND",
            Self::ExecutionHashMismatch => "VALIDATION_EXECUTION_HASH_MISMATCH",
            Self::SignedOperationMissing => "VALIDATION_SIGNED_OPERATION_MISSING",
            Self::SignatureInvalid => "VALIDATION_SIGNATURE_INVALID",
            Self::SignatureExpired => "VALIDATION_SIGNATURE_EXPIRED",
            Self::SignaturePurposeInvalid => "VALIDATION_SIGNATURE_PURPOSE_INVALID",
            Self::SignatureSignerUnauthorised => "VALIDATION_SIGNATURE_SIGNER_UNAUTHORISED",
            Self::PolicyNotFound => "VALIDATION_POLICY_NOT_FOUND",
            Self::PolicyArchived => "VALIDATION_POLICY_ARCHIVED",
            Self::PolicyNotApproved => "VALIDATION_POLICY_NOT_APPROVED",
            Self::PolicyVersionMismatch => "APPLY_STALE_POLICY_VERSION",
            Self::PolicyHashChanged => "VALIDATION_POLICY_HASH_CHANGED",
            Self::StaleExecutionHash => "APPLY_STALE_EXECUTION_HASH",
            Self::RequiresResimulation => "VALIDATION_REQUIRES_RESIMULATION",
            Self::Database => "VALIDATION_DATABASE",
        }
    }
}

/// Structured validation failure (fail-closed; no protocol mutation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub code: ValidationErrorCode,
    pub gate: GateId,
    pub message: String,
    /// Partial gate results collected before failure (boxed for Result size).
    pub partial: Box<ValidationResult>,
}

impl ValidationError {
    pub fn new(
        code: ValidationErrorCode,
        gate: GateId,
        message: impl Into<String>,
        partial: ValidationResult,
    ) -> Self {
        Self {
            code,
            gate,
            message: message.into(),
            partial: Box::new(partial),
        }
    }

    pub fn code(&self) -> ValidationErrorCode {
        self.code
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}: {}",
            self.code.as_str(),
            self.gate.as_str(),
            self.message
        )
    }
}

impl std::error::Error for ValidationError {}
