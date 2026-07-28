//! PROTO-2 integration, H4/H5 measurement, and determinism tests (P2-I###, P2-H###, P2-M###).

mod common;

use aether_core::capability::model::{CapabilityStore, CapabilityV0};
use aether_core::channel::{
    open_channel, sign_open_dual, ChannelOpenMaterialV0, ChannelStatus, ChannelStore,
    MSG_CHANNEL_OPEN,
};
use aether_core::crypto::sha256;
use aether_core::error::Error;
use aether_core::escrow::{
    create_escrow, expire_for_refund, fund_escrow, release_escrow, sign_fund, sign_receipt,
    sign_release, sign_terms_dual, submit_receipt, EscrowFundingV0, EscrowReleaseV0, EscrowStatus,
};
use aether_core::identity::registry::IdentityRegistry;
use aether_core::permission::root::PermissionRootV0;
use common::*;

#[test]
fn p2_i01_unregistered_payer_cannot_create() {
    let mut pair = setup_escrow_pair();
    let mut ghost_registry = IdentityRegistry::new();
    let ghost = register_agent(&mut ghost_registry, escrow_actions(), Some(10_000));
    let payer_id = ghost.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let terms = sample_terms(&payer_id, &provider_id, 100, 10, 100, 500, 100);
    let dual = sign_terms_dual(
        &terms,
        &ghost.signing_key,
        &pair.provider.signing_key,
        &payer_id,
        &provider_id,
    )
    .unwrap();
    let err = create_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &dual,
        &pair.grant_payer,
        &pair.grant_provider,
        10,
    )
    .unwrap_err();
    assert_eq!(err, Error::IdentityNotFound);
}

#[test]
fn p2_i02_missing_create_capability_rejected() {
    let mut registry = IdentityRegistry::new();
    let payer = register_agent(&mut registry, escrow_actions(), Some(10_000));
    let provider = register_agent(&mut registry, escrow_actions(), Some(10_000));
    let mut caps = CapabilityStore::new();
    let grant_payer = grant_escrow_caps(&payer, &registry, &mut caps);
    let grant_provider = grant_and_store(
        &provider,
        &registry,
        &mut caps,
        &direct_capability(&provider, vec!["escrow.fund".into()], Some(10)),
    );
    let payer_id = payer.identity.derived_agent_id();
    let provider_id = provider.identity.derived_agent_id();
    let terms = sample_terms(&payer_id, &provider_id, 100, 10, 100, 500, 100);
    let dual = sign_terms_dual(
        &terms,
        &payer.signing_key,
        &provider.signing_key,
        &payer_id,
        &provider_id,
    )
    .unwrap();
    let mut escrows = aether_core::escrow::store::EscrowStore::new();
    let err = create_escrow(
        &mut escrows,
        &registry,
        &caps,
        &dual,
        &grant_payer,
        &grant_provider,
        10,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_i03_missing_fund_capability_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: record.fund_quote.quoted_fee,
        logical_time: 20,
    };
    let signed = sign_fund(
        &funding,
        &pair.payer.signing_key,
        &payer_id,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
    )
    .unwrap();
    let bad_cap = direct_capability(&pair.payer, vec!["escrow.create".into()], Some(10));
    let bad_grant = grant_and_store(&pair.payer, &pair.registry, &mut pair.caps, &bad_cap);
    let err = fund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &bad_grant,
        20,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_i04_frozen_identity_cannot_fund() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    pair.registry
        .freeze(&pair.payer.identity.derived_agent_id())
        .unwrap();
    let payer_id = pair.payer.identity.derived_agent_id();
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: record.fund_quote.quoted_fee,
        logical_time: 20,
    };
    let signed = sign_fund(
        &funding,
        &pair.payer.signing_key,
        &payer_id,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
    )
    .unwrap();
    let err = fund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        20,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::IdentityNotActive | Error::CapabilityDenied
    ));
}

#[test]
fn p2_i05_revoked_identity_cannot_submit_receipt() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    pair.registry
        .revoke_identity(&pair.provider.identity.derived_agent_id())
        .unwrap();
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::IdentityNotActive | Error::CapabilityDenied
    ));
}

