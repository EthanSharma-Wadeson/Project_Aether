use chrono::Utc;

use super::models::{DriftFinding, DriftKind, SyncStatus};
use super::queries::{
    allocation_is_active, allocation_is_expired, allocation_is_inactive_authority,
    capability_is_active, finding, now_unix, SyncSnapshot,
};

/// Pure comparison engine — no I/O, no mutations.
pub fn compare_snapshot(snap: &SyncSnapshot) -> (SyncStatus, Vec<DriftFinding>) {
    let now_u = now_unix();
    let now = Utc::now();
    let mut findings = Vec::new();

    let active_caps: Vec<_> = snap
        .capabilities
        .iter()
        .filter(|c| capability_is_active(c, now_u))
        .collect();
    let active_allocs: Vec<_> = snap
        .allocations
        .iter()
        .filter(|a| allocation_is_active(a))
        .collect();

    // Capability exists without allocation (agent-level).
    for cap in &active_caps {
        let has_alloc = active_allocs
            .iter()
            .any(|a| a.agent_id == cap.subject_agent_id && assets_compatible(cap.asset.as_deref(), &a.asset_id));
        if !has_alloc {
            findings.push(finding(
                DriftKind::CapabilityWithoutAllocation,
                &cap.subject_agent_id,
                Some(cap.capability_id.clone()),
                None,
                format!(
                    "active capability {} has no matching active allocation",
                    cap.capability_id
                ),
            ));
        }
    }

    // Allocation exists without capability.
    for alloc in &active_allocs {
        let has_cap = active_caps.iter().any(|c| {
            c.subject_agent_id == alloc.agent_id
                && assets_compatible(c.asset.as_deref(), &alloc.asset_id)
        });
        if !has_cap {
            findings.push(finding(
                DriftKind::AllocationWithoutCapability,
                &alloc.agent_id,
                None,
                Some(alloc.allocation_id.clone()),
                format!(
                    "active allocation {} has no matching active capability",
                    alloc.allocation_id
                ),
            ));
        }
    }

    // Capability limit exceeds allocation ceiling (pair-wise best match).
    for cap in &active_caps {
        let Some(max_spend) = cap.max_spend else {
            continue;
        };
        let max_spend_i = max_spend as i64;
        for alloc in active_allocs
            .iter()
            .filter(|a| a.agent_id == cap.subject_agent_id)
            .filter(|a| assets_compatible(cap.asset.as_deref(), &a.asset_id))
        {
            if max_spend_i > alloc.ceiling_minor {
                findings.push(finding(
                    DriftKind::CapabilityLimitExceedsAllocation,
                    &cap.subject_agent_id,
                    Some(cap.capability_id.clone()),
                    Some(alloc.allocation_id.clone()),
                    format!(
                        "capability max_spend {max_spend} exceeds allocation ceiling {}",
                        alloc.ceiling_minor
                    ),
                ));
            }
        }
    }

    // Expired allocation with active capability.
    for alloc in snap.allocations.iter().filter(|a| allocation_is_expired(a, now)) {
        let has_active_cap = active_caps.iter().any(|c| {
            c.subject_agent_id == alloc.agent_id
                && assets_compatible(c.asset.as_deref(), &alloc.asset_id)
        });
        if has_active_cap {
            let cap_id = active_caps
                .iter()
                .find(|c| c.subject_agent_id == alloc.agent_id)
                .map(|c| c.capability_id.clone());
            findings.push(finding(
                DriftKind::ExpiredAllocationActiveCapability,
                &alloc.agent_id,
                cap_id,
                Some(alloc.allocation_id.clone()),
                format!(
                    "expired/time-elapsed allocation {} with active capability",
                    alloc.allocation_id
                ),
            ));
        }
    }

    // Inactive (closed/frozen/expired status) allocation with active capability.
    for alloc in snap
        .allocations
        .iter()
        .filter(|a| allocation_is_inactive_authority(a))
    {
        // Avoid double-counting pure expiry findings already emitted.
        if allocation_is_expired(alloc, now) && alloc.status == "expired" {
            continue;
        }
        if alloc.status == "expired" {
            continue;
        }
        let has_active_cap = active_caps.iter().any(|c| {
            c.subject_agent_id == alloc.agent_id
                && assets_compatible(c.asset.as_deref(), &alloc.asset_id)
        });
        if has_active_cap {
            let cap_id = active_caps
                .iter()
                .find(|c| c.subject_agent_id == alloc.agent_id)
                .map(|c| c.capability_id.clone());
            findings.push(finding(
                DriftKind::InactiveAllocationActiveCapability,
                &alloc.agent_id,
                cap_id,
                Some(alloc.allocation_id.clone()),
                format!(
                    "allocation {} status={} with active capability (treat as revoked funding authority)",
                    alloc.allocation_id, alloc.status
                ),
            ));
        }
    }

    let status = findings
        .iter()
        .fold(SyncStatus::SyncOk, |acc, f| acc.escalate(f.severity));
    (status, findings)
}

fn assets_compatible(cap_asset: Option<&str>, alloc_asset: &str) -> bool {
    match cap_asset {
        None | Some("") => true,
        Some(a) => a == alloc_asset,
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::models::capabilities::CapabilityView;
    use crate::treasury::models::AllocationDto;

    fn cap(agent: &str, id: &str, max: Option<u64>, asset: Option<&str>) -> CapabilityView {
        CapabilityView {
            capability_id: id.into(),
            issuer: "iss".into(),
            subject_agent_id: agent.into(),
            actions: vec!["spend".into()],
            max_spend: max,
            asset: asset.map(str::to_string),
            valid_after: None,
            valid_before: None,
            delegation_depth: 0,
            revoked: false,
            granted_under_root_version: 1,
        }
    }

    fn alloc(agent: &str, id: &str, ceiling: i64, status: &str) -> AllocationDto {
        AllocationDto {
            allocation_id: id.into(),
            organisation_id: "org".into(),
            treasury_id: "tr".into(),
            agent_id: agent.into(),
            asset_id: "GBP".into(),
            ceiling_minor: ceiling,
            remaining_minor: ceiling,
            status: status.into(),
            expires_at: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn matching_state_is_sync_ok() {
        let snap = SyncSnapshot {
            organisation_id: "org".into(),
            capabilities: vec![cap("a1", "c1", Some(100), Some("GBP"))],
            allocations: vec![alloc("a1", "al1", 100, "active")],
        };
        let (st, f) = compare_snapshot(&snap);
        assert_eq!(st, SyncStatus::SyncOk);
        assert!(f.is_empty());
    }

    #[test]
    fn missing_allocation_detected() {
        let snap = SyncSnapshot {
            organisation_id: "org".into(),
            capabilities: vec![cap("a1", "c1", Some(100), None)],
            allocations: vec![],
        };
        let (st, f) = compare_snapshot(&snap);
        assert_eq!(st, SyncStatus::DriftDetected);
        assert!(f.iter().any(|x| x.kind == DriftKind::CapabilityWithoutAllocation));
    }

    #[test]
    fn excess_capability_requires_review() {
        let snap = SyncSnapshot {
            organisation_id: "org".into(),
            capabilities: vec![cap("a1", "c1", Some(500), Some("GBP"))],
            allocations: vec![alloc("a1", "al1", 100, "active")],
        };
        let (st, f) = compare_snapshot(&snap);
        assert_eq!(st, SyncStatus::RequiresReview);
        assert!(f
            .iter()
            .any(|x| x.kind == DriftKind::CapabilityLimitExceedsAllocation));
    }
}
