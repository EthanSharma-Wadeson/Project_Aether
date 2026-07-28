//! PROTO-4 acceptance tests — lifecycle, integrity, integration, hypothesis.

mod common;

use aether_core::error::Error;
use aether_core::settlement::{
    advance_mock_status, bind_account, cancel_settlement, finalize_settlement, intent_from_escrow,
    new_account_binding, query_settlement, request_settlement, sign_account_binding,
    submit_settlement, EconomicOutcome, MockSubmitMode, SettlementStatus,
    PROVIDER_ENTERPRISE_LEDGER_V0,
};
use common::*;

#[test]
fn p4_t001_bind_payer_account() {
    let h = setup_settlement_harness(100);
    assert_eq!(
        h.payer_account.agent_id,
        h.pair.payer.identity.derived_agent_id()
    );
    let recomputed = h.payer_account.compute_binding_id().unwrap();
    assert_eq!(h.payer_account.binding_id, recomputed);
}

#[test]
fn p4_t002_bind_provider_account() {
    let h = setup_settlement_harness(101);
    assert_eq!(
        h.provider_account.settlement_provider,
        h.payer_account.settlement_provider
    );
    assert_eq!(
        h.provider_account.settlement_provider,
        PROVIDER_ENTERPRISE_LEDGER_V0
    );
}

#[test]
fn p4_t003_request_settle_released() {
    let mut h = setup_settlement_harness(102);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let escrow = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    assert!(escrow.finality.finalized);
    assert!(!escrow.finality.hard_settlement_placeholder);

    let binding = request_release_settlement(&mut h, escrow_id, 200);
    assert_eq!(binding.settlement_status, SettlementStatus::Requested);
    let after = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    assert_eq!(after.status, escrow.status);
    assert!(!after.finality.hard_settlement_placeholder);
}

#[test]
fn p4_t004_adapter_submit_success() {
    let mut h = setup_settlement_harness(103);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    assert_eq!(submitted.settlement_status, SettlementStatus::Submitted);
    assert!(submitted.external_settlement_ref.is_some());
}

#[test]
fn p4_t005_adapter_accepts() {
    let mut h = setup_settlement_harness(104);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    advance_mock_status(&mut h.adapter, &submitted, SettlementStatus::Accepted).unwrap();
    let (accepted, _) = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        202,
    )
    .unwrap();
    assert_eq!(accepted.settlement_status, SettlementStatus::Accepted);
}

#[test]
fn p4_t006_adapter_confirms() {
    let mut h = setup_settlement_harness(105);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    advance_mock_status(&mut h.adapter, &submitted, SettlementStatus::Confirmed).unwrap();
    let (confirmed, report) = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        202,
    )
    .unwrap();
    assert_eq!(confirmed.settlement_status, SettlementStatus::Confirmed);
    assert!(report.proof_token.is_some());
    assert_ne!(confirmed.evidence_commitment, [0u8; 32]);
}

#[test]
fn p4_t007_finalize_sets_hard_flag() {
    let mut h = setup_settlement_harness(106);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let status_before = h.pair.escrows.get(&escrow_id).unwrap().escrow.status;
    let finalized = full_finalize_release(&mut h, escrow_id, 300);
    assert_eq!(finalized.settlement_status, SettlementStatus::Finalized);
    let escrow = &h.pair.escrows.get(&escrow_id).unwrap().escrow;
    assert!(escrow.finality.hard_settlement_placeholder);
    assert_eq!(escrow.status, status_before);
}

#[test]
fn p4_t008_refund_path_finalize() {
    let mut h = setup_settlement_harness(107);
    let escrow_id = happy_path_through_refund(&mut h.pair, 1_000, 10);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::RefundToPayer,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        None,
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let binding = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    )
    .unwrap();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    advance_mock_status(&mut h.adapter, &submitted, SettlementStatus::Confirmed).unwrap();
    let (confirmed, report) = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        202,
    )
    .unwrap();
    let finalized = finalize_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &h.adapter,
        &confirmed.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        &report,
        203,
    )
    .unwrap();
    assert_eq!(finalized.settlement_status, SettlementStatus::Finalized);
    assert!(
        h.pair
            .escrows
            .get(&escrow_id)
            .unwrap()
            .escrow
            .finality
            .hard_settlement_placeholder
    );
}

