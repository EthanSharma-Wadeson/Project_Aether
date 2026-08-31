use serde::Serialize;

/// Overall sync observation outcome (read-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyncStatus {
    SyncOk,
    DriftDetected,
    RequiresReview,
}

impl SyncStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SyncOk => "SYNC_OK",
            Self::DriftDetected => "DRIFT_DETECTED",
            Self::RequiresReview => "REQUIRES_REVIEW",
        }
    }

    pub fn escalate(self, other: Self) -> Self {
        use SyncStatus::*;
        match (self, other) {
            (RequiresReview, _) | (_, RequiresReview) => RequiresReview,
            (DriftDetected, _) | (_, DriftDetected) => DriftDetected,
            _ => SyncOk,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftKind {
    CapabilityWithoutAllocation,
    AllocationWithoutCapability,
    CapabilityLimitExceedsAllocation,
    ExpiredAllocationActiveCapability,
    InactiveAllocationActiveCapability,
}

impl DriftKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityWithoutAllocation => "capability_without_allocation",
            Self::AllocationWithoutCapability => "allocation_without_capability",
            Self::CapabilityLimitExceedsAllocation => "capability_limit_exceeds_allocation",
            Self::ExpiredAllocationActiveCapability => "expired_allocation_active_capability",
            Self::InactiveAllocationActiveCapability => "inactive_allocation_active_capability",
        }
    }

    pub fn severity(self) -> SyncStatus {
        match self {
            Self::CapabilityWithoutAllocation | Self::AllocationWithoutCapability => {
                SyncStatus::DriftDetected
            }
            Self::CapabilityLimitExceedsAllocation
            | Self::ExpiredAllocationActiveCapability
            | Self::InactiveAllocationActiveCapability => SyncStatus::RequiresReview,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftFinding {
    pub kind: DriftKind,
    pub agent_id: String,
    pub capability_id: Option<String>,
    pub allocation_id: Option<String>,
    pub detail: String,
    pub severity: SyncStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncObservationReport {
    pub status: SyncStatus,
    pub status_code: &'static str,
    pub organisation_id: String,
    pub capabilities_scanned: usize,
    pub allocations_scanned: usize,
    pub findings: Vec<DriftFinding>,
    pub request_id: String,
    pub observed_at: String,
    /// Hard guarantee for operators / API consumers.
    pub observation_only: bool,
    pub enforcement: bool,
    pub auto_repair: bool,
    pub ledger_notice: &'static str,
}

pub const SYNC_LEDGER_NOTICE: &str =
    "Observation only — no Apply execution, PROTO-0 mutation, treasury write, or auto-repair.";
