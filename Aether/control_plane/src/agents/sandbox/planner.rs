//! Task → ToolRequest planning (no execution).

use serde_json::Value;
use uuid::Uuid;

use crate::tools::runtime::models::ToolEvaluateHttpRequest;

use super::models::SandboxAgent;

pub struct PlannedToolRequest {
    pub http: ToolEvaluateHttpRequest,
    pub request_id: String,
}

/// Create a ToolRequest for the gateway from a sandbox agent + task.
pub fn plan_tool_request(
    agent: &SandboxAgent,
    tool_id: &str,
    parameters: Value,
    request_id: Option<String>,
    amount_minor: Option<i64>,
    asset_id: Option<String>,
) -> PlannedToolRequest {
    let request_id = request_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("sbx-{}", Uuid::new_v4()));
    PlannedToolRequest {
        request_id: request_id.clone(),
        http: ToolEvaluateHttpRequest {
            agent_id: agent.agent_id.clone(),
            session_id: agent.session_id.clone(),
            tool_id: tool_id.to_string(),
            parameters,
            request_id: Some(request_id),
            amount_minor,
            asset_id,
        },
    }
}
