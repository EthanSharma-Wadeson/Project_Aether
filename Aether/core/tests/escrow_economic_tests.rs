//! PROTO-2 escrow economic rules tests (P2-E###).

mod common;

use aether_core::error::Error;
use aether_core::escrow::{
    fund_escrow, refund_escrow, release_escrow, sign_fund, sign_refund, sign_release,
    EscrowFundingV0, EscrowRefundV0, EscrowReleaseV0, EscrowStatus,
};
use common::*;

fn system_total(pair: &EscrowPair, escrow_id: [u8; 32]) -> u64 {
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let mut total =
        pair.ledger.balance(&payer_id) + pair.ledger.balance(&provider_id) + pair.ledger.treasury;
    if let Some(rec) = pair.escrows.get(&escrow_id) {
        total += rec.escrow.funded_amount + rec.escrow.fee_reserved;
    }
    total
}

fn ledger_total(pair: &EscrowPair) -> u64 {
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    pair.ledger.balance(&payer_id) + pair.ledger.balance(&provider_id) + pair.ledger.treasury
}

#[test]
fn p2_e001_double_fund_same_escrow_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let payer_id = pair.payer.identity.derived_agent_id();
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: record.fund_quote.quoted_fee,
        logical_time: 21,
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
        21,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_e002_fund_amount_not_principal_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let record = pair.escrows.get(&escrow_id).unwrap();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 99,
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
    assert_eq!(err, Error::InvalidEscrowTerms);
}

#[test]
fn p2_e003_fund_beyond_payer_balance_rejected() {
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
    assert_eq!(err, Error::InsufficientBalance);
}

#[test]
fn p2_e004_fee_reservation_over_max_protocol_fee_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    let payer_id = pair.payer.identity.derived_agent_id();
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: 100,
        fee_reservation: 11,
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

#[test]
fn p2_e005_no_value_creation_on_release() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let total_before = system_total(&pair, escrow_id);
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
    let total_after = system_total(&pair, escrow_id);
    assert_eq!(total_after, total_before);
    pair.escrows
        .get(&escrow_id)
        .unwrap()
        .escrow
        .check_conservation()
        .unwrap();
    assert_eq!(
        pair.escrows.get(&escrow_id).unwrap().escrow.status,
        EscrowStatus::Released
    );
}

#[test]
fn p2_e006_no_value_creation_on_refund() {
    let mut pair = setup_escrow_pair();
    let ledger_before = ledger_total(&pair);
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    refund_escrow_ready(&mut pair, escrow_id, 25);
    let ledger_after = ledger_total(&pair);
    assert_eq!(ledger_after, ledger_before);
    pair.escrows
        .get(&escrow_id)
        .unwrap()
        .escrow
        .check_conservation()
        .unwrap();
}

#[test]
fn p2_e007_double_release_rejected() {
    let mut pair = setup_escrow_pair();
    let escrow_id = happy_path_through_release(&mut pair, 100, 10);
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
        logical_time: 500,
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
        500,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_e008_double_refund_rejected() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    refund_escrow_ready(&mut pair, escrow_id, 25);
    let payer_id = pair.payer.identity.derived_agent_id();
    let refund = EscrowRefundV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        actor: payer_id.clone(),
        reason: "again".into(),
        logical_time: 30,
    };
    let signed = sign_refund(&refund, &pair.payer.signing_key).unwrap();
    let err = refund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidEscrowStatus);
}

#[test]
fn p2_e009_fee_consumed_le_fee_reserved() {
    let mut pair = setup_escrow_pair();
    let escrow_id = happy_path_through_release(&mut pair, 100, 10);
    let record = pair.escrows.get(&escrow_id).unwrap();
    assert!(record.escrow.fee_consumed <= record.escrow.fee_reserved);
    let sum_consumed: u64 = record.fee_ledger.iter().map(|e| e.amount).sum();
    assert_eq!(sum_consumed, record.escrow.fee_consumed);
}

#[test]
fn p2_e010_insufficient_fee_budget_blocks_release() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 1, 100, 500, 100, 10);
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
        deadline,
    )
    .unwrap_err();
    assert_eq!(err, Error::FeeBudgetExceeded);
}

#[test]
fn p2_e011_terminal_outcomes_mutually_exclusive() {
    let mut pair = setup_escrow_pair();
    let escrow_id = happy_path_through_release(&mut pair, 100, 10);
    let status = pair.escrows.get(&escrow_id).unwrap().escrow.status;
    assert_eq!(status, EscrowStatus::Released);
    assert_ne!(status, EscrowStatus::Refunded);
    let mut pair2 = setup_escrow_pair();
    let (_, id2) = create_escrow_ready(&mut pair2, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair2, id2, 20);
    refund_escrow_ready(&mut pair2, id2, 25);
    let status2 = pair2.escrows.get(&id2).unwrap().escrow.status;
    assert_eq!(status2, EscrowStatus::Refunded);
    assert_ne!(status2, EscrowStatus::Released);
}

#[test]
fn p2_e012_provider_cannot_self_refund_principal() {
    let mut pair = setup_escrow_pair();
    let (_, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let provider_id = pair.provider.identity.derived_agent_id();
    let refund = EscrowRefundV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        actor: provider_id.clone(),
        reason: "self".into(),
        logical_time: 25,
    };
    let signed = sign_refund(&refund, &pair.provider.signing_key).unwrap();
    let err = refund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_provider,
        25,
    )
    .unwrap_err();
    assert_eq!(err, Error::UntrustedTerminalOperation);
}
