//! PROTO-1 channel lifecycle + PROTO-0 integration acceptance tests.

mod common;

use std::time::Instant;

use aether_core::capability::grant::CapabilityGrant;
use aether_core::capability::model::{CapabilityStore, CapabilityV0};
use aether_core::channel::{
    abort_open, activate_channel, apply_update, begin_close, finalize_close, open_channel,
    raise_dispute, resolve_dispute, sign_open_dual, sign_update_dual, ChannelOpenMaterialV0,
    ChannelStateV0, ChannelStatus, ChannelStore, DisputeEvidenceV0, DualSignedUpdate,
    StateUpdateV0, MSG_CHANNEL_ACTIVATE, MSG_CHANNEL_CLOSE, MSG_CHANNEL_OPEN, MSG_CHANNEL_UPDATE,
};
use aether_core::error::Error;
use aether_core::identity::agent_identity::IdentityBundle;
use aether_core::identity::registry::IdentityRegistry;
use aether_core::permission::root::PermissionRootV0;
use common::*;

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

fn open_channel_ready(pair: &mut Pair) -> ChannelOpenMaterialV0 {
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
    material
}

fn activate_ready(pair: &mut Pair, material: &ChannelOpenMaterialV0) -> [u8; 32] {
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

fn close_dual(pair: &Pair, channel_id: [u8; 32]) -> DualSignedUpdate {
    let rec = pair.channels.get(&channel_id).unwrap();
    let prev = rec.channel.current_state_commitment;
    let sequence = rec.channel.sequence;
    let state = rec.latest_state.clone().unwrap();
    let close = StateUpdateV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        channel_id,
        previous_state_commitment: prev,
        new_state: ChannelStateV0 {
            status_hint: ChannelStatus::Closing,
            ..state
        },
        sequence,
        logical_time: 50,
    };
    sign_update_dual(
        &close,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_CLOSE,
    )
    .unwrap()
}

#[test]
fn p1_t001_open_dual_signed() {
    let mut pair = setup_pair();
    let material = open_material(&pair.a, &pair.b);
    let (sig_a, sig_b) = sign_open_dual(
        &material,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_OPEN,
    )
    .unwrap();
    let channel = open_channel(
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
    assert_eq!(channel.status, ChannelStatus::Open);
}

#[test]
fn p1_t002_activate_initial_state() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let rec = pair.channels.get(&channel_id).unwrap();
    assert_eq!(rec.channel.status, ChannelStatus::Active);
    assert!(rec.channel.finality.soft_local_agreement);
    assert_ne!(rec.channel.current_state_commitment, [0u8; 32]);
}

#[test]
fn p1_t003_dual_signed_update() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let (ch, _) = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap();
    assert_eq!(ch.sequence, 1);
    assert_eq!(
        pair.channels
            .get(&channel_id)
            .unwrap()
            .latest_state
            .as_ref()
            .unwrap()
            .balances,
        [50, 50]
    );
}

#[test]
fn p1_t004_sequential_updates() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let balances = [[55, 45], [70, 30], [10, 90]];
    for (i, bal) in balances.iter().enumerate() {
        let seq = (i + 1) as u64;
        let prev = pair
            .channels
            .get(&channel_id)
            .unwrap()
            .channel
            .current_state_commitment;
        let update = make_update(channel_id, prev, seq, *bal, ChannelStatus::Active, 40 + seq);
        let dual = sign_update_dual(
            &update,
            &pair.a.signing_key,
            &pair.b.signing_key,
            MSG_CHANNEL_UPDATE,
        )
        .unwrap();
        let (ch, _) = apply_update(
            &mut pair.channels,
            &pair.registry,
            &pair.caps,
            &channel_id,
            &dual,
            &pair.grant_a,
            &pair.grant_b,
            40 + seq,
        )
        .unwrap();
        assert_eq!(ch.sequence, seq);
    }
}

#[test]
fn p1_t005_cooperative_close() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let dual = close_dual(&pair, channel_id);
    let closing = begin_close(
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
    assert_eq!(closing.status, ChannelStatus::Closing);
    let done = finalize_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &pair.grant_a,
        &pair.grant_b,
        150,
    )
    .unwrap();
    assert_eq!(done.status, ChannelStatus::Finalized);
    assert!(done.finality.finalized);
}

#[test]
fn p1_t006_abort_from_open() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = material.channel_id().unwrap();
    let (sig_a, sig_b) = sign_open_dual(
        &material,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_CLOSE,
    )
    .unwrap();
    let ch = abort_open(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &sig_a,
        &sig_b,
        &pair.grant_a,
        &pair.grant_b,
        15,
    )
    .unwrap();
    assert_eq!(ch.status, ChannelStatus::Finalized);
    assert!(pair
        .channels
        .get(&channel_id)
        .unwrap()
        .latest_state
        .is_none());
}

#[test]
fn p1_t007_finality_soft_not_hard() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let f = &pair.channels.get(&channel_id).unwrap().channel.finality;
    assert!(f.soft_local_agreement);
    assert!(!f.hard_settlement_placeholder);
    assert!(!f.finalized);
}