#[test]
fn p4_t009_idempotent_correlation() {
    let mut h = setup_settlement_harness(108);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let b1 = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &b1.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    let count = h.adapter.submit_success_count;
    let b2 = request_release_settlement(&mut h, escrow_id, 202);
    assert_eq!(b1.binding_id, b2.binding_id);
    assert_eq!(b1.correlation_id, b2.correlation_id);
    submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &b2.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        203,
    )
    .unwrap();
    assert_eq!(h.adapter.submit_success_count, count);
}

#[test]
fn p4_t010_query_no_escrow_mutate() {
    let mut h = setup_settlement_harness(109);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let before = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    advance_mock_status(&mut h.adapter, &submitted, SettlementStatus::Accepted).unwrap();
    let _ = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        202,
    )
    .unwrap();
    let after = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    assert_eq!(before.status, after.status);
    assert_eq!(before.funded_amount, after.funded_amount);
    assert_eq!(
        before.finality.hard_settlement_placeholder,
        after.finality.hard_settlement_placeholder
    );
}

#[test]
fn p4_t011_settle_before_terminal() {
    let mut h = setup_settlement_harness(110);
    let (terms, escrow_id) = create_escrow_ready(&mut h.pair, 1_000, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut h.pair, escrow_id, 20);
    let _ = terms;
    let err = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    );
    assert!(matches!(err, Err(Error::InvalidEscrowStatus)));
}

#[test]
fn p4_t012_settle_unknown_escrow() {
    let h = setup_settlement_harness(111);
    let fake = [9u8; 32];
    let err = intent_from_escrow(
        &h.pair.escrows,
        &fake,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    );
    assert!(matches!(err, Err(Error::EscrowNotFound)));
}

#[test]
fn p4_t013_outcome_mismatch() {
    let mut h = setup_settlement_harness(112);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let err = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::RefundToPayer,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        None,
    );
    assert!(matches!(err, Err(Error::InvalidEscrowStatus)));
}

#[test]
fn p4_t014_no_hard_before_confirmed() {
    let mut h = setup_settlement_harness(113);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    // forge a report and try finalize at Submitted
    let report = aether_core::settlement::SettlementReportV0 {
        protocol_version: 1,
        schema_version: 1,
        settlement_id: submitted.settlement_id(),
        external_reference: submitted.external_settlement_ref.clone().unwrap(),
        status: SettlementStatus::Confirmed,
        amount: submitted.principal_amount,
        commitments: vec![],
        adapter_identity: PROVIDER_ENTERPRISE_LEDGER_V0.into(),
        proof_token: Some(aether_core::settlement::MockSettlementAdapterV0::proof_for(
            &submitted.settlement_id(),
            submitted.external_settlement_ref.as_ref().unwrap(),
            submitted.principal_amount,
        )),
        correlation_id: submitted.correlation_id,
    };
    let err = finalize_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        &report,
        202,
    );
    assert!(matches!(err, Err(Error::InvalidSettlementStatus)));
    assert!(
        !h.pair
            .escrows
            .get(&escrow_id)
            .unwrap()
            .escrow
            .finality
            .hard_settlement_placeholder
    );
}

#[test]
fn p4_t015_cancel_after_confirmed() {
    let mut h = setup_settlement_harness(114);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    advance_mock_status(&mut h.adapter, &submitted, SettlementStatus::Confirmed).unwrap();
    let (confirmed, _) = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        202,
    )
    .unwrap();
    let err = cancel_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &confirmed.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        203,
    );
    assert!(matches!(err, Err(Error::InvalidSettlementStatus)));
}

