use serde_json::json;

use crate::db::Db;
use crate::error::Result;
use crate::db::audit;

use super::models::SandboxRunResult;

pub async fn record_sandbox_run(db: &Db, actor: &str, result: &SandboxRunResult) -> Result<()> {
    audit::append(
        db.pool(),
        Some(actor),
        "SANDBOX_TASK_COMPLETED",
        Some(&result.agent_id),
        Some(json!({
            "request_id": result.request_id,
            "organisation_id": result.organisation_id,
            "session_id": result.session_id,
            "tool_id": result.tool_id,
            "decision": result.decision,
            "reason": result.reason,
            "simulated_result": result.simulated_result.as_str(),
            "review_event_id": result.review_event_id,
            "external_execution": false,
        })),
    )
    .await?;
    Ok(())
}