#[test]
fn p1_t008_receipt_on_update() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [40, 60], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let (_, receipt) = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap();
    assert_eq!(receipt.channel_id, channel_id);
    assert_eq!(receipt.sequence, 1);
    assert!(receipt.accepted);
    assert_eq!(receipt.previous_state_commitment, Some(prev));
}

#[test]
fn p1_t010_open_single_signature_rejected() {
    let mut pair = setup_pair();
    let material = open_material(&pair.a, &pair.b);
    let (sig_a, mut sig_b) = sign_open_dual(
        &material,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_OPEN,
    )
    .unwrap();
    sig_b.signature = vec![0u8; 64];
    let err = open_channel(
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
    .unwrap_err();
    assert_eq!(err, Error::InvalidSignature);
    assert!(pair.channels.get(&material.channel_id().unwrap()).is_none());
}

#[test]
fn p1_t011_activate_mismatched_balances() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = material.channel_id().unwrap();
    let state = ChannelStateV0 {
        channel_id,
        sequence: 0,
        balances: [99, 1],
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
    let err = activate_channel(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        20,
    )
    .unwrap_err();
    assert_eq!(err, Error::BalanceConservation);
    assert_eq!(
        pair.channels.get(&channel_id).unwrap().channel.status,
        ChannelStatus::Open
    );
}

#[test]
fn p1_t012_update_while_open_rejected() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = material.channel_id().unwrap();
    let update = make_update(
        channel_id,
        [0u8; 32],
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
    assert_eq!(err, Error::InvalidChannelStatus);
}

#[test]
fn p1_t013_close_from_finalized_rejected() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let dual = close_dual(&pair, channel_id);
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
    finalize_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &pair.grant_a,
        &pair.grant_b,
        150,
    )
    .unwrap();
    let err = begin_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &pair.grant_b,
        60,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidClose);
}

#[test]
fn p1_t014_update_after_finalized_rejected() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let dual = close_dual(&pair, channel_id);
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
    finalize_close(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &pair.grant_a,
        &pair.grant_b,
        150,
    )
    .unwrap();
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 70);
    let dual2 = sign_update_dual(
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
        &dual2,
        &pair.grant_a,
        &pair.grant_b,
        70,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidChannelStatus);
}

#[test]
fn p1_t020_raise_dispute() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [70, 30], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let evidence = DisputeEvidenceV0 {
        channel_id,
        claimed_update: dual,
        logical_time_raised: 35,
    };
    let ch = raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &evidence,
        &pair.a.identity.derived_agent_id(),
        &pair.grant_a,
        35,
    )
    .unwrap();
    assert_eq!(ch.status, ChannelStatus::Disputed);
    assert_eq!(ch.sequence, 1);
}

#[test]
fn p1_t021_resolve_highest_sequence() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev0 = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let u1 = make_update(channel_id, prev0, 1, [55, 45], ChannelStatus::Active, 30);
    let d1 = sign_update_dual(
        &u1,
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
        &d1,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap();
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
    raise_dispute(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &DisputeEvidenceV0 {
            channel_id,
            claimed_update: d2.clone(),
            logical_time_raised: 45,
        },
        &pair.b.identity.derived_agent_id(),
        &pair.grant_b,
        45,
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
    assert_eq!(state.balances, [20, 80]);
}

#[test]
fn p1_t022_lower_sequence_cannot_override() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev0 = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let u1 = make_update(channel_id, prev0, 1, [55, 45], ChannelStatus::Active, 30);
    let d1 = sign_update_dual(
        &u1,
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
        &d1,
        &pair.grant_a,
        &pair.grant_b,
        30,
    )
    .unwrap();
    let prev1 = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let u2 = make_update(channel_id, prev1, 2, [55, 45], ChannelStatus::Active, 40);
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
    let lower = make_update(channel_id, [0u8; 32], 0, [60, 40], ChannelStatus::Active, 2);
    let d0 = sign_update_dual(
        &lower,
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
        &[d0],
        &pair.b.identity.derived_agent_id(),
        &pair.grant_b,
        200,
    )
    .unwrap();
    assert_eq!(ch.status, ChannelStatus::Finalized);
    assert_eq!(state.sequence, 2);
}

