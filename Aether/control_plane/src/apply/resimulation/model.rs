//! Mandatory re-simulation domain types (Phase 7 — evidence only, no execution).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Stable re-simulation decision codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResimulationDecisionCode {
    Approved,
    HashMismatch,
    StateChanged,
    PolicyChanged,
    Failed,
}

impl ResimulationDecisionCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "RESIMULATION_APPROVED",
            Self::HashMismatch => "RESIMULATION_HASH_MISMATCH",
            Self::StateChanged => "RESIMULATION_STATE_CHANGED",
            Self::PolicyChanged => "RESIMULATION_POLICY_CHANGED",
            Self::Failed => "RESIMULATION_FAILED",
        }
    }
}

/// Outcome of mandatory re-simulation (never executes Apply / PROTO-0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResimulationResult {
    pub approved: bool,
    pub dry_run_id: String,
    pub operation_id: String,
    pub execution_hash: String,
    pub comparison_hash: String,
    pub changes_match: bool,
    pub validated_at: DateTime<Utc>,
    pub decision_code: String,
}

/// Inputs to `resimulate_apply_intent`.
#[derive(Debug, Clone)]
pub struct ResimulationRequest {
    pub request_id: String,
    pub operation_id: String,
    /// Operator id for audit only (no RBAC mutation).
    pub actor_id: String,
}

/// Canonical comparison snapshot used for determinism / `comparison_hash`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonSnapshot {
    pub policy_id: String,
    pub policy_version: i64,
    pub policy_hash: String,
    pub operation_intent: String,
    pub protocol_operation_kind: String,
    pub target_agent: Option<String>,
    pub execution_hash: String,
    pub simulation_executable: bool,
    pub predicted_changes: Value,
}
