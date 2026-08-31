//! Policy template → PROTO-0 operation mapping (shared by simulation and adapter).
//!
//! See APPLY_PROTO_ADAPTER_SPEC.md §2–§5.

use std::sync::LazyLock;

use aether_core::capability::model::CapabilityV0;
use aether_core::permission::root::Constraints;
use aether_core::types::{ActionSelector, RateLimit, SubjectRef};
use regex::Regex;
use serde_json::Value;

use crate::apply::canonical_json::canonicalize_json;
use crate::apply::errors::ApplyErrorCode;
use crate::execution::types::ProtocolOperationKind;
use crate::models::policies::PolicyTemplate;
use crate::protocol::proto0;

use super::PolicyMappingError;

const DEFAULT_ASSET: &str = "AETHER_TEST";
const DEFAULT_RATE_MAX_OPS: u64 = 100;
const DEFAULT_RATE_WINDOW_SECS: u64 = 60;

static ACTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9._-]+$").expect("action pattern"));

/// Enterprise issuer reference for grant signing (Phase 1: agent id only).
#[derive(Debug, Clone)]
pub struct EnterpriseIssuerRef {
    pub agent_id: String,
}

/// Mapped grant payload (unsigned `CapabilityV0`).
#[derive(Debug, Clone)]
pub struct MappedGrant {
    pub capability: CapabilityV0,
}

/// Mapped revoke payload.
#[derive(Debug, Clone)]
pub struct MappedRevoke {
    pub capability_id: [u8; 32],
}

/// Mapped freeze payload.
#[derive(Debug, Clone)]
pub struct MappedFreeze {
    pub agent_id: String,
}

/// Result of policy → PROTO-0 mapping.
#[derive(Debug, Clone)]
pub enum MappedOperation {
    Grant(Box<MappedGrant>),
    Revoke(MappedRevoke),
    Freeze(MappedFreeze),
}

/// Resolve `capability_intent` from policy template (first match wins).
pub fn predict_protocol_operation(policy: &PolicyTemplate) -> ProtocolOperationKind {
    let t = policy.policy_type.to_lowercase();

    if t.contains("revoke") {
        return ProtocolOperationKind::CapabilityRevoke;
    }
    if policy.policy_data.get("action").and_then(|v| v.as_str()) == Some("revoke")
        && policy.policy_data.get("capability_id").is_some()
    {
        return ProtocolOperationKind::CapabilityRevoke;
    }
    if t.contains("freeze") {
        return ProtocolOperationKind::FreezeIdentity;
    }
    if t.contains("grant") || t.contains("capability") {
        return ProtocolOperationKind::CapabilityGrant;
    }
    if policy.policy_type == "capability_constraints" {
        return ProtocolOperationKind::CapabilityGrant;
    }
    if policy.policy_type == "policy_apply" {
        return ProtocolOperationKind::CapabilityGrant;
    }
    ProtocolOperationKind::PolicyApply
}

/// Validate policy and produce mapped PROTO-0 operation (no mutation).
pub fn validate_and_map(
    policy: &PolicyTemplate,
    state: &crate::protocol::state::ProtocolState,
    issuer: &EnterpriseIssuerRef,
) -> Result<MappedOperation, PolicyMappingError> {
    let intent = predict_protocol_operation(policy);
    match intent {
        ProtocolOperationKind::CapabilityRevoke => {
            Ok(MappedOperation::Revoke(map_revoke_policy(policy, state)?))
        }
        ProtocolOperationKind::FreezeIdentity => {
            Ok(MappedOperation::Freeze(map_freeze_policy(policy, state)?))
        }
        ProtocolOperationKind::CapabilityGrant | ProtocolOperationKind::PolicyApply => Ok(
            MappedOperation::Grant(Box::new(map_grant_policy(policy, state, issuer)?)),
        ),
        ProtocolOperationKind::Unknown => Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyUnsupportedOperation,
            "unsupported protocol operation",
        )),
    }
}

