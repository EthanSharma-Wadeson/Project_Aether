use chrono::{Duration, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::auth::middleware::AuthContext;

use super::errors::TreasuryWriteError;
use super::models::{
    CreateMutationBody, MutationRequest, MutationStatus, TreasuryOperation,
};

pub const DEFAULT_MUTATION_TTL_MINS: i64 = 15;

pub fn payload_hash(payload: &Value) -> String {
    let bytes = serde_json::to_vec(payload).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", hex::encode(digest))
}

#[derive(Clone)]
pub struct MutationStore {
    pool: SqlitePool,
}

impl MutationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create_or_get_idempotent(
        &self,
        org: &str,
        ctx: &AuthContext,
        request_id: &str,
        body: &CreateMutationBody,
    ) -> Result<(MutationRequest, bool), TreasuryWriteError> {
        if body.idempotency_key.trim().is_empty() {
            return Err(TreasuryWriteError::BadRequest(
                "idempotency_key required".into(),
            ));
        }
        if let Some(existing) = self
            .get_by_idempotency(org, &body.idempotency_key)
            .await?
        {
            return Ok((existing, true));
        }

        let mutation_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::minutes(DEFAULT_MUTATION_TTL_MINS);
        let hash = payload_hash(&body.payload);
        let target = body
            .target
            .clone()
            .or_else(|| body.payload.get("treasury_id").and_then(|v| v.as_str()).map(str::to_string))
            .or_else(|| {
                body.payload
                    .get("allocation_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            });

        let result = sqlx::query(
            r#"
            INSERT INTO treasury_mutation_requests (
              mutation_id, organisation_id, request_id, idempotency_key, operation,
              payload_json, payload_hash, target, requested_by, requester_role, status,
              approval_id, approved_by, approved_at, executed_at, journal_batch_id,
              outcome, expires_at, created_at, exec_idempotency_key
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, ?, ?, NULL)
            "#,
        )
        .bind(&mutation_id)
        .bind(org)
        .bind(request_id)
        .bind(&body.idempotency_key)
        .bind(body.operation.as_str())
        .bind(body.payload.to_string())
        .bind(&hash)
        .bind(&target)
        .bind(&ctx.operator_id)
        .bind(ctx.role.as_str())
        .bind(MutationStatus::Pending.as_str())
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok((
                MutationRequest {
                    mutation_id,
                    organisation_id: org.into(),
                    request_id: request_id.into(),
                    idempotency_key: body.idempotency_key.clone(),
                    operation: body.operation,
                    payload: body.payload.clone(),
                    payload_hash: hash,
                    target,
                    requested_by: ctx.operator_id.clone(),
                    requester_role: ctx.role.as_str().to_string(),
                    status: MutationStatus::Pending,
                    approval_id: None,
                    approved_by: None,
                    approved_at: None,
                    executed_at: None,
                    journal_batch_id: None,
                    outcome: None,
                    expires_at,
                    created_at: now,
                    exec_idempotency_key: None,
                },
                false,
            )),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                let existing = self
                    .get_by_idempotency(org, &body.idempotency_key)
                    .await?
                    .ok_or_else(|| {
                        TreasuryWriteError::Conflict("idempotency race".into())
                    })?;
                Ok((existing, true))
            }
            Err(e) => Err(TreasuryWriteError::Unavailable(e.to_string())),
        }
    }

    pub async fn get(
        &self,
        org: &str,
        mutation_id: &str,
    ) -> Result<MutationRequest, TreasuryWriteError> {
        let row = sqlx::query(
            r#"
            SELECT mutation_id, organisation_id, request_id, idempotency_key, operation,
                   payload_json, payload_hash, target, requested_by, requester_role, status,
                   approval_id, approved_by, approved_at, executed_at, journal_batch_id,
                   outcome, expires_at, created_at, exec_idempotency_key
            FROM treasury_mutation_requests
            WHERE mutation_id = ? AND organisation_id = ?
            "#,
        )
        .bind(mutation_id)
        .bind(org)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        let Some(row) = row else {
            return Err(TreasuryWriteError::NotFound("mutation not found".into()));
        };
        map_row(row)
    }

    pub async fn get_by_idempotency(
        &self,
        org: &str,
        key: &str,
    ) -> Result<Option<MutationRequest>, TreasuryWriteError> {
        let row = sqlx::query(
            r#"
            SELECT mutation_id, organisation_id, request_id, idempotency_key, operation,
                   payload_json, payload_hash, target, requested_by, requester_role, status,
                   approval_id, approved_by, approved_at, executed_at, journal_batch_id,
                   outcome, expires_at, created_at, exec_idempotency_key
            FROM treasury_mutation_requests
            WHERE organisation_id = ? AND idempotency_key = ?
            "#,
        )
        .bind(org)
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
        row.map(map_row).transpose()
    }

    pub async fn list(
        &self,
        org: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MutationRequest>, TreasuryWriteError> {
        let rows = if let Some(st) = status {
            sqlx::query(
                r#"
                SELECT mutation_id, organisation_id, request_id, idempotency_key, operation,
                       payload_json, payload_hash, target, requested_by, requester_role, status,
                       approval_id, approved_by, approved_at, executed_at, journal_batch_id,
                       outcome, expires_at, created_at, exec_idempotency_key
                FROM treasury_mutation_requests
                WHERE organisation_id = ? AND status = ?
                ORDER BY created_at DESC LIMIT ?
                "#,
            )
            .bind(org)
            .bind(st)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                SELECT mutation_id, organisation_id, request_id, idempotency_key, operation,
                       payload_json, payload_hash, target, requested_by, requester_role, status,
                       approval_id, approved_by, approved_at, executed_at, journal_batch_id,
                       outcome, expires_at, created_at, exec_idempotency_key
                FROM treasury_mutation_requests
                WHERE organisation_id = ?
                ORDER BY created_at DESC LIMIT ?
                "#,
            )
            .bind(org)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        rows.into_iter().map(map_row).collect()
    }

    pub async fn approve(
        &self,
        org: &str,
        mutation_id: &str,
        ctx: &AuthContext,
        reason: Option<&str>,
        request_id: &str,
    ) -> Result<MutationRequest, TreasuryWriteError> {
        let mut m = self.get(org, mutation_id).await?;
        if m.status != MutationStatus::Pending {
            return Err(TreasuryWriteError::Conflict(format!(
                "mutation not pending: {}",
                m.status.as_str()
            )));
        }
        if m.expires_at <= Utc::now() {
            let _ = self
                .set_status(org, mutation_id, MutationStatus::Failed, Some("expired"))
                .await;
            return Err(TreasuryWriteError::Conflict(
                "mutation approval expired".into(),
            ));
        }
        if m.requested_by == ctx.operator_id {
            return Err(TreasuryWriteError::Forbidden(
                "Separation of duties: cannot approve own request".into(),
            ));
        }

        let approval_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::minutes(DEFAULT_MUTATION_TTL_MINS);

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        let updated = sqlx::query(
            r#"
            UPDATE treasury_mutation_requests
            SET status = ?, approval_id = ?, approved_by = ?, approved_at = ?, expires_at = ?
            WHERE mutation_id = ? AND organisation_id = ? AND status = 'pending'
            "#,
        )
        .bind(MutationStatus::Approved.as_str())
        .bind(&approval_id)
        .bind(&ctx.operator_id)
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .bind(mutation_id)
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        if updated.rows_affected() != 1 {
            return Err(TreasuryWriteError::Conflict(
                "concurrent approval conflict".into(),
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO treasury_mutation_approvals (
              approval_id, mutation_id, organisation_id, approved_by, approver_role,
              decision, reason, payload_hash, status, created_at, expires_at, consumed_at, request_id
            ) VALUES (?, ?, ?, ?, ?, 'approve', ?, ?, 'active', ?, ?, NULL, ?)
            "#,
        )
        .bind(&approval_id)
        .bind(mutation_id)
        .bind(org)
        .bind(&ctx.operator_id)
        .bind(ctx.role.as_str())
        .bind(reason)
        .bind(&m.payload_hash)
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        m = self.get(org, mutation_id).await?;
        Ok(m)
    }

    pub async fn reject(
        &self,
        org: &str,
        mutation_id: &str,
        ctx: &AuthContext,
        reason: Option<&str>,
        request_id: &str,
    ) -> Result<MutationRequest, TreasuryWriteError> {
        let m = self.get(org, mutation_id).await?;
        if m.status != MutationStatus::Pending {
            return Err(TreasuryWriteError::Conflict("mutation not pending".into()));
        }
        if m.requested_by == ctx.operator_id {
            return Err(TreasuryWriteError::Forbidden(
                "Separation of duties: cannot reject own request as sole control".into(),
            ));
        }
        let approval_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
        let updated = sqlx::query(
            r#"
            UPDATE treasury_mutation_requests
            SET status = ?, approval_id = ?, approved_by = ?, approved_at = ?, outcome = ?
            WHERE mutation_id = ? AND organisation_id = ? AND status = 'pending'
            "#,
        )
        .bind(MutationStatus::Rejected.as_str())
        .bind(&approval_id)
        .bind(&ctx.operator_id)
        .bind(now.to_rfc3339())
        .bind("rejected")
        .bind(mutation_id)
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
        if updated.rows_affected() != 1 {
            return Err(TreasuryWriteError::Conflict("concurrent reject conflict".into()));
        }
        sqlx::query(
            r#"
            INSERT INTO treasury_mutation_approvals (
              approval_id, mutation_id, organisation_id, approved_by, approver_role,
              decision, reason, payload_hash, status, created_at, expires_at, consumed_at, request_id
            ) VALUES (?, ?, ?, ?, ?, 'reject', ?, ?, 'consumed', ?, ?, ?, ?)
            "#,
        )
        .bind(&approval_id)
        .bind(mutation_id)
        .bind(org)
        .bind(&ctx.operator_id)
        .bind(ctx.role.as_str())
        .bind(reason)
        .bind(&m.payload_hash)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
        self.get(org, mutation_id).await
    }

    pub async fn cancel(
        &self,
        org: &str,
        mutation_id: &str,
        ctx: &AuthContext,
    ) -> Result<MutationRequest, TreasuryWriteError> {
        let m = self.get(org, mutation_id).await?;
        if m.status != MutationStatus::Pending {
            return Err(TreasuryWriteError::Conflict("only pending can cancel".into()));
        }
        if m.requested_by != ctx.operator_id && ctx.role.as_str() != "admin" {
            return Err(TreasuryWriteError::Forbidden(
                "only requester or admin may cancel".into(),
            ));
        }
        self.set_status(org, mutation_id, MutationStatus::Cancelled, Some("cancelled"))
            .await?;
        self.get(org, mutation_id).await
    }

    pub async fn begin_execute(
        &self,
        org: &str,
        mutation_id: &str,
        exec_key: &str,
    ) -> Result<MutationRequest, TreasuryWriteError> {
        let m = self.get(org, mutation_id).await?;
        if m.status == MutationStatus::Executed {
            if m.exec_idempotency_key.as_deref() == Some(exec_key) {
                return Ok(m);
            }
            return Err(TreasuryWriteError::Conflict(
                "mutation already executed".into(),
            ));
        }
        if m.status != MutationStatus::Approved {
            return Err(TreasuryWriteError::Conflict(format!(
                "mutation not approved: {}",
                m.status.as_str()
            )));
        }
        if m.expires_at <= Utc::now() {
            let _ = self
                .set_status(org, mutation_id, MutationStatus::Failed, Some("approval_expired"))
                .await;
            return Err(TreasuryWriteError::Conflict(
                "mutation approval expired".into(),
            ));
        }

        let updated = sqlx::query(
            r#"
            UPDATE treasury_mutation_requests
            SET status = ?, exec_idempotency_key = ?
            WHERE mutation_id = ? AND organisation_id = ? AND status = 'approved'
            "#,
        )
        .bind(MutationStatus::Executing.as_str())
        .bind(exec_key)
        .bind(mutation_id)
        .bind(org)
        .execute(&self.pool)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        if updated.rows_affected() != 1 {
            // Concurrent execute — if other finished with same key, return it.
            let again = self.get(org, mutation_id).await?;
            if again.status == MutationStatus::Executed
                && again.exec_idempotency_key.as_deref() == Some(exec_key)
            {
                return Ok(again);
            }
            if again.status == MutationStatus::Executing
                && again.exec_idempotency_key.as_deref() == Some(exec_key)
            {
                return Err(TreasuryWriteError::Conflict(
                    "concurrent execution in progress".into(),
                ));
            }
            return Err(TreasuryWriteError::Conflict(
                "concurrent execution conflict".into(),
            ));
        }

        if let Some(aid) = &m.approval_id {
            sqlx::query(
                r#"
                UPDATE treasury_mutation_approvals
                SET status = 'consumed', consumed_at = ?
                WHERE approval_id = ? AND status = 'active'
                "#,
            )
            .bind(Utc::now().to_rfc3339())
            .bind(aid)
            .execute(&self.pool)
            .await
            .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
        }

        self.get(org, mutation_id).await
    }

    pub async fn finish_execute(
        &self,
        org: &str,
        mutation_id: &str,
        journal_batch_id: Option<&str>,
        outcome: &str,
        ok: bool,
    ) -> Result<MutationRequest, TreasuryWriteError> {
        let status = if ok {
            MutationStatus::Executed
        } else {
            MutationStatus::Failed
        };
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE treasury_mutation_requests
            SET status = ?, executed_at = ?, journal_batch_id = ?, outcome = ?
            WHERE mutation_id = ? AND organisation_id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(now.to_rfc3339())
        .bind(journal_batch_id)
        .bind(outcome)
        .bind(mutation_id)
        .bind(org)
        .execute(&self.pool)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
        self.get(org, mutation_id).await
    }

    async fn set_status(
        &self,
        org: &str,
        mutation_id: &str,
        status: MutationStatus,
        outcome: Option<&str>,
    ) -> Result<(), TreasuryWriteError> {
        sqlx::query(
            r#"
            UPDATE treasury_mutation_requests
            SET status = ?, outcome = COALESCE(?, outcome)
            WHERE mutation_id = ? AND organisation_id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(outcome)
        .bind(mutation_id)
        .bind(org)
        .execute(&self.pool)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
        Ok(())
    }
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<MutationRequest, TreasuryWriteError> {
    let op_s: String = row.get("operation");
    let st_s: String = row.get("status");
    let payload_s: String = row.get("payload_json");
    let payload: Value = serde_json::from_str(&payload_s)
        .map_err(|e| TreasuryWriteError::BadRequest(e.to_string()))?;
    let parse_dt = |key: &str| -> Result<DateTimeWrapper, TreasuryWriteError> {
        let s: String = row.get(key);
        DateTimeWrapper::parse(&s)
    };
    let parse_opt_dt = |key: &str| -> Result<Option<chrono::DateTime<Utc>>, TreasuryWriteError> {
        let s: Option<String> = row.get(key);
        match s {
            Some(v) => Ok(Some(DateTimeWrapper::parse(&v)?.0)),
            None => Ok(None),
        }
    };

    Ok(MutationRequest {
        mutation_id: row.get("mutation_id"),
        organisation_id: row.get("organisation_id"),
        request_id: row.get("request_id"),
        idempotency_key: row.get("idempotency_key"),
        operation: TreasuryOperation::parse(&op_s)
            .ok_or_else(|| TreasuryWriteError::BadRequest(format!("bad op {op_s}")))?,
        payload,
        payload_hash: row.get("payload_hash"),
        target: row.get("target"),
        requested_by: row.get("requested_by"),
        requester_role: row.get("requester_role"),
        status: MutationStatus::parse(&st_s)
            .ok_or_else(|| TreasuryWriteError::BadRequest(format!("bad status {st_s}")))?,
        approval_id: row.get("approval_id"),
        approved_by: row.get("approved_by"),
        approved_at: parse_opt_dt("approved_at")?,
        executed_at: parse_opt_dt("executed_at")?,
        journal_batch_id: row.get("journal_batch_id"),
        outcome: row.get("outcome"),
        expires_at: parse_dt("expires_at")?.0,
        created_at: parse_dt("created_at")?.0,
        exec_idempotency_key: row.get("exec_idempotency_key"),
    })
}

struct DateTimeWrapper(chrono::DateTime<Utc>);

impl DateTimeWrapper {
    fn parse(s: &str) -> Result<Self, TreasuryWriteError> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| Self(d.with_timezone(&Utc)))
            .map_err(|e| TreasuryWriteError::BadRequest(e.to_string()))
    }
}
