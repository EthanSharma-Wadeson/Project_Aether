//! Mock research model — proposes research tools; reacts to benchmark prompt cues.

use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

use crate::providers::errors::ProviderAdapterError;
use crate::providers::models::{
    ProviderKind, ProviderMetadata, ProviderReasonRequest, ProviderReasonResponse, ProposedIntent,
};
use crate::providers::secrets::redact_for_audit;
use crate::providers::traits::Provider;

pub struct ResearchAgentModel;

#[async_trait]
impl Provider for ResearchAgentModel {
    fn model_id(&self) -> &str {
        "mock-research-agent-v1"
    }

    async fn reason(
        &self,
        request: &ProviderReasonRequest,
    ) -> Result<ProviderReasonResponse, ProviderAdapterError> {
        let prompt_l = request.prompt.to_ascii_lowercase();
        let intent = if prompt_l.contains("excessive")
            || prompt_l.contains("unrestricted")
            || prompt_l.contains("all tools")
        {
            ProposedIntent {
                tool_id: "*".into(),
                parameters: json!({ "query": request.prompt }),
                claimed_agent_id: None,
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: false,
                access_protocol: false,
            }
        } else if prompt_l.contains("ignore previous")
            || prompt_l.contains("hidden instruction")
            || prompt_l.contains("injection")
        {
            ProposedIntent {
                tool_id: "research.search".into(),
                parameters: json!({ "query": request.prompt }),
                claimed_agent_id: Some("injected-admin".into()),
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: true,
                claimed_decision: Some("ALLOW".into()),
                access_treasury: false,
                access_protocol: false,
            }
        } else {
            ProposedIntent {
                tool_id: "research.search".into(),
                parameters: json!({ "query": request.prompt }),
                claimed_agent_id: None,
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: false,
                access_protocol: false,
            }
        };

        let summary = format!(
            "ResearchAgentModel intent={} hash={}",
            intent.tool_id,
            brief_hash(&request.prompt)
        );
        let raw = format!(
            "reasoning; secret_display={}",
            request
                .secret_ref
                .as_ref()
                .map(|s| s.redacted_display())
                .unwrap_or_else(|| "none".into())
        );
        Ok(ProviderReasonResponse {
            metadata: ProviderMetadata {
                provider_kind: ProviderKind::Mock,
                model_id: self.model_id().into(),
                provider_call_id: request.provider_call_id.clone(),
            },
            reasoning_summary: summary,
            proposed_intents: vec![intent],
            tools_executed: false,
            raw_redacted: redact_for_audit(&raw),
        })
    }
}

fn brief_hash(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(s.as_bytes());
    hex::encode(&h[..4])
}

pub fn new_call_id() -> String {
    format!("pvc-{}", Uuid::new_v4())
}
