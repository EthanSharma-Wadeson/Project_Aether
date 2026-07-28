//! PROTO-NET-0 lifecycle tests — discovery, hello, session, envelope delivery.

mod common;

use aether_core::error::Error;
use aether_core::network::{
    accept_hello, close_session, complete_hello, create_hello, deliver_envelope, initiate_hello,
    make_envelope, sign_envelope, sign_hello, AgentDirectoryV0, SessionStatus, MSG_NET_HELLO,
    MSG_NET_HELLO_ACCEPT,
};
use common::*;

#[test]
fn net_t001_directory_register_and_lookup() {
    let pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let entry = pair.directory.lookup(&a_id).unwrap();
    assert_eq!(entry.endpoint, "sim://agent-a");
}

#[test]
fn net_t002_directory_unknown_agent_rejected() {
    let dir = AgentDirectoryV0::new();
    assert!(matches!(
        dir.lookup("unknown"),
        Err(Error::AgentNotInDirectory)
    ));
}

#[test]
fn net_t003_directory_requires_active_identity() {
    let registry = aether_core::identity::registry::IdentityRegistry::new();
    let mut dir = AgentDirectoryV0::new();
    let err = dir.register(&registry, "ghost", "sim://x", 1);
    assert!(matches!(err, Err(Error::UnknownNetworkAgent)));
}

#[test]
fn net_t004_hello_exchange_establishes_session() {
    let mut pair = setup_network_pair();
    let (id_a, id_b) = establish_session(&mut pair, 100, 200, 10, 1_000);
    assert_eq!(id_a, id_b);
    let sess = pair.store_a.get(&id_a).unwrap();
    assert_eq!(sess.session.status, SessionStatus::Established);
    assert!(sess.session.established_at.is_some());
    assert!(!sess.session.negotiated_features.is_empty());
}

#[test]
fn net_t005_session_id_deterministic_across_agents() {
    let mut pair = setup_network_pair_seeded(7);
    let (id_a, id_b) = establish_session(&mut pair, 11, 22, 5, 500);
    assert_eq!(id_a, id_b);
    // Re-derive independently
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let derived = aether_core::network::SecureSessionV0::derive_session_id(
        &a_id,
        &b_id,
        11,
        22,
        PROTOCOL_VERSION,
    )
    .unwrap();
    assert_eq!(derived, id_a);
}

#[test]
fn net_t006_valid_envelope_delivery() {
    let mut pair = setup_network_pair();
    let (sid_a, sid_b) = establish_session(&mut pair, 1, 2, 10, 1_000);
    assert_eq!(sid_a, sid_b);

    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let payload = b"hello-payload";
    let env = make_envelope(&pair.store_a, &sid_a, &a_id, "app.ping", payload, 1, 20).unwrap();
    let signed = sign_envelope(&env, &pair.a.signing_key).unwrap();

    let delivered = deliver_envelope(
        &mut pair.store_b,
        &pair.registry,
        &signed,
        &b_id,
        payload,
        20,
    )
    .unwrap();
    assert_eq!(delivered.message_type, "app.ping");
    assert_eq!(delivered.payload_commitment, env.payload_commitment);
}

#[test]
fn net_t007_close_session() {
    let mut pair = setup_network_pair();
    let (sid, _) = establish_session(&mut pair, 3, 4, 10, 1_000);
    let a_id = pair.a.identity.derived_agent_id();
    let closed = close_session(&mut pair.store_a, &sid, &a_id, 50).unwrap();
    assert_eq!(closed.status, SessionStatus::Closed);
}

#[test]
fn net_t008_envelope_requires_established_session() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();

    let hello = create_hello(&a_id, 10, 55, None);
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

    // No established session id yet — make_envelope fails
    let fake_sid = [1u8; 32];
    let err = make_envelope(&pair.store_a, &fake_sid, &a_id, "x", b"p", 1, 11);
    assert!(matches!(err, Err(Error::SessionNotFound)));
}

#[test]
fn net_t009_hello_feature_negotiation() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();

    let hello = create_hello(
        &a_id,
        10,
        70,
        Some(vec![
            "net.envelope.v0".into(),
            "net.session.v0".into(),
            "extra.feature".into(),
        ]),
    );
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

    let accept = create_hello(
        &b_id,
        11,
        71,
        Some(vec!["net.envelope.v0".into(), "net.session.v0".into()]),
    );
    let signed_accept = sign_hello(&accept, &pair.b.signing_key, MSG_NET_HELLO_ACCEPT).unwrap();
    let sess_b = accept_hello(
        &mut pair.store_b,
        &pair.registry,
        &b_id,
        &signed_hello,
        &signed_accept,
        11,
        1_000,
    )
    .unwrap();
    assert!(sess_b
        .negotiated_features
        .contains(&"net.envelope.v0".into()));
    assert!(!sess_b.negotiated_features.contains(&"extra.feature".into()));
}

#[test]
fn net_t010_discover_then_session() {
    let mut pair = setup_network_pair();
    let b_id = pair.b.identity.derived_agent_id();
    // Discovery before session
    let endpoint = pair.directory.lookup(&b_id).unwrap().endpoint.clone();
    assert_eq!(endpoint, "sim://agent-b");
    let (sid, _) = establish_session(&mut pair, 8, 9, 10, 1_000);
    assert_ne!(sid, [0u8; 32]);
}

#[test]
fn net_t011_complete_hello_without_pending_fails() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    let accept = create_hello(&b_id, 10, 99, None);
    let signed = sign_hello(&accept, &pair.b.signing_key, MSG_NET_HELLO_ACCEPT).unwrap();
    let err = complete_hello(
        &mut pair.store_a,
        &pair.registry,
        &a_id,
        &b_id,
        1,
        &signed,
        10,
    );
    assert!(matches!(err, Err(Error::SessionNotFound)));
}

#[test]
fn net_t012_mutual_identity_verified_through_proto0() {
    let mut pair = setup_network_pair();
    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();
    assert!(pair.registry.get(&a_id).is_some());
    assert!(pair.registry.get(&b_id).is_some());
    let (sid, _) = establish_session(&mut pair, 15, 16, 10, 1_000);
    let sess = &pair.store_a.get(&sid).unwrap().session;
    assert_eq!(sess.local_agent_id, a_id);
    assert_eq!(sess.peer_agent_id, b_id);
}
