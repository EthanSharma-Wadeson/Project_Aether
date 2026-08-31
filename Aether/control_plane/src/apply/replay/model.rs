//! Replay store domain types and errors.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lifecycle status for a signed Apply operation in the replay store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayStatus {
    Reserved,
    Executing,
    Executed,
    Rejected,
    Aborted,
    Stuck,
}

impl ReplayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Executing => "executing",
            Self::Executed => "executed",
            Self::Rejected => "rejected",
            Self::Aborted => "aborted",
            Self::Stuck => "stuck",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "reserved" => Some(Self::Reserved),
            "executing" => Some(Self::Executing),
            "executed" => Some(Self::Executed),
            "rejected" => Some(Self::Rejected),
            "aborted" => Some(Self::Aborted),
            "stuck" => Some(Self::Stuck),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Executed | Self::Rejected | Self::Aborted | Self::Stuck
        )
    }
}

/// Durable replay row — one row per `operation_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayRecord {
    pub operation_id: String,
    pub execution_hash: String,
    pub dry_run_id: String,
    pub status: ReplayStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub reserved_at: DateTime<Utc>,
    pub terminal_reason: Option<String>,
    pub audit_reference: Option<String>,
    pub finalised_at: Option<DateTime<Utc>>,
}

/// Inputs required to reserve a replay slot (no PROTO-0 side effects).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReserveRequest {
    pub operation_id: String,
    pub execution_hash: String,
    pub dry_run_id: String,
}

/// Atomic prepare: reserve replay + consume approval in one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicPrepareRequest {
    pub operation_id: String,
    pub execution_hash: String,
    pub dry_run_id: String,
    pub approval_id: String,
}

/// Stable replay error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReplayErrorCode {
    ReplayDuplicate,
    ReplayInProgress,
    ReplayInvalidTransition,
    ReplayNotFound,
    ApprovalConsumeFailed,
}

impl ReplayErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReplayDuplicate => "REPLAY_DUPLICATE",
            Self::ReplayInProgress => "REPLAY_IN_PROGRESS",
            Self::ReplayInvalidTransition => "REPLAY_INVALID_TRANSITION",
            Self::ReplayNotFound => "REPLAY_NOT_FOUND",
            Self::ApprovalConsumeFailed => "APPLY_EXECUTION_APPROVAL_CONSUME_FAILED",
        }
    }
}

/// Structured replay store errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    Duplicate {
        record: Box<ReplayRecord>,
    },
    InProgress {
        record: Box<ReplayRecord>,
    },
    InvalidTransition {
        from: ReplayStatus,
        to: ReplayStatus,
    },
    NotFound {
        operation_id: String,
    },
    ApprovalConsumeFailed {
        message: String,
    },
    Database(String),
}

impl ReplayError {
    pub fn code(&self) -> ReplayErrorCode {
        match self {
            Self::Duplicate { .. } => ReplayErrorCode::ReplayDuplicate,
            Self::InProgress { .. } => ReplayErrorCode::ReplayInProgress,
            Self::InvalidTransition { .. } => ReplayErrorCode::ReplayInvalidTransition,
            Self::NotFound { .. } => ReplayErrorCode::ReplayNotFound,
            Self::ApprovalConsumeFailed { .. } => ReplayErrorCode::ApprovalConsumeFailed,
            Self::Database(_) => ReplayErrorCode::ReplayNotFound,
        }
    }
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duplicate { record } => {
                write!(
                    f,
                    "{}: operation {} already terminal ({})",
                    ReplayErrorCode::ReplayDuplicate.as_str(),
                    record.operation_id,
                    record.status.as_str()
                )
            }
            Self::InProgress { record } => {
                write!(
                    f,
                    "{}: operation {} in progress ({})",
                    ReplayErrorCode::ReplayInProgress.as_str(),
                    record.operation_id,
                    record.status.as_str()
                )
            }
            Self::InvalidTransition { from, to } => {
                write!(
                    f,
                    "{}: cannot transition {:?} -> {:?}",
                    ReplayErrorCode::ReplayInvalidTransition.as_str(),
                    from,
                    to
                )
            }
            Self::NotFound { operation_id } => {
                write!(
                    f,
                    "{}: operation {operation_id} not found",
                    ReplayErrorCode::ReplayNotFound.as_str()
                )
            }
            Self::ApprovalConsumeFailed { message } => {
                write!(
                    f,
                    "{}: {message}",
                    ReplayErrorCode::ApprovalConsumeFailed.as_str()
                )
            }
            Self::Database(msg) => write!(f, "replay database error: {msg}"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Normative transition guard (no undocumented transitions).
pub fn allowed_transition(from: ReplayStatus, to: ReplayStatus) -> bool {
    matches!(
        (from, to),
        (ReplayStatus::Reserved, ReplayStatus::Executing)
            | (ReplayStatus::Reserved, ReplayStatus::Stuck)
            | (ReplayStatus::Reserved, ReplayStatus::Aborted)
            | (ReplayStatus::Executing, ReplayStatus::Executed)
            | (ReplayStatus::Executing, ReplayStatus::Rejected)
            | (ReplayStatus::Executing, ReplayStatus::Aborted)
            | (ReplayStatus::Executing, ReplayStatus::Stuck)
            // Admin reconcile only — never auto-retries PROTO-0.
            | (ReplayStatus::Stuck, ReplayStatus::Aborted)
    )
}
