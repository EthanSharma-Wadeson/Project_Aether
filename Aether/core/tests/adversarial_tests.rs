//! PROTO-0 adversarial tests P0-A01 → P0-A12.

mod common;

use aether_core::capability::model::{CapabilityStore, CapabilityV0};
use aether_core::capability::grant::{grant_capability, CapabilityGrant};
use aether_core::crypto::signing::{sign_body, DOMAIN_TAG};
use aether_core::crypto::verify::SignedMessage;
use aether_core::permission::root::{Constraints, PermissionRootV0};
use aether_core::types::{RateLimit, RejectReason, SubjectRef};
use aether_core::verifier::authorise::{
    authorise_action, authorise_with_secondary_signature_only,
};
use common::*;

#[test]
fn p0_a01_capability_escalation() {
    // P0-A01 — attacker widens spend / actions
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
        actions: vec!["A".into(), "B".into()],
        constraints: Constraints {
            max_spend: Some(200),
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 100,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(9_999),
        },
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
fn p0_a02_forged_parent_link() {
    // P0-A02
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
        parent_capability_id: Some([0xab; 32]),
    };
    let grant = CapabilityGrant::sign(&bundle.signing_key, &child).unwrap();
    assert_eq!(
        grant_capability(
            &grant,
            &registry,
            &mut store,
            &bundle.identity.operational_public_key,
        )
        .unwrap_err(),
        aether_core::error::Error::ParentMissing
    );
}

#[test]
fn p0_a03_excessive_delegation_depth() {
    // P0-A03
    let (bundle, registry) = fresh_agent(vec!["A".into()], Some(100));
    let mut store = CapabilityStore::new();
    let parent = direct_capability(&bundle, vec!["A".into()], Some(100));
    grant_and_store(&bundle, &registry, &mut store, &parent);
    let child = CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions: vec!["A".into()],
        constraints: parent.constraints.clone(),
        delegation_depth: 9,
        parent_capability_id: Some(parent.capability_id().unwrap()),
    };
    let grant = CapabilityGrant::sign(&bundle.signing_key, &child).unwrap();
    let err = grant_capability(
        &grant,
        &registry,
        &mut store,
        &bundle.identity.operational_public_key,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        aether_core::error::Error::InvalidDelegationDepth
            | aether_core::error::Error::ExcessiveDelegationDepth
    ));
}

#[test]
fn p0_a04_expired_capability_replay() {
    // P0-A04
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let mut cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    cap.constraints.valid_before = Some(10);
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    assert_eq!(
        authorise_action(
            &registry,
            &store,
            &bundle.identity.derived_agent_id(),
            Some(&grant),
            &action("a", Some(1)),
            100,
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::CapabilityExpired
    );
}

#[test]
fn p0_a05_revoked_capability_use() {
    // P0-A05
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    store.revoke(cap.capability_id().unwrap(), 1);
    assert_eq!(
        authorise_action(
            &registry,
            &store,
            &bundle.identity.derived_agent_id(),
            Some(&grant),
            &action("a", Some(1)),
            100,
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::CapabilityRevoked
    );
}

#[test]
fn p0_a06_stale_permission_root() {
    // P0-A06
    let (bundle, mut registry) = fresh_agent(vec!["a".into()], Some(100));
    let mut store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(50));
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    let agent_id = bundle.identity.derived_agent_id();

    let mut new_authority = bundle.root_authority.clone();
    new_authority.constraints.max_spend = Some(80);
    let new_root = PermissionRootV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        agent_id: agent_id.clone(),
        root_version: 2,
        root_authority_capability_id: new_authority.capability_id(),
    };
    registry
        .update_permission_root(&agent_id, &bundle.signing_key, new_root, new_authority)
        .unwrap();

    assert_eq!(
        authorise_action(
            &registry,
            &store,
            &agent_id,
            Some(&grant),
            &action("a", Some(1)),
            100,
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::StaleRootVersion
    );
}

#[test]
fn p0_a07_mismatched_agent_id() {
    // P0-A07
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let (other, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut store = CapabilityStore::new();
    let mut cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    // Claim other agent as subject while signed by bundle
    cap.subject = SubjectRef::AgentId(other.identity.derived_agent_id());
    let grant = grant_and_store(&bundle, &registry, &mut store, &cap);
    assert_eq!(
        authorise_action(
            &registry,
            &store,
            &bundle.identity.derived_agent_id(),
            Some(&grant),
            &action("a", Some(1)),
            100,
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::AgentIdMismatch
    );
}

#[test]
fn p0_a08_altered_capability_constraints() {
    // P0-A08
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(100));
    let store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(50));
    let mut grant = CapabilityGrant::sign(&bundle.signing_key, &cap).unwrap();
    let mut mutated = cap;
    mutated.constraints.max_spend = Some(999);
    grant.message.body = mutated.encode().unwrap();
    assert_eq!(
        authorise_action(
            &registry,
            &store,
            &bundle.identity.derived_agent_id(),
            Some(&grant),
            &action("a", Some(1)),
            100,
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::InvalidSignature
    );
}

#[test]
fn p0_a09_invalid_signature() {
    // P0-A09
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let mut grant = CapabilityGrant::sign(&bundle.signing_key, &cap).unwrap();
    grant.message.signature[3] ^= 0xaa;
    assert_eq!(
        authorise_action(
            &registry,
            &store,
            &bundle.identity.derived_agent_id(),
            Some(&grant),
            &action("a", Some(1)),
            100,
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::InvalidSignature
    );
}

#[test]
fn p0_a10_signing_context_mismatch() {
    // P0-A10
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let store = CapabilityStore::new();
    let cap = direct_capability(&bundle, vec!["a".into()], Some(10));
    let body = cap.encode().unwrap();
    let (_d, signature) = sign_body(
        &bundle.signing_key,
        DOMAIN_TAG,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
        "capability.grant",
        &body,
    );
    let grant = CapabilityGrant {
        message: SignedMessage {
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
            message_type: "capability.grant".into(),
            body,
            signer_key_id: "operational:0".into(),
            signature: signature.to_bytes().to_vec(),
            domain_tag: "aether:v0:sign:WRONG".into(),
        },
    };
    assert_eq!(
        authorise_action(
            &registry,
            &store,
            &bundle.identity.derived_agent_id(),
            Some(&grant),
            &action("a", Some(1)),
            100,
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::SigningContextMismatch
    );
}

#[test]
fn p0_a11_identity_only_bypass() {
    // P0-A11
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let store = CapabilityStore::new();
    assert_eq!(
        authorise_action(
            &registry,
            &store,
            &bundle.identity.derived_agent_id(),
            None,
            &action("a", Some(1)),
            100,
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::IdentityOnlyBypass
    );
}

#[test]
fn p0_a12_secondary_signature_without_capability() {
    // P0-A12
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let body = bundle.identity.encode().unwrap();
    let (_d, signature) = sign_body(
        &bundle.signing_key,
        DOMAIN_TAG,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
        "payment.update",
        &body,
    );
    let secondary = SignedMessage {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        message_type: "payment.update".into(),
        body,
        signer_key_id: "operational:0".into(),
        signature: signature.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    };
    assert_eq!(
        authorise_with_secondary_signature_only(
            &registry,
            &bundle.identity.derived_agent_id(),
            &secondary,
            &action("a", Some(1)),
        )
        .expect_rejected()
        .unwrap(),
        RejectReason::MissingCapability
    );
}