/// Build unsigned `CapabilityV0` from an approved grant policy.
pub fn map_grant_policy(
    policy: &PolicyTemplate,
    state: &crate::protocol::state::ProtocolState,
    issuer: &EnterpriseIssuerRef,
) -> Result<MappedGrant, PolicyMappingError> {
    let target = policy
        .target_agent_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                "target_agent_id required for grant",
            )
        })?;

    let agent = proto0::get_agent(state, target).map_err(|_| {
        PolicyMappingError::new(
            ApplyErrorCode::ApplyAgentNotFound,
            format!("target agent not found: {target}"),
        )
    })?;

    match agent.status.as_str() {
        "frozen" => {
            return Err(PolicyMappingError::new(
                ApplyErrorCode::ApplyIdentityFrozen,
                "Identity frozen — grant would be rejected",
            ));
        }
        "revoked" => {
            return Err(PolicyMappingError::new(
                ApplyErrorCode::ApplyIdentityRevoked,
                "Identity revoked — grant would be rejected",
            ));
        }
        "active" => {}
        other => {
            return Err(PolicyMappingError::new(
                ApplyErrorCode::ApplyAgentNotActive,
                format!("agent status '{other}' would reject grant"),
            ));
        }
    }

    let actions = parse_actions(&policy.policy_data)?;
    let issuer_agent_id = policy
        .policy_data
        .get("issuer_agent_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(issuer.agent_id.as_str());

    if proto0::get_agent(state, issuer_agent_id).is_err() {
        return Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyAgentNotFound,
            format!("issuer agent not found: {issuer_agent_id}"),
        ));
    }

    let delegation_depth = policy
        .policy_data
        .get("delegation_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let parent_capability_id = if delegation_depth > 0 {
        let parent_hex = policy
            .policy_data
            .get("parent_capability_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                PolicyMappingError::new(
                    ApplyErrorCode::ApplyInvalidPolicy,
                    "parent_capability_id required when delegation_depth > 0",
                )
            })?;
        let id = decode_capability_id_hex(parent_hex)?;
        let cap = proto0::get_capability(state, &id).map_err(|_| {
            PolicyMappingError::new(
                ApplyErrorCode::ApplyCapabilityNotFound,
                format!("parent capability not found: {parent_hex}"),
            )
        })?;
        if cap.revoked {
            return Err(PolicyMappingError::new(
                ApplyErrorCode::ApplyCapabilityRevoked,
                "parent capability revoked",
            ));
        }
        Some(id)
    } else {
        None
    };

    if let Some(max_spend) = policy.policy_data.get("max_spend") {
        if !max_spend.is_null() {
            let spend = max_spend.as_u64().ok_or_else(|| {
                PolicyMappingError::new(
                    ApplyErrorCode::ApplyInvalidPolicy,
                    "max_spend must be a positive integer",
                )
            })?;
            if spend == 0 {
                return Err(PolicyMappingError::new(
                    ApplyErrorCode::ApplyInvalidPolicy,
                    "max_spend must be > 0 when set",
                ));
            }
        }
    }

    let constraints = map_constraints(&policy.policy_data)?;

    let capability = CapabilityV0 {
        protocol_version: 1,
        schema_version: 1,
        issuer: issuer_agent_id.to_string(),
        subject: SubjectRef::AgentId(target.to_string()),
        actions,
        constraints,
        delegation_depth,
        parent_capability_id,
    };

    // Prove adapter can read capability index (read-only).
    let _ = proto0::capabilities_for_agent(state, target);

    Ok(MappedGrant { capability })
}

/// Map revoke policy to capability id.
pub fn map_revoke_policy(
    policy: &PolicyTemplate,
    state: &crate::protocol::state::ProtocolState,
) -> Result<MappedRevoke, PolicyMappingError> {
    let cap_hex = policy
        .policy_data
        .get("capability_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                "capability_id required for revoke",
            )
        })?;

    let capability_id = decode_capability_id_hex(cap_hex)?;

    match proto0::get_capability(state, &capability_id) {
        Ok(cap) if cap.revoked => Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyCapabilityRevoked,
            "Capability already revoked",
        )),
        Ok(_) => Ok(MappedRevoke { capability_id }),
        Err(_) => Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyCapabilityNotFound,
            format!("capability not found: {cap_hex}"),
        )),
    }
}

/// Map freeze policy to agent id.
pub fn map_freeze_policy(
    policy: &PolicyTemplate,
    state: &crate::protocol::state::ProtocolState,
) -> Result<MappedFreeze, PolicyMappingError> {
    let agent_id = policy
        .target_agent_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                "target_agent_id required for freeze",
            )
        })?
        .to_string();

    let agent = proto0::get_agent(state, &agent_id).map_err(|_| {
        PolicyMappingError::new(
            ApplyErrorCode::ApplyAgentNotFound,
            format!("agent not found: {agent_id}"),
        )
    })?;

    match agent.status.as_str() {
        "frozen" => Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyIdentityFrozen,
            "agent already frozen",
        )),
        "revoked" => Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyIdentityRevoked,
            "agent revoked — cannot freeze",
        )),
        _ => Ok(MappedFreeze { agent_id }),
    }
}

fn parse_actions(policy_data: &Value) -> Result<Vec<ActionSelector>, PolicyMappingError> {
    let arr = policy_data
        .get("actions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                "policy_data.actions must be a non-empty array",
            )
        })?;

    if arr.is_empty() {
        return Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyInvalidPolicy,
            "policy_data.actions must be a non-empty array",
        ));
    }

    let mut actions = Vec::with_capacity(arr.len());
    for item in arr {
        let action = item.as_str().ok_or_else(|| {
            PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                "each action must be a string",
            )
        })?;
        if action.is_empty() || action.len() > 128 || !ACTION_RE.is_match(action) {
            return Err(PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                format!("invalid action selector: {action}"),
            ));
        }
        actions.push(action.to_string());
    }
    Ok(actions)
}

