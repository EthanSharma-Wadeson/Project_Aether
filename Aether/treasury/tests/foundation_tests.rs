use aether_treasury::*;
use chrono::{Duration, Utc};
use std::sync::Arc;

async fn setup() -> TreasuryEngine {
    let eng = TreasuryEngine::in_memory().await.expect("engine");
    eng.register_asset(AssetId::new("GBP"), 2, "fiat")
        .await
        .unwrap();
    eng.register_asset(AssetId::new("USD"), 2, "fiat")
        .await
        .unwrap();
    eng.register_asset(AssetId::new("EUR"), 2, "fiat")
        .await
        .unwrap();
    eng
}

async fn org_treasury(eng: &TreasuryEngine) -> Treasury {
    eng.create_treasury("org-1", None, TreasuryKind::Organisation, "Org Root")
        .await
        .unwrap()
}

#[tokio::test]
async fn journal_batch_balances() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    let batch = eng
        .fund(
            &t.treasury_id,
            AssetId::new("GBP"),
            Amount(10_000),
            "req-1",
            "idem-fund-1",
        )
        .await
        .unwrap();
    eng.assert_journal_balanced(&batch.batch_id).await.unwrap();
    let avail = eng
        .available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
        .await
        .unwrap();
    assert_eq!(avail.0, 10_000);
}

#[tokio::test]
async fn rejects_negative_amount_and_overdraft() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    let err = eng
        .fund(
            &t.treasury_id,
            AssetId::new("GBP"),
            Amount(-1),
            "r",
            "i",
        )
        .await
        .unwrap_err();
    assert!(matches!(err, TreasuryError::NegativeAmount));

    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(100),
        "r2",
        "i2",
    )
    .await
    .unwrap();

    let (alloc, _) = eng
        .create_allocation(
            &t.treasury_id,
            "agent-1",
            AssetId::new("GBP"),
            Amount(500),
            Amount(0),
            None,
            "r3",
            "i3",
        )
        .await
        .unwrap();

    let err = eng
        .reserve(
            &t.treasury_id,
            &alloc.allocation_id,
            AssetId::new("GBP"),
            Amount(200),
            "r4",
            "i4",
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, TreasuryError::InsufficientFunds));
}

#[tokio::test]
async fn allocation_and_reservation_lifecycle() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(5_000),
        "f",
        "f-idem",
    )
    .await
    .unwrap();

    let (alloc, batch) = eng
        .create_allocation(
            &t.treasury_id,
            "agent-a",
            AssetId::new("GBP"),
            Amount(3_000),
            Amount(2_000),
            None,
            "a",
            "a-idem",
        )
        .await
        .unwrap();
    assert!(batch.is_some());
    eng.assert_journal_balanced(&batch.unwrap().batch_id)
        .await
        .unwrap();

    assert_eq!(
        eng.available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        3_000
    );
    assert_eq!(
        eng.reserved_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        2_000
    );

    // Move more from available via reservation against allocation remaining
    // remaining starts at 2000; reserve 500 from available into reserved (budget)
    // Wait - create_allocation with initial 2000 already moved to reserved and set remaining=2000.
    // reserve() moves available→reserved and decrements remaining.
    let (res, b) = eng
        .reserve(
            &t.treasury_id,
            &alloc.allocation_id,
            AssetId::new("GBP"),
            Amount(500),
            "res",
            "res-idem",
            None,
        )
        .await
        .unwrap();
    eng.assert_journal_balanced(&b.batch_id).await.unwrap();
    assert_eq!(res.status, ReservationStatus::Active);
    assert_eq!(
        eng.available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        2_500
    );

    let (res2, _) = eng
        .release_reservation(&res.reservation_id, "rel", "rel-idem")
        .await
        .unwrap();
    assert_eq!(res2.status, ReservationStatus::Released);
    assert_eq!(
        eng.available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        3_000
    );
}

