//! PROTO-1 channel adversarial acceptance tests.

mod common;

use aether_core::capability::grant::CapabilityGrant;
use aether_core::capability::model::CapabilityStore;
use aether_core::channel::{
    activate_channel, apply_update, begin_close, finalize_close, open_channel, raise_dispute,
    resolve_dispute, sign_open_dual, sign_update_dual, ChannelOpenMaterialV0, ChannelStateV0,
    ChannelStatus, ChannelStore, DisputeEvidenceV0, DualSignedUpdate, StateUpdateV0,
    MSG_CHANNEL_ACTIVATE, MSG_CHANNEL_CLOSE, MSG_CHANNEL_OPEN, MSG_CHANNEL_UPDATE,
};
use aether_core::crypto::signing::{sign_body, DOMAIN_TAG};
use aether_core::crypto::verify::SignedMessage;
use aether_core::error::Error;
use aether_core::identity::agent_identity::IdentityBundle;
use aether_core::identity::registry::IdentityRegistry;
use ciborium::value::Value;
use common::*;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

struct Pair {
    registry: IdentityRegistry,
    caps: CapabilityStore,
    channels: ChannelStore,
    a: IdentityBundle,
    b: IdentityBundle,
    grant_a: CapabilityGrant,
    grant_b: CapabilityGrant,
}

fn setup_pair() -> Pair {
    let mut registry = IdentityRegistry::new();
    let a = register_agent(&mut registry, channel_actions(), Some(1_000));
    let b = register_agent(&mut registry, channel_actions(), Some(1_000));
    let mut caps = CapabilityStore::new();
    let grant_a = grant_channel_caps(&a, &registry, &mut caps);
    let grant_b = grant_channel_caps(&b, &registry, &mut caps);
    Pair {
        registry,
        caps,
        channels: ChannelStore::new(),
        a,
        b,
        grant_a,
        grant_b,
    }
}

fn open_material(a: &IdentityBundle, b: &IdentityBundle) -> ChannelOpenMaterialV0 {
    ChannelOpenMaterialV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        party_a: a.identity.derived_agent_id(),
        party_b: b.identity.derived_agent_id(),
        asset: "AETHER_TEST".into(),
        opening_balances: [60, 40],
        dispute_window: 100,
        created_at: 1,
    }
}

fn open_and_activate(pair: &mut Pair) -> [u8; 32] {
    let material = open_material(&pair.a, &pair.b);
    let (sig_a, sig_b) = sign_open_dual(
        &material,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_OPEN,
    )
    .unwrap();
    open_channel(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &material,
        &sig_a,
        &sig_b,
        &pair.grant_a,
        &pair.grant_b,
        10,
    )
    .unwrap();
    let channel_id = material.channel_id().unwrap();
    let state = ChannelStateV0 {
        channel_id,
        sequence: 0,
        balances: material.opening_balances,
        asset: material.asset.clone(),
        status_hint: ChannelStatus::Active,
        created_at: 2,
        metadata_commitment: None,
    };
    let update = StateUpdateV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        channel_id,
        previous_state_commitment: [0u8; 32],
        new_state: state,
        sequence: 0,
        logical_time: 2,
    };
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_ACTIVATE,
    )
    .unwrap();
    activate_channel(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        20,
    )
    .unwrap();
    channel_id
}

fn make_update(
    channel_id: [u8; 32],
    prev: [u8; 32],
    sequence: u64,
    balances: [u64; 2],
    status_hint: ChannelStatus,
    logical_time: u64,
) -> StateUpdateV0 {
    StateUpdateV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        channel_id,
        previous_state_commitment: prev,
        new_state: ChannelStateV0 {
            channel_id,
            sequence,
            balances,
            asset: "AETHER_TEST".into(),
            status_hint,
            created_at: logical_time,
            metadata_commitment: None,
        },
        sequence,
        logical_time,
    }
}

fn apply_seq(
    pair: &mut Pair,
    channel_id: [u8; 32],
    seq: u64,
    balances: [u64; 2],
) -> DualSignedUpdate {
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(
        channel_id,
        prev,
        seq,
        balances,
        ChannelStatus::Active,
        30 + seq,
    );
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30 + seq,
    )
    .unwrap();
    dual
}

#[test]
fn p1_a01_forged_update_rejected() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let mut dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    dual.sig_a.signature = vec![1u8; 64];
    dual.sig_b.signature = vec![2u8; 64];
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidSignature);
}