fn map_constraints(policy_data: &Value) -> Result<Constraints, PolicyMappingError> {
    let max_spend = match policy_data.get("max_spend") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_u64().ok_or_else(|| {
            PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                "max_spend must be an integer",
            )
        })?),
    };

    let asset = match policy_data.get("asset") {
        None | Some(Value::Null) => Some(DEFAULT_ASSET.to_string()),
        Some(v) => {
            let s = v.as_str().ok_or_else(|| {
                PolicyMappingError::new(ApplyErrorCode::ApplyInvalidPolicy, "asset must be string")
            })?;
            if s.is_empty() || s.len() > 32 {
                return Err(PolicyMappingError::new(
                    ApplyErrorCode::ApplyInvalidPolicy,
                    "asset must be 1–32 characters",
                ));
            }
            Some(s.to_string())
        }
    };

    let counterparties = match policy_data.get("counterparties") {
        None | Some(Value::Null) => None,
        Some(v) => {
            let arr = v.as_array().ok_or_else(|| {
                PolicyMappingError::new(
                    ApplyErrorCode::ApplyInvalidPolicy,
                    "counterparties must be an array",
                )
            })?;
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                let id = item.as_str().ok_or_else(|| {
                    PolicyMappingError::new(
                        ApplyErrorCode::ApplyInvalidPolicy,
                        "counterparty must be a string agent id",
                    )
                })?;
                out.push(id.to_string());
            }
            Some(out)
        }
    };

    let rate_limit = match policy_data.get("rate_limit") {
        None | Some(Value::Null) => Some(RateLimit {
            max_ops: DEFAULT_RATE_MAX_OPS,
            window_seconds: DEFAULT_RATE_WINDOW_SECS,
        }),
        Some(v) => {
            let obj = v.as_object().ok_or_else(|| {
                PolicyMappingError::new(
                    ApplyErrorCode::ApplyInvalidPolicy,
                    "rate_limit must be an object",
                )
            })?;
            let max_ops = obj
                .get("max_ops")
                .and_then(|n| n.as_u64())
                .unwrap_or(DEFAULT_RATE_MAX_OPS);
            let window_seconds = obj
                .get("window_seconds")
                .and_then(|n| n.as_u64())
                .unwrap_or(DEFAULT_RATE_WINDOW_SECS);
            if max_ops == 0 || window_seconds == 0 {
                return Err(PolicyMappingError::new(
                    ApplyErrorCode::ApplyInvalidPolicy,
                    "rate_limit max_ops and window_seconds must be > 0",
                ));
            }
            Some(RateLimit {
                max_ops,
                window_seconds,
            })
        }
    };

    let valid_after = policy_data.get("valid_after").and_then(|v| v.as_u64());
    let valid_before = match policy_data.get("valid_before") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_u64().ok_or_else(|| {
            PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                "valid_before must be an integer",
            )
        })?),
    };

    if let (Some(after), Some(before)) = (valid_after, valid_before) {
        if before <= after {
            return Err(PolicyMappingError::new(
                ApplyErrorCode::ApplyInvalidPolicy,
                "valid_before must be > valid_after",
            ));
        }
    }

    // Reject floats anywhere in policy_data used for mapping.
    let _ = canonicalize_json(policy_data)?;

    Ok(Constraints {
        max_spend,
        asset,
        counterparties,
        rate_limit,
        valid_after,
        valid_before,
    })
}

fn decode_capability_id_hex(hex_str: &str) -> Result<[u8; 32], PolicyMappingError> {
    let bytes = hex::decode(hex_str).map_err(|_| {
        PolicyMappingError::new(
            ApplyErrorCode::ApplyInvalidPolicy,
            "capability_id is not valid hex",
        )
    })?;
    if bytes.len() != 32 {
        return Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyInvalidPolicy,
            "capability_id must be 32 bytes",
        ));
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::policies::PolicyStatus;
    use crate::protocol::state::ProtocolState;
    use chrono::Utc;
    use serde_json::json;

    fn sample_policy(policy_type: &str, data: Value, target: Option<&str>) -> PolicyTemplate {
        PolicyTemplate {
            id: "pol-1".into(),
            name: "demo".into(),
            description: "d".into(),
            target_agent_id: target.map(str::to_string),
            policy_type: policy_type.into(),
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
            version: 1,
            hash: "h".into(),
        }
    }

    #[test]
    fn predict_freeze_before_grant() {
        let p = sample_policy("identity_freeze", json!({}), Some("agent-a"));
        assert_eq!(
            predict_protocol_operation(&p),
            ProtocolOperationKind::FreezeIdentity
        );
    }

    #[test]
    fn predict_revoke_by_action() {
        let p = sample_policy(
            "custom",
            json!({"action": "revoke", "capability_id": "00".repeat(32)}),
            None,
        );
        assert_eq!(
            predict_protocol_operation(&p),
            ProtocolOperationKind::CapabilityRevoke
        );
    }

    #[test]
    fn map_grant_requires_actions() {
        let state = ProtocolState::bootstrap().unwrap();
        let agent_id = state.index.agent_ids[0].clone();
        let issuer = EnterpriseIssuerRef {
            agent_id: agent_id.clone(),
        };
        let p = sample_policy("capability_grant", json!({}), Some(&agent_id));
        let err = map_grant_policy(&p, &state, &issuer).unwrap_err();
        assert_eq!(err.code, ApplyErrorCode::ApplyInvalidPolicy);
    }
}
