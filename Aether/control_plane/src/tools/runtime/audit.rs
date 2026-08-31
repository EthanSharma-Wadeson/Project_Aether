use serde_json::json;

use crate::db::Db;
use crate::error::Result;
use crate::db::audit;

use super::models::GatewayDecision;

pub async fn record_requested(
    db: &Db,
    actor: &str,
    decision: &GatewayDecision,
) -> Result<()> {
    audit::append(
        db.pool(),
        Some(actor),
        "TOOL_REQUESTED",
        Some(&decision.tool_id),
        Some(json!({
            "agent_id": decision.agent_id,
            "organisation_id": decision.evidence.organisation_id,
            "session_id": decision.session_id,
            "tool_id": decision.tool_id,
            "request_id": decision.request_id,
            "parameters_hash": decision.evidence.parameters_hash,
        })),
    )
    .await?;
    Ok(())
}

pub async fn record_outcome(db: &Db, actor: &str, decision: &GatewayDecision) -> Result<()> {
    let action = match decision.decision {
        super::models::GatewayOutcome::Allow => "TOOL_ALLOWED",
        super::models::GatewayOutcome::Deny => "TOOL_DENIED",
        super::models::GatewayOutcome::RequiresReview => "TOOL_REVIEW_REQUIRED",
    };
    audit::append(
        db.pool(),
        Some(actor),
        action,
        Some(&decision.tool_id),
        Some(json!({
            "agent_id": decision.agent_id,
            "organisation_id": decision.evidence.organisation_id,
            "session_id": decision.session_id,
            "tool_id": decision.tool_id,
            "decision": decision.decision_code,
            "reason": decision.reason,
            "request_id": decision.request_id,
        })),
    )
    .await?;
    Ok(())
}
