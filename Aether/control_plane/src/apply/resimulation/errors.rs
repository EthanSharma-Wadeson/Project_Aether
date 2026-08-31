//! Mandatory re-simulation errors.

use super::model::ResimulationDecisionCode;

/// Stable re-simulation error codes (infrastructure / load failures).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResimulationErrorCode {
    SignedOperationMissing,
    ApprovalMissing,
    AttestationMissing,
    PolicyMissing,
    Database,
    Failed,
}

impl ResimulationErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SignedOperationMissing => "RESIMULATION_SIGNED_OPERATION_MISSING",
            Self::ApprovalMissing => "RESIMULATION_APPROVAL_MISSING",
            Self::AttestationMissing => "RESIMULATION_ATTESTATION_MISSING",
            Self::PolicyMissing => "RESIMULATION_POLICY_MISSING",
            Self::Database => "RESIMULATION_DATABASE",
            Self::Failed => "RESIMULATION_FAILED",
        }
    }
}

/// Structured re-simulation failure (load / infrastructure). Comparison
/// mismatches are returned as [`super::model::ResimulationResult`] with
/// `approved = false`, not as this error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResimulationError {
    pub code: ResimulationErrorCode,
    pub message: String,
    pub decision_code: ResimulationDecisionCode,
}

impl ResimulationError {
    pub fn new(code: ResimulationErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            decision_code: ResimulationDecisionCode::Failed,
        }
    }

    pub fn code(&self) -> ResimulationErrorCode {
        self.code
    }
}

impl std::fmt::Display for ResimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ResimulationError {}
