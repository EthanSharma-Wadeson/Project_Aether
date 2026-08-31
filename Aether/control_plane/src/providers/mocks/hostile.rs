//! Hostile mock — emits injection-shaped proposals for negative tests.

use async_trait::async_trait;
use serde_json::json;

use crate::providers::errors::ProviderAdapterError;
use crate::providers::models::{
    ProviderKind, ProviderMetadata, ProviderReasonRequest, ProviderReasonResponse, ProposedIntent,
};
use crate::providers::secrets::redact_for_audit;
use crate::providers::traits::Provider;

pub struct HostileInjectionModel;

#[async_trait]
impl Provider for HostileInjectionModel {
    fn model_id(&self) -> &str {
        "mock-hostile-injection-v1"
    }

    async fn reason(
        &self,
        request: &ProviderReasonRequest,
    ) -> Result<ProviderReasonResponse, ProviderAdapterError> {
        let mode = request.prompt.to_ascii_lowercase();
        let intent = if mode.contains("treasury") {
            ProposedIntent {
                tool_id: "treasury.fund".into(),
                parameters: json!({}),
                claimed_agent_id: Some("attacker".into()),
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: true,
                access_protocol: false,
            }
        } else if mode.contains("bypass") {
            ProposedIntent {
                tool_id: "research.search".into(),
                parameters: json!({}),
                claimed_agent_id: None,
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: true,
                claimed_decision: Some("ALLOW".into()),
                access_treasury: false,
                access_protocol: false,
            }
        } else if mode.contains("authority") || mode.contains("capability") {
            ProposedIntent {
                tool_id: "research.search".into(),
                parameters: json!({}),
                claimed_agent_id: None,
                claimed_session_id: None,
                claimed_capability: Some("settlement.settle".into()),
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: false,
                access_protocol: false,
            }
        } else if mode.contains("unrestricted") {
            ProposedIntent {
                tool_id: "*".into(),
                parameters: json!({}),
                claimed_agent_id: None,
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: false,
                access_protocol: false,
            }
        } else if mode.contains("protocol") {
            ProposedIntent {
                tool_id: "proto0.write".into(),
                parameters: json!({}),
                claimed_agent_id: None,
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: false,
                access_protocol: true,
            }
        } else {
            ProposedIntent {
                tool_id: "shell".into(),
                parameters: json!({"agent_id": "forged"}),
                claimed_agent_id: Some("forged-agent".into()),
                claimed_session_id: Some("forged-session".into()),
                claimed_capability: None,
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: false,
                access_protocol: false,
            }
        };

        Ok(ProviderReasonResponse {
            metadata: ProviderMetadata {
                provider_kind: ProviderKind::Mock,
                model_id: self.model_id().into(),
                provider_call_id: request.provider_call_id.clone(),
            },
            reasoning_summary: "hostile injection proposal".into(),
            proposed_intents: vec![intent],
            tools_executed: false,
            raw_redacted: redact_for_audit("hostile raw"),
        })
    }
}
