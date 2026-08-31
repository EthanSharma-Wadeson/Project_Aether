//! Durable replay store — SQLite-backed one-shot execution guard.

use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use super::model::{
    allowed_transition, AtomicPrepareRequest, ReplayError, ReplayRecord, ReplayStatus,
    ReserveRequest,
};

/// Default reserve timeout before `reserved` → `stuck` (REPLAY_RESERVE_TTL).
pub const DEFAULT_RESERVE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Replay store — enforces at-most-once Apply execution per `operation_id`.
#[derive(Clone)]
pub struct ReplayStore {
    pool: SqlitePool,
    reserve_timeout: Duration,
}

impl ReplayStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            reserve_timeout: DEFAULT_RESERVE_TIMEOUT,
        }
    }

    pub fn with_reserve_timeout(pool: SqlitePool, reserve_timeout: Duration) -> Self {
        Self {
            pool,
            reserve_timeout,
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn reserve_timeout(&self) -> Duration {
        self.reserve_timeout
    }

    /// Insert a new replay row (`NotPresent` → `Reserved`).
    ///
    /// Duplicate `operation_id` fails deterministically with [`ReplayError::Duplicate`] or
    /// [`ReplayError::InProgress`].
    pub async fn reserve(&self, request: ReserveRequest) -> Result<ReplayRecord, ReplayError> {
        let mut conn = self.pool.acquire().await.map_err(db_err)?;
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *conn)
            .await
            .map_err(db_err)?;

        if let Some(existing) = fetch_record_conn(&mut *conn, &request.operation_id).await? {
            sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
            return Err(classify_duplicate(existing));
        }

        let now = Utc::now();
        let ts = now.to_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO signed_operation_replay (
                operation_id, execution_hash, dry_run_id, status,
                created_at, updated_at, reserved_at,
                terminal_reason, audit_reference, finalised_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL)
            "#,
        )
        .bind(&request.operation_id)
        .bind(&request.execution_hash)
        .bind(&request.dry_run_id)
        .bind(ReplayStatus::Reserved.as_str())
        .bind(&ts)
        .bind(&ts)
        .bind(&ts)
        .execute(&mut *conn)
        .await;

        match result {
            Ok(_) => {}
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                if let Some(existing) = fetch_record_conn(&mut *conn, &request.operation_id).await?
                {
                    sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
                    return Err(classify_duplicate(existing));
                }
            }
            Err(e) => {
                sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
                return Err(db_err(e));
            }
        }

        sqlx::query("COMMIT")
            .execute(&mut *conn)
            .await
            .map_err(db_err)?;

        Ok(ReplayRecord {
            operation_id: request.operation_id,
            execution_hash: request.execution_hash,
            dry_run_id: request.dry_run_id,
            status: ReplayStatus::Reserved,
            created_at: now,
            updated_at: now,
            reserved_at: now,
            terminal_reason: None,
            audit_reference: None,
            finalised_at: None,
        })
    }

    /// Atomically reserve a replay slot and consume the approval (C2).
    ///
    /// Single `BEGIN IMMEDIATE` transaction:
    /// - INSERT replay (`reserved`)
    /// - UPDATE approval (`active` → `consumed`)
    ///
    /// On any failure: ROLLBACK — neither reserved nor consumed.
    /// No protocol mutation.
    pub async fn reserve_and_consume_approval(
        &self,
        request: AtomicPrepareRequest,
    ) -> Result<ReplayRecord, ReplayError> {
        let mut conn = self.pool.acquire().await.map_err(db_err)?;
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *conn)
            .await
            .map_err(db_err)?;

        if let Some(existing) = fetch_record_conn(&mut *conn, &request.operation_id).await? {
            sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
            return Err(classify_duplicate(existing));
        }

        let approval_row = sqlx::query(
            r#"
            SELECT status, expires_at FROM apply_approvals WHERE approval_id = ?
            "#,
        )
        .bind(&request.approval_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db_err)?;

        let Some(approval_row) = approval_row else {
            sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
            return Err(ReplayError::ApprovalConsumeFailed {
                message: format!("approval {} not found", request.approval_id),
            });
        };

        let status: String = approval_row.try_get("status").map_err(db_err)?;
        let expires_at_s: String = approval_row.try_get("expires_at").map_err(db_err)?;
        let expires_at = parse_ts(&expires_at_s);
        if status != "active" {
            sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
            return Err(ReplayError::ApprovalConsumeFailed {
                message: format!(
                    "approval {} not active (status={status})",
                    request.approval_id
                ),
            });
        }
        if Utc::now() >= expires_at {
            let _ = sqlx::query(
                "UPDATE apply_approvals SET status = 'expired' WHERE approval_id = ? AND status = 'active'",
            )
            .bind(&request.approval_id)
            .execute(&mut *conn)
            .await;
            sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
            return Err(ReplayError::ApprovalConsumeFailed {
                message: format!("approval {} expired", request.approval_id),
            });
        }

        let now = Utc::now();
        let ts = now.to_rfc3339();
        let insert = sqlx::query(
            r#"
            INSERT INTO signed_operation_replay (
                operation_id, execution_hash, dry_run_id, status,
                created_at, updated_at, reserved_at,
                terminal_reason, audit_reference, finalised_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL)
            "#,
        )
        .bind(&request.operation_id)
        .bind(&request.execution_hash)
        .bind(&request.dry_run_id)
        .bind(ReplayStatus::Reserved.as_str())
        .bind(&ts)
        .bind(&ts)
        .bind(&ts)
        .execute(&mut *conn)
        .await;

        match insert {
            Ok(_) => {}
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                if let Some(existing) = fetch_record_conn(&mut *conn, &request.operation_id).await?
                {
                    sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
                    return Err(classify_duplicate(existing));
                }
                sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
                return Err(ReplayError::Database("unique violation without row".into()));
            }
            Err(e) => {
                sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
                return Err(db_err(e));
            }
        }

        let consume = sqlx::query(
            r#"
            UPDATE apply_approvals
            SET status = 'consumed',
                consumed_at = ?,
                consumed_by_operation_id = ?
            WHERE approval_id = ? AND status = 'active'
            "#,
        )
        .bind(&ts)
        .bind(&request.operation_id)
        .bind(&request.approval_id)
        .execute(&mut *conn)
        .await;

        match consume {
            Ok(r) if r.rows_affected() == 1 => {}
            Ok(_) => {
                sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
                return Err(ReplayError::ApprovalConsumeFailed {
                    message: format!(
                        "approval {} could not be consumed (race or inactive)",
                        request.approval_id
                    ),
                });
            }
            Err(e) => {
                sqlx::query("ROLLBACK").execute(&mut *conn).await.ok();
                return Err(db_err(e));
            }
        }

        sqlx::query("COMMIT")
            .execute(&mut *conn)
            .await
            .map_err(db_err)?;

        Ok(ReplayRecord {
            operation_id: request.operation_id,
            execution_hash: request.execution_hash,
            dry_run_id: request.dry_run_id,
            status: ReplayStatus::Reserved,
            created_at: now,
            updated_at: now,
            reserved_at: now,
            terminal_reason: None,
            audit_reference: None,
            finalised_at: None,
        })
    }

    /// `Reserved` → `Executing`.
    pub async fn begin_execution(&self, operation_id: &str) -> Result<ReplayRecord, ReplayError> {
        self.transition(
            operation_id,
            ReplayStatus::Reserved,
            ReplayStatus::Executing,
            None,
            None,
            false,
        )
        .await
    }

    /// `Executing` → `Executed`.
    pub async fn mark_executed(
        &self,
        operation_id: &str,
        audit_reference: Option<String>,
    ) -> Result<ReplayRecord, ReplayError> {
        self.transition(
            operation_id,
            ReplayStatus::Executing,
            ReplayStatus::Executed,
            None,
            audit_reference,
            true,
        )
        .await
    }

    /// `Executing` → `Rejected`.
    pub async fn mark_rejected(
        &self,
        operation_id: &str,
        terminal_reason: Option<String>,
        audit_reference: Option<String>,
    ) -> Result<ReplayRecord, ReplayError> {
        self.transition(
            operation_id,
            ReplayStatus::Executing,
            ReplayStatus::Rejected,
            terminal_reason,
            audit_reference,
            true,
        )
        .await
    }

    /// `Reserved` | `Executing` | `Stuck` → `Aborted` (stuck only via admin reconcile).
    pub async fn mark_aborted(
        &self,
        operation_id: &str,
        terminal_reason: Option<String>,
        audit_reference: Option<String>,
    ) -> Result<ReplayRecord, ReplayError> {
        let record =
            self.fetch_record(operation_id)
                .await?
                .ok_or_else(|| ReplayError::NotFound {
                    operation_id: operation_id.to_string(),
                })?;

        let from = record.status;
        match from {
            ReplayStatus::Reserved | ReplayStatus::Executing | ReplayStatus::Stuck => {}
            ReplayStatus::Executed | ReplayStatus::Rejected | ReplayStatus::Aborted => {
                return Err(ReplayError::InvalidTransition {
                    from,
                    to: ReplayStatus::Aborted,
                });
            }
        }

        self.transition(
            operation_id,
            from,
            ReplayStatus::Aborted,
            terminal_reason,
            audit_reference,
            true,
        )
        .await
    }

    /// `Reserved` → `Stuck` (reserve timeout or explicit mark).
    pub async fn mark_stuck(
        &self,
        operation_id: &str,
        terminal_reason: Option<String>,
    ) -> Result<ReplayRecord, ReplayError> {
        self.transition(
            operation_id,
            ReplayStatus::Reserved,
            ReplayStatus::Stuck,
            terminal_reason.or_else(|| Some("reserve_timeout".into())),
            None,
            true,
        )
        .await
    }

    /// `Executing` → `Stuck` (execution timeout — never assumes PROTO-0 success).
    pub async fn mark_executing_stuck(
        &self,
        operation_id: &str,
        terminal_reason: Option<String>,
    ) -> Result<ReplayRecord, ReplayError> {
        self.transition(
            operation_id,
            ReplayStatus::Executing,
            ReplayStatus::Stuck,
            terminal_reason.or_else(|| Some("execution_timeout".into())),
            None,
            true,
        )
        .await
    }

    async fn transition(
        &self,
        operation_id: &str,
        expected_from: ReplayStatus,
        to: ReplayStatus,
        terminal_reason: Option<String>,
        audit_reference: Option<String>,
        set_finalised: bool,
    ) -> Result<ReplayRecord, ReplayError> {
        if !allowed_transition(expected_from, to) {
            return Err(ReplayError::InvalidTransition {
                from: expected_from,
                to,
            });
        }

        let now = Utc::now();
        let updated_at = now.to_rfc3339();
        let finalised_at = if set_finalised {
            Some(updated_at.clone())
        } else {
            None
        };

        let rows = sqlx::query(
            r#"
            UPDATE signed_operation_replay
            SET status = ?,
                updated_at = ?,
                terminal_reason = COALESCE(?, terminal_reason),
                audit_reference = COALESCE(?, audit_reference),
                finalised_at = COALESCE(?, finalised_at)
            WHERE operation_id = ? AND status = ?
            "#,
        )
        .bind(to.as_str())
        .bind(&updated_at)
        .bind(&terminal_reason)
        .bind(&audit_reference)
        .bind(&finalised_at)
        .bind(operation_id)
        .bind(expected_from.as_str())
        .execute(self.pool())
        .await
        .map_err(db_err)?
        .rows_affected();

        if rows == 0 {
            let record =
                self.fetch_record(operation_id)
                    .await?
                    .ok_or_else(|| ReplayError::NotFound {
                        operation_id: operation_id.to_string(),
                    })?;
            return Err(ReplayError::InvalidTransition {
                from: record.status,
                to,
            });
        }

        self.fetch_record(operation_id)
            .await?
            .ok_or_else(|| ReplayError::NotFound {
                operation_id: operation_id.to_string(),
            })
    }

    pub(crate) async fn fetch_record(
        &self,
        operation_id: &str,
    ) -> Result<Option<ReplayRecord>, ReplayError> {
        fetch_record_pool(self.pool(), operation_id).await
    }
}

