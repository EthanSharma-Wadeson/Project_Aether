//! Secret reference abstraction — API keys are handles, never identity.

use serde::{Deserialize, Serialize};

use super::errors::ProviderAdapterError;

/// Opaque reference to a secret in an org secret store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef {
    /// e.g. `secret://org/{org_id}/providers/anthropic/api_key`
    pub handle: String,
    pub organisation_id: String,
    pub provider_label: String,
}

/// Short-lived lab API key material — Debug never prints the secret.
pub struct LabApiKey {
    value: String,
}

impl LabApiKey {
    pub fn expose(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Debug for LabApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LabApiKey(REDACTED)")
    }
}

impl Drop for LabApiKey {
    fn drop(&mut self) {
        // Best-effort clear (String may leave allocator copies; still avoid Debug leaks).
        let len = self.value.len();
        self.value.clear();
        self.value.shrink_to_fit();
        let _ = len;
    }
}

impl SecretRef {
    pub fn parse(handle: &str, organisation_id: &str) -> Result<Self, ProviderAdapterError> {
        let handle = handle.trim();
        if handle.is_empty() {
            return Err(ProviderAdapterError::InvalidSecretRef("empty handle".into()));
        }
        if looks_like_raw_api_key(handle) {
            return Err(ProviderAdapterError::InvalidSecretRef(
                "raw API key material rejected — use secret:// handle only".into(),
            ));
        }
        if !handle.starts_with("secret://") {
            return Err(ProviderAdapterError::InvalidSecretRef(
                "handle must start with secret://".into(),
            ));
        }
        let provider_label = handle
            .rsplit('/')
            .nth(1)
            .unwrap_or("unknown")
            .to_string();
        Ok(Self {
            handle: handle.to_string(),
            organisation_id: organisation_id.to_string(),
            provider_label,
        })
    }

    /// Lab never materialises the secret in logs. Returns redacted marker only.
    pub fn redacted_display(&self) -> String {
        format!(
            "SecretRef(handle={}, material=REDACTED)",
            redact_handle(&self.handle)
        )
    }

    /// Resolve lab API key from environment — never from request body raw keys.
    ///
    /// Env (first match):
    /// - `AETHER_LAB_ANTHROPIC_API_KEY` when handle contains `anthropic` / `claude`
    /// - `AETHER_LAB_PROVIDER_API_KEY` fallback
    pub fn resolve_lab_api_key(&self) -> Result<LabApiKey, ProviderAdapterError> {
        let label = self.provider_label.to_ascii_lowercase();
        let handle_l = self.handle.to_ascii_lowercase();
        let mut candidates: Vec<&str> = Vec::new();
        if label.contains("anthropic")
            || label.contains("claude")
            || handle_l.contains("anthropic")
            || handle_l.contains("claude")
        {
            candidates.push("AETHER_LAB_ANTHROPIC_API_KEY");
        }
        candidates.push("AETHER_LAB_PROVIDER_API_KEY");

        for env_name in candidates {
            if let Ok(v) = std::env::var(env_name) {
                let v = v.trim().to_string();
                if !v.is_empty() {
                    return Ok(LabApiKey { value: v });
                }
            }
        }
        Err(ProviderAdapterError::InvalidSecretRef(
            "lab API key not found in env (set AETHER_LAB_ANTHROPIC_API_KEY or AETHER_LAB_PROVIDER_API_KEY)"
                .into(),
        ))
    }
}

fn looks_like_raw_api_key(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("sk-") || lower.starts_with("sk-ant-") || lower.starts_with("AIza") {
        return true;
    }
    !s.starts_with("secret://") && s.len() >= 32 && !s.contains('/')
}

fn redact_handle(handle: &str) -> String {
    if handle.len() <= 16 {
        return "secret://***".into();
    }
    format!(
        "{}…{}",
        &handle[..12],
        &handle[handle.len().saturating_sub(4)..]
    )
}

/// Redact likely secrets from text before audit / logs.
pub fn redact_for_audit(input: &str) -> String {
    let mut out = input.to_string();
    for pattern in ["sk-ant-", "sk-", "AIza", "Bearer "] {
        if let Some(idx) = out.find(pattern) {
            let end = (idx + pattern.len() + 8).min(out.len());
            out.replace_range(idx..end, &format!("{pattern}***REDACTED***"));
        }
    }
    if out.contains("api_key=") {
        out = out.replace("api_key=", "api_key=REDACTED&");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_raw_keys() {
        assert!(SecretRef::parse("sk-ant-secretvalue1234567890123456", "org").is_err());
        assert!(SecretRef::parse("secret://org/x/providers/anthropic/api_key", "org").is_ok());
    }

    #[test]
    fn redacts_keys_in_text() {
        let r = redact_for_audit("calling with sk-ant-ABCDEFGH1234 rest");
        assert!(r.contains("REDACTED"));
        assert!(!r.contains("ABCDEFGH1234"));
    }
}
