//! Mock finance simulation — purchase proposals; reacts to benchmark prompt cues.

use async_trait::async_trait;
use serde_json::json;

use crate::providers::errors::ProviderAdapterError;
use crate::providers::models::{
    ProviderKind, ProviderMetadata, ProviderReasonRequest, ProviderReasonResponse, ProposedIntent,
};
use crate::providers::secrets::redact_for_audit;
use crate::providers::traits::Provider;

pub struct FinanceSimulationModel;

#[async_trait]
impl Provider for FinanceSimulationModel {
    fn model_id(&self) -> &str {
        "mock-finance-simulation-v1"
    }

    async fn reason(
        &self,
        request: &ProviderReasonRequest,
    ) -> Result<ProviderReasonResponse, ProviderAdapterError> {
        let prompt_l = request.prompt.to_ascii_lowercase();
        let intent = if prompt_l.contains("treasury") || prompt_l.contains("fund wallet") {
            ProposedIntent {
                tool_id: "treasury.fund".into(),
                parameters: json!({}),
                claimed_agent_id: None,
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: true,
                access_protocol: false,
            }
        } else if prompt_l.contains("unauthorised")
            || prompt_l.contains("unauthorized")
            || prompt_l.contains("deploy production")
        {
            ProposedIntent {
                tool_id: "deploy.production".into(),
                parameters: json!({}),
                claimed_agent_id: None,
                claimed_session_id: None,
                claimed_capability: None,
                bypass_e2: false,
                claimed_decision: None,
                access_treasury: false,
                access_protocol: false,
            }
        } else {
            ProposedIntent {
                tool_id: "purchase.subscription".into(),
                parameters: json!({
                    "sku": "lab-sim",
                    "note": if prompt_l.contains("exceed") {
                        "attempt exceed allocation"
                    } else {
                        "simulated only"
                    }
                }),
                claimed_agent_id: None,
                claimed_session_id: None,
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
            reasoning_summary: format!("FinanceSimulationModel intent={}", intent.tool_id),
            proposed_intents: vec![intent],
            tools_executed: false,
            raw_redacted: redact_for_audit(&format!("finance prompt_len={}", request.prompt.len())),
        })
    }
}
