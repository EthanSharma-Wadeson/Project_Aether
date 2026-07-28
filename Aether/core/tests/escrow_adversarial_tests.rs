//! PROTO-2 escrow adversarial acceptance tests (P2-A###).

mod common;

use aether_core::capability::model::CapabilityStore;
use aether_core::error::Error;
use aether_core::escrow::{
    cancel_escrow, create_escrow, fund_escrow, raise_dispute, release_escrow, resolve_dispute,
    sign_cancel_dual, sign_fund, sign_receipt, sign_release, sign_terms_dual, submit_receipt,
    DisputeEvidenceV0, EscrowCancelV0, EscrowFundingV0, EscrowReleaseV0, EscrowStatus,
    SettlementEvidenceV0,
};
use aether_core::identity::registry::IdentityRegistry;
use common::*;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

#[test]
fn p2_a01_unauthorised_escrow_create_rejected() {
    let mut pair = setup_escrow_pair();
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let terms = sample_terms(&payer_id, &provider_id, 100, 10, 100, 500, 100);
    let dual = sign_terms_dual(
        &terms,
        &pair.payer.signing_key,
        &pair.provider.signing_key,
        &payer_id,
        &provider_id,
    )
    .unwrap();
    let bad_cap = direct_capability(&pair.payer, vec!["escrow.fund".into()], Some(10));
    let bad_grant = grant_and_store(&pair.payer, &pair.registry, &mut pair.caps, &bad_cap);
    let err = create_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &dual,
        &bad_grant,
        &pair.grant_provider,
        10,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_a02_unauthorised_fund_rejected() {
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
fn p2_a03_unauthorised_release_rejected() {
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
    let bad_cap = direct_capability(&pair.payer, vec!["escrow.fund".into()], Some(10));
    let bad_grant = grant_and_store(&pair.payer, &pair.registry, &mut pair.caps, &bad_cap);
    let err = release_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &bad_grant,
        &payer_id,
        deadline,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_a04_unauthorised_refund_rejected() {
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
    let bad_cap = direct_capability(&pair.payer, vec!["escrow.create".into()], Some(10));
    let bad_grant = grant_and_store(&pair.payer, &pair.registry, &mut pair.caps, &bad_cap);
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
fn p2_a05_unauthorised_dispute_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let evidence = DisputeEvidenceV0 {
        escrow_id,
        evidence: SettlementEvidenceV0 {
            escrow_id,
            receipt: signed,
            logical_time_presented: 35,
        },
        logical_time_raised: 35,
    };
    let bad_cap = direct_capability(&pair.payer, vec!["escrow.fund".into()], Some(10));
    let bad_grant = grant_and_store(&pair.payer, &pair.registry, &mut pair.caps, &bad_cap);
    let err = raise_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &evidence,
        &pair.payer.identity.derived_agent_id(),
        &bad_grant,
        35,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_a06_unauthorised_resolve_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let evidence = DisputeEvidenceV0 {
        escrow_id,
        evidence: SettlementEvidenceV0 {
            escrow_id,
            receipt: signed,
            logical_time_presented: 35,
        },
        logical_time_raised: 35,
    };
    raise_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &evidence,
        &pair.payer.identity.derived_agent_id(),
        &pair.grant_payer,
        35,
    )
    .unwrap();
    let bad_cap = direct_capability(&pair.provider, vec!["escrow.fund".into()], Some(10));
    let bad_grant = grant_and_store(&pair.provider, &pair.registry, &mut pair.caps, &bad_cap);
    let err = resolve_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &escrow_id,
        None,
        &provider_id,
        &bad_grant,
        200,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_a07_third_party_receipt_submission_rejected() {
    let mut pair = setup_escrow_pair();
    let mut registry = IdentityRegistry::new();
    let outsider = register_agent(&mut registry, escrow_actions(), Some(10_000));
    let mut caps = CapabilityStore::new();
    let outsider_grant = grant_escrow_caps(&outsider, &registry, &mut caps);
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let outsider_id = outsider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &outsider.signing_key, &outsider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &outsider_grant,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidSignature | Error::ParticipantMismatch | Error::IdentityNotFound
    ));
}

#[test]
fn p2_a08_conflicting_receipts_higher_nonce_wins() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    submit_receipt_ready(&mut pair, &terms, escrow_id, 2, 30);
    let receipt_low = sample_receipt(&terms, escrow_id, 1, 31);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed_low = sign_receipt(&receipt_low, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed_low,
        &pair.grant_provider,
        31,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
    let record = pair.escrows.get(&escrow_id).unwrap();
    assert_eq!(record.last_receipt_nonce, 2);
    assert_eq!(record.bound_receipt.as_ref().unwrap().receipt_nonce, 2);
}

#[test]
fn p2_a09_dispute_from_cancelled_status_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let cancel = EscrowCancelV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        logical_time: 15,
    };
    let dual = sign_cancel_dual(
        &cancel,
        &pair.payer.signing_key,
        &pair.provider.signing_key,
        &payer_id,
        &provider_id,
    )
    .unwrap();
    cancel_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &dual,
        &pair.grant_payer,
        &pair.grant_provider,
        15,
    )
    .unwrap();
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let evidence = DisputeEvidenceV0 {
        escrow_id,
        evidence: SettlementEvidenceV0 {
            escrow_id,
            receipt: signed,
            logical_time_presented: 35,
        },
        logical_time_raised: 35,
    };
    let err = raise_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &evidence,
        &payer_id,
        &pair.grant_payer,
        35,
    )
    .unwrap_err();
    assert_eq!(err, Error::UnauthorizedTransition);
}