#[tokio::test]
async fn escrow_settlement_refund_chargeback() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(1_000),
        "f",
        "f1",
    )
    .await
    .unwrap();
    let (alloc, _) = eng
        .create_allocation(
            &t.treasury_id,
            "agent-a",
            AssetId::new("GBP"),
            Amount(1_000),
            Amount(0),
            None,
            "a",
            "a1",
        )
        .await
        .unwrap();

    let (res, b) = eng
        .escrow_reserve(
            &t.treasury_id,
            Some(&alloc.allocation_id),
            AssetId::new("GBP"),
            Amount(400),
            "escrow-1",
            "e",
            "e-idem",
            None,
        )
        .await
        .unwrap();
    eng.assert_journal_balanced(&b.batch_id).await.unwrap();
    assert_eq!(
        eng.escrow_reserved_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        400
    );

    let (res, b) = eng
        .settlement_post(&res.reservation_id, "s", "s-idem")
        .await
        .unwrap();
    eng.assert_journal_balanced(&b.batch_id).await.unwrap();
    assert_eq!(res.status, ReservationStatus::Consumed);
    assert_eq!(
        eng.escrow_reserved_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        0
    );

    let b = eng
        .refund(
            &t.treasury_id,
            AssetId::new("GBP"),
            Amount(100),
            "rf",
            "rf-idem",
        )
        .await
        .unwrap();
    eng.assert_journal_balanced(&b.batch_id).await.unwrap();

    let b = eng
        .chargeback(
            &t.treasury_id,
            AssetId::new("GBP"),
            Amount(50),
            "cb",
            "cb-idem",
        )
        .await
        .unwrap();
    eng.assert_journal_balanced(&b.batch_id).await.unwrap();
}

#[tokio::test]
async fn multi_asset_correctness() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(100),
        "g",
        "g1",
    )
    .await
    .unwrap();
    eng.fund(
        &t.treasury_id,
        AssetId::new("USD"),
        Amount(200),
        "u",
        "u1",
    )
    .await
    .unwrap();
    assert_eq!(
        eng.available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        100
    );
    assert_eq!(
        eng.available_balance("org-1", &t.treasury_id, &AssetId::new("USD"))
            .await
            .unwrap()
            .0,
        200
    );

    let (alloc, _) = eng
        .create_allocation(
            &t.treasury_id,
            "agent",
            AssetId::new("GBP"),
            Amount(100),
            Amount(0),
            None,
            "a",
            "a1",
        )
        .await
        .unwrap();
    let err = eng
        .reserve(
            &t.treasury_id,
            &alloc.allocation_id,
            AssetId::new("USD"),
            Amount(10),
            "r",
            "r1",
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, TreasuryError::AssetMismatch { .. }));
}

#[tokio::test]
async fn rejects_unsupported_asset_closed_and_expired() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    let err = eng
        .fund(
            &t.treasury_id,
            AssetId::new("AETH"),
            Amount(1),
            "r",
            "i",
        )
        .await
        .unwrap_err();
    assert!(matches!(err, TreasuryError::UnsupportedAsset(_)));

    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(50),
        "f",
        "f1",
    )
    .await
    .unwrap();
    // drain to close
    eng.adjustment_to_suspense(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(50),
        "adj",
        "adj1",
    )
    .await
    .unwrap();
    eng.close_treasury(&t.treasury_id).await.unwrap();
    let err = eng
        .fund(
            &t.treasury_id,
            AssetId::new("GBP"),
            Amount(1),
            "x",
            "x1",
        )
        .await
        .unwrap_err();
    assert!(matches!(err, TreasuryError::TreasuryClosed(_)));

    let t2 = eng
        .create_treasury("org-2", None, TreasuryKind::Organisation, "Org2")
        .await
        .unwrap();
    eng.fund(
        &t2.treasury_id,
        AssetId::new("EUR"),
        Amount(100),
        "f",
        "f2",
    )
    .await
    .unwrap();
    let expired = Utc::now() - Duration::hours(1);
    let (alloc, _) = eng
        .create_allocation(
            &t2.treasury_id,
            "agent",
            AssetId::new("EUR"),
            Amount(100),
            Amount(0),
            Some(expired),
            "a",
            "a2",
        )
        .await
        .unwrap();
    let err = eng
        .reserve(
            &t2.treasury_id,
            &alloc.allocation_id,
            AssetId::new("EUR"),
            Amount(10),
            "r",
            "r2",
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, TreasuryError::AllocationExpired(_)));
}

#[tokio::test]
async fn duplicate_reservation_idempotent_replay() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(1_000),
        "f",
        "f1",
    )
    .await
    .unwrap();
    let (alloc, _) = eng
        .create_allocation(
            &t.treasury_id,
            "agent",
            AssetId::new("GBP"),
            Amount(1_000),
            Amount(0),
            None,
            "a",
            "a1",
        )
        .await
        .unwrap();

    let (r1, b1) = eng
        .reserve(
            &t.treasury_id,
            &alloc.allocation_id,
            AssetId::new("GBP"),
            Amount(100),
            "req",
            "same-key",
            None,
        )
        .await
        .unwrap();
    let (r2, b2) = eng
        .reserve(
            &t.treasury_id,
            &alloc.allocation_id,
            AssetId::new("GBP"),
            Amount(100),
            "req",
            "same-key",
            None,
        )
        .await
        .unwrap();
    assert_eq!(r1.reservation_id, r2.reservation_id);
    assert_eq!(b1.batch_id, b2.batch_id);
    assert_eq!(
        eng.available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        900
    );
}