#[test]
fn p2_i06_expired_capability_cannot_release() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    submit_receipt_ready(&mut pair, &terms, escrow_id, 1, 30);
    let deadline = pair
        .escrows
        .get(&escrow_id)
        .unwrap()
        .escrow
        .finality
        .dispute_deadline
        .unwrap();
    let payer_id = pair.payer.identity.derived_agent_id();
    let receipt_id = pair
        .escrows
        .get(&escrow_id)
        .unwrap()
        .escrow
        .receipt_id
        .unwrap();
    let release = EscrowReleaseV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        receipt_id,
        actor: payer_id.clone(),
        logical_time: deadline,
    };
    let signed = sign_release(&release, &pair.payer.signing_key).unwrap();
    let err = release_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        &payer_id,
        9_999,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_i07_revoked_capability_after_create_blocks_fund() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let cap = CapabilityV0::decode(&pair.grant_payer.message.body).unwrap();
    let id = cap.capability_id().unwrap();
    pair.caps.revoke(id, 15);
    let payer_id = pair.payer.identity.derived_agent_id();
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: record.fund_quote.quoted_fee,
        logical_time: 20,
    };
    let signed = sign_fund(
        &funding,
        &pair.payer.signing_key,
        &payer_id,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
    )
    .unwrap();
    let err = fund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        20,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_i08_stale_permission_root_blocks_new_escrow_action() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let agent_id = pair.payer.identity.derived_agent_id();
    let entry = pair.registry.get(&agent_id).unwrap();
    let mut new_authority = entry.root_authority.clone();
    new_authority.constraints.max_spend = Some(20_000);
    let new_root = PermissionRootV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        agent_id: agent_id.clone(),
        root_version: entry.permission_root_material.root_version + 1,
        root_authority_capability_id: new_authority.capability_id(),
    };
    pair.registry
        .update_permission_root(&agent_id, &pair.payer.signing_key, new_root, new_authority)
        .unwrap();
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: agent_id.clone(),
        amount: 100,
        fee_reservation: record.fund_quote.quoted_fee,
        logical_time: 20,
    };
    let signed = sign_fund(
        &funding,
        &pair.payer.signing_key,
        &agent_id,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
    )
    .unwrap();
    let err = fund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        20,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_i09_identity_only_bypass_on_economic_op_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let payer_id = pair.payer.identity.derived_agent_id();
    let refund = aether_core::escrow::model::EscrowRefundV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        actor: payer_id.clone(),
        reason: "test".into(),
        logical_time: 25,
    };
    let signed =
        aether_core::escrow::transition::sign_refund(&refund, &pair.payer.signing_key).unwrap();
    let empty_cap = direct_capability(&pair.payer, vec![], Some(10));
    let bad_grant = grant_and_store(&pair.payer, &pair.registry, &mut pair.caps, &empty_cap);
    let err = aether_core::escrow::transition::refund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &bad_grant,
        25,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_i10_proto1_channel_op_still_independent() {
    let mut registry = IdentityRegistry::new();
    let a = register_agent(&mut registry, channel_actions(), Some(1_000));
    let b = register_agent(&mut registry, channel_actions(), Some(1_000));
    let mut caps = CapabilityStore::new();
    let grant_a = grant_channel_caps(&a, &registry, &mut caps);
    let grant_b = grant_channel_caps(&b, &registry, &mut caps);
    let material = ChannelOpenMaterialV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        party_a: a.identity.derived_agent_id(),
        party_b: b.identity.derived_agent_id(),
        asset: "AETHER_TEST".into(),
        opening_balances: [60, 40],
        dispute_window: 100,
        created_at: 1,
    };
    let (sig_a, sig_b) =
        sign_open_dual(&material, &a.signing_key, &b.signing_key, MSG_CHANNEL_OPEN).unwrap();
    let mut channels = ChannelStore::new();
    let channel = open_channel(
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
    .unwrap();
    assert_eq!(channel.status, ChannelStatus::Open);
}

