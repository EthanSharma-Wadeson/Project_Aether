//! PROTO-2 escrow lifecycle acceptance tests (P2-T###).

mod common;

use aether_core::error::Error;
use aether_core::escrow::{
    cancel_escrow, create_escrow, expire_for_refund, fund_escrow, raise_dispute, refund_escrow,
    release_escrow, resolve_dispute, sign_cancel_dual, sign_fund, sign_refund, sign_release,
    sign_terms_dual, submit_receipt, DisputeEvidenceV0, EscrowCancelV0, EscrowFundingV0,
    EscrowRefundV0, EscrowReleaseV0, EscrowStatus, SettlementEvidenceV0,
};
use common::*;

#[test]
fn p2_t001_dual_signed_terms_create() {
    let mut pair = setup_escrow_pair();
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let terms = sample_terms(&payer_id, &provider_id, 100, 10, 100, 500, 100);
    let expected_id = terms.escrow_id().unwrap();
    let dual = sign_terms_dual(
        &terms,
        &pair.payer.signing_key,
        &pair.provider.signing_key,
        &payer_id,
        &provider_id,
    )
    .unwrap();
    let escrow = create_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &dual,
        &pair.grant_payer,
        &pair.grant_provider,
        10,
    )
    .unwrap();
    assert_eq!(escrow.status, EscrowStatus::Proposed);
    assert_eq!(escrow.escrow_id, expected_id);
}

#[test]
fn p2_t002_payer_funds_valid() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let balance_before = pair.ledger.balance(&payer_id);
    let escrow = fund_escrow_ready(&mut pair, escrow_id, 20);
    assert_eq!(escrow.status, EscrowStatus::Funded);
    assert_eq!(escrow.funded_amount, terms.principal_amount);
    assert_eq!(
        escrow.fee_reserved,
        pair.escrows.get(&escrow_id).unwrap().fund_quote.quoted_fee
    );
    let total_debit = terms.principal_amount + escrow.fee_reserved;
    assert_eq!(pair.ledger.balance(&payer_id), balance_before - total_debit);
}

#[test]
fn p2_t003_provider_submits_valid_receipt() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let escrow = submit_receipt_ready(&mut pair, &terms, escrow_id, 1, 30);
    assert_eq!(escrow.status, EscrowStatus::ReceiptAccepted);
    assert!(escrow.receipt_id.is_some());
    assert!(pair
        .escrows
        .get(&escrow_id)
        .unwrap()
        .bound_receipt
        .is_some());
}

#[test]
fn p2_t004_release_after_dispute_window() {
    let mut pair = setup_escrow_pair();
    let principal = 100u64;
    let max_fee = 10u64;
    let provider_id = pair.provider.identity.derived_agent_id();
    let balance_before = pair.ledger.balance(&provider_id);
    let escrow_id = happy_path_through_release(&mut pair, principal, max_fee);
    let escrow = pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    assert_eq!(escrow.status, EscrowStatus::Released);
    let release_fee = pair
        .escrows
        .get(&escrow_id)
        .unwrap()
        .release_quote
        .quoted_fee;
    let expected_credit = principal - release_fee;
    assert_eq!(
        pair.ledger.balance(&provider_id),
        balance_before + expected_credit
    );
    assert_eq!(pair.ledger.treasury, release_fee);
}

#[test]
fn p2_t005_cooperative_refund_before_receipt() {
    let mut pair = setup_escrow_pair();
    let payer_id = pair.payer.identity.derived_agent_id();
    let balance_before = pair.ledger.balance(&payer_id);
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let escrow = refund_escrow_ready(&mut pair, escrow_id, 25);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert!(pair.ledger.balance(&payer_id) >= balance_before);
}

#[test]
fn p2_t006_timeout_without_receipt() {
    let mut pair = setup_escrow_pair();
    let payer_id = pair.payer.identity.derived_agent_id();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let expired = expire_for_refund(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &payer_id,
        &pair.grant_payer,
        501,
    )
    .unwrap();
    assert_eq!(expired.status, EscrowStatus::Expired);
    let refunded = refund_escrow_ready(&mut pair, escrow_id, 502);
    assert_eq!(refunded.status, EscrowStatus::Refunded);
}

