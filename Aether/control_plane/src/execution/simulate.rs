//! Read-only protocol simulation — never mutates protocol state.

pub use crate::apply::policy_mapping::predict_protocol_operation;

use crate::apply::policy_mapping::{validate_and_map, EnterpriseIssuerRef};
use crate::execution::types::SimulationOutcome;
use crate::models::policies::{PolicyStatus, PolicyTemplate};
use crate::protocol::state::ProtocolState;

/// Simulate applying an approved policy (dispatches grant/revoke/freeze checks).
///
/// **Does not** call PROTO-0 mutation APIs or modify `ProtocolState`.
pub fn simulate_policy_apply(state: &ProtocolState, policy: &PolicyTemplate) -> SimulationOutcome {
    if policy.status != PolicyStatus::Approved {
        return SimulationOutcome {
            executable: false,
            reason: Some(format!(
                "policy must be approved (have {})",
                policy.status.as_str()
            )),
        };
    }

    let issuer = default_enterprise_issuer(state);
    match validate_and_map(policy, state, &issuer) {
        Ok(_) => SimulationOutcome {
            executable: true,
            reason: None,
        },
        Err(e) => SimulationOutcome {
            executable: false,
            reason: Some(e.message),
        },
    }
}

fn default_enterprise_issuer(state: &ProtocolState) -> EnterpriseIssuerRef {
    EnterpriseIssuerRef {
        agent_id: state
            .index
            .agent_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "enterprise".into()),
    }
}

/// Would a capability grant be accepted? Read-only checks only.
pub fn simulate_capability_grant(
    state: &ProtocolState,
    target_agent_id: Option<&str>,
    policy_data: &serde_json::Value,
) -> SimulationOutcome {
    use chrono::Utc;

    let policy = PolicyTemplate {
        id: "sim-grant".into(),
        name: "sim".into(),
        description: String::new(),
        target_agent_id: target_agent_id.map(str::to_string),
        policy_type: "capability_grant".into(),
        policy_data: policy_data.clone(),
        status: PolicyStatus::Approved,
        created_by: "sim".into(),
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
        hash: "sim".into(),
    };
    simulate_policy_apply(state, &policy)
}

/// Would a capability revoke be accepted? Read-only checks only.
pub fn simulate_capability_revoke(
    state: &ProtocolState,
    capability_id_hex: Option<&str>,
    target_agent_id: Option<&str>,
) -> SimulationOutcome {
    use chrono::Utc;
    use serde_json::json;

    let mut policy_data = serde_json::Map::new();
    if let Some(cap) = capability_id_hex {
        policy_data.insert("capability_id".into(), json!(cap));
    }
    let policy = PolicyTemplate {
        id: "sim-revoke".into(),
        name: "sim".into(),
        description: String::new(),
        target_agent_id: target_agent_id.map(str::to_string),
        policy_type: "capability_revoke".into(),
        policy_data: serde_json::Value::Object(policy_data),
        status: PolicyStatus::Approved,
        created_by: "sim".into(),
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
        hash: "sim".into(),
    };
    simulate_policy_apply(state, &policy)
}

/// Count-based snapshot used **only** to prove dry-run did not mutate protocol indexes.
///
/// **Not** an Apply binding. Future Apply must use [`crate::execution::hash::compute_execution_hash`].
pub fn protocol_observation_fingerprint(state: &ProtocolState) -> String {
    format!(
        "agents={};caps={};escrows={};settlements={}",
        state.index.agent_ids.len(),
        state.index.capability_ids.len(),
        state.index.escrow_ids.len(),
        state.index.settlement_binding_ids.len()
    )
}