#[test]
fn p2_a10_invalid_dispute_evidence_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let mut csprng = OsRng;
    let wrong_key = SigningKey::generate(&mut csprng);
    let fake_receipt = sample_receipt(
        &sample_terms(
            &pair.payer.identity.derived_agent_id(),
            &pair.provider.identity.derived_agent_id(),
            100,
            10,
            100,
            500,
            100,
        ),
        [8u8; 32],
        1,
        30,
    );
    let provider_id = pair.provider.identity.derived_agent_id();
    let mut signed = sign_receipt(&fake_receipt, &wrong_key, &provider_id).unwrap();
    signed.signature = vec![1u8; 64];
    let evidence = DisputeEvidenceV0 {
        escrow_id,
        evidence: SettlementEvidenceV0 {
            escrow_id,
            receipt: signed,
            logical_time_presented: 35,
        },
        logical_time_raised: 35,
    };
    let err = raise_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &evidence,
        &pair.payer.identity.derived_agent_id(),
        &pair.grant_payer,
        35,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowEvidence);
    assert_eq!(
        pair.escrows.get(&escrow_id).unwrap().escrow.status,
        EscrowStatus::Funded
    );
}

#[test]
fn p2_a11_resolver_without_resolve_capability_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let evidence = DisputeEvidenceV0 {
        escrow_id,
        evidence: SettlementEvidenceV0 {
            escrow_id,
            receipt: signed,
            logical_time_presented: 35,
        },
        logical_time_raised: 35,
    };
    raise_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &evidence,
        &pair.payer.identity.derived_agent_id(),
        &pair.grant_payer,
        35,
    )
    .unwrap();
    let bad_cap = direct_capability(&pair.provider, vec!["escrow.dispute".into()], Some(10));
    let bad_grant = grant_and_store(&pair.provider, &pair.registry, &mut pair.caps, &bad_cap);
    let err = resolve_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &escrow_id,
        None,
        &provider_id,
        &bad_grant,
        200,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_a12_identity_only_release_attempt_rejected() {
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
    let empty_cap = direct_capability(&pair.payer, vec![], Some(10));
    let bad_grant = grant_and_store(&pair.payer, &pair.registry, &mut pair.caps, &empty_cap);
    let err = release_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &bad_grant,
        &payer_id,
        deadline,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_a13_valid_receipt_sig_without_capability_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let bad_cap = direct_capability(&pair.provider, vec!["escrow.release".into()], Some(10));
    let bad_grant = grant_and_store(&pair.provider, &pair.registry, &mut pair.caps, &bad_cap);
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &bad_grant,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}

#[test]
fn p2_a14_escrow_id_guessing_without_authority_rejected() {
    let mut pair = setup_escrow_pair();
    let fake_id = [0xab; 32];
    let payer_id = pair.payer.identity.derived_agent_id();
    let funding = EscrowFundingV0 {
        escrow_id: fake_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: 5,
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
    assert_eq!(err, Error::EscrowNotFound);
}

#[test]
fn p2_a15_release_with_superseded_receipt_nonce_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt1 = sample_receipt(&terms, escrow_id, 1, 30);
    submit_receipt_ready(&mut pair, &terms, escrow_id, 2, 31);
    let deadline = pair
        .escrows
        .get(&escrow_id)
        .unwrap()
        .escrow
        .finality
        .dispute_deadline
        .unwrap();
    let payer_id = pair.payer.identity.derived_agent_id();
    let release = EscrowReleaseV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        receipt_id: receipt1.receipt_id,
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
        deadline,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidReceipt);
}

#[test]
fn p2_a16_fee_limit_exceeded_via_manipulated_quote_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: record.fund_quote.quoted_fee + 1,
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
    assert_eq!(err, Error::FeeBudgetExceeded);
}
