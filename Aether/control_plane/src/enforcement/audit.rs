use serde_json::json;

use crate::db::Db;
use crate::error::Result;
use crate::db::audit;

use super::models::{EnforcementDecision, EnforcementOutcome};

pub async fn record_decision(
    db: &Db,
    actor_id: &str,
    decision: &EnforcementDecision,
) -> Result<()> {
    let action = match decision.decision {
        EnforcementOutcome::Allow => "ENFORCEMENT_ALLOWED",
        EnforcementOutcome::Deny => "ENFORCEMENT_DENIED",
        EnforcementOutcome::RequiresReview => "ENFORCEMENT_REVIEW_REQUIRED",
    };

    // Always record the request marker first for correlation.
    audit::append(
        db.pool(),
        Some(actor_id),
        "ENFORCEMENT_REQUESTED",
        Some(&decision.agent_id),
        Some(json!({
            "request_id": decision.request_id,
            "organisation_id": decision.organisation_id,
            "action": decision.requested_action,
            "decision_code": decision.decision_code,
        })),
    )
    .await?;

    audit::append(
        db.pool(),
        Some(actor_id),
        action,
        Some(&decision.agent_id),
        Some(json!({
            "request_id": decision.request_id,
            "organisation_id": decision.organisation_id,
            "agent_id": decision.agent_id,
            "capability_id": decision.evidence.capability_id,
            "allocation_id": decision.evidence.allocation_id,
            "policy_id": decision.evidence.policy_id,
            "decision_code": decision.decision_code,
            "deny_reason": decision.deny_reason.map(|d| d.as_str()),
            "risk_result": decision.risk_result.as_str(),
            "apply_invoked": false,
            "treasury_mutated": false,
            "proto0_mutated": false,
        })),
    )
    .await?;

    Ok(())
}
