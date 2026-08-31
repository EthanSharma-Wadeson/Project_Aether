//! Deterministic decision engine (pure — no I/O).

use chrono::Utc;
use uuid::Uuid;

use super::models::{
    AgentAuthoritySnapshot, AllocationSnap, CheckEvidence, DenyReason, EnforcementDecision,
    EnforcementOutcome, EnforcementRequest, RemediationHint, RiskResult,
};
use super::rules::{
    allocation_expired, allocation_inactive, assets_compatible, capability_covers_action,
    effective_limit, is_spend_scoped, validate_request,
};

const REMEDIATION_NOTE: &str =
    "E2 only — remediation requires future E1 workflow / E3 Apply proposal; no auto mutation.";

/// Evaluate authority for an agent action. Fail-closed. Deterministic for fixed inputs.
pub fn decide(
    req: &EnforcementRequest,
    snap: &AgentAuthoritySnapshot,
    organisation_id: &str,
    request_id: Option<String>,
) -> EnforcementDecision {
    let request_id = request_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let timestamp = Utc::now();
    let base = |outcome: EnforcementOutcome,
                deny: Option<DenyReason>,
                reason: Option<String>,
                cap_ok: bool,
                alloc_ok: bool,
                pol_ok: bool,
                risk: RiskResult,
                evidence: CheckEvidence,
                remediation: bool,
                proposal: Option<String>| EnforcementDecision {
        decision: outcome,
        decision_code: outcome.as_str().to_string(),
        reason: reason.or_else(|| deny.map(|d| d.as_str().to_string())),
        deny_reason: deny,
        agent_id: req.agent_id.clone(),
        organisation_id: organisation_id.to_string(),
        requested_action: req.action.clone(),
        capability_checked: cap_ok,
        allocation_checked: alloc_ok,
        policy_checked: pol_ok,
        risk_result: risk,
        evidence,
        remediation: RemediationHint {
            requires_remediation: remediation,
            proposal_candidate: proposal,
            note: REMEDIATION_NOTE,
        },
        request_id: request_id.clone(),
        timestamp,
        observation_only_execution: true,
        apply_invoked: false,
        treasury_mutated: false,
        proto0_mutated: false,
    };

    let empty_ev = CheckEvidence {
        capability_id: None,
        capability_max_spend: None,
        allocation_id: None,
        allocation_remaining_minor: None,
        allocation_status: None,
        treasury_id: None,
        treasury_status: None,
        policy_id: None,
        policy_status: None,
        agent_status: Some(snap.agent_status.clone()),
        effective_limit_minor: None,
    };

    if let Err(msg) = validate_request(req) {
        return base(
            EnforcementOutcome::Deny,
            Some(DenyReason::InvalidRequest),
            Some(msg),
            false,
            false,
            false,
            RiskResult::Block,
            empty_ev,
            false,
            None,
        );
    }

    if let Some(ref org) = req.organisation_id {
        if org != organisation_id {
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::OrganisationMismatch),
                Some("request organisation_id does not match control-plane treasury org".into()),
                false,
                false,
                false,
                RiskResult::Block,
                empty_ev,
                false,
                None,
            );
        }
    }

    // Identity
    match snap.agent_status.as_str() {
        "frozen" => {
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::AgentFrozen),
                None,
                false,
                false,
                false,
                RiskResult::Block,
                empty_ev,
                true,
                Some("propose_freeze_review_or_unfreeze_via_apply".into()),
            );
        }
        "revoked" => {
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::AgentRevoked),
                None,
                false,
                false,
                false,
                RiskResult::Block,
                empty_ev,
                true,
                Some("propose_capability_cleanup".into()),
            );
        }
        "active" => {}
        "not_found" | "" => {
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::AgentNotFound),
                None,
                false,
                false,
                false,
                RiskResult::Block,
                empty_ev,
                false,
                None,
            );
        }
        other => {
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::AgentNotFound),
                Some(format!("unknown agent status: {other}")),
                false,
                false,
                false,
                RiskResult::Block,
                empty_ev,
                false,
                None,
            );
        }
    }

    // Unsupported action (empty already validated)
    if req.action.chars().any(|c| c == '\0') {
        return base(
            EnforcementOutcome::Deny,
            Some(DenyReason::UnsupportedAction),
            None,
            false,
            false,
            false,
            RiskResult::Block,
            empty_ev,
            false,
            None,
        );
    }

    let spend = is_spend_scoped(&req.action);
    let amount = req.amount_minor.unwrap_or(0);

    // Asset known?
    if let Some(ref asset) = req.asset_id {
        if !snap.known_assets.iter().any(|a| a == asset) && spend {
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::UnknownAsset),
                Some(format!("asset {asset} not registered in treasury observation")),
                false,
                false,
                false,
                RiskResult::Block,
                empty_ev,
                false,
                None,
            );
        }
    }

    // Capability
    let matching_caps: Vec<_> = snap
        .capabilities
        .iter()
        .filter(|c| !c.revoked && capability_covers_action(c, &req.action))
        .collect();

    if matching_caps.is_empty() {
        let would_match_if_active = snap.capabilities.iter().any(|c| {
            c.actions.iter().any(|x| x == &req.action || x == "*")
        });
        let deny = if would_match_if_active && snap.capabilities.iter().all(|c| c.revoked) {
            DenyReason::CapabilityRevoked
        } else if snap
            .capabilities
            .iter()
            .any(|c| !c.revoked && !capability_covers_action(c, &req.action))
        {
            DenyReason::CapabilityActionMismatch
        } else {
            DenyReason::CapabilityMissing
        };
        return base(
            EnforcementOutcome::Deny,
            Some(deny),
            None,
            true,
            false,
            false,
            RiskResult::Block,
            empty_ev,
            true,
            Some("propose_capability_grant_or_fund".into()),
        );
    }

    // Prefer cap with matching asset if provided
    let cap = matching_caps
        .iter()
        .find(|c| match (&c.asset, &req.asset_id) {
            (Some(a), Some(r)) => a == r,
            _ => true,
        })
        .copied()
        .unwrap_or(matching_caps[0]);

    if let Some(max) = cap.max_spend {
        if spend && amount > 0 && (amount as u64) > max {
            let mut ev = empty_ev.clone();
            ev.capability_id = Some(cap.capability_id.clone());
            ev.capability_max_spend = Some(max);
            ev.agent_status = Some(snap.agent_status.clone());
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::CapabilityLimitExceeded),
                Some(format!("amount {amount} exceeds capability max_spend {max}")),
                true,
                false,
                false,
                RiskResult::Block,
                ev,
                true,
                Some("propose_capability_reduce_or_lower_request".into()),
            );
        }
    }

    // Policy
    let mut policy_checked = true;
    let mut policy_id = None;
    let mut policy_status = None;
    if let Some(ref pol) = snap.policy {
        policy_id = Some(pol.policy_id.clone());
        policy_status = Some(pol.status.clone());
        if pol.status != "approved" {
            let mut ev = empty_ev.clone();
            ev.capability_id = Some(cap.capability_id.clone());
            ev.capability_max_spend = cap.max_spend;
            ev.policy_id = policy_id.clone();
            ev.policy_status = policy_status.clone();
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::PolicyBlocked),
                Some(format!("policy {} status is {}", pol.policy_id, pol.status)),
                true,
                false,
                true,
                RiskResult::Block,
                ev,
                false,
                None,
            );
        }
        if pol.blocked_actions.iter().any(|a| a == &req.action)
            || pol.policy_type == "spend_block"
                && is_spend_scoped(&req.action)
        {
            let mut ev = empty_ev.clone();
            ev.capability_id = Some(cap.capability_id.clone());
            ev.policy_id = policy_id;
            ev.policy_status = policy_status;
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::PolicyBlocked),
                Some("approved policy blocks this action".into()),
                true,
                false,
                true,
                RiskResult::Block,
                ev,
                false,
                None,
            );
        }
    } else if req.policy_id.is_some() {
        let mut ev = empty_ev.clone();
        ev.capability_id = Some(cap.capability_id.clone());
        ev.policy_id = req.policy_id.clone();
        return base(
            EnforcementOutcome::Deny,
            Some(DenyReason::PolicyBlocked),
            Some("required policy_id not found or not readable".into()),
            true,
            false,
            true,
            RiskResult::Block,
            ev,
            false,
            None,
        );
    } else {
        policy_checked = true; // no blocking policy required
    }

    // Treasury allocation (required for spend-scoped)
    let mut alloc_checked = false;
    let mut chosen_alloc: Option<&AllocationSnap> = None;

    if spend {
        alloc_checked = true;
        let now = timestamp;
        let candidates: Vec<_> = snap
            .allocations
            .iter()
            .filter(|a| a.agent_id == req.agent_id)
            .filter(|a| a.organisation_id == organisation_id)
            .filter(|a| assets_compatible(cap.asset.as_deref(), req.asset_id.as_deref(), &a.asset_id))
            .collect();

        if candidates.is_empty() {
            let mut ev = empty_ev.clone();
            ev.capability_id = Some(cap.capability_id.clone());
            ev.capability_max_spend = cap.max_spend;
            ev.policy_id = policy_id.clone();
            ev.policy_status = policy_status.clone();
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::AllocationMissing),
                None,
                true,
                true,
                policy_checked,
                RiskResult::Block,
                ev,
                true,
                Some("propose_fund_allocation_or_revoke_capability".into()),
            );
        }

        // Prefer active non-expired with enough remaining
        for a in &candidates {
            let tstatus = snap
                .treasury_status_by_id
                .get(&a.treasury_id)
                .cloned()
                .unwrap_or_else(|| "unknown".into());
            if tstatus == "frozen" {
                let mut ev = empty_ev.clone();
                ev.capability_id = Some(cap.capability_id.clone());
                ev.allocation_id = Some(a.allocation_id.clone());
                ev.allocation_remaining_minor = Some(a.remaining_minor);
                ev.allocation_status = Some(a.status.clone());
                ev.treasury_id = Some(a.treasury_id.clone());
                ev.treasury_status = Some(tstatus);
                ev.policy_id = policy_id.clone();
                ev.policy_status = policy_status.clone();
                return base(
                    EnforcementOutcome::Deny,
                    Some(DenyReason::TreasuryFrozen),
                    None,
                    true,
                    true,
                    policy_checked,
                    RiskResult::Block,
                    ev,
                    true,
                    Some("propose_treasury_unfreeze_dual_control".into()),
                );
            }
        }

        if let Some(a) = candidates.iter().find(|a| allocation_expired(a, now)) {
            let mut ev = empty_ev.clone();
            ev.capability_id = Some(cap.capability_id.clone());
            ev.allocation_id = Some(a.allocation_id.clone());
            ev.allocation_remaining_minor = Some(a.remaining_minor);
            ev.allocation_status = Some(a.status.clone());
            ev.treasury_id = Some(a.treasury_id.clone());
            ev.policy_id = policy_id.clone();
            ev.policy_status = policy_status.clone();
            // If ALL candidates expired / none active — DENY expired
            let any_active = candidates
                .iter()
                .any(|x| x.status == "active" && !allocation_expired(x, now));
            if !any_active {
                return base(
                    EnforcementOutcome::Deny,
                    Some(DenyReason::ExpiredAllocation),
                    None,
                    true,
                    true,
                    policy_checked,
                    RiskResult::Block,
                    ev,
                    true,
                    Some("propose_renew_allocation_or_revoke_capability".into()),
                );
            }
        }

        if candidates
            .iter()
            .all(|a| allocation_inactive(a) || allocation_expired(a, now))
        {
            let a = candidates[0];
            let mut ev = empty_ev.clone();
            ev.capability_id = Some(cap.capability_id.clone());
            ev.allocation_id = Some(a.allocation_id.clone());
            ev.allocation_status = Some(a.status.clone());
            let deny = if allocation_expired(a, now) {
                DenyReason::ExpiredAllocation
            } else {
                DenyReason::InactiveAllocation
            };
            return base(
                EnforcementOutcome::Deny,
                Some(deny),
                None,
                true,
                true,
                policy_checked,
                RiskResult::Block,
                ev,
                true,
                Some("propose_reactivate_or_revoke".into()),
            );
        }

        let active: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|a| a.status == "active" && !allocation_expired(a, now))
            .collect();

        let a = match active
            .iter()
            .find(|a| a.remaining_minor >= amount)
            .copied()
            .or_else(|| active.first().copied())
        {
            Some(a) => a,
            None => {
                let mut ev = empty_ev.clone();
                ev.capability_id = Some(cap.capability_id.clone());
                return base(
                    EnforcementOutcome::Deny,
                    Some(DenyReason::AllocationMissing),
                    None,
                    true,
                    true,
                    policy_checked,
                    RiskResult::Block,
                    ev,
                    true,
                    Some("propose_fund_allocation".into()),
                );
            }
        };

        if a.remaining_minor < amount {
            let mut ev = empty_ev.clone();
            ev.capability_id = Some(cap.capability_id.clone());
            ev.capability_max_spend = cap.max_spend;
            ev.allocation_id = Some(a.allocation_id.clone());
            ev.allocation_remaining_minor = Some(a.remaining_minor);
            ev.allocation_status = Some(a.status.clone());
            ev.treasury_id = Some(a.treasury_id.clone());
            ev.effective_limit_minor = Some(effective_limit(cap.max_spend, a.remaining_minor));
            return base(
                EnforcementOutcome::Deny,
                Some(DenyReason::AllocationExceeded),
                Some(format!(
                    "amount {amount} exceeds allocation remaining {}",
                    a.remaining_minor
                )),
                true,
                true,
                policy_checked,
                RiskResult::Block,
                ev,
                true,
                Some("propose_increase_allocation_or_reduce_request".into()),
            );
        }

        chosen_alloc = Some(a);
    }

    // Risk
    let risk = evaluate_risk(req, amount, spend);
    if risk == RiskResult::Block {
        let ev = evidence_from(cap, chosen_alloc, &policy_id, &policy_status, snap);
        return base(
            EnforcementOutcome::Deny,
            Some(DenyReason::PolicyBlocked),
            Some("risk control blocked action".into()),
            true,
            alloc_checked,
            policy_checked,
            risk,
            ev,
            true,
            Some("requires_risk_review".into()),
        );
    }

    let mut ev = evidence_from(cap, chosen_alloc, &policy_id, &policy_status, snap);
    if let Some(a) = chosen_alloc {
        ev.effective_limit_minor = Some(effective_limit(cap.max_spend, a.remaining_minor));
    }

    if risk == RiskResult::Escalate || req.force_review {
        return base(
            EnforcementOutcome::RequiresReview,
            None,
            Some("risk escalation or force_review".into()),
            true,
            alloc_checked,
            policy_checked,
            RiskResult::Escalate,
            ev,
            false,
            Some("human_review_before_allow".into()),
        );
    }

    base(
        EnforcementOutcome::Allow,
        None,
        Some("all authority checks passed".into()),
        true,
        alloc_checked || !spend,
        policy_checked,
        RiskResult::Pass,
        ev,
        false,
        None,
    )
}

