//! Pure rule helpers for E2 evaluation.

use chrono::{DateTime, Utc};

use super::models::{AllocationSnap, CapabilitySnap, EnforcementRequest};

/// Actions that require treasury allocation + spend limits.
pub fn is_spend_scoped(action: &str) -> bool {
    let a = action.to_ascii_lowercase();
    a.contains("spend")
        || a.contains("settle")
        || a.contains("payment")
        || a.contains("transfer")
        || a == "reserve"
        || a.contains("escrow")
}

pub fn allocation_expired(a: &AllocationSnap, now: DateTime<Utc>) -> bool {
    if a.status == "expired" {
        return true;
    }
    if let Some(ref exp) = a.expires_at {
        if let Ok(dt) = DateTime::parse_from_rfc3339(exp) {
            return dt.with_timezone(&Utc) <= now;
        }
    }
    false
}

pub fn allocation_inactive(a: &AllocationSnap) -> bool {
    matches!(a.status.as_str(), "closed" | "frozen" | "expired")
}

pub fn capability_covers_action(cap: &CapabilitySnap, action: &str) -> bool {
    if cap.revoked {
        return false;
    }
    cap.actions.iter().any(|x| {
        x == action || x == "*" || (action.starts_with("settlement.") && x.starts_with("settlement."))
    })
}

pub fn assets_compatible(cap_asset: Option<&str>, req_asset: Option<&str>, alloc_asset: &str) -> bool {
    if let Some(req) = req_asset {
        if req != alloc_asset {
            return false;
        }
    }
    match cap_asset {
        None | Some("") => true,
        Some(a) => a == alloc_asset,
    }
}

pub fn validate_request(req: &EnforcementRequest) -> Result<(), String> {
    if req.agent_id.trim().is_empty() {
        return Err("agent_id required".into());
    }
    if req.action.trim().is_empty() {
        return Err("action required".into());
    }
    if let Some(amt) = req.amount_minor {
        if amt < 0 {
            return Err("amount_minor must be >= 0".into());
        }
    }
    Ok(())
}

/// Effective numeric limit when both cap and allocation present.
pub fn effective_limit(max_spend: Option<u64>, remaining_minor: i64) -> i64 {
    match max_spend {
        Some(m) => (m as i64).min(remaining_minor),
        None => remaining_minor,
    }
}
