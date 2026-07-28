//! PROTO-0 capability + delegation acceptance tests.

mod common;

use aether_core::capability::model::{CapabilityStore, CapabilityV0};
use aether_core::capability::grant::{grant_capability, CapabilityGrant};
use aether_core::permission::root::Constraints;
use aether_core::types::{RateLimit, RejectReason, SubjectRef};
use aether_core::verifier::authorise::authorise_action;
use common::*;

#[test]
fn p0_t040_capability_within_authority() {
    // P0-T040
    let (bundle, registry) = fresh_agent(vec!["payment.channel.open".into()], Some(100));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["payment.channel.open".into()], Some(50));
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("payment.channel.open", Some(40)),
        100,
    );
    assert!(decision.is_authorised());
}

#[test]
fn p0_t041_valid_expiry_window() {
    // P0-T041
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    assert!(authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("a", Some(1)),
        100,
    )
    .is_authorised());
}

#[test]
fn p0_t042_valid_issuer_chain() {
    // P0-T042
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let grant = CapabilityGrant::sign(&bundle.signing_key, &cap).unwrap();
    grant_capability(
        &grant,
        &registry,
        &mut store,
        &bundle.identity.operational_public_key,
    )
    .unwrap();
}

#[test]
fn p0_t043_direct_grant_depth_zero() {
    // P0-T043
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    assert_eq!(cap.delegation_depth, 0);
    assert!(cap.parent_capability_id.is_none());
    grant_and_store(&bundle, &registry, &mut store, &cap);
}

#[test]
fn p0_t050_forged_capability_rejected() {
    // P0-T050
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let mut grant = CapabilityGrant::sign(&bundle.signing_key, &cap).unwrap();
    grant.message.signature = vec![0u8; 64];
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("a", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::InvalidSignature
    );
}

#[test]
fn p0_t051_altered_constraints_rejected() {
    // P0-T051
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(100));
    let store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(50));
    let mut grant = CapabilityGrant::sign(&bundle.signing_key, &cap).unwrap();
    let mut mutated = cap.clone();
    mutated.constraints.max_spend = Some(200);
    grant.message.body = mutated.encode().unwrap();
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("a", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::InvalidSignature
    );
}

#[test]
fn p0_t052_invalid_action_rejected() {
    // P0-T052
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("b", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::ActionNotPermitted
    );
}

#[test]
fn p0_t053_invalid_issuer_rejected() {
    // P0-T053
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let (other, _) = fresh_agent(vec!["a".into()], Some(10));
    let store = CapabilityStore::new();
    let mut cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    cap.issuer = other.identity.derived_agent_id();
    let grant = CapabilityGrant::sign(&bundle.signing_key, &cap).unwrap();
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("a", Some(1)),
        100,
    );
    assert!(matches!(
        decision.expect_rejected().unwrap(),
        RejectReason::InvalidSignature | RejectReason::AgentIdMismatch
    ));
}

#[test]
fn p0_t054_expired_capability_rejected() {
    // P0-T054
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let mut cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    cap.constraints.valid_before = Some(50);
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("a", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::CapabilityExpired
    );
}

#[test]
fn p0_t055_not_yet_valid_rejected() {
    // P0-T055
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let mut cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    cap.constraints.valid_after = Some(500);
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("a", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::CapabilityNotYetValid
    );
}

#[test]
fn p0_t060_valid_narrowing_delegation() {
    // P0-T060 Parent spend 100 action A; child spend 50 action A
    let (bundle, registry) = fresh_agent(vec!["A".into()], Some(100));
    let mut store = CapabilityStore::new();
    let parent = direct_capability(&bundle, vec!["A".into()], Some(100));
    let parent_grant = grant_and_store(&bundle, &registry, &mut store, &parent);
    let parent_id = parent.capability_id().unwrap();

    let child = CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions: vec!["A".into()],
        constraints: Constraints {
            max_spend: Some(50),
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 5,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(5_000),
        },
        delegation_depth: 1,
        parent_capability_id: Some(parent_id),
    };
    let child_grant = grant_and_store(&bundle, &registry, &mut store, &child);
    assert!(authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&child_grant),
        &action("A", Some(40)),
        100,
    )
    .is_authorised());
    let _ = parent_grant;
}

#[test]
fn p0_t070_child_spend_escalation_rejected() {
    // P0-T070
    let (bundle, registry) = fresh_agent(vec!["A".into()], Some(100));
    let mut store = CapabilityStore::new();
    let parent = direct_capability(&bundle, vec!["A".into()], Some(100));
    grant_and_store(&bundle, &registry, &mut store, &parent);
    let parent_id = parent.capability_id().unwrap();

    let child = CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions: vec!["A".into()],
        constraints: Constraints {
            max_spend: Some(200),
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 5,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(5_000),
        },
        delegation_depth: 1,
        parent_capability_id: Some(parent_id),
    };
    let grant = CapabilityGrant::sign(&bundle.signing_key, &child).unwrap();
    let err = grant_capability(
        &grant,
        &registry,
        &mut store,
        &bundle.identity.operational_public_key,
    )
    .unwrap_err();
    assert!(matches!(err, aether_core::error::Error::Escalation(_)));
}

#[test]
fn p0_t071_child_extra_action_rejected() {
    // P0-T071
    let (bundle, registry) = fresh_agent(vec!["A".into(), "B".into()], Some(100));
    let mut store = CapabilityStore::new();
    let parent = direct_capability(&bundle, vec!["A".into()], Some(100));
    grant_and_store(&bundle, &registry, &mut store, &parent);
    let parent_id = parent.capability_id().unwrap();

    let child = CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions: vec!["A".into(), "B".into()],
        constraints: parent.constraints.clone(),
        delegation_depth: 1,
        parent_capability_id: Some(parent_id),
    };
    let grant = CapabilityGrant::sign(&bundle.signing_key, &child).unwrap();
    assert!(grant_capability(
        &grant,
        &registry,
        &mut store,
        &bundle.identity.operational_public_key,
    )
    .is_err());
}

