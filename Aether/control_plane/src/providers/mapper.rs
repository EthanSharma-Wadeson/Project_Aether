//! Map untrusted model proposals → ToolEvaluateHttpRequest.
//! Strips all authority claims; allowlists tools; rejects injection patterns.

use serde_json::json;

use crate::tools::runtime::models::ToolEvaluateHttpRequest;

use super::errors::ProviderAdapterError;
use super::models::ProposedIntent;

/// Tools that mocks may propose (lab allowlist). Unknown → reject before gateway.
const RESEARCH_ALLOWLIST: &[&str] = &["research.search"];
const FINANCE_ALLOWLIST: &[&str] = &["purchase.subscription", "research.search"];

const FORBIDDEN_PREFIXES: &[&str] = &[
    "treasury.",
    "proto0.",
    "proto.",
    "apply.",
    "admin.",
    "wallet.",
    "capability.",
    "allocation.",
];

const FORBIDDEN_TOOLS: &[&str] = &[
    "*",
    "unrestricted",
    "shell",
    "bash",
    "http.fetch",
    "treasury.fund",
    "treasury.write",
    "proto0.write",
    "apply.execute",
    "create_capability",
    "grant_authority",
];

#[derive(Debug, Clone, Copy)]
pub enum MockAllowlistProfile {
    Research,
    Finance,
    /// Hostile profile still maps through the same sanitizer (for injection tests).
    Hostile,
}

impl MockAllowlistProfile {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "research" | "research_agent" | "agent_a" => Some(Self::Research),
            "finance" | "finance_simulation" | "agent_b" => Some(Self::Finance),
            "hostile" | "injection" => Some(Self::Hostile),
            // External lab profiles map to allowlists (provider selected separately).
            "anthropic" | "claude" | "external" | "external_research" => Some(Self::Research),
            "anthropic_finance" | "claude_finance" | "external_finance" => Some(Self::Finance),
            _ => None,
        }
    }

    pub fn allowlist(self) -> &'static [&'static str] {
        match self {
            Self::Research => RESEARCH_ALLOWLIST,
            Self::Finance | Self::Hostile => FINANCE_ALLOWLIST,
        }
    }
}

/// Result of sanitizing one proposed intent.
pub struct MappedIntent {
    pub http: ToolEvaluateHttpRequest,
    /// True if model attempted authority / bypass / treasury access (stripped or rejected).
    pub injection_flags: Vec<String>,
}

/// Sanitize a single untrusted intent. Trusted identity comes from caller args only.
pub fn map_proposed_intent(
    intent: &ProposedIntent,
    trusted_agent_id: &str,
    trusted_session_id: &str,
    profile: MockAllowlistProfile,
    request_id: String,
    amount_minor: Option<i64>,
    asset_id: Option<String>,
) -> Result<MappedIntent, ProviderAdapterError> {
    let mut flags = Vec::new();

    if intent.bypass_e2 {
        flags.push("bypass_e2_claimed".into());
        return Err(ProviderAdapterError::UnsafeProposal(
            "model claimed bypass_e2 — rejected; models cannot skip E2".into(),
        ));
    }
    if intent.access_treasury {
        flags.push("access_treasury_claimed".into());
        return Err(ProviderAdapterError::UnsafeProposal(
            "model claimed treasury access — rejected".into(),
        ));
    }
    if intent.access_protocol {
        flags.push("access_protocol_claimed".into());
        return Err(ProviderAdapterError::UnsafeProposal(
            "model claimed protocol access — rejected".into(),
        ));
    }
    if intent.claimed_decision.is_some() {
        flags.push("claimed_decision".into());
        return Err(ProviderAdapterError::UnsafeProposal(
            "model claimed ALLOW/DENY decision — rejected; gateway decides".into(),
        ));
    }
    if let Some(ref claimed) = intent.claimed_agent_id {
        if claimed != trusted_agent_id {
            flags.push("claimed_agent_id_mismatch".into());
        }
        flags.push("claimed_agent_id_ignored".into());
        // Do not adopt claimed id — continue with trusted id, but flag.
    }
    if intent.claimed_session_id.is_some() {
        flags.push("claimed_session_id_ignored".into());
    }
    if intent.claimed_capability.is_some() {
        flags.push("claimed_capability_rejected".into());
        return Err(ProviderAdapterError::UnsafeProposal(
            "model attempted to create/claim capability authority — rejected".into(),
        ));
    }

    let tool_id = intent.tool_id.trim();
    if tool_id.is_empty() {
        return Err(ProviderAdapterError::UnsafeProposal("empty tool_id".into()));
    }
    let tool_lower = tool_id.to_ascii_lowercase();
    if FORBIDDEN_TOOLS.iter().any(|t| *t == tool_lower.as_str())
        || FORBIDDEN_PREFIXES
            .iter()
            .any(|p| tool_lower.starts_with(p))
    {
        return Err(ProviderAdapterError::UnsafeProposal(format!(
            "forbidden or unrestricted tool_id: {tool_id}"
        )));
    }
    if !profile.allowlist().iter().any(|t| *t == tool_id) {
        return Err(ProviderAdapterError::UnsafeProposal(format!(
            "tool_id not on mock allowlist: {tool_id}"
        )));
    }

    // Strip any authority-shaped keys from parameters.
    let parameters = strip_authority_params(&intent.parameters, &mut flags);

    Ok(MappedIntent {
        http: ToolEvaluateHttpRequest {
            agent_id: trusted_agent_id.to_string(),
            session_id: trusted_session_id.to_string(),
            tool_id: tool_id.to_string(),
            parameters,
            request_id: Some(request_id),
            amount_minor,
            asset_id,
        },
        injection_flags: flags,
    })
}