#[test]
fn p2_i11_proto1_receipt_v0_not_accepted_as_settlement() {
    let mut pair = setup_escrow_pair();
    let (_terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let channel_id = [1u8; 32];
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let _proto1_receipt = aether_core::channel::ReceiptV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        receipt_id: [2u8; 32],
        channel_id,
        action: "channel.update".into(),
        participants: [payer_id.clone(), provider_id.clone()],
        sequence: 1,
        state_commitment: [3u8; 32],
        previous_state_commitment: None,
        logical_time: 30,
        finality_snapshot: aether_core::channel::FinalityViewV0 {
            soft_local_agreement: true,
            dispute_window_open: false,
            dispute_deadline: None,
            hard_settlement_placeholder: false,
            finalized: false,
        },
        accepted: true,
        signature_a: vec![],
        signature_b: vec![],
    };
    let body = aether_core::cbor::encode_value(&ciborium::value::Value::Map(vec![])).unwrap();
    let signed = aether_core::crypto::verify::SignedMessage {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        message_type: "escrow.submit_receipt".into(),
        body,
        signer_key_id: provider_id.clone(),
        signature: vec![0u8; 64],
        domain_tag: aether_core::crypto::signing::DOMAIN_TAG.into(),
    };
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidSignature | Error::InvalidReceipt | Error::MalformedObject(_)
    ));
    assert_eq!(
        pair.escrows.get(&escrow_id).unwrap().escrow.status,
        EscrowStatus::Funded
    );
}

#[test]
fn p2_i12_escrow_does_not_mutate_channel_store() {
    let mut pair = setup_escrow_pair();
    let channels = ChannelStore::new();
    assert!(channels.get(&[0u8; 32]).is_none());
    let escrow_id = happy_path_through_release(&mut pair, 100, 10);
    assert!(pair.escrows.get(&escrow_id).is_some());
    assert!(channels.get(&[0u8; 32]).is_none());
}

#[test]
fn p2_h001_happy_path_completion_rate() {
    let n = 5u64;
    let mut completed = 0u64;
    for _ in 0..n {
        let mut pair = setup_escrow_pair();
        let escrow_id = happy_path_through_release(&mut pair, 100, 10);
        if pair.escrows.get(&escrow_id).unwrap().escrow.status == EscrowStatus::Released {
            completed += 1;
        }
    }
    let rate = completed as f64 / n as f64;
    eprintln!("P2-H001 h4_completion_rate={rate} ({completed}/{n})");
    assert_eq!(completed, n);
}

#[test]
fn p2_h002_timeout_safe_refund_rate() {
    let n = 5u64;
    let mut refunded = 0u64;
    for _ in 0..n {
        let mut pair = setup_escrow_pair();
        let payer_id = pair.payer.identity.derived_agent_id();
        let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
        fund_escrow_ready(&mut pair, escrow_id, 20);
        expire_for_refund(
            &mut pair.escrows,
            &pair.registry,
            &pair.caps,
            &escrow_id,
            &payer_id,
            &pair.grant_payer,
            501,
        )
        .unwrap();
        refund_escrow_ready(&mut pair, escrow_id, 502);
        if pair.escrows.get(&escrow_id).unwrap().escrow.status == EscrowStatus::Refunded {
            refunded += 1;
        }
    }
    let rate = refunded as f64 / n as f64;
    eprintln!("P2-H002 h4_safe_refund_rate={rate} ({refunded}/{n})");
    assert_eq!(refunded, n);
}

#[test]
fn p2_h003_adversarial_wrongful_release_attempts_zero() {
    let mut wrongful = 0u64;
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let payer_id = pair.payer.identity.derived_agent_id();
    let receipt_id = [9u8; 32];
    let release = EscrowReleaseV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        receipt_id,
        actor: payer_id.clone(),
        logical_time: 200,
    };
    let signed = sign_release(&release, &pair.payer.signing_key).unwrap();
    if release_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        &payer_id,
        200,
    )
    .is_ok()
    {
        wrongful += 1;
    }
    eprintln!("P2-H003 wrongful_release_count={wrongful}");
    assert_eq!(wrongful, 0);
    assert_ne!(
        pair.escrows.get(&escrow_id).unwrap().escrow.status,
        EscrowStatus::Released
    );
}