#[test]
fn p1_a02_one_invalid_signer_rejected() {
    // Unilateral / broken second signature
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let mut dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    dual.sig_b.signature = vec![0u8; 64];
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidSignature);
}

#[test]
fn p1_a03_replay_older_update() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let d1 = apply_seq(&mut pair, channel_id, 1, [55, 45]);
    apply_seq(&mut pair, channel_id, 2, [70, 30]);
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &d1,
        &pair.grant_a,
        &pair.grant_b,
        50,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::SequenceStale | Error::StateCommitmentMismatch
    ));
}

#[test]
fn p1_a04_stale_sequence() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    apply_seq(&mut pair, channel_id, 1, [55, 45]);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [40, 60], ChannelStatus::Active, 40);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        40,
    )
    .unwrap_err();
    assert_eq!(err, Error::SequenceStale);
}

#[test]
fn p1_a05_skipped_sequence() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 2, [50, 50], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::SequenceSkip);
}

#[test]
fn p1_a06_participant_mismatch_wrong_signer() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let mut csprng = OsRng;
    let outsider = SigningKey::generate(&mut csprng);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let dual =
        sign_update_dual(&update, &pair.a.signing_key, &outsider, MSG_CHANNEL_UPDATE).unwrap();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidSignature);
}

#[test]
fn p1_a07_unauthorized_disputed_to_active() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let u1 = make_update(channel_id, prev, 1, [55, 45], ChannelStatus::Active, 30);
    let d1 = sign_update_dual(
        &u1,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &DisputeEvidenceV0 {
            channel_id,
            claimed_update: d1,
            logical_time_raised: 35,
        },
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        35,
    )
    .unwrap();
    let prev2 = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev2, 2, [50, 50], ChannelStatus::Active, 40);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        40,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidChannelStatus);
}

#[test]
fn p1_a08_invalid_close_request() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let state = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .latest_state
        .clone()
        .unwrap();
    let close = StateUpdateV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        channel_id,
        previous_state_commitment: prev,
        new_state: ChannelStateV0 {
            status_hint: ChannelStatus::Closing,
            ..state
        },
        sequence: 0,
        logical_time: 50,
    };
    let mut dual = sign_update_dual(
        &close,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_CLOSE,
    )
    .unwrap();
    dual.sig_b.signature = vec![0u8; 64];
    let err = begin_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        50,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidSignature);
}

#[test]
fn p1_a09_balance_inflation() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [200, 200], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::BalanceConservation);
}

#[test]
fn p1_a10_negative_balance_cbor_rejected() {
    use aether_core::cbor::{bytes, encode_value, map, text, u32_value, u64_value};
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let new_state = map(vec![
        (text("channel_id"), bytes(channel_id.to_vec())),
        (text("sequence"), u64_value(1)),
        (
            text("balances"),
            Value::Array(vec![Value::Integer((-1i64).into()), u64_value(101)]),
        ),
        (text("asset"), text("AETHER_TEST")),
        (text("status_hint"), text("active")),
        (text("created_at"), u64_value(30)),
        (text("metadata_commitment"), Value::Null),
    ]);
    let body_val = map(vec![
        (text("protocol_version"), u32_value(PROTOCOL_VERSION)),
        (text("schema_version"), u32_value(SCHEMA_VERSION)),
        (text("channel_id"), bytes(channel_id.to_vec())),
        (text("previous_state_commitment"), bytes(prev.to_vec())),
        (text("new_state"), new_state),
        (text("sequence"), u64_value(1)),
        (text("logical_time"), u64_value(30)),
    ]);
    let body = encode_value(&body_val).unwrap();
    let (_d, sa) = sign_body(
        &pair.a.signing_key,
        DOMAIN_TAG,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
        MSG_CHANNEL_UPDATE,
        &body,
    );
    let (_d, sb) = sign_body(
        &pair.b.signing_key,
        DOMAIN_TAG,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
        MSG_CHANNEL_UPDATE,
        &body,
    );
    let dual = DualSignedUpdate {
        body: body.clone(),
        sig_a: SignedMessage {
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
            message_type: MSG_CHANNEL_UPDATE.into(),
            body: body.clone(),
            signer_key_id: "operational:0".into(),
            signature: sa.to_bytes().to_vec(),
            domain_tag: DOMAIN_TAG.into(),
        },
        sig_b: SignedMessage {
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
            message_type: MSG_CHANNEL_UPDATE.into(),
            body,
            signer_key_id: "operational:0".into(),
            signature: sb.to_bytes().to_vec(),
            domain_tag: DOMAIN_TAG.into(),
        },
    };
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::MalformedObject(_) | Error::InvalidSignature
    ));
}

