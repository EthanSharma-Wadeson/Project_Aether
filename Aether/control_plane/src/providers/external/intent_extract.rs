//! Extract untrusted ProposedIntent(s) from model text. Fail closed on garbage.

use serde::Deserialize;
use serde_json::Value;

use crate::providers::models::ProposedIntent;
use crate::providers::secrets::redact_for_audit;

#[derive(Debug, Deserialize)]
struct IntentEnvelope {
    #[serde(default)]
    proposed_intents: Vec<ProposedIntent>,
    /// Alternate single-intent shape some models emit.
    #[serde(default)]
    tool_id: Option<String>,
    #[serde(default)]
    parameters: Option<Value>,
    #[serde(default)]
    bypass_e2: Option<bool>,
    #[serde(default)]
    claimed_capability: Option<String>,
    #[serde(default)]
    access_treasury: Option<bool>,
    #[serde(default)]
    access_protocol: Option<bool>,
    #[serde(default)]
    claimed_decision: Option<String>,
    #[serde(default)]
    claimed_agent_id: Option<String>,
}

/// Parse model completion into proposed intents. Never treats prose as authority.
pub fn extract_proposed_intents(model_text: &str) -> Vec<ProposedIntent> {
    let redacted_preview = redact_for_audit(model_text);
    let _ = redacted_preview; // ensure we never log raw in callers carelessly

    if let Some(intents) = try_parse_json_blob(model_text) {
        return intents;
    }
    // Try fenced ```json blocks
    if let Some(start) = model_text.find("```") {
        let after = &model_text[start + 3..];
        let after = after.strip_prefix("json").unwrap_or(after);
        if let Some(end) = after.find("```") {
            if let Some(intents) = try_parse_json_blob(after[..end].trim()) {
                return intents;
            }
        }
    }
    // Scan for first {...} object
    if let Some(start) = model_text.find('{') {
        if let Some(end) = model_text.rfind('}') {
            if end > start {
                if let Some(intents) = try_parse_json_blob(&model_text[start..=end]) {
                    return intents;
                }
            }
        }
    }
    Vec::new()
}

fn try_parse_json_blob(s: &str) -> Option<Vec<ProposedIntent>> {
    let v: IntentEnvelope = serde_json::from_str(s.trim()).ok()?;
    if !v.proposed_intents.is_empty() {
        return Some(v.proposed_intents);
    }
    if let Some(tool_id) = v.tool_id {
        return Some(vec![ProposedIntent {
            tool_id,
            parameters: v.parameters.unwrap_or(Value::Object(Default::default())),
            claimed_agent_id: v.claimed_agent_id,
            claimed_session_id: None,
            claimed_capability: v.claimed_capability,
            bypass_e2: v.bypass_e2.unwrap_or(false),
            claimed_decision: v.claimed_decision,
            access_treasury: v.access_treasury.unwrap_or(false),
            access_protocol: v.access_protocol.unwrap_or(false),
        }]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_envelope() {
        let t = r#"{"proposed_intents":[{"tool_id":"research.search","parameters":{"q":"a"}}]}"#;
        let i = extract_proposed_intents(t);
        assert_eq!(i.len(), 1);
        assert_eq!(i[0].tool_id, "research.search");
    }

    #[test]
    fn extracts_bypass_flags() {
        let t = r#"{"tool_id":"research.search","bypass_e2":true,"claimed_decision":"ALLOW"}"#;
        let i = extract_proposed_intents(t);
        assert!(i[0].bypass_e2);
    }

    #[test]
    fn empty_on_prose() {
        assert!(extract_proposed_intents("Sure, I will help you.").is_empty());
    }
}