fn evidence_from(
    cap: &super::models::CapabilitySnap,
    alloc: Option<&AllocationSnap>,
    policy_id: &Option<String>,
    policy_status: &Option<String>,
    snap: &AgentAuthoritySnapshot,
) -> CheckEvidence {
    CheckEvidence {
        capability_id: Some(cap.capability_id.clone()),
        capability_max_spend: cap.max_spend,
        allocation_id: alloc.map(|a| a.allocation_id.clone()),
        allocation_remaining_minor: alloc.map(|a| a.remaining_minor),
        allocation_status: alloc.map(|a| a.status.clone()),
        treasury_id: alloc.map(|a| a.treasury_id.clone()),
        treasury_status: alloc.and_then(|a| snap.treasury_status_by_id.get(&a.treasury_id).cloned()),
        policy_id: policy_id.clone(),
        policy_status: policy_status.clone(),
        agent_status: Some(snap.agent_status.clone()),
        effective_limit_minor: None,
    }
}

fn evaluate_risk(req: &EnforcementRequest, amount: i64, spend: bool) -> RiskResult {
    if req.force_review {
        return RiskResult::Escalate;
    }
    // Soft velocity / size gate — escalate large spends; never silent allow around it.
    if spend && amount >= 1_000_000 {
        return RiskResult::Escalate;
    }
    RiskResult::Pass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enforcement::models::{AllocationSnap, CapabilitySnap};
    use std::collections::HashMap;

    fn req(action: &str, amount: i64) -> EnforcementRequest {
        EnforcementRequest {
            agent_id: "agent-1".into(),
            organisation_id: Some("org".into()),
            action: action.into(),
            asset_id: Some("AETHER_TEST".into()),
            amount_minor: Some(amount),
            policy_id: None,
            force_review: false,
        }
    }

    fn snap_ok(remaining: i64) -> AgentAuthoritySnapshot {
        let mut treasuries = HashMap::new();
        treasuries.insert("tr1".into(), "active".into());
        AgentAuthoritySnapshot {
            agent_id: "agent-1".into(),
            agent_status: "active".into(),
            capabilities: vec![CapabilitySnap {
                capability_id: "cap1".into(),
                actions: vec!["settlement.settle".into()],
                max_spend: Some(5_000),
                asset: Some("AETHER_TEST".into()),
                revoked: false,
            }],
            allocations: vec![AllocationSnap {
                allocation_id: "al1".into(),
                organisation_id: "org".into(),
                treasury_id: "tr1".into(),
                agent_id: "agent-1".into(),
                asset_id: "AETHER_TEST".into(),
                ceiling_minor: 5_000,
                remaining_minor: remaining,
                status: "active".into(),
                expires_at: None,
            }],
            treasury_status_by_id: treasuries,
            known_assets: vec!["AETHER_TEST".into(), "GBP".into()],
            policy: None,
        }
    }

    #[test]
    fn allow_when_funded() {
        let d = decide(&req("settlement.settle", 100), &snap_ok(5_000), "org", Some("r1".into()));
        assert_eq!(d.decision, EnforcementOutcome::Allow);
        assert_eq!(d.request_id, "r1");
        assert!(!d.apply_invoked);
        assert!(!d.treasury_mutated);
    }

    #[test]
    fn deny_missing_capability() {
        let mut s = snap_ok(5_000);
        s.capabilities.clear();
        let d = decide(&req("settlement.settle", 100), &s, "org", None);
        assert_eq!(d.decision, EnforcementOutcome::Deny);
        assert_eq!(d.deny_reason, Some(DenyReason::CapabilityMissing));
    }

    #[test]
    fn deny_insufficient_allocation() {
        let d = decide(&req("settlement.settle", 200), &snap_ok(100), "org", None);
        assert_eq!(d.decision, EnforcementOutcome::Deny);
        assert_eq!(d.deny_reason, Some(DenyReason::AllocationExceeded));
    }

    #[test]
    fn deny_expired_allocation() {
        let mut s = snap_ok(5_000);
        s.allocations[0].status = "expired".into();
        let d = decide(&req("settlement.settle", 100), &s, "org", None);
        assert_eq!(d.decision, EnforcementOutcome::Deny);
        assert_eq!(d.deny_reason, Some(DenyReason::ExpiredAllocation));
    }

    #[test]
    fn deny_frozen_agent() {
        let mut s = snap_ok(5_000);
        s.agent_status = "frozen".into();
        let d = decide(&req("settlement.settle", 100), &s, "org", None);
        assert_eq!(d.deny_reason, Some(DenyReason::AgentFrozen));
    }

    #[test]
    fn deterministic_same_inputs() {
        let s = snap_ok(5_000);
        let r = req("settlement.settle", 50);
        let a = decide(&r, &s, "org", Some("same".into()));
        let b = decide(&r, &s, "org", Some("same".into()));
        assert_eq!(a.decision, b.decision);
        assert_eq!(a.deny_reason, b.deny_reason);
        assert_eq!(a.decision_code, b.decision_code);
        assert_eq!(a.evidence.allocation_id, b.evidence.allocation_id);
    }
}
