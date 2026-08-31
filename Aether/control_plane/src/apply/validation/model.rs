//! Apply validation result types (Phase 6 — pre-execution only).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthContext;

/// Fixed gate identifiers in normative Phase 6 order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateId {
    G1,
    G2,
    G3,
    G4,
    G5,
    G6,
    G7,
    G8,
    G9,
    G10,
    G11,
    G12,
    G13,
    G14,
}

impl GateId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::G1 => "G1",
            Self::G2 => "G2",
            Self::G3 => "G3",
            Self::G4 => "G4",
            Self::G5 => "G5",
            Self::G6 => "G6",
            Self::G7 => "G7",
            Self::G8 => "G8",
            Self::G9 => "G9",
            Self::G10 => "G10",
            Self::G11 => "G11",
            Self::G12 => "G12",
            Self::G13 => "G13",
            Self::G14 => "G14",
        }
    }

    /// Stable pipeline order (G1 → G14).
    pub const ORDER: [GateId; 14] = [
        GateId::G1,
        GateId::G2,
        GateId::G3,
        GateId::G4,
        GateId::G5,
        GateId::G6,
        GateId::G7,
        GateId::G8,
        GateId::G9,
        GateId::G10,
        GateId::G11,
        GateId::G12,
        GateId::G13,
        GateId::G14,
    ];
}

/// Per-gate outcome recorded on every validation run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_id: String,
    pub passed: bool,
    pub failure_code: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl GateResult {
    pub fn passed(gate: GateId) -> Self {
        Self {
            gate_id: gate.as_str().into(),
            passed: true,
            failure_code: None,
            timestamp: Utc::now(),
        }
    }

    pub fn failed(gate: GateId, code: impl Into<String>) -> Self {
        Self {
            gate_id: gate.as_str().into(),
            passed: false,
            failure_code: Some(code.into()),
            timestamp: Utc::now(),
        }
    }
}

/// Outcome of the full validation chain (no protocol mutation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    /// `true` when G1–G13 passed; G14 records that re-simulation is still required.
    pub approved: bool,
    pub operation_id: String,
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub validated_at: DateTime<Utc>,
    pub gate_results: Vec<GateResult>,
    /// When `approved`, always `VALIDATION_REQUIRES_RESIMULATION` in Phase 6.
    pub decision_code: Option<String>,
}

/// Inputs to `validate_apply_request`.
///
/// G1–G4 are request preconditions normally established by HTTP middleware /
/// write guards; the pipeline re-checks them fail-closed.
#[derive(Debug, Clone)]
pub struct ApplyValidationRequest {
    pub request_id: String,
    pub operation_id: String,
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    /// G1 — request authentication established.
    pub authenticated: bool,
    /// G2 — JWT validated and claims resolved into `actor`.
    pub jwt_valid: bool,
    /// G4 — CSRF validated for this mutating request.
    pub csrf_valid: bool,
    pub actor: AuthContext,
}
