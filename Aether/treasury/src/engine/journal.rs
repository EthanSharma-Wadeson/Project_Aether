use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::error::{Result, TreasuryError};
use crate::models::{JournalBatch, JournalLine};
use crate::types::{now, JournalEventType};

use super::store::{apply_balance_delta_tx, find_batch_by_idem};

pub struct BatchSpec<'a> {
    pub organisation_id: String,
    pub event_type: JournalEventType,
    pub request_id: String,
    pub idempotency_key: String,
    pub lines: &'a mut [JournalLine],
    pub enforce_non_negative_accounts: &'a [String],
}

fn validate_lines(lines: &[JournalLine]) -> Result<(i64, i64)> {
    let mut debit_sum = 0i64;
    let mut credit_sum = 0i64;
    for line in lines {
        if line.debit_minor < 0 || line.credit_minor < 0 {
            return Err(TreasuryError::NegativeAmount);
        }
        if line.debit_minor > 0 && line.credit_minor > 0 {
            return Err(TreasuryError::Validation(
                "line cannot be both debit and credit".into(),
            ));
        }
        debit_sum = debit_sum
            .checked_add(line.debit_minor)
            .ok_or(TreasuryError::NegativeAmount)?;
        credit_sum = credit_sum
            .checked_add(line.credit_minor)
            .ok_or(TreasuryError::NegativeAmount)?;
    }
    if debit_sum != credit_sum || debit_sum == 0 {
        return Err(TreasuryError::UnbalancedBatch);
    }
    Ok((debit_sum, credit_sum))
}

pub fn parse_event(s: &str) -> JournalEventType {
    match s {
        "funding" => JournalEventType::Funding,
        "allocation" => JournalEventType::Allocation,
        "reservation" => JournalEventType::Reservation,
        "release" => JournalEventType::Release,
        "escrow_reservation" => JournalEventType::EscrowReservation,
        "settlement_post" => JournalEventType::SettlementPost,
        "refund" => JournalEventType::Refund,
        "chargeback" => JournalEventType::Chargeback,
        "adjustment" => JournalEventType::Adjustment,
        _ => JournalEventType::Adjustment,
    }
}

pub async fn post_batch_tx(
    tx: &mut Transaction<'_, Sqlite>,
    spec: BatchSpec<'_>,
) -> Result<JournalBatch> {
    validate_lines(spec.lines)?;

    if let Some(existing) =
        find_batch_by_idem_tx(tx, &spec.organisation_id, &spec.idempotency_key).await?
    {
        return Ok(existing);
    }

    let batch_id = Uuid::new_v4().to_string();
    let created_at = now();

    let insert = sqlx::query(
        r#"
        INSERT INTO journal_batches
          (batch_id, organisation_id, event_type, request_id, idempotency_key, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&batch_id)
    .bind(&spec.organisation_id)
    .bind(spec.event_type.as_str())
    .bind(&spec.request_id)
    .bind(&spec.idempotency_key)
    .bind(created_at.to_rfc3339())
    .execute(&mut **tx)
    .await;

    if let Err(e) = insert {
        let msg = e.to_string();
        if msg.contains("UNIQUE") || msg.contains("unique") {
            if let Some(existing) =
                find_batch_by_idem_tx(tx, &spec.organisation_id, &spec.idempotency_key).await?
            {
                return Ok(existing);
            }
            return Err(TreasuryError::DuplicateIdempotency(spec.idempotency_key));
        }
        return Err(e.into());
    }

    for line in spec.lines.iter() {
        let entry_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO journal_entries
              (entry_id, batch_id, organisation_id, event_type, treasury_id, allocation_id,
               reservation_id, asset_id, account_code, debit_minor, credit_minor,
               request_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&entry_id)
        .bind(&batch_id)
        .bind(&spec.organisation_id)
        .bind(spec.event_type.as_str())
        .bind(&line.treasury_id)
        .bind(&line.allocation_id)
        .bind(&line.reservation_id)
        .bind(line.asset_id.as_str())
        .bind(&line.account_code)
        .bind(line.debit_minor)
        .bind(line.credit_minor)
        .bind(&spec.request_id)
        .bind(created_at.to_rfc3339())
        .execute(&mut **tx)
        .await?;

        let delta = line.debit_minor - line.credit_minor;
        apply_balance_delta_tx(
            tx,
            &spec.organisation_id,
            &line.account_code,
            line.asset_id.as_str(),
            delta,
            spec.enforce_non_negative_accounts,
        )
        .await?;
    }

    Ok(JournalBatch {
        batch_id,
        organisation_id: spec.organisation_id,
        event_type: spec.event_type,
        request_id: spec.request_id,
        idempotency_key: spec.idempotency_key,
        created_at,
    })
}

async fn find_batch_by_idem_tx(
    tx: &mut Transaction<'_, Sqlite>,
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
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(
        |(batch_id, organisation_id, event_type, request_id, idempotency_key, created_at)| {
            JournalBatch {
                batch_id,
                organisation_id,
                event_type: parse_event(&event_type),
                request_id,
                idempotency_key,
                created_at: created_at.parse().unwrap_or_else(|_| now()),
            }
        },
    ))
}

/// Post a balanced, append-only journal batch. Idempotent on (org, idempotency_key).
pub async fn post_batch(pool: &sqlx::SqlitePool, spec: BatchSpec<'_>) -> Result<JournalBatch> {
    validate_lines(spec.lines)?;

    if let Some(existing) =
        find_batch_by_idem(pool, &spec.organisation_id, &spec.idempotency_key).await?
    {
        return Ok(existing);
    }

    let mut tx = pool.begin().await?;
    let batch = post_batch_tx(&mut tx, spec).await?;
    tx.commit().await?;
    Ok(batch)
}