#[test]
fn p1_a11_asset_mutation() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let mut update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    update.new_state.asset = "OTHER".into();
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::BalanceConservation);
}

#[test]
fn p1_a12_previous_commitment_mismatch() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let update = make_update(
        channel_id,
        [9u8; 32],
        1,
        [50, 50],
        ChannelStatus::Active,
        30,
    );
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::StateCommitmentMismatch);
}

#[test]
fn p1_a13_dual_sigs_over_different_bodies() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let u1 = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let u2 = make_update(channel_id, prev, 1, [40, 60], ChannelStatus::Active, 30);
    let d1 = sign_update_dual(
        &u1,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let d2 = sign_update_dual(
        &u2,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let dual = DualSignedUpdate {
        body: d1.body.clone(),
        sig_a: d1.sig_a,
        sig_b: d2.sig_b,
    };
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidSignature | Error::UnilateralUpdate
    ));
}

#[test]
fn p1_a14_soft_vs_hard_finality_distinct() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let f = &pair.channels.get(&channel_id).unwrap().channel.finality;
    assert!(f.soft_local_agreement);
    assert!(!f.hard_settlement_placeholder);
    assert!(!f.finalized);
}

#[test]
fn p1_a15_dispute_higher_state_during_closing() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let d1 = apply_seq(&mut pair, channel_id, 1, [55, 45]);
    let prev1 = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let u2 = make_update(channel_id, prev1, 2, [20, 80], ChannelStatus::Active, 40);
    let d2 = sign_update_dual(
        &u2,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    // Close at seq=1 without applying seq=2.
    let state = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .latest_state
        .clone()
        .unwrap();
    let close = StateUpdateV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        channel_id,
        previous_state_commitment: prev1,
        new_state: ChannelStateV0 {
            status_hint: ChannelStatus::Closing,
            ..state
        },
        sequence: 1,
        logical_time: 50,
    };
    let close_dual = sign_update_dual(
        &close,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_CLOSE,
    )
    .unwrap();
    begin_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &close_dual,
        &pair.grant_a,
        &pair.grant_b,
        50,
    )
    .unwrap();
    raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &DisputeEvidenceV0 {
            channel_id,
            claimed_update: d2.clone(),
            logical_time_raised: 55,
        },
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        55,
    )
    .unwrap();
    let (ch, state) = resolve_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &[d1, d2],
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        200,
    )
    .unwrap();
    assert_eq!(ch.status, ChannelStatus::Finalized);
    assert_eq!(state.sequence, 2);
}

#[test]
fn p1_a16_dispute_single_signature_rejected() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [55, 45], ChannelStatus::Active, 30);
    let mut dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    dual.sig_b.signature = vec![0u8; 64];
    let err = raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &DisputeEvidenceV0 {
            channel_id,
            claimed_update: dual,
            logical_time_raised: 35,
        },
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        35,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidDisputeEvidence);
    assert_eq!(
        pair.channels.get(&channel_id).unwrap().channel.status,
        ChannelStatus::Active
    );
}

#[test]
fn p1_extra_wrong_channel_id() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let mut update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    update.channel_id = [7u8; 32];
    update.new_state.channel_id = [7u8; 32];
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::ParticipantMismatch);
}

#[test]
fn p1_extra_modified_signed_contents() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let mut dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    // Tamper body after signing
    dual.body[10] ^= 0xff;
    dual.sig_a.body = dual.body.clone();
    dual.sig_b.body = dual.body.clone();
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidSignature | Error::MalformedObject(_)
    ));
}

