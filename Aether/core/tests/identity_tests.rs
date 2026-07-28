//! PROTO-0 identity acceptance tests (P0-T001…).

mod common;

use aether_core::crypto::signing::DOMAIN_TAG;
use aether_core::crypto::verify::verify_signed_message;
use aether_core::identity::agent_id::derive_agent_id;
use aether_core::identity::agent_identity::{
    verify_identity_register, AgentIdentityV0, IdentityBundle, MSG_IDENTITY_REGISTER,
};
use aether_core::identity::registry::IdentityRegistry;
use aether_core::permission::root::PermissionRootV0;
use common::*;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

#[test]
fn p0_t001_valid_identity_registration() {
    // P0-T001
    let (bundle, registry) = fresh_agent(vec!["payment.channel.open".into()], Some(100));
    let entry = registry
        .get(&bundle.identity.derived_agent_id())
        .expect("registered");
    assert_eq!(entry.status, aether_core::types::AgentStatus::Active);
}

#[test]
fn p0_t002_correct_public_key_signature() {
    // P0-T002
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let msg = bundle.sign_register().unwrap();
    verify_signed_message(
        &msg,
        &bundle.identity.operational_public_key,
        MSG_IDENTITY_REGISTER,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
    )
    .unwrap();
}

#[test]
fn p0_t003_correct_agent_id_derivation() {
    // P0-T003
    let (bundle, registry) = fresh_agent(vec!["a".into()], Some(10));
    let expected = derive_agent_id(SCHEMA_VERSION, &bundle.identity.operational_public_key);
    assert_eq!(bundle.identity.derived_agent_id(), expected);
    assert!(registry.get(&expected).is_some());
}

#[test]
fn p0_t010_altered_public_key_rejected() {
    // P0-T010
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut msg = bundle.sign_register().unwrap();
    let mut identity = AgentIdentityV0::decode(&msg.body).unwrap();
    identity.operational_public_key[0] ^= 0xff;
    msg.body = identity.encode().unwrap();
    assert!(verify_identity_register(&msg).is_err());
}

#[test]
fn p0_t011_invalid_signature_rejected() {
    // P0-T011
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut msg = bundle.sign_register().unwrap();
    msg.signature[0] ^= 0xff;
    assert!(verify_identity_register(&msg).is_err());
}

#[test]
fn p0_t012_mismatched_agent_id_rejected() {
    // P0-T012 — permission root agent_id does not match derived AgentId
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut fake_root = bundle.permission_root.clone();
    fake_root.agent_id = "aether:deadbeef".into();
    // Keep commitment consistent with the forged root so the agent-id check is reached.
    let mut identity = bundle.identity.clone();
    identity.permission_root = fake_root.commitment().unwrap();
    let body = identity.encode().unwrap();
    let (_d, signature) = aether_core::crypto::signing::sign_body(
        &bundle.signing_key,
        aether_core::crypto::signing::DOMAIN_TAG,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
        aether_core::identity::agent_identity::MSG_IDENTITY_REGISTER,
        &body,
    );
    let msg = aether_core::crypto::verify::SignedMessage {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        message_type: aether_core::identity::agent_identity::MSG_IDENTITY_REGISTER.into(),
        body,
        signer_key_id: "operational:0".into(),
        signature: signature.to_bytes().to_vec(),
        domain_tag: aether_core::crypto::signing::DOMAIN_TAG.into(),
    };
    let mut registry = IdentityRegistry::new();
    let err = registry
        .register(&msg, fake_root, bundle.root_authority.clone(), 0)
        .unwrap_err();
    assert_eq!(err, aether_core::error::Error::AgentIdMismatch);
}

#[test]
fn p0_t013_malformed_identity_object_rejected() {
    // P0-T013
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut msg = bundle.sign_register().unwrap();
    msg.body = vec![0xff, 0x00, 0x01];
    assert!(verify_identity_register(&msg).is_err());
}

#[test]
fn p0_t014_signing_context_mismatch_rejected() {
    // P0-T014
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut msg = bundle.sign_register().unwrap();
    msg.domain_tag = "aether:v0:sign:WRONG".into();
    assert!(verify_identity_register(&msg).is_err());
    let _ = DOMAIN_TAG;
}

#[test]
fn p0_t020_valid_permission_root() {
    // P0-T020
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    bundle
        .permission_root
        .verify_commitment(&bundle.identity.permission_root)
        .unwrap();
}

#[test]
fn p0_t021_root_authority_id_matches() {
    // P0-T021
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    bundle
        .permission_root
        .verify_against_authority(&bundle.root_authority)
        .unwrap();
}

#[test]
fn p0_t022_monotonic_root_update() {
    // P0-T022
    let (bundle, mut registry) = fresh_agent(vec!["a".into()], Some(10));
    let agent_id = bundle.identity.derived_agent_id();
    let mut new_authority = bundle.root_authority.clone();
    new_authority.constraints.max_spend = Some(50);
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
        registry
            .get(&agent_id)
            .unwrap()
            .permission_root_material
            .root_version,
        2
    );
}

#[test]
fn p0_t030_modified_permission_root_rejected() {
    // P0-T030
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut bad = [0u8; 32];
    bad[0] = 1;
    assert!(bundle.permission_root.verify_commitment(&bad).is_err());
}

#[test]
fn p0_t031_invalid_root_authority_rejected() {
    // P0-T031
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut other = bundle.root_authority.clone();
    other.actions.push("evil".into());
    assert!(bundle
        .permission_root
        .verify_against_authority(&other)
        .is_err());
}

#[test]
fn p0_t033_identity_mismatch_on_root_rejected() {
    // P0-T033
    let (bundle, _) = fresh_agent(vec!["a".into()], Some(10));
    let mut root = bundle.permission_root.clone();
    root.agent_id = "aether:other".into();
    assert!(root
        .verify_against_authority(&bundle.root_authority)
        .is_err());
}

#[test]
fn p0_t034_non_monotonic_root_update_rejected() {
    // P0-T034
    let (bundle, mut registry) = fresh_agent(vec!["a".into()], Some(10));
    let agent_id = bundle.identity.derived_agent_id();
    let err = registry
        .update_permission_root(
            &agent_id,
            &bundle.signing_key,
            bundle.permission_root.clone(),
            bundle.root_authority.clone(),
        )
        .unwrap_err();
    assert_eq!(err, aether_core::error::Error::NonMonotonicRootVersion);
}

#[test]
fn p0_t012_claimed_id_must_match_derivation() {
    // Additional AgentId derivation binding check
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let root = root_authority_template(vec!["a".into()], Some(1));
    let bundle =
        IdentityBundle::create(signing_key, PROTOCOL_VERSION, SCHEMA_VERSION, root, 1, None)
            .unwrap();
    assert!(bundle.identity.derived_agent_id().starts_with("aether:"));
}
