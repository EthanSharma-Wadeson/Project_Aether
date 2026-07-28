//! Delegation narrowing rules — children may reduce authority, never increase it.

use crate::error::{Error, Result};
use crate::permission::root::Constraints;
use crate::types::ActionSelector;

pub fn is_subset_actions(child: &[ActionSelector], parent: &[ActionSelector]) -> bool {
    child.iter().all(|a| parent.iter().any(|p| p == a))
}

/// Returns Ok if child constraints are equal to or narrower than parent.
pub fn narrows_constraints(child: &Constraints, parent: &Constraints) -> Result<()> {
    // max_spend: child must be <= parent when parent sets a ceiling
    match (child.max_spend, parent.max_spend) {
        (Some(c), Some(p)) if c > p => return Err(Error::Escalation("max_spend")),
        (None, Some(_)) => return Err(Error::Escalation("max_spend unbound")),
        (Some(_), None) | (None, None) | (Some(_), Some(_)) => {}
    }

    // asset: child may not widen; if parent set, child must match
    match (&child.asset, &parent.asset) {
        (Some(c), Some(p)) if c != p => return Err(Error::Escalation("asset")),
        (None, Some(_)) => return Err(Error::Escalation("asset unbound")),
        _ => {}
    }

    // counterparties: child set must be subset of parent set when parent restricts
    match (&child.counterparties, &parent.counterparties) {
        (Some(c), Some(p)) => {
            if !c.iter().all(|x| p.iter().any(|y| y == x)) {
                return Err(Error::Escalation("counterparties"));
            }
        }
        (None, Some(_)) => return Err(Error::Escalation("counterparties unbound")),
        _ => {}
    }

    // rate_limit: equal/lower max_ops; window must not widen beyond parent
    match (&child.rate_limit, &parent.rate_limit) {
        (Some(c), Some(p)) => {
            if c.max_ops > p.max_ops {
                return Err(Error::Escalation("rate_limit max_ops"));
            }
            if c.window_seconds > p.window_seconds {
                return Err(Error::Escalation("rate_limit window"));
            }
        }
        (None, Some(_)) => return Err(Error::Escalation("rate_limit unbound")),
        _ => {}
    }

    // valid_after: child must not start before parent
    match (child.valid_after, parent.valid_after) {
        (Some(c), Some(p)) if c < p => return Err(Error::Escalation("valid_after")),
        (None, Some(_)) => return Err(Error::Escalation("valid_after unbound")),
        _ => {}
    }

    // valid_before: child must not end after parent
    match (child.valid_before, parent.valid_before) {
        (Some(c), Some(p)) if c > p => return Err(Error::Escalation("valid_before")),
        (None, Some(_)) => return Err(Error::Escalation("valid_before unbound")),
        _ => {}
    }

    Ok(())
}
