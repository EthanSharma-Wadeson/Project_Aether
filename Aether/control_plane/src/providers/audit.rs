use serde_json::json;

use crate::db::Db;
use crate::db::audit;
use crate::error::Result;

use super::models::GovernedProviderResult;
use super::secrets::redact_for_audit;

pub async fn record_provider_reasoned(
    db: &Db,
    actor: &str,
    correlation_id: &str,
    provider_call_id: &str,
    model_id: &str,
    agent_id: &str,
    org: &str,
    summary_redacted: &str,
) -> Result<()> {
    audit::append(
        db.pool(),
        Some(actor),
        "PROVIDER_REASONED",
        Some(agent_id),
        Some(json!({
            "correlation_id": correlation_id,
            "provider_call_id": provider_call_id,
            "model_id": model_id,
            "organisation_id": org,
            "summary": redact_for_audit(summary_redacted),
            "provider_has_authority": false,
        })),
    )
    .await?;
    Ok(())
}

pub async fn record_provider_proposal_rejected(
    db: &Db,
    actor: &str,
    correlation_id: &str,
    agent_id: &str,
    reason: &str,
) -> Result<()> {
    audit::append(
        db.pool(),
        Some(actor),
        "PROVIDER_PROPOSAL_REJECTED",
        Some(agent_id),
        Some(json!({
            "correlation_id": correlation_id,
            "reason": reason,
            "provider_has_authority": false,
        })),
    )
    .await?;
    Ok(())
}

pub async fn record_provider_governed(
    db: &Db,
    actor: &str,
    result: &GovernedProviderResult,
) -> Result<()> {
    audit::append(
        db.pool(),
        Some(actor),
        "PROVIDER_GOVERNED_DECISION",
        Some(&result.agent_id),
        Some(json!({
            "correlation_id": result.correlation_id,
            "provider_call_id": result.provider_call_id,
            "organisation_id": result.organisation_id,
            "session_id": result.session_id,
            "tool_id": result.tool_id,
            "decision": result.decision,
            "reason": result.reason,
            "tools_executed": false,
            "external_execution": false,
            "apply_invoked": false,
            "proto0_mutated": false,
            "treasury_mutated": false,
            "provider_has_authority": false,
        })),
    )
    .await?;
    Ok(())
}
