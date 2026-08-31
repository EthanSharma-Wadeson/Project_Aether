use chrono::Utc;
use serde::Serialize;
use sqlx::Row;

use crate::config::Config;
use crate::treasury::write::errors::TreasuryWriteError;
use crate::treasury::TreasuryAdapter;

#[derive(Debug, Clone, Serialize)]
pub struct TreasuryOpsMetrics {
    pub organisation_id: String,
    pub as_of: String,
    pub reservation_age_max_secs: i64,
    pub expired_reservations: i64,
    pub due_active_reservations: i64,
    pub stuck_reservations: i64,
    pub stuck_operations: i64,
    pub audit_gap_count: i64,
    pub failed_mutations: i64,
    pub approval_latency_avg_secs: Option<f64>,
    pub execution_latency_avg_secs: Option<f64>,
    pub active_reservations: i64,
    pub ledger_notice: &'static str,
}

pub async fn collect_metrics(
    adapter: &TreasuryAdapter,
    pool: &sqlx::SqlitePool,
    config: &Config,
) -> Result<TreasuryOpsMetrics, TreasuryWriteError> {
    let org = adapter.organisation_id();
    let counts = adapter
        .engine()
        .ops_reservation_counts(org)
        .await
        .map_err(TreasuryWriteError::from)?;

    let max_age: Option<String> = sqlx::query_scalar(
        r#"
        SELECT created_at FROM reservations
        WHERE organisation_id = ? AND status IN ('active', 'expired', 'stuck')
        ORDER BY created_at ASC LIMIT 1
        "#,
    )
    .bind(org)
    .fetch_optional(adapter.engine().db().pool())
    .await
    .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

    let reservation_age_max_secs = max_age
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| {
            let age = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
            age.num_seconds().max(0)
        })
        .unwrap_or(0);

    let stuck_ops: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM treasury_mutation_requests
        WHERE organisation_id = ? AND status = 'executing'
        "#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

    let failed_mutations: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM treasury_mutation_requests
        WHERE organisation_id = ? AND status = 'failed'
        "#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

    let audit_gap_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM treasury_mutation_audit
        WHERE organisation_id = ? AND outcome = 'audit_gap'
        "#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

    let approval_latency_avg_secs = avg_latency_secs(
        pool,
        org,
        "SELECT approved_at, created_at FROM treasury_mutation_requests
         WHERE organisation_id = ? AND approved_at IS NOT NULL
         ORDER BY created_at DESC LIMIT 100",
    )
    .await?;

    let execution_latency_avg_secs = avg_latency_secs(
        pool,
        org,
        "SELECT executed_at, approved_at FROM treasury_mutation_requests
         WHERE organisation_id = ? AND executed_at IS NOT NULL AND approved_at IS NOT NULL
         ORDER BY created_at DESC LIMIT 100",
    )
    .await?;

    let _ = config;
    Ok(TreasuryOpsMetrics {
        organisation_id: org.into(),
        as_of: Utc::now().to_rfc3339(),
        reservation_age_max_secs,
        expired_reservations: counts.expired,
        due_active_reservations: counts.due_active,
        stuck_reservations: counts.stuck,
        stuck_operations: stuck_ops,
        audit_gap_count,
        failed_mutations,
        approval_latency_avg_secs,
        execution_latency_avg_secs,
        active_reservations: counts.active,
        ledger_notice: crate::treasury::write::models::LEDGER_NOTICE,
    })
}

async fn avg_latency_secs(
    pool: &sqlx::SqlitePool,
    org: &str,
    sql: &str,
) -> Result<Option<f64>, TreasuryWriteError> {
    let rows = sqlx::query(sql)
        .bind(org)
        .fetch_all(pool)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
    if rows.is_empty() {
        return Ok(None);
    }
    let mut total = 0.0;
    let mut n = 0.0;
    for row in rows {
        let end: String = row.get(0);
        let start: String = row.get(1);
        if let (Ok(e), Ok(s)) = (
            chrono::DateTime::parse_from_rfc3339(&end),
            chrono::DateTime::parse_from_rfc3339(&start),
        ) {
            total += (e.with_timezone(&Utc) - s.with_timezone(&Utc))
                .num_seconds()
                .max(0) as f64;
            n += 1.0;
        }
    }
    if n == 0.0 {
        Ok(None)
    } else {
        Ok(Some(total / n))
    }
}
