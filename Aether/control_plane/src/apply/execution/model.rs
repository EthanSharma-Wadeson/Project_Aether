//! Apply execution pipeline domain types (Phase 8 — orchestration only).

use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthContext;

/// Orchestration phase for a single Apply execution attempt.
///
/// Distinct from durable [`crate::apply::replay::ReplayStatus`], but must
/// stay consistent with replay transitions once reservation begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyExecutionPhase {
    Created,
    Validated,
    ResimulationApproved,
    ReplayReserved,
    Executing,
    Executed,
    Rejected,
    Aborted,
    Stuck,
}

impl ApplyExecutionPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Validated => "validated",
            Self::ResimulationApproved => "resimulation_approved",
            Self::ReplayReserved => "replay_reserved",
            Self::Executing => "executing",
            Self::Executed => "executed",
            Self::Rejected => "rejected",
            Self::Aborted => "aborted",
            Self::Stuck => "stuck",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Executed | Self::Rejected | Self::Aborted | Self::Stuck
        )
    }
}

/// Normative execution-phase transitions (orchestration state machine).
pub fn allowed_execution_transition(from: ApplyExecutionPhase, to: ApplyExecutionPhase) -> bool {
    use ApplyExecutionPhase::*;
    matches!(
        (from, to),
        (Created, Validated)
            | (Validated, ResimulationApproved)
            | (ResimulationApproved, ReplayReserved)
            | (ReplayReserved, Executing)
            | (ReplayReserved, Aborted)
            | (ReplayReserved, Stuck)
            | (Executing, Executed)
            | (Executing, Rejected)
            | (Executing, Aborted)
            | (Executing, Stuck)
    )
}

/// Single source of truth for one Apply execution attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyExecutionContext {
    pub operation_id: String,
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub operation_intent: String,
    pub actor_id: String,
    pub actor_username: String,
    pub request_id: String,
    pub audit_correlation_id: String,
    pub phase: ApplyExecutionPhase,
}

/// Inputs to the guarded Apply execution pipeline.
#[derive(Debug, Clone)]
pub struct ExecuteApplyRequest {
    pub request_id: String,
    pub operation_id: String,
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub authenticated: bool,
    pub jwt_valid: bool,
    pub csrf_valid: bool,
    pub actor: AuthContext,
    /// C9 — required `true` when `apply_enabled()`; ignored while disabled.
    pub confirm: bool,
}

/// Completed pipeline outcome (Phase 8: always blocked before mutation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecuteApplyResult {
    pub context: ApplyExecutionContext,
    pub outcome_code: String,
    pub protocol_unchanged: bool,
    pub replay_reserved: bool,
}
