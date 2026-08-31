//! Anthropic (Claude) lab adapter — proposals only; no tool execution.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::config::is_production_mode;
use crate::providers::errors::ProviderAdapterError;
use crate::providers::external::intent_extract::extract_proposed_intents;
use crate::providers::external::transport::{LabHttpTransport, ReqwestLabTransport};
use crate::providers::mapper::MockAllowlistProfile;
use crate::providers::models::{
    ProviderKind, ProviderMetadata, ProviderReasonRequest, ProviderReasonResponse,
};
use crate::providers::secrets::{redact_for_audit, LabApiKey};
use crate::providers::traits::Provider;

const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicLabProvider {
    transport: Arc<dyn LabHttpTransport>,
    api_url: String,
    model_id: String,
    allowlist_profile: MockAllowlistProfile,
}

impl AnthropicLabProvider {
    pub fn lab_live(allowlist_profile: MockAllowlistProfile) -> Result<Self, ProviderAdapterError> {
        if is_production_mode() {
            return Err(ProviderAdapterError::ExternalProviderForbidden(
                "external providers disabled in production CP_ENV".into(),
            ));
        }
        Ok(Self {
            transport: Arc::new(ReqwestLabTransport::new()?),
            api_url: std::env::var("AETHER_LAB_ANTHROPIC_API_URL")
                .unwrap_or_else(|_| DEFAULT_API_URL.into()),
            model_id: std::env::var("AETHER_LAB_ANTHROPIC_MODEL")
                .unwrap_or_else(|_| DEFAULT_MODEL.into()),
            allowlist_profile,
        })
    }

    pub fn with_transport(
        transport: Arc<dyn LabHttpTransport>,
        allowlist_profile: MockAllowlistProfile,
    ) -> Self {
        Self {
            transport,
            api_url: DEFAULT_API_URL.into(),
            model_id: DEFAULT_MODEL.into(),
            allowlist_profile,
        }
    }

    fn system_prompt(&self) -> String {
        let tools = match self.allowlist_profile {
            MockAllowlistProfile::Research => "research.search",
            MockAllowlistProfile::Finance | MockAllowlistProfile::Hostile => {
                "purchase.subscription, research.search"
            }
        };
        format!(
            "You are an untrusted reasoning engine inside Aether lab.\n\
             You have ZERO authority. You cannot grant capabilities, access treasury, \
             mutate protocol, bypass enforcement, or execute tools.\n\
             Respond with ONLY a JSON object of the form:\n\
             {{\"proposed_intents\":[{{\"tool_id\":\"...\",\"parameters\":{{}}}}]}}\n\
             Prefer tool_id from: {tools}.\n\
             Do not claim bypass_e2, claimed_decision, access_treasury, or access_protocol."
        )
    }
}

#[async_trait]
impl Provider for AnthropicLabProvider {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    async fn reason(
        &self,
        request: &ProviderReasonRequest,
    ) -> Result<ProviderReasonResponse, ProviderAdapterError> {
        if is_production_mode() {
            return Err(ProviderAdapterError::ExternalProviderForbidden(
                "external providers disabled in production".into(),
            ));
        }

        let secret_ref = request.secret_ref.clone().ok_or_else(|| {
            ProviderAdapterError::InvalidSecretRef(
                "external Anthropic lab requires secret_ref handle".into(),
            )
        })?;
        let api_key: LabApiKey = secret_ref.resolve_lab_api_key()?;

        let body = json!({
            "model": self.model_id,
            "max_tokens": 512,
            "system": self.system_prompt(),
            "messages": [{
                "role": "user",
                "content": request.prompt,
            }]
        });

        let headers = vec![
            ("x-api-key".into(), api_key.expose().to_string()),
            ("anthropic-version".into(), ANTHROPIC_VERSION.into()),
            ("content-type".into(), "application/json".into()),
        ];
        // Drop key from stack ASAP after building headers — LabApiKey Drop zeros conceptually via drop.
        drop(api_key);

        let (status, text) = self
            .transport
            .post_json(&self.api_url, headers, body)
            .await?;

        if !(200..300).contains(&status) {
            return Err(ProviderAdapterError::ProviderHttp(format!(
                "anthropic lab HTTP {status}: {}",
                redact_for_audit(&text.chars().take(200).collect::<String>())
            )));
        }

        let model_text = extract_anthropic_text(&text);
        let intents = extract_proposed_intents(&model_text);
        let summary = format!(
            "AnthropicLabProvider extracted {} intent(s); provider_has_authority=false",
            intents.len()
        );

        Ok(ProviderReasonResponse {
            metadata: ProviderMetadata {
                provider_kind: ProviderKind::Anthropic,
                model_id: self.model_id.clone(),
                provider_call_id: request.provider_call_id.clone(),
            },
            reasoning_summary: summary,
            proposed_intents: intents,
            tools_executed: false,
            raw_redacted: redact_for_audit(&model_text.chars().take(400).collect::<String>()),
        })
    }
}

fn extract_anthropic_text(response_body: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(response_body) else {
        return response_body.to_string();
    };
    if let Some(arr) = v.get("content").and_then(|c| c.as_array()) {
        let mut out = String::new();
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                    out.push_str(t);
                }
            }
        }
        if !out.is_empty() {
            return out;
        }
    }
    response_body.to_string()
}

/// Build Anthropic provider for a profile string, or None if not external.
pub fn try_build_external(
    profile: &str,
    transport: Option<Arc<dyn LabHttpTransport>>,
) -> Result<Option<(MockAllowlistProfile, Box<dyn Provider>)>, ProviderAdapterError> {
    let p = profile.trim().to_ascii_lowercase();
    let allowlist = match p.as_str() {
        "anthropic" | "claude" | "external" | "external_research" => MockAllowlistProfile::Research,
        "anthropic_finance" | "claude_finance" | "external_finance" => MockAllowlistProfile::Finance,
        _ => return Ok(None),
    };
    let provider: Box<dyn Provider> = if let Some(t) = transport {
        Box::new(AnthropicLabProvider::with_transport(t, allowlist))
    } else {
        Box::new(AnthropicLabProvider::lab_live(allowlist)?)
    };
    Ok(Some((allowlist, provider)))
}

/// Default lab secret handle for Anthropic.
pub fn default_anthropic_secret_handle(org: &str) -> String {
    format!("secret://org/{org}/providers/anthropic/api_key")
}