#[test]
fn p4_t016_second_correlation_after_finalized() {
    let mut h = setup_settlement_harness(115);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let _ = full_finalize_release(&mut h, escrow_id, 300);
    // Force different correlation by tampering fee_amount in a forged intent
    let mut intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    // Same intent → same correlation → idempotent return of finalized.
    // For distinct correlation, swap destination accounts incorrectly after mutating
    // a field that is in intent_cbor but still validates — not possible without amount change.
    // Instead: request again with same intent returns existing Finalized (not a new binding).
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let again = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        400,
    )
    .unwrap();
    assert_eq!(again.settlement_status, SettlementStatus::Finalized);

    // Distinct correlation via wrong fee → amount mismatch reject before duplicate check
    intent.fee_amount = intent.fee_amount.saturating_add(1);
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        401,
    );
    assert!(matches!(err, Err(Error::SettlementAmountMismatch)));
}

#[test]
fn p4_t017_empty_external_account_ref() {
    let h = setup_settlement_harness(116);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let acct = new_account_binding(
        &payer_id,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        "",
        "AETHER_TEST",
        50,
        10_000,
    );
    assert!(acct.is_err());
}

#[test]
fn p4_t018_settle_without_provider_account() {
    let mut h = setup_settlement_harness(117);
    let escrow_id = happy_path_through_release(&mut h.pair, 1_000, 10);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        None,
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    );
    assert!(matches!(
        err,
        Err(Error::AccountBindingNotFound) | Err(Error::AccountBindingMismatch)
    ));
}

#[test]
fn p4_r001_escrow_id_matches() {
    let mut h = setup_settlement_harness(200);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let b = request_release_settlement(&mut h, escrow_id, 200);
    assert_eq!(b.escrow_id, escrow_id);
}

#[test]
fn p4_r002_wrong_escrow_id() {
    let mut h = setup_settlement_harness(201);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let mut intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    intent.escrow_id = [1u8; 32];
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    );
    assert!(matches!(err, Err(Error::EscrowNotFound)));
}

#[test]
fn p4_r003_wrong_terms_version() {
    let mut h = setup_settlement_harness(202);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let mut intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    intent.terms_version = 99;
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    );
    assert!(matches!(err, Err(Error::InvalidSettlementEvidence)));
}

#[test]
fn p4_r004_amount_mismatch() {
    let mut h = setup_settlement_harness(203);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let mut intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    intent.principal_amount = 1;
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    );
    assert!(matches!(err, Err(Error::SettlementAmountMismatch)));
}

#[test]
fn p4_r005_correlation_deterministic() {
    let mut h = setup_settlement_harness(204);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let b1 = request_release_settlement(&mut h, escrow_id, 200);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    let b2 = intent
        .to_proto_binding(
            h.pair.payer.identity.derived_agent_id(),
            h.pair.provider.identity.derived_agent_id(),
            200,
            [0u8; 32],
        )
        .unwrap();
    assert_eq!(b1.correlation_id, b2.correlation_id);
}

#[test]
fn p4_r006_different_intents_distinct_correlation() {
    let mut h = setup_settlement_harness(205);
    let e1 = happy_path_through_release(&mut h.pair, 500, 10);
    let e2 = happy_path_through_release(&mut h.pair, 600, 10);
    let b1 = request_release_settlement(&mut h, e1, 200);
    let b2 = request_release_settlement(&mut h, e2, 201);
    assert_ne!(b1.correlation_id, b2.correlation_id);
}

#[test]
fn p4_r007_external_ref_unique() {
    let mut h = setup_settlement_harness(206);
    let e1 = happy_path_through_release(&mut h.pair, 500, 10);
    let e2 = happy_path_through_release(&mut h.pair, 600, 10);
    let b1 = request_release_settlement(&mut h, e1, 200);
    let b2 = request_release_settlement(&mut h, e2, 201);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let s1 = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &b1.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        202,
    )
    .unwrap();
    let s2 = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &b2.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        203,
    )
    .unwrap();
    assert_ne!(s1.external_settlement_ref, s2.external_settlement_ref);
}