#[test]
fn p0_t072_invalid_depth_step_rejected() {
    // P0-T072
    let (bundle, registry) = fresh_agent(vec!["A".into()], Some(100));
    let mut store = CapabilityStore::new();
    let parent = direct_capability(&bundle, vec!["A".into()], Some(100));
    grant_and_store(&bundle, &registry, &mut store, &parent);
    let parent_id = parent.capability_id().unwrap();

    let child = CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions: vec!["A".into()],
        constraints: parent.constraints.clone(),
        delegation_depth: 2, // should be 1
        parent_capability_id: Some(parent_id),
    };
    let grant = CapabilityGrant::sign(&bundle.signing_key, &child).unwrap();
    let err = grant_capability(
        &grant,
        &registry,
        &mut store,
        &bundle.identity.operational_public_key,
    )
    .unwrap_err();
    assert_eq!(err, aether_core::error::Error::InvalidDelegationDepth);
}

#[test]
fn p0_t073_excessive_depth_rejected() {
    // P0-T073 max_delegation_depth = 2, attempt depth 3
    let (bundle, registry) = fresh_agent(vec!["A".into()], Some(100));
    let mut store = CapabilityStore::new();
    let parent = direct_capability(&bundle, vec!["A".into()], Some(100));
    grant_and_store(&bundle, &registry, &mut store, &parent);
    let mut current_id = parent.capability_id().unwrap();
    for depth in 1..=2 {
        let child = CapabilityV0 {
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
            issuer: bundle.identity.derived_agent_id(),
            subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
            actions: vec!["A".into()],
            constraints: Constraints {
                max_spend: Some(100 - depth as u64),
                asset: Some("AETHER_TEST".into()),
                counterparties: None,
                rate_limit: Some(RateLimit {
                    max_ops: 5,
                    window_seconds: 60,
                }),
                valid_after: Some(0),
                valid_before: Some(5_000),
            },
            delegation_depth: depth,
            parent_capability_id: Some(current_id),
        };
        grant_and_store(&bundle, &registry, &mut store, &child);
        current_id = child.capability_id().unwrap();
    }
    let over = CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions: vec!["A".into()],
        constraints: Constraints {
            max_spend: Some(1),
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 1,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(5_000),
        },
        delegation_depth: 3,
        parent_capability_id: Some(current_id),
    };
    let grant = CapabilityGrant::sign(&bundle.signing_key, &over).unwrap();
    let err = grant_capability(
        &grant,
        &registry,
        &mut store,
        &bundle.identity.operational_public_key,
    )
    .unwrap_err();
    assert_eq!(err, aether_core::error::Error::ExcessiveDelegationDepth);
}

#[test]
fn p0_t076_forged_parent_link_rejected() {
    // P0-T076
    let (bundle, registry) = fresh_agent(vec!["A".into()], Some(100));
    let mut store = CapabilityStore::new();
    let child = CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions: vec!["A".into()],
        constraints: Constraints {
            max_spend: Some(10),
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 5,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(5_000),
        },
        delegation_depth: 1,
        parent_capability_id: Some([9u8; 32]),
    };
    let grant = CapabilityGrant::sign(&bundle.signing_key, &child).unwrap();
    let err = grant_capability(
        &grant,
        &registry,
        &mut store,
        &bundle.identity.operational_public_key,
    )
    .unwrap_err();
    assert_eq!(err, aether_core::error::Error::ParentMissing);
}

#[test]
fn p0_t080_revoked_capability_rejected() {
    // P0-T080
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    let id = cap.capability_id().unwrap();
    store.revoke(id, 200);
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&grant),
        &action("a", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::CapabilityRevoked
    );
}

#[test]
fn p0_t081_frozen_identity_rejected() {
    // P0-T081
    let (bundle, mut registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    let agent_id = bundle.identity.derived_agent_id();
    registry.freeze(&agent_id).unwrap();
    let decision = authorise_action(
        &registry,
        &store,
        &agent_id,
        Some(&grant),
        &action("a", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::IdentityFrozen
    );
}

#[test]
fn p0_t082_revoked_identity_rejected() {
    // P0-T082
    let (bundle, mut registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    let agent_id = bundle.identity.derived_agent_id();
    registry.revoke_identity(&agent_id).unwrap();
    let decision = authorise_action(
        &registry,
        &store,
        &agent_id,
        Some(&grant),
        &action("a", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::IdentityRevoked
    );
}

#[test]
fn p0_t083_parent_revoked_rejects_child() {
    // P0-T083
    let (bundle, registry) = fresh_agent(vec!["A".into()], Some(100));
    let mut store = CapabilityStore::new();
    let parent = direct_capability(&bundle, vec!["A".into()], Some(100));
    grant_and_store(&bundle, &registry, &mut store, &parent);
    let parent_id = parent.capability_id().unwrap();
    let child = CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions: vec!["A".into()],
        constraints: Constraints {
            max_spend: Some(50),
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 5,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(5_000),
        },
        delegation_depth: 1,
        parent_capability_id: Some(parent_id),
    };
    let child_grant = grant_and_store(&bundle, &registry, &mut store, &child);
    store.revoke(parent_id, 1);
    let decision = authorise_action(
        &registry,
        &store,
        &bundle.identity.derived_agent_id(),
        Some(&child_grant),
        &action("A", Some(1)),
        100,
    );
    assert_eq!(
        decision.expect_rejected().unwrap(),
        RejectReason::CapabilityRevoked
    );
}
