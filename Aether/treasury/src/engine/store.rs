use chrono::{DateTime, Utc};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::{Result, TreasuryError};
use crate::models::*;
use crate::types::*;

use super::journal::parse_event;

pub async fn ensure_asset_enabled(pool: &SqlitePool, asset_id: &str) -> Result<()> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT enabled FROM asset_types WHERE asset_id = ?")
            .bind(asset_id)
            .fetch_optional(pool)
            .await?;
    match row {
        Some((1,)) => Ok(()),
        Some(_) => Err(TreasuryError::UnsupportedAsset(asset_id.into())),
        None => Err(TreasuryError::UnsupportedAsset(asset_id.into())),
    }
}

pub async fn get_asset(pool: &SqlitePool, asset_id: &str) -> Result<AssetType> {
    let row: (String, i64, String, i64, String) = sqlx::query_as(
        "SELECT asset_id, scale, class, enabled, created_at FROM asset_types WHERE asset_id = ?",
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| TreasuryError::UnsupportedAsset(asset_id.into()))?;
    Ok(AssetType {
        asset_id: AssetId::new(row.0),
        scale: row.1,
        class: row.2,
        enabled: row.3 == 1,
        created_at: parse_dt(&row.4),
    })
}

pub async fn list_assets(pool: &SqlitePool) -> Result<Vec<AssetType>> {
    let rows: Vec<(String, i64, String, i64, String)> =
        sqlx::query_as("SELECT asset_id, scale, class, enabled, created_at FROM asset_types")
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|r| AssetType {
            asset_id: AssetId::new(r.0),
            scale: r.1,
            class: r.2,
            enabled: r.3 == 1,
            created_at: parse_dt(&r.4),
        })
        .collect())
}

pub async fn get_treasury(pool: &SqlitePool, treasury_id: &str) -> Result<Treasury> {
    let row: Option<(
        String,
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        Option<String>,
    )> = sqlx::query_as(
        r#"
        SELECT treasury_id, organisation_id, parent_treasury_id, kind, name, status, created_at, closed_at
        FROM treasuries WHERE treasury_id = ?
        "#,
    )
    .bind(treasury_id)
    .fetch_optional(pool)
    .await?;
    let row = row.ok_or_else(|| TreasuryError::TreasuryNotFound(treasury_id.into()))?;
    Ok(Treasury {
        treasury_id: row.0,
        organisation_id: row.1,
        parent_treasury_id: row.2,
        kind: TreasuryKind::parse(&row.3)?,
        name: row.4,
        status: TreasuryStatus::parse(&row.5)?,
        created_at: parse_dt(&row.6),
        closed_at: row.7.map(|s| parse_dt(&s)),
    })
}

pub async fn get_allocation(pool: &SqlitePool, allocation_id: &str) -> Result<Allocation> {
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        String,
        Option<String>,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT allocation_id, organisation_id, treasury_id, agent_id, asset_id,
               ceiling_minor, remaining_minor, status, expires_at, created_at
        FROM allocations WHERE allocation_id = ?
        "#,
    )
    .bind(allocation_id)
    .fetch_optional(pool)
    .await?;
    let row = row.ok_or_else(|| TreasuryError::AllocationNotFound(allocation_id.into()))?;
    Ok(Allocation {
        allocation_id: row.0,
        organisation_id: row.1,
        treasury_id: row.2,
        agent_id: row.3,
        asset_id: AssetId::new(row.4),
        ceiling_minor: row.5,
        remaining_minor: row.6,
        status: AllocationStatus::parse(&row.7)?,
        expires_at: row.8.map(|s| parse_dt(&s)),
        created_at: parse_dt(&row.9),
    })
}

pub async fn get_reservation(pool: &SqlitePool, reservation_id: &str) -> Result<Reservation> {
    let row: Option<(
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        String,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT reservation_id, organisation_id, treasury_id, allocation_id, asset_id,
               amount_minor, kind, status, escrow_id, idempotency_key, expires_at,
               created_at, updated_at
        FROM reservations WHERE reservation_id = ?
        "#,
    )
    .bind(reservation_id)
    .fetch_optional(pool)
    .await?;
    let row = row.ok_or_else(|| TreasuryError::ReservationNotFound(reservation_id.into()))?;
    Ok(Reservation {
        reservation_id: row.0,
        organisation_id: row.1,
        treasury_id: row.2,
        allocation_id: row.3,
        asset_id: AssetId::new(row.4),
        amount_minor: row.5,
        kind: ReservationKind::parse(&row.6)?,
        status: ReservationStatus::parse(&row.7)?,
        escrow_id: row.8,
        idempotency_key: row.9,
        expires_at: row.10.map(|s| parse_dt(&s)),
        created_at: parse_dt(&row.11),
        updated_at: parse_dt(&row.12),
    })
}

