//! Gateway policy helpers — risk → review escalation (no execution).

use super::models::{GatewayOutcome, RiskLevel, ToolStatus};
use crate::enforcement::models::EnforcementOutcome;

/// Compose gateway outcome from tool status + risk + E2.
pub fn compose_decision(
    tool_status: ToolStatus,
    risk: RiskLevel,
    e2: EnforcementOutcome,
) -> GatewayOutcome {
    if tool_status == ToolStatus::Disabled {
        return GatewayOutcome::Deny;
    }
    match e2 {
        EnforcementOutcome::Deny => GatewayOutcome::Deny,
        EnforcementOutcome::RequiresReview => GatewayOutcome::RequiresReview,
        EnforcementOutcome::Allow => {
            if tool_status == ToolStatus::Restricted || risk.forces_review() {
                GatewayOutcome::RequiresReview
            } else {
                GatewayOutcome::Allow
            }
        }
    }
}

pub fn deny_reason_for_disabled() -> &'static str {
    "TOOL_DISABLED"
}

pub fn deny_reason_unknown() -> &'static str {
    "TOOL_NOT_REGISTERED"
}
