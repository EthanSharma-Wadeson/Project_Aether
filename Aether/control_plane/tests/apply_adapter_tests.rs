//! Apply adapter mapping, error mapping, and simulation parity tests.

use aether_control_plane::apply::errors::{map_core_error, map_reject_reason, ApplyErrorCode};
use aether_control_plane::apply::policy_mapping::{
    map_freeze_policy, map_grant_policy, map_revoke_policy, predict_protocol_operation,
    validate_and_map, EnterpriseIssuerRef,
};
use aether_control_plane::execution::simulate::simulate_policy_apply;
use aether_control_plane::execution::types::ProtocolOperationKind;
use aether_control_plane::models::policies::{PolicyStatus, PolicyTemplate};
use aether_control_plane::protocol::proto0_write::{
    execute, ApplyContext, Proto0Result, Proto0WriteRequest,
};
use aether_control_plane::protocol::state::ProtocolState;
use aether_core::error::Error as CoreError;
use aether_core::types::RejectReason;
use chrono::Utc;
use serde_json::json;

fn sample_policy(
    policy_type: &str,
    data: serde_json::Value,
    target: Option<&str>,
) -> PolicyTemplate {
    PolicyTemplate {
        id: "pol-test".into(),
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

fn issuer_for(state: &ProtocolState) -> EnterpriseIssuerRef {
    EnterpriseIssuerRef {
        agent_id: state.index.agent_ids[0].clone(),
    }
}

#[test]
fn freeze_identity_intent_mapping() {
    let p = sample_policy("identity_freeze", json!({}), Some("agent-x"));
    assert_eq!(
        predict_protocol_operation(&p),
        ProtocolOperationKind::FreezeIdentity
    );
}

#[test]
fn adapter_error_mapping_reject_reason() {
    assert_eq!(
        map_reject_reason(&RejectReason::SpendExceeded),
        ApplyErrorCode::ApplyConstraintViolation
    );
    assert_eq!(
        map_core_error(&CoreError::CapabilityDenied),
        ApplyErrorCode::ApplyCapabilityDenied
    );
}

#[test]
fn simulation_and_adapter_parity_grant() {
    let state = ProtocolState::bootstrap().unwrap();
    let agent_id = state.index.agent_ids[0].clone();
    let issuer = issuer_for(&state);
    let policy = sample_policy(
        "capability_grant",
        json!({"actions": ["transfer"]}),
        Some(&agent_id),
    );

    let sim = simulate_policy_apply(&state, &policy);
    let mapped = validate_and_map(&policy, &state, &issuer);
    assert_eq!(sim.executable, mapped.is_ok(), "{sim:?} vs {mapped:?}");
}

#[test]
fn simulation_and_adapter_parity_revoke_not_found() {
    let state = ProtocolState::bootstrap().unwrap();
    let issuer = issuer_for(&state);
    let cap_id = format!("{:064x}", 0u128); // 64 hex chars → 32 bytes
    let policy = sample_policy("capability_revoke", json!({"capability_id": cap_id}), None);

    let sim = simulate_policy_apply(&state, &policy);
    let mapped = validate_and_map(&policy, &state, &issuer);
    assert!(!sim.executable);
    assert!(mapped.is_err());
    assert_eq!(
        mapped.unwrap_err().code,
        ApplyErrorCode::ApplyCapabilityNotFound
    );
}

#[test]
fn proto0_write_execute_disabled_after_validation() {
    let mut state = ProtocolState::bootstrap_owned().unwrap();
    let agent_id = state.index.agent_ids[0].clone();
    let policy = sample_policy("identity_freeze", json!({}), Some(&agent_id));
    let request = Proto0WriteRequest {
        ctx: ApplyContext {
            operation_id: "op-1".into(),
            request_id: "req-1".into(),
            execution_hash: "deadbeef".into(),
            operator_id: "operator".into(),
            capability_intent: ProtocolOperationKind::FreezeIdentity,
        },
        policy,
        enterprise_issuer: issuer_for(&state),
        grant_key: None,
        revoked_at: 0,
    };
    let outcome = execute(&mut state, request);
    assert_eq!(outcome.result, Proto0Result::Rejected);
    assert_eq!(
        outcome.apply_error_code.as_deref(),
        Some(ApplyErrorCode::ApplyDisabled.as_str())
    );
}

#[test]
fn map_grant_policy_builds_capability() {
    let state = ProtocolState::bootstrap().unwrap();
    let agent_id = state.index.agent_ids[0].clone();
    let issuer = issuer_for(&state);
    let policy = sample_policy(
        "capability_constraints",
        json!({"actions": ["transfer"], "max_spend": 100}),
        Some(&agent_id),
    );
    let grant = map_grant_policy(&policy, &state, &issuer).unwrap();
    assert_eq!(grant.capability.actions, vec!["transfer".to_string()]);
}

#[test]
fn map_freeze_rejects_missing_target() {
    let state = ProtocolState::bootstrap().unwrap();
    let policy = sample_policy("identity_freeze", json!({}), None);
    let err = map_freeze_policy(&policy, &state).unwrap_err();
    assert_eq!(err.code, ApplyErrorCode::ApplyInvalidPolicy);
}

#[test]
fn map_revoke_requires_valid_hex() {
    let state = ProtocolState::bootstrap().unwrap();
    let policy = sample_policy(
        "capability_revoke",
        json!({"capability_id": "not-hex"}),
        None,
    );
    let err = map_revoke_policy(&policy, &state).unwrap_err();
    assert_eq!(err.code, ApplyErrorCode::ApplyInvalidPolicy);
}