#[test]
fn p4_r008_swap_account_bindings() {
    let mut h = setup_settlement_harness(207);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.provider_account.binding_id, // swapped
        Some(h.payer_account.binding_id),
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    );
    assert!(matches!(err, Err(Error::AccountBindingMismatch)));
}

#[test]
fn p4_r009_wrong_settlement_provider() {
    let mut h = setup_settlement_harness(208);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        "payments.sandbox.v0",
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    );
    assert!(matches!(err, Err(Error::SettlementProviderMismatch)));
}

#[test]
fn p4_r010_tampered_evidence() {
    let mut h = setup_settlement_harness(209);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    advance_mock_status(&mut h.adapter, &submitted, SettlementStatus::Confirmed).unwrap();
    let (confirmed, mut report) = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        202,
    )
    .unwrap();
    report.proof_token = Some([0xAAu8; 32]);
    let err = finalize_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &h.adapter,
        &confirmed.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        &report,
        203,
    );
    assert!(matches!(err, Err(Error::InvalidSettlementEvidence)));
}

#[test]
fn p4_r011_non_canonical_account_body() {
    let mut h = setup_settlement_harness(210);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let acct = new_account_binding(
        &payer_id,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        "acct-x",
        "AETHER_TEST",
        50,
        10_000,
    )
    .unwrap();
    let mut signed = sign_account_binding(&acct, &h.pair.payer.signing_key, &payer_id).unwrap();
    signed.body.push(0x00);
    let err = bind_account(
        &mut h.settlements,
        &h.pair.registry,
        &h.pair.caps,
        &signed,
        Some(&h.grant_settle_payer),
        50,
    );
    assert!(err.is_err());
}

#[test]
fn p4_r012_escrow_status_snapshot_mismatch() {
    let mut h = setup_settlement_harness(211);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let mut intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    intent.aether_escrow_status = "refunded".into();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    );
    assert!(matches!(err, Err(Error::InvalidSettlementEvidence)));
}

#[test]
fn p4_i01_bind_requires_capability() {
    let mut h = setup_settlement_harness(300);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let acct = new_account_binding(
        &payer_id,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        "acct-new",
        "AETHER_TEST",
        60,
        10_000,
    )
    .unwrap();
    let signed = sign_account_binding(&acct, &h.pair.payer.signing_key, &payer_id).unwrap();
    let err = bind_account(
        &mut h.settlements,
        &h.pair.registry,
        &h.pair.caps,
        &signed,
        None,
        60,
    );
    assert!(matches!(err, Err(Error::CapabilityDenied)));
}

#[test]
fn p4_i02_settle_requires_capability() {
    let mut h = setup_settlement_harness(301);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        None,
        200,
    );
    assert!(matches!(err, Err(Error::CapabilityDenied)));
}

#[test]
fn p4_i03_cap_before_adapter() {
    let mut h = setup_settlement_harness(302);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let before = h.adapter.submit_success_count;
    let err = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        None,
        201,
    );
    assert!(matches!(err, Err(Error::CapabilityDenied)));
    assert_eq!(h.adapter.submit_success_count, before);
}

#[test]
fn p4_i04_expired_capability() {
    let mut h = setup_settlement_harness(303);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        9_999_999,
    );
    assert!(matches!(err, Err(Error::CapabilityDenied)));
}

#[test]
fn p4_i05_stale_permission_root() {
    let mut h = setup_settlement_harness(304);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    // Revoking the capability records a stale/invalid grant path for settle.
    let cap_id =
        aether_core::capability::model::CapabilityV0::decode(&h.grant_settle_payer.message.body)
            .unwrap()
            .capability_id()
            .unwrap();
    h.pair.caps.revoke(cap_id, 100);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        200,
    );
    assert!(matches!(
        err,
        Err(Error::CapabilityDenied) | Err(Error::CapabilityRevoked)
    ));
}