#[test]
fn p2_t007_cancel_unfunded_dual_signed() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
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
    let escrow = cancel_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &dual,
        &pair.grant_payer,
        &pair.grant_provider,
        15,
    )
    .unwrap();
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
fn p2_t008_dispute_resolve_with_valid_receipt() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = aether_core::escrow::transition::sign_receipt(
        &receipt,
        &pair.provider.signing_key,
        &provider_id,
    )
    .unwrap();
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
    let ev = SettlementEvidenceV0 {
        escrow_id,
        receipt: evidence.evidence.receipt,
        logical_time_presented: 35,
    };
    let escrow = resolve_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &escrow_id,
        Some(&ev),
        &pair.provider.identity.derived_agent_id(),
        &pair.grant_provider,
        200,
    )
    .unwrap();
    assert_eq!(escrow.status, EscrowStatus::ResolvedReleased);
}

#[test]
fn p2_t009_dispute_resolve_without_valid_receipt() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = aether_core::escrow::transition::sign_receipt(
        &receipt,
        &pair.provider.signing_key,
        &provider_id,
    )
    .unwrap();
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
    let escrow = resolve_dispute(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &escrow_id,
        None,
        &pair.provider.identity.derived_agent_id(),
        &pair.grant_provider,
        200,
    )
    .unwrap();
    assert_eq!(escrow.status, EscrowStatus::ResolvedRefunded);
}

#[test]
fn p2_t010_finality_on_terminal() {
    let mut pair = setup_escrow_pair();
    let escrow_id = happy_path_through_release(&mut pair, 100, 10);
    let f = &pair.escrows.get(&escrow_id).unwrap().escrow.finality;
    assert!(f.finalized);
    assert!(!f.hard_settlement_placeholder);
}

#[test]
fn p2_t011_create_single_signature_rejected() {
    let mut pair = setup_escrow_pair();
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let terms = sample_terms(&payer_id, &provider_id, 100, 10, 100, 500, 100);
    let mut dual = sign_terms_dual(
        &terms,
        &pair.payer.signing_key,
        &pair.provider.signing_key,
        &payer_id,
        &provider_id,
    )
    .unwrap();
    dual.sig_provider.signature = vec![0u8; 64];
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
    assert_eq!(err, Error::InvalidSignature);
    assert!(pair.escrows.get(&terms.escrow_id().unwrap()).is_none());
}

#[test]
fn p2_t012_fund_before_create_rejected() {
    let mut pair = setup_escrow_pair();
    let payer_id = pair.payer.identity.derived_agent_id();
    let fake_id = [9u8; 32];
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
fn p2_t013_submit_receipt_before_fund_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let receipt = sample_receipt(&terms, escrow_id, 1, 15);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = aether_core::escrow::transition::sign_receipt(
        &receipt,
        &pair.provider.signing_key,
        &provider_id,
    )
    .unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        15,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_t014_release_before_fund_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let release = EscrowReleaseV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        receipt_id: [1u8; 32],
        actor: payer_id.clone(),
        logical_time: 20,
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
        20,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_t015_release_after_refunded_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    refund_escrow_ready(&mut pair, escrow_id, 25);
    let payer_id = pair.payer.identity.derived_agent_id();
    let receipt_id = sample_receipt(&terms, escrow_id, 1, 30).receipt_id;
    let release = EscrowReleaseV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        receipt_id,
        actor: payer_id.clone(),
        logical_time: 200,
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
        200,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_t016_refund_after_released_rejected() {
    let mut pair = setup_escrow_pair();
    let escrow_id = happy_path_through_release(&mut pair, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let refund = EscrowRefundV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        actor: payer_id.clone(),
        reason: "late".into(),
        logical_time: 500,
    };
    let signed = sign_refund(&refund, &pair.payer.signing_key).unwrap();
    let err = refund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        500,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_t017_transition_from_terminal_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
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
    let funding = EscrowFundingV0 {
        escrow_id,
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
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_t018_fund_after_fund_before_deadline_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 50, 500, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: record.fund_quote.quoted_fee,
        logical_time: 51,
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
        51,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_t019_receipt_after_receipt_before_deadline_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 200, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 201);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = aether_core::escrow::transition::sign_receipt(
        &receipt,
        &pair.provider.signing_key,
        &provider_id,
    )
    .unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        201,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidReceipt);
}

#[test]
fn p2_t020_release_before_dispute_window_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    submit_receipt_ready(&mut pair, &terms, escrow_id, 1, 30);
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
        logical_time: 50,
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
        50,
    )
    .unwrap_err();
    assert_eq!(err, Error::DisputeWindowOpen);
}