#[test]
fn p1_extra_fake_dispute_and_invalid_highest() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let d1 = apply_seq(&mut pair, channel_id, 1, [55, 45]);
    let prev1 = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let u2 = make_update(channel_id, prev1, 2, [50, 50], ChannelStatus::Active, 40);
    let d2 = sign_update_dual(
        &u2,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &DisputeEvidenceV0 {
            channel_id,
            claimed_update: d2,
            logical_time_raised: 35,
        },
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        35,
    )
    .unwrap();
    let prev = [0u8; 32];
    let fake = make_update(channel_id, prev, 99, [1, 99], ChannelStatus::Active, 99);
    let mut fake_dual = sign_update_dual(
        &fake,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    fake_dual.sig_a.signature = vec![0u8; 64];
    let (ch, state) = resolve_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &[fake_dual, d1],
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        200,
    )
    .unwrap();
    assert_eq!(ch.status, ChannelStatus::Finalized);
    assert_eq!(state.sequence, 2);
}

#[test]
fn p1_r01_orphan_high_sequence_cannot_win_dispute() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let _ = apply_seq(&mut pair, channel_id, 1, [55, 45]);
    let prev1 = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let u2 = make_update(channel_id, prev1, 2, [40, 60], ChannelStatus::Active, 40);
    let d2 = sign_update_dual(
        &u2,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &DisputeEvidenceV0 {
            channel_id,
            claimed_update: d2,
            logical_time_raised: 45,
        },
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        45,
    )
    .unwrap();
    let orphan = make_update(
        channel_id,
        [1u8; 32],
        99,
        [10, 90],
        ChannelStatus::Active,
        99,
    );
    let d_orphan = sign_update_dual(
        &orphan,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let (ch, state) = resolve_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &[d_orphan],
        &pair.b.identity.derived_agent_id(),
        &pair.grant_b,
        200,
    )
    .unwrap();
    assert_eq!(ch.status, ChannelStatus::Finalized);
    assert_eq!(state.sequence, 2);
    assert_eq!(state.balances, [40, 60]);
}

#[test]
fn p1_r02_raise_rejects_skipped_sequence_evidence() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let skip = make_update(channel_id, prev, 3, [50, 50], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &skip,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let err = raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &DisputeEvidenceV0 {
            channel_id,
            claimed_update: dual,
            logical_time_raised: 35,
        },
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        35,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::SequenceSkip | Error::StateCommitmentMismatch | Error::InvalidDisputeEvidence
    ));
}

#[test]
fn p1_r03_finalize_requires_close_capability() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let state = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .latest_state
        .clone()
        .unwrap();
    let close = StateUpdateV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        channel_id,
        previous_state_commitment: prev,
        new_state: ChannelStateV0 {
            status_hint: ChannelStatus::Closing,
            ..state
        },
        sequence: 0,
        logical_time: 50,
    };
    let dual = sign_update_dual(
        &close,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_CLOSE,
    )
    .unwrap();
    begin_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        50,
    )
    .unwrap();
    let open_only = direct_capability(&pair.a, vec!["channel.open".into()], Some(10));
    let bad = grant_and_store(&pair.a, &pair.registry, &mut pair.caps, &open_only);
    let err = finalize_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &bad,
        &pair.grant_b,
        150,
    )
    .unwrap_err();
    assert_eq!(err, Error::UntrustedTerminalOperation);
}

#[test]
fn p1_r04_finalize_before_dispute_window_rejected() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let state = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .latest_state
        .clone()
        .unwrap();
    let close = StateUpdateV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        channel_id,
        previous_state_commitment: prev,
        new_state: ChannelStateV0 {
            status_hint: ChannelStatus::Closing,
            ..state
        },
        sequence: 0,
        logical_time: 50,
    };
    let dual = sign_update_dual(
        &close,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_CLOSE,
    )
    .unwrap();
    begin_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        50,
    )
    .unwrap();
    let err = finalize_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &pair.grant_a,
        &pair.grant_b,
        100,
    )
    .unwrap_err();
    assert_eq!(err, Error::DisputeWindowOpen);
}

#[test]
fn p1_r05_resolve_requires_dispute_capability() {
    let mut pair = setup_pair();
    let channel_id = open_and_activate(&mut pair);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [55, 45], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &DisputeEvidenceV0 {
            channel_id,
            claimed_update: dual.clone(),
            logical_time_raised: 35,
        },
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        35,
    )
    .unwrap();
    let open_only = direct_capability(&pair.b, vec!["channel.open".into()], Some(10));
    let bad = grant_and_store(&pair.b, &pair.registry, &mut pair.caps, &open_only);
    let err = resolve_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &[dual],
        &pair.b.identity.derived_agent_id(),
        &bad,
        200,
    )
    .unwrap_err();
    assert_eq!(err, Error::UntrustedTerminalOperation);
}