fn strip_authority_params(value: &serde_json::Value, flags: &mut Vec<String>) -> serde_json::Value {
    let Some(obj) = value.as_object() else {
        return value.clone();
    };
    let mut clean = serde_json::Map::new();
    for (k, v) in obj {
        let kl = k.to_ascii_lowercase();
        if matches!(
            kl.as_str(),
            "agent_id"
                | "session_id"
                | "organisation_id"
                | "capability"
                | "bypass_e2"
                | "api_key"
                | "secret"
                | "treasury"
                | "proto0"
                | "apply"
                | "decision"
        ) {
            flags.push(format!("stripped_param:{k}"));
            continue;
        }
        clean.insert(k.clone(), v.clone());
    }
    if clean.is_empty() && !obj.is_empty() {
        json!({})
    } else {
        serde_json::Value::Object(clean)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_bypass_e2() {
        let intent = ProposedIntent {
            tool_id: "research.search".into(),
            parameters: json!({}),
            claimed_agent_id: None,
            claimed_session_id: None,
            claimed_capability: None,
            bypass_e2: true,
            claimed_decision: None,
            access_treasury: false,
            access_protocol: false,
        };
        assert!(map_proposed_intent(
            &intent,
            "a1",
            "s1",
            MockAllowlistProfile::Research,
            "r1".into(),
            None,
            None
        )
        .is_err());
    }

    #[test]
    fn rejects_treasury_tool() {
        let intent = ProposedIntent {
            tool_id: "treasury.fund".into(),
            parameters: json!({}),
            claimed_agent_id: None,
            claimed_session_id: None,
            claimed_capability: None,
            bypass_e2: false,
            claimed_decision: None,
            access_treasury: false,
            access_protocol: false,
        };
        assert!(map_proposed_intent(
            &intent,
            "a1",
            "s1",
            MockAllowlistProfile::Hostile,
            "r1".into(),
            None,
            None
        )
        .is_err());
    }

    #[test]
    fn ignores_claimed_agent_uses_trusted() {
        let intent = ProposedIntent {
            tool_id: "research.search".into(),
            parameters: json!({"agent_id": "evil", "q": "ok"}),
            claimed_agent_id: Some("evil-agent".into()),
            claimed_session_id: Some("evil-sess".into()),
            claimed_capability: None,
            bypass_e2: false,
            claimed_decision: None,
            access_treasury: false,
            access_protocol: false,
        };
        let m = map_proposed_intent(
            &intent,
            "trusted-agent",
            "trusted-sess",
            MockAllowlistProfile::Research,
            "r1".into(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(m.http.agent_id, "trusted-agent");
        assert_eq!(m.http.session_id, "trusted-sess");
        assert!(m.injection_flags.iter().any(|f| f.contains("claimed_agent")));
        assert_eq!(m.http.parameters["q"], "ok");
        assert!(m.http.parameters.get("agent_id").is_none());
    }
}