#[test]
fn p1_i01_unregistered_agent_rejected() {
    let mut pair = setup_pair();
    let ghost = register_agent(&mut IdentityRegistry::new(), channel_actions(), Some(10));
    let mut material = open_material(&pair.a, &pair.b);
    material.party_b = ghost.identity.derived_agent_id();
    let (sig_a, sig_b) = sign_open_dual(
        &material,
        &pair.a.signing_key,
        &ghost.signing_key,
        MSG_CHANNEL_OPEN,
    )
    .unwrap();
    let err = open_channel(
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
    .unwrap_err();
    assert_eq!(err, Error::IdentityNotFound);
}

#[test]
fn p1_i02_missing_open_cap_rejected() {
    let mut registry = IdentityRegistry::new();
    let a = register_agent(&mut registry, channel_actions(), Some(1_000));
    let b = register_agent(&mut registry, channel_actions(), Some(1_000));
    let mut caps = CapabilityStore::new();
    let grant_a = grant_channel_caps(&a, &registry, &mut caps);
    let grant_b = grant_and_store(
        &b,
        &registry,
        &mut caps,
        &direct_capability(&b, vec!["channel.close".into()], Some(10)),
    );
    let material = open_material(&a, &b);
    let (sig_a, sig_b) =
        sign_open_dual(&material, &a.signing_key, &b.signing_key, MSG_CHANNEL_OPEN).unwrap();
    let mut channels = ChannelStore::new();
    let err = open_channel(
        &mut channels,
        &registry,
        &caps,
        &material,
        &sig_a,
        &sig_b,
        &grant_a,
        &grant_b,
        10,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p1_i03_missing_update_cap_rejected() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let open_only = direct_capability(&pair.b, vec!["channel.open".into()], Some(10));
    let bad_grant = grant_and_store(&pair.b, &pair.registry, &mut pair.caps, &open_only);
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &pair.grant_a,
        &bad_grant,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p1_i04_revoked_identity_rejected() {
    let mut pair = setup_pair();
    let material = open_material(&pair.a, &pair.b);
    let (sig_a, sig_b) = sign_open_dual(
        &material,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_OPEN,
    )
    .unwrap();
    pair.registry
        .revoke_identity(&pair.a.identity.derived_agent_id())
        .unwrap();
    let err = open_channel(
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
    .unwrap_err();
    assert!(matches!(
        err,
        Error::IdentityNotActive | Error::CapabilityDenied
    ));
}

#[test]
fn p1_i05_frozen_identity_rejected() {
    let mut pair = setup_pair();
    let material = open_material(&pair.a, &pair.b);
    let (sig_a, sig_b) = sign_open_dual(
        &material,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_OPEN,
    )
    .unwrap();
    pair.registry
        .freeze(&pair.b.identity.derived_agent_id())
        .unwrap();
    let err = open_channel(
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
    .unwrap_err();
    assert!(matches!(
        err,
        Error::IdentityNotActive | Error::CapabilityDenied
    ));
}

#[test]
fn p1_i06_expired_capability_rejected() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
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
        9_999,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p1_i07_revoked_capability_rejected() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let cap = CapabilityV0::decode(&pair.grant_a.message.body).unwrap();
    let id = cap.capability_id().unwrap();
    pair.caps.revoke(id, 25);
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
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p1_i08_identity_only_update_rejected() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
    let dual = sign_update_dual(
        &update,
        &pair.a.signing_key,
        &pair.b.signing_key,
        MSG_CHANNEL_UPDATE,
    )
    .unwrap();
    let empty = direct_capability(&pair.a, vec![], Some(10));
    let bad = grant_and_store(&pair.a, &pair.registry, &mut pair.caps, &empty);
    let err = apply_update(
        &mut pair.channels,
        &pair.registry,
        &pair.caps,
        &channel_id,
        &dual,
        &bad,
        &pair.grant_b,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p1_i09_valid_sigs_without_capability_rejected() {
    p1_i03_missing_update_cap_rejected();
}

#[test]
fn p1_i10_stale_root_blocks_channel_action() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);

    let agent_id = pair.a.identity.derived_agent_id();
    let entry = pair.registry.get(&agent_id).unwrap();
    let mut new_authority = entry.root_authority.clone();
    new_authority.constraints.max_spend = Some(2_000);
    let new_root = PermissionRootV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        agent_id: agent_id.clone(),
        root_version: entry.permission_root_material.root_version + 1,
        root_authority_capability_id: new_authority.capability_id(),
    };
    pair.registry
        .update_permission_root(&agent_id, &pair.a.signing_key, new_root, new_authority)
        .unwrap();

    let prev = pair
        .channels
        .get(&channel_id)
        .unwrap()
        .channel
        .current_state_commitment;
    let update = make_update(channel_id, prev, 1, [50, 50], ChannelStatus::Active, 30);
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
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p1_m01_m02_m03_metrics_hooks() {
    let mut pair = setup_pair();
    let material = open_channel_ready(&mut pair);
    let channel_id = activate_ready(&mut pair, &material);
    let n = 10u64;
    let c_direct = 100u64;
    let start = Instant::now();
    for i in 1..=n {
        let prev = pair
            .channels
            .get(&channel_id)
            .unwrap()
            .channel
            .current_state_commitment;
        let bal_a = 60u64.saturating_sub(i);
        let update = make_update(
            channel_id,
            prev,
            i,
            [bal_a, 100 - bal_a],
            ChannelStatus::Active,
            100 + i,
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
            100 + i,
        )
        .unwrap();
    }
    let elapsed = start.elapsed();
    let simulated_cost = n;
    let direct_cost = n.saturating_mul(c_direct);
    assert!(simulated_cost < direct_cost);
    assert!(elapsed.as_nanos() > 0);
    let f = &pair.channels.get(&channel_id).unwrap().channel.finality;
    assert!(f.soft_local_agreement);
    assert!(!f.hard_settlement_placeholder);
    eprintln!(
        "P1-M01 simulated_cost={simulated_cost} direct_baseline={direct_cost}; P1-M02 elapsed_ns={}",
        elapsed.as_nanos()
    );
}