fn classify_duplicate(record: ReplayRecord) -> ReplayError {
    match record.status {
        ReplayStatus::Reserved | ReplayStatus::Executing => ReplayError::InProgress {
            record: Box::new(record),
        },
        _ => ReplayError::Duplicate {
            record: Box::new(record),
        },
    }
}

async fn fetch_record_pool(
    pool: &SqlitePool,
    operation_id: &str,
) -> Result<Option<ReplayRecord>, ReplayError> {
    let row = sqlx::query(
        r#"
        SELECT operation_id, execution_hash, dry_run_id, status,
               created_at, updated_at, reserved_at,
               terminal_reason, audit_reference, finalised_at
        FROM signed_operation_replay
        WHERE operation_id = ?
        "#,
    )
    .bind(operation_id)
    .fetch_optional(pool)
    .await
    .map_err(db_err)?;

    row.map(map_row).transpose()
}

async fn fetch_record_conn<'e, E>(
    executor: E,
    operation_id: &str,
) -> Result<Option<ReplayRecord>, ReplayError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row = sqlx::query(
        r#"
        SELECT operation_id, execution_hash, dry_run_id, status,
               created_at, updated_at, reserved_at,
               terminal_reason, audit_reference, finalised_at
        FROM signed_operation_replay
        WHERE operation_id = ?
        "#,
    )
    .bind(operation_id)
    .fetch_optional(executor)
    .await
    .map_err(db_err)?;

    row.map(map_row).transpose()
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
