//! Apply replay reconciliation and recovery primitives (Phase 11 / C6).
//!
//! Rules:
//! - never blindly retry PROTO-0
//! - never assume success after timeout
//! - PROTO-0 / observation state is source of truth for success
//! - admin abort only finalises as aborted (no mutation)

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::cleanup::{sweep_executing_timeouts, sweep_reserved_timeouts};
use super::model::{ReplayError, ReplayRecord, ReplayStatus};
use super::store::ReplayStore;
use crate::db::audit;

/// Snapshot of non-terminal / recoverable replay rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub scanned_at: DateTime<Utc>,
    pub reserved: Vec<ReplayRecord>,
    pub executing: Vec<ReplayRecord>,
    pub stuck: Vec<ReplayRecord>,
    pub reserved_timed_out: u64,
    pub executing_timed_out: u64,
}

impl ReconciliationReport {
    pub fn non_terminal_count(&self) -> usize {
        self.reserved.len() + self.executing.len() + self.stuck.len()
    }
}

/// Fetch a replay row by `operation_id`.
pub async fn get(
    store: &ReplayStore,
    operation_id: &str,
) -> Result<Option<ReplayRecord>, ReplayError> {
    store.fetch_record(operation_id).await
}

/// List all replay rows in `stuck` status.
pub async fn list_stuck(store: &ReplayStore) -> Result<Vec<ReplayRecord>, ReplayError> {
    list_by_status(store, ReplayStatus::Stuck).await
}

/// List reserved rows.
pub async fn list_reserved(store: &ReplayStore) -> Result<Vec<ReplayRecord>, ReplayError> {
    list_by_status(store, ReplayStatus::Reserved).await
}

/// List executing rows.
pub async fn list_executing(store: &ReplayStore) -> Result<Vec<ReplayRecord>, ReplayError> {
    list_by_status(store, ReplayStatus::Executing).await
}

/// List all terminal replay rows (`executed`, `rejected`, `aborted`, `stuck`).
pub async fn list_terminal(store: &ReplayStore) -> Result<Vec<ReplayRecord>, ReplayError> {
    let rows = sqlx::query(
        r#"
        SELECT operation_id, execution_hash, dry_run_id, status,
               created_at, updated_at, reserved_at,
               terminal_reason, audit_reference, finalised_at
        FROM signed_operation_replay
        WHERE status IN ('executed', 'rejected', 'aborted', 'stuck')
        ORDER BY updated_at ASC
        "#,
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ReplayError::Database(e.to_string()))?;

    rows.into_iter().map(map_row).collect()
}

/// Startup / periodic scan: detect timeouts and return outstanding work.
///
/// Does **not** retry PROTO-0. Timed-out reserved/executing rows become `stuck`.
pub async fn startup_scan(store: &ReplayStore) -> Result<ReconciliationReport, ReplayError> {
    let reserved_timed_out = sweep_reserved_timeouts(store).await?;
    let executing_timed_out = sweep_executing_timeouts(store).await?;

    let report = ReconciliationReport {
        scanned_at: Utc::now(),
        reserved: list_reserved(store).await?,
        executing: list_executing(store).await?,
        stuck: list_stuck(store).await?,
        reserved_timed_out,
        executing_timed_out,
    };

    let _ = audit::append(
        store.pool(),
        None,
        "APPLY_RECONCILE_SCAN",
        None,
        Some(serde_json::json!({
            "reserved": report.reserved.len(),
            "executing": report.executing.len(),
            "stuck": report.stuck.len(),
            "reserved_timed_out": reserved_timed_out,
            "executing_timed_out": executing_timed_out,
            "result": "scanned",
        })),
    )
    .await;

    Ok(report)
}

/// Admin-only recovery: mark a stuck (or reserved/executing) operation aborted.
///
/// Never retries PROTO-0. Never marks executed.
pub async fn admin_abort(
    store: &ReplayStore,
    operation_id: &str,
    admin_id: &str,
    reason: &str,
) -> Result<ReplayRecord, ReplayError> {
    let record = get(store, operation_id)
        .await?
        .ok_or_else(|| ReplayError::NotFound {
            operation_id: operation_id.to_string(),
        })?;

    match record.status {
        ReplayStatus::Stuck | ReplayStatus::Reserved | ReplayStatus::Executing => {}
        other => {
            return Err(ReplayError::InvalidTransition {
                from: other,
                to: ReplayStatus::Aborted,
            });
        }
    }

    let aborted = store
        .mark_aborted(
            operation_id,
            Some(format!("admin_abort:{reason}")),
            Some(format!("admin:{admin_id}")),
        )
        .await?;

    let _ = audit::append(
        store.pool(),
        Some(admin_id),
        "APPLY_RECONCILED",
        Some(operation_id),
        Some(serde_json::json!({
            "operation_id": operation_id,
            "actor": admin_id,
            "from_status": record.status.as_str(),
            "to_status": "aborted",
            "reason": reason,
            "result": "aborted",
        })),
    )
    .await;

    Ok(aborted)
}

async fn list_by_status(
    store: &ReplayStore,
    status: ReplayStatus,
) -> Result<Vec<ReplayRecord>, ReplayError> {
    let rows = sqlx::query(
        r#"
        SELECT operation_id, execution_hash, dry_run_id, status,
               created_at, updated_at, reserved_at,
               terminal_reason, audit_reference, finalised_at
        FROM signed_operation_replay
        WHERE status = ?
        ORDER BY reserved_at ASC
        "#,
    )
    .bind(status.as_str())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ReplayError::Database(e.to_string()))?;

    rows.into_iter().map(map_row).collect()
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<ReplayRecord, ReplayError> {
    let status_str: String = row.try_get("status").map_err(db_err)?;
    let status = ReplayStatus::parse(&status_str)
        .ok_or_else(|| ReplayError::Database(format!("unknown replay status: {status_str}")))?;

    Ok(ReplayRecord {
        operation_id: row.try_get("operation_id").map_err(db_err)?,
        execution_hash: row.try_get("execution_hash").map_err(db_err)?,
        dry_run_id: row.try_get("dry_run_id").map_err(db_err)?,
        status,
        created_at: parse_ts(row.try_get("created_at").map_err(db_err)?),
        updated_at: parse_ts(row.try_get("updated_at").map_err(db_err)?),
        reserved_at: parse_ts(row.try_get("reserved_at").map_err(db_err)?),
        terminal_reason: row.try_get("terminal_reason").map_err(db_err)?,
        audit_reference: row.try_get("audit_reference").map_err(db_err)?,
        finalised_at: row
            .try_get::<Option<String>, _>("finalised_at")
            .map_err(db_err)?
            .map(|s| parse_ts(&s)),
    })
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    s.parse().unwrap_or_else(|_| Utc::now())
}

fn db_err(e: impl std::fmt::Display) -> ReplayError {
    ReplayError::Database(e.to_string())
}

/// Helper for cutoff calculations in tests.
#[allow(dead_code)]
pub fn cutoff_before(timeout: std::time::Duration) -> DateTime<Utc> {
    Utc::now() - ChronoDuration::from_std(timeout).unwrap_or_else(|_| ChronoDuration::minutes(5))
}
