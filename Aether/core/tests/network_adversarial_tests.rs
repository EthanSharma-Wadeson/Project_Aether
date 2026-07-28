//! PROTO-NET-0 adversarial tests — NET-A01 … NET-A10.

mod common;

use aether_core::crypto::signing::{sign_body, DOMAIN_TAG};
use aether_core::crypto::verify::SignedMessage;
use aether_core::error::Error;
use aether_core::network::{
    accept_hello, close_session, create_hello, deliver_envelope, initiate_hello, make_envelope,
    sign_envelope, sign_hello, MessageEnvelopeV0, MSG_NET_ENVELOPE, MSG_NET_HELLO,
    MSG_NET_HELLO_ACCEPT, NETWORK_PROTOCOL_VERSION, NETWORK_SCHEMA_VERSION,
};
use common::*;

fn established_pair() -> (common::NetworkPair, [u8; 32], [u8; 32]) {
    let mut pair = setup_network_pair();
    let (a, b) = establish_session(&mut pair, 1001, 1002, 10, 1_000);
    (pair, a, b)
}

/// NET-A01 — Invalid signature rejection
#[test]
fn net_a01_invalid_signature_rejected() {
    let (mut pair, sid_a, _) = established_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let payload = b"data";
    let env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.x", payload, 1, 20).unwrap();
    let mut signed = sign_envelope(&env, &pair.a.signing_key).unwrap();
    signed.signature[0] ^= 0xff;

    let err = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        &b_id,
        payload,
        20,
    );
    assert!(matches!(err, Err(Error::InvalidSignature)));
}

/// NET-A02 — Wrong receiver rejection
#[test]
fn net_a02_wrong_receiver_rejected() {
    let (mut pair, sid_a, _) = established_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let payload = b"data";
    let env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.x", payload, 1, 20).unwrap();
    let signed = sign_envelope(&env, &pair.a.signing_key).unwrap();

    let err = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        "not-the-receiver",
        payload,
        20,
    );
    assert!(matches!(err, Err(Error::EnvelopeReceiverMismatch)));
}

/// NET-A03 — Replay attack rejection (re-deliver same envelope)
#[test]
fn net_a03_replay_attack_rejected() {
    let (mut pair, sid_a, _) = established_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let payload = b"data";
    let env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.x", payload, 1, 20).unwrap();
    let signed = sign_envelope(&env, &pair.a.signing_key).unwrap();

    deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        &b_id,
        payload,
        20,
    )
    .unwrap();

    let err = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        &b_id,
        payload,
        21,
    );
    assert!(matches!(err, Err(Error::DuplicateMessageId)));
}

/// NET-A04 — Duplicate message ID rejection
#[test]
fn net_a04_duplicate_message_id_rejected() {
    // Same as replay: delivering two envelopes with identical message_id
    net_a03_replay_attack_rejected();
}

/// NET-A05 — Unsupported protocol version rejection
#[test]
fn net_a05_unsupported_protocol_version_rejected() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();

    let mut hello = create_hello(&a_id, 10, 50, None);
    hello.protocol_version = 99;
    let body = hello.encode().unwrap();
    let (_, sig) = sign_body(
        &pair.a.signing_key,
        DOMAIN_TAG,
        99,
        NETWORK_SCHEMA_VERSION,
        MSG_NET_HELLO,
        &body,
    );
    let signed = SignedMessage {
        protocol_version: 99,
        schema_version: NETWORK_SCHEMA_VERSION,
        message_type: MSG_NET_HELLO.into(),
        body,
        signer_key_id: a_id.clone(),
        signature: sig.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    };

    let err = initiate_hello(
        &mut pair.store_a,
        &pair.registry,
        &a_id,
        &b_id,
        &signed,
        10,
        1_000,
    );
    assert!(matches!(err, Err(Error::UnsupportedProtocolVersion)));
}

/// NET-A06 — Expired session rejection
#[test]
fn net_a06_expired_session_rejected() {
    let mut pair = setup_network_pair();
    let (sid_a, _) = establish_session(&mut pair, 60, 61, 10, 50); // expires_at = 60
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let payload = b"late";
    let env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.x", payload, 1, 20).unwrap();
    let signed = sign_envelope(&env, &pair.a.signing_key).unwrap();

    let err = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        &b_id,
        payload,
        100, // past expiry
    );
    assert!(matches!(err, Err(Error::SessionExpired)));
}

/// NET-A07 — Unknown agent rejection
#[test]
fn net_a07_unknown_agent_rejected() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();

    let hello = create_hello("totally-unknown-agent", 10, 77, None);
    // Sign with A's key but claim unknown agent id — signature verify looks up claimed id
    let body = hello.encode().unwrap();
    let (_, sig) = sign_body(
        &pair.a.signing_key,
        DOMAIN_TAG,
        NETWORK_PROTOCOL_VERSION,
        NETWORK_SCHEMA_VERSION,
        MSG_NET_HELLO,
        &body,
    );
    let signed = SignedMessage {
        protocol_version: NETWORK_PROTOCOL_VERSION,
        schema_version: NETWORK_SCHEMA_VERSION,
        message_type: MSG_NET_HELLO.into(),
        body,
        signer_key_id: "totally-unknown-agent".into(),
        signature: sig.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    };

    let err = initiate_hello(
        &mut pair.store_a,
        &pair.registry,
        &a_id,
        &pair.b.identity.derived_agent_id(),
        &signed,
        10,
        1_000,
    );
    assert!(matches!(err, Err(Error::UnknownNetworkAgent)));
}

