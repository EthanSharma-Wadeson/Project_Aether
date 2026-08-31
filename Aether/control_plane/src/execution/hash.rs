//! Content-addressed execution binding (`execution_hash`).
//!
//! Replaces count-based fingerprints as the Apply binding primitive.
//! Count-based `protocol_observation_fingerprint` remains only as a dry-run
//! non-mutation sanity check — it must never authorise Apply.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::apply::canonical_json::canonicalize_json;
use crate::apply::PolicyMappingError;
use crate::execution::types::ProtocolOperationKind;
use crate::models::policies::PolicyTemplate;

const HASH_SCHEMA: &str = "aether.cp.execution_hash.v1";

/// Inputs that uniquely identify the intended mutation for Apply binding.
#[derive(Debug, Clone)]
pub struct ExecutionBindingInput<'a> {
    pub policy: &'a PolicyTemplate,
    pub capability_intent: ProtocolOperationKind,
    pub execution_parameters: Value,
}

/// Derive stable execution parameters from policy contents (no randomness).
pub fn execution_parameters_from_policy(
    policy: &PolicyTemplate,
) -> Result<Value, PolicyMappingError> {
    let policy_data = canonicalize_json(&policy.policy_data)?;
    let mut map = Map::new();
    map.insert(
        "policy_type".into(),
        Value::String(policy.policy_type.clone()),
    );
    map.insert("name".into(), Value::String(policy.name.clone()));
    map.insert(
        "description".into(),
        Value::String(policy.description.clone()),
    );
    map.insert(
        "status".into(),
        Value::String(policy.status.as_str().to_string()),
    );
    map.insert("policy_data".into(), policy_data);
    Ok(Value::Object(map))
}

/// SHA-256 hex digest uniquely identifying the intended mutation.
///
/// Binding covers: policy contents (via `policy.hash` + canonical `policy_data`),
/// policy version, target agent, capability intent, and execution parameters.
pub fn compute_execution_hash(input: &ExecutionBindingInput<'_>) -> String {
    try_compute_execution_hash(input).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "execution_hash canonicalization failed");
        String::new()
    })
}

/// Fallible variant used by conformance tests and strict callers.
pub fn try_compute_execution_hash(
    input: &ExecutionBindingInput<'_>,
) -> Result<String, PolicyMappingError> {
    let execution_parameters = if input.execution_parameters.is_object() {
        input.execution_parameters.clone()
    } else {
        execution_parameters_from_policy(input.policy)?
    };

    let mut root = Map::new();
    root.insert("schema".into(), Value::String(HASH_SCHEMA.into()));
    root.insert("policy_id".into(), Value::String(input.policy.id.clone()));
    root.insert(
        "policy_version".into(),
        Value::Number(input.policy.version.into()),
    );
    root.insert(
        "policy_content_hash".into(),
        Value::String(input.policy.hash.clone()),
    );
    root.insert(
        "policy_type".into(),
        Value::String(input.policy.policy_type.clone()),
    );
    root.insert("policy_data".into(), input.policy.policy_data.clone());
    root.insert(
        "target_agent".into(),
        input
            .policy
            .target_agent_id
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .unwrap_or(Value::Null),
    );
    root.insert(
        "capability_intent".into(),
        Value::String(input.capability_intent.as_str().to_string()),
    );
    root.insert("execution_parameters".into(), execution_parameters);

    digest_execution_hash_document(&Value::Object(root))
}

/// Hash a pre-built execution hash document (full recursive canonicalization per §6.3).
pub fn digest_execution_hash_document(doc: &Value) -> Result<String, PolicyMappingError> {
    let canonical = canonicalize_json(doc)?;
    let bytes = serde_json::to_vec(&canonical).map_err(|e| {
        PolicyMappingError::new(
            crate::apply::errors::ApplyErrorCode::ApplyInvalidPolicy,
            format!("execution hash encode failed: {e}"),
        )
    })?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

/// Convenience: hash a policy for a predicted protocol operation.
pub fn execution_hash_for_policy(
    policy: &PolicyTemplate,
    capability_intent: ProtocolOperationKind,
) -> String {
    let execution_parameters = execution_parameters_from_policy(policy).unwrap_or(Value::Null);
    compute_execution_hash(&ExecutionBindingInput {
        policy,
        capability_intent,
        execution_parameters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::policies::PolicyStatus;
    use chrono::Utc;
    use serde_json::json;

    fn sample_policy(version: i64, data: Value, target: Option<&str>) -> PolicyTemplate {
        PolicyTemplate {
            id: "pol-1".into(),
            name: "grant-demo".into(),
            description: "d".into(),
            target_agent_id: target.map(str::to_string),
            policy_type: "capability_grant".into(),
            policy_data: data,
            status: PolicyStatus::Approved,
            created_by: "op".into(),
            created_by_username: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            submitted_by: None,
            submitted_at: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            rejection_reason: None,
            version,
            hash: format!("content-hash-v{version}"),
        }
    }

    #[test]
    fn execution_hash_is_stable() {
        let p = sample_policy(3, json!({"scope":"read"}), Some("agent-a"));
        let a = execution_hash_for_policy(&p, ProtocolOperationKind::CapabilityGrant);
        let b = execution_hash_for_policy(&p, ProtocolOperationKind::CapabilityGrant);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn execution_hash_changes_with_policy_data() {
        let p1 = sample_policy(1, json!({"scope":"read"}), Some("agent-a"));
        let p2 = sample_policy(1, json!({"scope":"write"}), Some("agent-a"));
        assert_ne!(
            execution_hash_for_policy(&p1, ProtocolOperationKind::CapabilityGrant),
            execution_hash_for_policy(&p2, ProtocolOperationKind::CapabilityGrant)
        );
    }

    #[test]
    fn execution_hash_changes_with_version() {
        let p1 = sample_policy(1, json!({}), Some("agent-a"));
        let p2 = sample_policy(2, json!({}), Some("agent-a"));
        assert_ne!(
            execution_hash_for_policy(&p1, ProtocolOperationKind::CapabilityGrant),
            execution_hash_for_policy(&p2, ProtocolOperationKind::CapabilityGrant)
        );
    }

    #[test]
    fn execution_hash_changes_with_target_and_intent() {
        let p = sample_policy(1, json!({}), Some("agent-a"));
        let h1 = execution_hash_for_policy(&p, ProtocolOperationKind::CapabilityGrant);
        let h2 = execution_hash_for_policy(&p, ProtocolOperationKind::CapabilityRevoke);
        assert_ne!(h1, h2);
        let p_other = sample_policy(1, json!({}), Some("agent-b"));
        assert_ne!(
            h1,
            execution_hash_for_policy(&p_other, ProtocolOperationKind::CapabilityGrant)
        );
    }
}