#[tokio::test]
async fn concurrent_reservations_do_not_overdraw() {
    let eng = Arc::new(setup().await);
    let t = org_treasury(&eng).await;
    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(1_000),
        "f",
        "f1",
    )
    .await
    .unwrap();
    let (alloc, _) = eng
        .create_allocation(
            &t.treasury_id,
            "agent",
            AssetId::new("GBP"),
            Amount(1_000),
            Amount(0),
            None,
            "a",
            "a1",
        )
        .await
        .unwrap();

    let mut handles = Vec::new();
    for i in 0..20 {
        let eng = Arc::clone(&eng);
        let tid = t.treasury_id.clone();
        let aid = alloc.allocation_id.clone();
        handles.push(tokio::spawn(async move {
            eng.reserve(
                &tid,
                &aid,
                AssetId::new("GBP"),
                Amount(100),
                format!("req-{i}"),
                format!("idem-{i}"),
                None,
            )
            .await
        }));
    }

    let mut ok = 0;
    let mut fail = 0;
    for h in handles {
        match h.await.unwrap() {
            Ok(_) => ok += 1,
            Err(TreasuryError::InsufficientFunds)
            | Err(TreasuryError::InsufficientAllocation)
            | Err(TreasuryError::NegativeBalance { .. }) => fail += 1,
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    assert_eq!(ok, 10, "exactly 10 reservations of 100 should succeed");
    assert_eq!(fail, 10);
    assert_eq!(
        eng.available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        0
    );
}

#[tokio::test]
async fn rollback_on_insufficient_leaves_balances_unchanged() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(50),
        "f",
        "f1",
    )
    .await
    .unwrap();
    let (alloc, _) = eng
        .create_allocation(
            &t.treasury_id,
            "agent",
            AssetId::new("GBP"),
            Amount(50),
            Amount(0),
            None,
            "a",
            "a1",
        )
        .await
        .unwrap();
    let before = eng
        .available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
        .await
        .unwrap()
        .0;
    let _ = eng
        .reserve(
            &t.treasury_id,
            &alloc.allocation_id,
            AssetId::new("GBP"),
            Amount(999),
            "r",
            "r1",
            None,
        )
        .await
        .unwrap_err();
    let after = eng
        .available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
        .await
        .unwrap()
        .0;
    assert_eq!(before, after);
}

#[tokio::test]
async fn immutability_and_replay() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    let batch = eng
        .fund(
            &t.treasury_id,
            AssetId::new("GBP"),
            Amount(777),
            "f",
            "f1",
        )
        .await
        .unwrap();
    let entries = eng.list_journal_entries("org-1").await.unwrap();
    assert!(!entries.is_empty());
    let entry_id = entries[0].entry_id.clone();
    let err = eng.try_mutate_journal_entry(&entry_id).await.unwrap_err();
    assert!(matches!(err, TreasuryError::ImmutabilityViolation));

    // Corrupt projection then replay
    sqlx::query("UPDATE account_balances SET balance_minor = 0")
        .execute(eng.db().pool())
        .await
        .unwrap();
    eng.replay_balances("org-1").await.unwrap();
    assert_eq!(
        eng.available_balance("org-1", &t.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        777
    );
    eng.assert_journal_balanced(&batch.batch_id).await.unwrap();
}

#[tokio::test]
async fn hierarchy_department_under_org() {
    let eng = setup().await;
    let org = org_treasury(&eng).await;
    let dept = eng
        .create_treasury(
            "org-1",
            Some(org.treasury_id.clone()),
            TreasuryKind::Department,
            "Engineering",
        )
        .await
        .unwrap();
    eng.fund(
        &org.treasury_id,
        AssetId::new("GBP"),
        Amount(500),
        "f",
        "f1",
    )
    .await
    .unwrap();
    // Fund department independently for foundation (transfer API later)
    eng.fund(
        &dept.treasury_id,
        AssetId::new("GBP"),
        Amount(200),
        "f2",
        "f2",
    )
    .await
    .unwrap();
    assert_eq!(
        eng.available_balance("org-1", &dept.treasury_id, &AssetId::new("GBP"))
            .await
            .unwrap()
            .0,
        200
    );
}

#[tokio::test]
async fn frozen_treasury_rejects_funding() {
    let eng = setup().await;
    let t = org_treasury(&eng).await;
    eng.freeze_treasury(&t.treasury_id).await.unwrap();
    let err = eng
        .fund(
            &t.treasury_id,
            AssetId::new("GBP"),
            Amount(1),
            "r",
            "i",
        )
        .await
        .unwrap_err();
    assert!(matches!(err, TreasuryError::TreasuryFrozen(_)));
}