/// NET-A08 — Payload commitment mismatch rejection
#[test]
fn net_a08_payload_commitment_mismatch_rejected() {
    let (mut pair, sid_a, _) = established_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let payload = b"original";
    let env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.x", payload, 1, 20).unwrap();
    let signed = sign_envelope(&env, &pair.a.signing_key).unwrap();

    let err = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        &b_id,
        b"tampered-payload",
        20,
    );
    assert!(matches!(err, Err(Error::PayloadCommitmentMismatch)));
}

/// NET-A09 — Session identity mismatch rejection
#[test]
fn net_a09_session_identity_mismatch_rejected() {
    let (mut pair, sid_a, _) = established_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let payload = b"data";

    // Build envelope but swap sender to B while signing with A
    let mut env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.x", payload, 1, 20).unwrap();
    env.sender_agent_id = b_id.clone();
    env.receiver_agent_id = a_id.clone();
    // Recompute won't help — sign with A claiming to be B
    let body = env.encode().unwrap();
    let (_, sig) = sign_body(
        &pair.a.signing_key,
        DOMAIN_TAG,
        env.protocol_version,
        env.schema_version,
        MSG_NET_ENVELOPE,
        &body,
    );
    let signed = SignedMessage {
        protocol_version: env.protocol_version,
        schema_version: env.schema_version,
        message_type: MSG_NET_ENVELOPE.into(),
        body,
        signer_key_id: b_id.clone(),
        signature: sig.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    };

    // verify looks up B's key; A's signature won't verify under B's key
    let err = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        &a_id,
        payload,
        20,
    );
    assert!(matches!(
        err,
        Err(Error::InvalidSignature) | Err(Error::SessionIdentityMismatch)
    ));
}

/// NET-A10 — Modified envelope field rejection
#[test]
fn net_a10_modified_envelope_field_rejected() {
    let (mut pair, sid_a, _) = established_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let payload = b"data";
    let env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.x", payload, 1, 20).unwrap();
    let signed = sign_envelope(&env, &pair.a.signing_key).unwrap();

    // Tamper body after signing
    let mut tampered = signed;
    let mut env2 = MessageEnvelopeV0::decode(&tampered.body).unwrap();
    env2.message_type = "app.evil".into();
    tampered.body = env2.encode().unwrap();

    let err = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &tampered,
        &b_id,
        payload,
        20,
    );
    assert!(matches!(err, Err(Error::InvalidSignature)));
}

#[test]
fn net_a11_hello_replay_nonce_rejected() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();

    let hello = create_hello(&a_id, 10, 42, None);
    let signed = sign_hello(&hello, &pair.a.signing_key, MSG_NET_HELLO).unwrap();
    initiate_hello(
        &mut pair.store_a,
        &pair.registry,
        &a_id,
        &b_id,
        &signed,
        10,
        1_000,
    )
    .unwrap();

    let err = initiate_hello(
        &mut pair.store_a,
        &pair.registry,
        &a_id,
        &b_id,
        &signed,
        11,
        1_000,
    );
    assert!(matches!(
        err,
        Err(Error::MessageReplay) | Err(Error::SessionAlreadyExists)
    ));
}

#[test]
fn net_a12_accept_with_forged_hello_rejected() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();

    let hello = create_hello(&a_id, 10, 88, None);
    let mut signed_hello = sign_hello(&hello, &pair.a.signing_key, MSG_NET_HELLO).unwrap();
    signed_hello.signature[1] ^= 0xaa;

    let accept = create_hello(&b_id, 11, 89, None);
    let signed_accept = sign_hello(&accept, &pair.b.signing_key, MSG_NET_HELLO_ACCEPT).unwrap();

    let err = accept_hello(
        &mut pair.store_b,
        &pair.registry,
        &b_id,
        &signed_hello,
        &signed_accept,
        11,
        1_000,
    );
    assert!(matches!(err, Err(Error::InvalidSignature)));
}

#[test]
fn net_a13_envelope_after_close_rejected() {
    let (mut pair, sid_a, sid_b) = established_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();

    close_session(&mut pair.store_b, &sid_b, &b_id, 30).unwrap();

    let payload = b"after-close";
    let env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.x", payload, 1, 31).unwrap();
    let signed = sign_envelope(&env, &pair.a.signing_key).unwrap();

    let err = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        &b_id,
        payload,
        31,
    );
    assert!(matches!(
        err,
        Err(Error::InvalidSessionStatus) | Err(Error::SessionExpired)
    ));
}

#[test]
fn net_a14_empty_feature_intersection_rejected() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();

    let hello = create_hello(&a_id, 10, 200, Some(vec!["feat.a".into()]));
    let signed_hello = sign_hello(&hello, &pair.a.signing_key, MSG_NET_HELLO).unwrap();
    initiate_hello(
        &mut pair.store_a,
        &pair.registry,
        &a_id,
        &b_id,
        &signed_hello,
        10,
        1_000,
    )
    .unwrap();

    let accept = create_hello(&b_id, 11, 201, Some(vec!["feat.b".into()]));
    let signed_accept = sign_hello(&accept, &pair.b.signing_key, MSG_NET_HELLO_ACCEPT).unwrap();
    let err = accept_hello(
        &mut pair.store_b,
        &pair.registry,
        &b_id,
        &signed_hello,
        &signed_accept,
        11,
        1_000,
    );
    assert!(matches!(err, Err(Error::UnsupportedProtocolVersion)));
}
