use chrono::Utc;

use crate::models::capabilities::CapabilityView;
use crate::protocol::proto0;
use crate::protocol::state::ProtocolState;
use crate::treasury::models::AllocationDto;
use crate::treasury::TreasuryAdapter;

use super::models::DriftFinding;

/// Snapshot inputs for comparison (all read-only).
#[derive(Debug, Clone)]
pub struct SyncSnapshot {
    pub organisation_id: String,
    pub capabilities: Vec<CapabilityView>,
    pub allocations: Vec<AllocationDto>,
}

pub async fn load_snapshot(
    protocol: &ProtocolState,
    treasury: &TreasuryAdapter,
) -> Result<SyncSnapshot, crate::treasury::errors::TreasuryCpError> {
    let organisation_id = treasury.organisation_id().to_string();
    let capabilities = proto0::list_capabilities(protocol);
    let rows = treasury
        .engine()
        .list_allocations_for_organisation(&organisation_id)
        .await
        .map_err(crate::treasury::errors::TreasuryCpError::from)?;
    let allocations = rows
        .into_iter()
        .filter(|a| a.organisation_id == organisation_id)
        .map(|a| AllocationDto {
            allocation_id: a.allocation_id,
            organisation_id: a.organisation_id,
            treasury_id: a.treasury_id,
            agent_id: a.agent_id,
            asset_id: a.asset_id.0,
            ceiling_minor: a.ceiling_minor,
            remaining_minor: a.remaining_minor,
            status: a.status.as_str().to_string(),
            expires_at: a.expires_at.map(|t| t.to_rfc3339()),
            created_at: a.created_at.to_rfc3339(),
        })
        .collect();
    Ok(SyncSnapshot {
        organisation_id,
        capabilities,
        allocations,
    })
}

pub async fn load_snapshot_for_agent(
    protocol: &ProtocolState,
    treasury: &TreasuryAdapter,
    agent_id: &str,
) -> Result<SyncSnapshot, crate::treasury::errors::TreasuryCpError> {
    let mut snap = load_snapshot(protocol, treasury).await?;
    snap.capabilities
        .retain(|c| c.subject_agent_id == agent_id);
    snap.allocations.retain(|a| a.agent_id == agent_id);
    Ok(snap)
}

/// Active capability for drift purposes = not revoked.
///
/// Demo/PROTO bootstrap often uses relative `valid_*` windows (not wall-clock
/// Unix). Observation therefore does not expire grants by wall clock — that
/// would hide real authority still present in the CapabilityStore.
pub fn capability_is_active(cap: &CapabilityView, _now_unix: u64) -> bool {
    !cap.revoked
}

pub fn allocation_is_active(a: &AllocationDto) -> bool {
    a.status == "active"
}

pub fn allocation_is_expired(a: &AllocationDto, now: chrono::DateTime<Utc>) -> bool {
    if a.status == "expired" {
        return true;
    }
    if let Some(ref exp) = a.expires_at {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(exp) {
            return dt.with_timezone(&Utc) <= now;
        }
    }
    false
}

pub fn allocation_is_inactive_authority(a: &AllocationDto) -> bool {
    matches!(a.status.as_str(), "closed" | "frozen" | "expired")
}

pub fn now_unix() -> u64 {
    Utc::now().timestamp().max(0) as u64
}

/// Finding builder helper used by compare.
pub fn finding(
    kind: super::models::DriftKind,
    agent_id: impl Into<String>,
    capability_id: Option<String>,
    allocation_id: Option<String>,
    detail: impl Into<String>,
) -> DriftFinding {
    let kind = kind;
    DriftFinding {
        severity: kind.severity(),
        kind,
        agent_id: agent_id.into(),
        capability_id,
        allocation_id,
        detail: detail.into(),
    }
}
