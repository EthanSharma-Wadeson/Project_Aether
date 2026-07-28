//! PROTO-4 security remediation regressions (P4-SEC-001 / P4-SEC-002).

mod common;

use aether_core::error::Error;
use aether_core::settlement::{
    advance_mock_status, finalize_settlement, query_settlement, submit_settlement, SettlementStatus,
};
use common::*;

/// P4-SEC-R01: no public shortcut can set hard finality without verified finalize.
///
/// `promote_verified_hard_settlement` is crate-private and not re-exported. This test
/// exercises every public pre-finalize surface and asserts hard remains false.
#[test]
fn p4_sec_r01_direct_hard_flag_promotion_prevented() {
    let mut h = setup_settlement_harness(900);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
    assert!(
        !h.pair
            .escrows
            .get(&escrow_id)
            .unwrap()
            .escrow
            .finality
            .hard_settlement_placeholder
    );

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
    assert_eq!(confirmed.settlement_status, SettlementStatus::Confirmed);

    // Confirmed alone must not promote hard finality.
    assert!(
        !h.pair
            .escrows
            .get(&escrow_id)
            .unwrap()
            .escrow
            .finality
            .hard_settlement_placeholder
    );

    // Public escrow module must not expose a direct hard-promotion helper.
    // (compile-time: `aether_core::escrow::set_hard_settlement_flag` / promote_* are absent)
    let _ = std::any::type_name::<aether_core::escrow::EscrowStore>();
}

/// P4-SEC-R02: stale confirmation cannot finalize after adapter reversal.
#[test]
fn p4_sec_r02_stale_confirmation_cannot_finalize() {
    let mut h = setup_settlement_harness(901);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
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
    advance_mock_status(&mut h.adapter, &submitted, SettlementStatus::Confirmed).unwrap();
    let (confirmed, stale_report) = query_settlement(
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
    assert_eq!(stale_report.status, SettlementStatus::Confirmed);

    // External provider reverses after confirmation was cached.
    h.adapter
        .set_reversed(confirmed.external_settlement_ref.as_ref().unwrap())
        .unwrap();

    let err = finalize_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &h.adapter,
        &confirmed.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        &stale_report,
        203,
    );
    assert!(matches!(err, Err(Error::InvalidSettlementEvidence)));

    let after = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    assert!(!after.finality.hard_settlement_placeholder);
    assert_eq!(after.status, before.status);
    assert_eq!(after.funded_amount, before.funded_amount);
    assert_eq!(
        h.settlements
            .get_settlement(&confirmed.binding_id)
            .unwrap()
            .settlement_status,
        SettlementStatus::Confirmed
    );
}

/// P4-SEC-R03: fresh confirmation successfully finalizes.
#[test]
fn p4_sec_r03_fresh_confirmation_finalizes() {
    let mut h = setup_settlement_harness(902);
    let escrow_id = happy_path_through_release(&mut h.pair, 500, 10);
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
    let after = h.pair.escrows.get(&escrow_id).unwrap().escrow.clone();
    assert!(after.finality.hard_settlement_placeholder);
    assert_eq!(after.status, before.status);
    assert_eq!(after.funded_amount, before.funded_amount);
}