pub async fn find_reservation_by_idem(
    pool: &SqlitePool,
    idempotency_key: &str,
) -> Result<Option<Reservation>> {
    let id: Option<String> =
        sqlx::query_scalar("SELECT reservation_id FROM reservations WHERE idempotency_key = ?")
            .bind(idempotency_key)
            .fetch_optional(pool)
            .await?;
    match id {
        Some(id) => Ok(Some(get_reservation(pool, &id).await?)),
        None => Ok(None),
    }
}

pub async fn find_batch_by_idem(
    pool: &SqlitePool,
    organisation_id: &str,
    idempotency_key: &str,
) -> Result<Option<JournalBatch>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        r#"
        SELECT batch_id, organisation_id, event_type, request_id, idempotency_key, created_at
        FROM journal_batches
        WHERE organisation_id = ? AND idempotency_key = ?
        "#,
    )
    .bind(organisation_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(batch_id, organisation_id, event_type, request_id, idempotency_key, created_at)| {
            JournalBatch {
                batch_id,
                organisation_id,
                event_type: parse_event(&event_type),
                request_id,
                idempotency_key,
                created_at: parse_dt(&created_at),
            }
        },
    ))
}

pub async fn get_balance(
    pool: &SqlitePool,
    organisation_id: &str,
    account_code: &str,
    asset_id: &str,
) -> Result<i64> {
    let bal: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT balance_minor FROM account_balances
        WHERE organisation_id = ? AND account_code = ? AND asset_id = ?
        "#,
    )
    .bind(organisation_id)
    .bind(account_code)
    .bind(asset_id)
    .fetch_optional(pool)
    .await?;
    Ok(bal.unwrap_or(0))
}

pub async fn get_balance_tx(
    tx: &mut Transaction<'_, Sqlite>,
    organisation_id: &str,
    account_code: &str,
    asset_id: &str,
) -> Result<i64> {
    let bal: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT balance_minor FROM account_balances
        WHERE organisation_id = ? AND account_code = ? AND asset_id = ?
        "#,
    )
    .bind(organisation_id)
    .bind(account_code)
    .bind(asset_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(bal.unwrap_or(0))
}

pub async fn apply_balance_delta(
    pool: &SqlitePool,
    organisation_id: &str,
    account_code: &str,
    asset_id: &str,
    delta: i64,
    enforce_non_negative: &[String],
) -> Result<()> {
    let mut tx = pool.begin().await?;
    apply_balance_delta_tx(
        &mut tx,
        organisation_id,
        account_code,
        asset_id,
        delta,
        enforce_non_negative,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn apply_balance_delta_tx(
    tx: &mut Transaction<'_, Sqlite>,
    organisation_id: &str,
    account_code: &str,
    asset_id: &str,
    delta: i64,
    enforce_non_negative: &[String],
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO account_balances (organisation_id, account_code, asset_id, balance_minor)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(organisation_id, account_code, asset_id)
        DO UPDATE SET balance_minor = balance_minor + excluded.balance_minor
        "#,
    )
    .bind(organisation_id)
    .bind(account_code)
    .bind(asset_id)
    .bind(delta)
    .execute(&mut **tx)
    .await?;

    if enforce_non_negative.iter().any(|a| a == account_code) {
        let bal = get_balance_tx(tx, organisation_id, account_code, asset_id).await?;
        if bal < 0 {
            return Err(TreasuryError::NegativeBalance {
                account: account_code.into(),
            });
        }
    }
    Ok(())
}

pub async fn list_entries(pool: &SqlitePool, organisation_id: &str) -> Result<Vec<JournalEntry>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        String,
        i64,
        i64,
        String,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT entry_id, batch_id, organisation_id, event_type, treasury_id, allocation_id,
               reservation_id, asset_id, account_code, debit_minor, credit_minor,
               request_id, created_at
        FROM journal_entries
        WHERE organisation_id = ?
        ORDER BY created_at ASC, entry_id ASC
        "#,
    )
    .bind(organisation_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| JournalEntry {
            entry_id: r.0,
            batch_id: r.1,
            organisation_id: r.2,
            event_type: parse_event(&r.3),
            treasury_id: r.4,
            allocation_id: r.5,
            reservation_id: r.6,
            asset_id: AssetId::new(r.7),
            account_code: r.8,
            debit_minor: r.9,
            credit_minor: r.10,
            request_id: r.11,
            created_at: parse_dt(&r.12),
        })
        .collect())
}

pub(crate) fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| now())
}