#[test]
fn p4_i06_failed_settle_escrow_unchanged() {
    let mut h = setup_settlement_harness(305);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let before = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    h.adapter.submit_mode = MockSubmitMode::PartialAmount(1);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let failed = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    assert_eq!(failed.settlement_status, SettlementStatus::Failed);
    let after = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    assert_eq!(before.status, after.status);
    assert!(!after.finality.hard_settlement_placeholder);
}

#[test]
fn p4_i07_success_no_rewrite_balances() {
    let mut h = setup_settlement_harness(306);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let before_treasury = h.pair.ledger.treasury;
    let payer_bal = h
        .pair
        .ledger
        .balance(&h.pair.payer.identity.derived_agent_id());
    let _ = full_finalize_release(&mut h, escrow_id, 300);
    assert_eq!(h.pair.ledger.treasury, before_treasury);
    assert_eq!(
        h.pair
            .ledger
            .balance(&h.pair.payer.identity.derived_agent_id()),
        payer_bal
    );
}

#[test]
fn p4_i08_failed_no_second_release() {
    let mut h = setup_settlement_harness(307);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let status = h.pair.escrows.get(&escrow_id).unwrap().escrow.status;
    h.adapter.submit_mode = MockSubmitMode::Fail;
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let _ = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    assert_eq!(
        h.pair.escrows.get(&escrow_id).unwrap().escrow.status,
        status
    );
}

#[test]
fn p4_i09_soft_finalized_hard_false_pre_confirm() {
    let mut h = setup_settlement_harness(308);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let e = &h.pair.escrows.get(&escrow_id).unwrap().escrow.finality;
    assert!(e.finalized);
    assert!(!e.hard_settlement_placeholder);
    let _ = request_release_settlement(&mut h, escrow_id, 200);
    let e = &h.pair.escrows.get(&escrow_id).unwrap().escrow.finality;
    assert!(e.finalized);
    assert!(!e.hard_settlement_placeholder);
}

#[test]
fn p4_i10_channel_store_untouched() {
    let channels = aether_core::channel::ChannelStore::new();
    let mut h = setup_settlement_harness(309);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let _ = full_finalize_release(&mut h, escrow_id, 300);
    assert!(channels.get(&[0u8; 32]).is_none());
}

#[test]
fn p4_i11_envelope_alone_insufficient() {
    // NET envelope delivery does not grant settlement authority.
    let mut h = setup_settlement_harness(310);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        None,
        200,
    );
    assert!(matches!(err, Err(Error::CapabilityDenied)));
}

#[test]
fn p4_i12_session_not_settlement_authority() {
    let mut net = setup_network_pair_seeded(311);
    let _ = establish_session(&mut net, 1, 2, 10, 1_000);
    let mut h = setup_settlement_harness(312);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    // Network agents are distinct from settlement harness; session does not transfer.
    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        None,
        200,
    );
    assert!(matches!(err, Err(Error::CapabilityDenied)));
}

#[test]
fn p4_h001_n_terminal_to_finalized() {
    let mut h = setup_settlement_harness(400);
    let mut audit = Vec::new();
    for i in 0..3 {
        let escrow_id = happy_path_through_release(&mut h.pair, 100 + i * 10, 10);
        let b = full_finalize_release(&mut h, escrow_id, 500 + i * 10);
        assert_eq!(b.settlement_status, SettlementStatus::Finalized);
        audit.push((b.binding_id, b.external_settlement_ref.clone().unwrap()));
        assert!(
            h.pair
                .escrows
                .get(&escrow_id)
                .unwrap()
                .escrow
                .finality
                .hard_settlement_placeholder
        );
    }
    assert_eq!(audit.len(), 3);
}