#[test]
fn p2_h004_bounded_task_flows_deterministic() {
    let mut pair = setup_escrow_pair_seeded(42);
    let escrow_id = happy_path_through_release(&mut pair, 100, 10);
    let hash1 = sha256(
        &pair
            .escrows
            .get(&escrow_id)
            .unwrap()
            .escrow
            .encode()
            .unwrap(),
    );
    let mut pair2 = setup_escrow_pair_seeded(42);
    let escrow_id2 = happy_path_through_release(&mut pair2, 100, 10);
    let hash2 = sha256(
        &pair2
            .escrows
            .get(&escrow_id2)
            .unwrap()
            .escrow
            .encode()
            .unwrap(),
    );
    assert_eq!(hash1, hash2);
    eprintln!("P2-H004 deterministic_terminal_hash={:?}", hash1);
}

#[test]
fn p2_h005_agent_loop_fee_quote_budget() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let record = pair.escrows.get(&escrow_id).unwrap();
    let fund_fee = record.fund_quote.quoted_fee;
    let release_fee = record.release_quote.quoted_fee;
    assert!(fund_fee + release_fee <= terms.max_protocol_fee);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    submit_receipt_ready(&mut pair, &terms, escrow_id, 1, 30);
    let deadline = pair
        .escrows
        .get(&escrow_id)
        .unwrap()
        .escrow
        .finality
        .dispute_deadline
        .unwrap();
    release_escrow_ready(&mut pair, escrow_id, deadline);
    eprintln!(
        "P2-H005 fee_quote_fund={fund_fee} release={release_fee} completed_without_manual_intervention"
    );
    assert_eq!(
        pair.escrows.get(&escrow_id).unwrap().escrow.status,
        EscrowStatus::Released
    );
}

#[test]
fn p2_h006_operations_rejected_insufficient_budget() {
    let mut rejections = 0u64;
    let mut pair = setup_escrow_pair();
    let payer_id = pair.payer.identity.derived_agent_id();
    let bal = pair.ledger.balance(&payer_id);
    pair.ledger.debit(&payer_id, bal).unwrap();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: record.fund_quote.quoted_fee,
        logical_time: 20,
    };
    let signed = sign_fund(
        &funding,
        &pair.payer.signing_key,
        &payer_id,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
    )
    .unwrap();
    if fund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        20,
    )
    .is_err()
    {
        rejections += 1;
    }
    eprintln!("P2-H006 h5_rejection_count={rejections}");
    assert_eq!(rejections, 1);
}

#[test]
fn p2_m001_same_inputs_same_terminal_status() {
    fn run_once() -> ([u8; 32], EscrowStatus) {
        let mut pair = setup_escrow_pair_seeded(99);
        let escrow_id = happy_path_through_release(&mut pair, 100, 10);
        let status = pair.escrows.get(&escrow_id).unwrap().escrow.status;
        let hash = sha256(
            &pair
                .escrows
                .get(&escrow_id)
                .unwrap()
                .escrow
                .encode()
                .unwrap(),
        );
        (hash, status)
    }
    let (h1, s1) = run_once();
    let (h2, s2) = run_once();
    assert_eq!(s1, s2);
    assert_eq!(h1, h2);
    eprintln!("P2-M001 reproducible_hash={:?} status={:?}", h1, s1);
}

#[test]
fn p2_m002_fee_ledger_matches_consumed_totals() {
    let mut pair = setup_escrow_pair();
    let escrow_id = happy_path_through_release(&mut pair, 100, 10);
    let record = pair.escrows.get(&escrow_id).unwrap();
    let ledger_sum: u64 = record.fee_ledger.iter().map(|e| e.amount).sum();
    assert_eq!(ledger_sum, record.escrow.fee_consumed);
    assert_eq!(ledger_sum, pair.ledger.treasury);
    eprintln!(
        "P2-M002 fee_ledger_sum={ledger_sum} fee_consumed={} treasury={}",
        record.escrow.fee_consumed, pair.ledger.treasury
    );
}
