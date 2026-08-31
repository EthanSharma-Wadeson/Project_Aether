//! Apply execution pipeline errors.

use super::model::{ApplyExecutionContext, ApplyExecutionPhase};

/// Stable execution-pipeline error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionPipelineErrorCode {
    InvalidTransition,
    ValidationFailed,
    ResimulationFailed,
    ResimulationNotApproved,
    ReplayDuplicate,
    ReplayInProgress,
    ReplayFailed,
    ApprovalConsumeFailed,
    SignedOperationMissing,
    PolicyMissing,
    StaleApproval,
    StaleSignature,
    StaleAttestation,
    PolicyConsistencyFailed,
    ConfirmRequired,
    UnsupportedOperation,
    ExecutionDisabled,
    Database,
    Failed,
}

impl ExecutionPipelineErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidTransition => "APPLY_EXECUTION_INVALID_TRANSITION",
            Self::ValidationFailed => "APPLY_EXECUTION_VALIDATION_FAILED",
            Self::ResimulationFailed => "APPLY_EXECUTION_RESIMULATION_FAILED",
            Self::ResimulationNotApproved => "APPLY_EXECUTION_RESIMULATION_NOT_APPROVED",
            Self::ReplayDuplicate => "REPLAY_DUPLICATE",
            Self::ReplayInProgress => "REPLAY_IN_PROGRESS",
            Self::ReplayFailed => "APPLY_EXECUTION_REPLAY_FAILED",
            Self::ApprovalConsumeFailed => "APPLY_EXECUTION_APPROVAL_CONSUME_FAILED",
            Self::SignedOperationMissing => "APPLY_EXECUTION_SIGNED_OPERATION_MISSING",
            Self::PolicyMissing => "APPLY_EXECUTION_POLICY_MISSING",
            Self::StaleApproval => "APPLY_EXECUTION_STALE_APPROVAL",
            Self::StaleSignature => "APPLY_EXECUTION_STALE_SIGNATURE",
            Self::StaleAttestation => "APPLY_EXECUTION_STALE_ATTESTATION",
            Self::PolicyConsistencyFailed => "APPLY_EXECUTION_POLICY_INCONSISTENT",
            Self::ConfirmRequired => "APPLY_EXECUTION_CONFIRM_REQUIRED",
            Self::UnsupportedOperation => "APPLY_UNSUPPORTED_OPERATION",
            Self::ExecutionDisabled => "APPLY_EXECUTION_DISABLED",
            Self::Database => "APPLY_EXECUTION_DATABASE",
            Self::Failed => "APPLY_EXECUTION_FAILED",
        }
    }
}

/// Fail-closed pipeline error (no protocol mutation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPipelineError {
    pub code: ExecutionPipelineErrorCode,
    pub message: String,
    pub phase: ApplyExecutionPhase,
    pub context: Option<Box<ApplyExecutionContext>>,
}

impl ExecutionPipelineError {
    pub fn new(
        code: ExecutionPipelineErrorCode,
        message: impl Into<String>,
        phase: ApplyExecutionPhase,
        context: Option<ApplyExecutionContext>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            phase,
            context: context.map(Box::new),
        }
    }

    pub fn code(&self) -> ExecutionPipelineErrorCode {
        self.code
    }
}

impl std::fmt::Display for ExecutionPipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}: {}",
            self.code.as_str(),
            self.phase.as_str(),
            self.message
        )
    }
}

impl std::error::Error for ExecutionPipelineError {}