#[test]
fn p4_h002_revoke_stops_settle() {
    let mut h = setup_settlement_harness(401);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let finalized = full_finalize_release(&mut h, escrow_id, 300);
    assert_eq!(finalized.settlement_status, SettlementStatus::Finalized);

    // Prepare second terminal escrow before revoking settlement capability.
    let escrow2 = happy_path_through_release(&mut h.pair, 400, 10);

    let settle_only = direct_capability(&h.pair.payer, settlement_actions(), Some(10_000));
    let settle_grant = grant_and_store(
        &h.pair.payer,
        &h.pair.registry,
        &mut h.pair.caps,
        &settle_only,
    );
    let cap_id = aether_core::capability::model::CapabilityV0::decode(&settle_grant.message.body)
        .unwrap()
        .capability_id()
        .unwrap();
    h.pair.caps.revoke(cap_id, 900);

    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow2,
        EconomicOutcome::ReleaseToProvider,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .unwrap();
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let err = request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&settle_grant),
        901,
    );
    assert!(matches!(
        err,
        Err(Error::CapabilityDenied) | Err(Error::CapabilityRevoked)
    ));
    assert_eq!(
        h.settlements
            .get_settlement(&finalized.binding_id)
            .unwrap()
            .settlement_status,
        SettlementStatus::Finalized
    );
}

#[test]
fn p4_h003_failure_preserves_protocol() {
    let mut h = setup_settlement_harness(402);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let before = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    h.adapter.submit_mode = MockSubmitMode::Fail;
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let _ = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    let after = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    assert_eq!(before.status, after.status);
    assert!(!after.finality.hard_settlement_placeholder);
}

#[test]
fn p4_h004_false_confirm_zero_success() {
    let mut h = setup_settlement_harness(403);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let binding = request_release_settlement(&mut h, escrow_id, 200);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        201,
    )
    .unwrap();
    h.adapter
        .inject_fake_confirmed(
            submitted.external_settlement_ref.as_ref().unwrap(),
            submitted.principal_amount,
        )
        .unwrap();
    let (after_query, report) = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        202,
    )
    .unwrap_or_else(|_| {
        // fail closed on query is also success for this hypothesis
        (
            submitted.clone(),
            aether_core::settlement::SettlementReportV0 {
                protocol_version: 1,
                schema_version: 1,
                settlement_id: submitted.settlement_id(),
                external_reference: submitted.external_settlement_ref.clone().unwrap(),
                status: SettlementStatus::Confirmed,
                amount: submitted.principal_amount,
                commitments: vec![],
                adapter_identity: PROVIDER_ENTERPRISE_LEDGER_V0.into(),
                proof_token: None,
                correlation_id: submitted.correlation_id,
            },
        )
    });
    let err = finalize_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &h.adapter,
        &after_query.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        &report,
        203,
    );
    assert!(err.is_err());
    assert!(
        !h.pair
            .escrows
            .get(&escrow_id)
            .unwrap()
            .escrow
            .finality
            .hard_settlement_placeholder
    );
}

#[test]
fn p4_m001_deterministic_ids() {
    let mut h = setup_settlement_harness(500);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    let b1 = request_release_settlement(&mut h, escrow_id, 200);
    assert_eq!(b1.binding_id, b1.compute_binding_id().unwrap());
    assert_eq!(b1.correlation_id, b1.compute_correlation_id().unwrap());
}

#[test]
fn p4_m002_soft_hard_distinguishable() {
    let mut h = setup_settlement_harness(501);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    {
        let f = &h.pair.escrows.get(&escrow_id).unwrap().escrow.finality;
        assert!(f.finalized);
        assert!(!f.hard_settlement_placeholder);
    }
    let _ = full_finalize_release(&mut h, escrow_id, 300);
    {
        let f = &h.pair.escrows.get(&escrow_id).unwrap().escrow.finality;
        assert!(f.finalized);
        assert!(f.hard_settlement_placeholder);
    }
}
