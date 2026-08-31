//! Durable signed Apply operation store — immutable cryptographic records.

use chrono::Utc;
use sqlx::{Row, SqlitePool};

use super::errors::SignatureError;
use super::model::{SignatureStatus, SignedOperationV1};

/// SQLite-backed signed operation store.
#[derive(Clone)]
pub struct SignatureStore {
    pool: SqlitePool,
}

impl SignatureStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Insert a prepared signed operation (immutable payload fields).
    pub async fn insert(&self, record: &SignedOperationV1) -> Result<(), SignatureError> {
        let result = sqlx::query(
            r#"
            INSERT INTO signed_operations (
                operation_id, approval_id, dry_run_id, execution_hash,
                payload_hash, signature, signer_id, signer_role, purpose,
                created_at, expires_at, status, policy_id, policy_version,
                operation_intent, signature_algorithm, request_id,
                signer_identity, signed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.operation_id)
        .bind(&record.approval_id)
        .bind(&record.dry_run_id)
        .bind(&record.execution_hash)
        .bind(&record.payload_hash)
        .bind(&record.signature)
        .bind(&record.signer_id)
        .bind(&record.signer_role)
        .bind(&record.purpose)
        .bind(record.created_at.to_rfc3339())
        .bind(record.expires_at.to_rfc3339())
        .bind(record.status.as_str())
        .bind(&record.policy_id)
        .bind(record.policy_version)
        .bind(&record.operation_intent)
        .bind(&record.signature_algorithm)
        .bind(&record.request_id)
        .bind(&record.signer_identity)
        .bind(&record.signed_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                Err(SignatureError::DuplicateOperationId {
                    operation_id: record.operation_id.clone(),
                })
            }
            Err(e) => Err(SignatureError::Database(e.to_string())),
        }
    }

    /// Load by operation_id; lazily expires past `expires_at`.
    pub async fn get(
        &self,
        operation_id: &str,
    ) -> Result<Option<SignedOperationV1>, SignatureError> {
        let mut record = match fetch(self.pool(), operation_id).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        if matches!(
            record.status,
            SignatureStatus::Prepared | SignatureStatus::Valid
        ) && Utc::now() >= record.expires_at
        {
            self.mark_expired(operation_id).await?;
            record = fetch(self.pool(), operation_id).await?.ok_or_else(|| {
                SignatureError::NotFound {
                    operation_id: operation_id.to_string(),
                }
            })?;
        }
        Ok(Some(record))
    }

    /// Transition `prepared` → `valid` only.
    pub async fn mark_valid(
        &self,
        operation_id: &str,
    ) -> Result<SignedOperationV1, SignatureError> {
        let rows = sqlx::query(
            r#"
            UPDATE signed_operations
            SET status = 'valid'
            WHERE operation_id = ? AND status = 'prepared'
            "#,
        )
        .bind(operation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| SignatureError::Database(e.to_string()))?
        .rows_affected();

        if rows == 0 {
            let current =
                self.get(operation_id)
                    .await?
                    .ok_or_else(|| SignatureError::NotFound {
                        operation_id: operation_id.to_string(),
                    })?;
            if current.status == SignatureStatus::Valid {
                return Ok(current);
            }
            return Err(SignatureError::InvalidTransition {
                from: current.status,
                to: SignatureStatus::Valid,
            });
        }

        self.get(operation_id)
            .await?
            .ok_or_else(|| SignatureError::NotFound {
                operation_id: operation_id.to_string(),
            })
    }

    async fn mark_expired(&self, operation_id: &str) -> Result<(), SignatureError> {
        sqlx::query(
            r#"
            UPDATE signed_operations
            SET status = 'expired'
            WHERE operation_id = ?
              AND status IN ('prepared', 'valid')
            "#,
        )
        .bind(operation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| SignatureError::Database(e.to_string()))?;
        Ok(())
    }
}

async fn fetch(
    pool: &SqlitePool,
    operation_id: &str,
) -> Result<Option<SignedOperationV1>, SignatureError> {
    let row = sqlx::query(
        r#"
        SELECT operation_id, approval_id, dry_run_id, execution_hash,
               payload_hash, signature, signer_id, signer_role, purpose,
               created_at, expires_at, status, policy_id, policy_version,
               operation_intent, signature_algorithm, request_id,
               signer_identity, signed_at
        FROM signed_operations
        WHERE operation_id = ?
        "#,
    )
    .bind(operation_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| SignatureError::Database(e.to_string()))?;

    row.map(map_row).transpose()
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<SignedOperationV1, SignatureError> {
    let status_str: String = row.try_get("status").map_err(db_err)?;
    let status = SignatureStatus::parse(&status_str)
        .ok_or_else(|| SignatureError::Database(format!("unknown status: {status_str}")))?;

    Ok(SignedOperationV1 {
        operation_id: row.try_get("operation_id").map_err(db_err)?,
        approval_id: row.try_get("approval_id").map_err(db_err)?,
        dry_run_id: row.try_get("dry_run_id").map_err(db_err)?,
        execution_hash: row.try_get("execution_hash").map_err(db_err)?,
        policy_id: row.try_get("policy_id").map_err(db_err)?,
        policy_version: row.try_get("policy_version").map_err(db_err)?,
        operation_intent: row.try_get("operation_intent").map_err(db_err)?,
        signer_id: row.try_get("signer_id").map_err(db_err)?,
        signer_role: row.try_get("signer_role").map_err(db_err)?,
        signer_identity: row.try_get("signer_identity").map_err(db_err)?,
        signed_at: row.try_get("signed_at").map_err(db_err)?,
        created_at: parse_ts(row.try_get("created_at").map_err(db_err)?),
        expires_at: parse_ts(row.try_get("expires_at").map_err(db_err)?),
        signature_algorithm: row.try_get("signature_algorithm").map_err(db_err)?,
        signature: row.try_get("signature").map_err(db_err)?,
        purpose: row.try_get("purpose").map_err(db_err)?,
        payload_hash: row.try_get("payload_hash").map_err(db_err)?,
        request_id: row.try_get("request_id").map_err(db_err)?,
        status,
    })
}

fn parse_ts(s: &str) -> chrono::DateTime<Utc> {
    s.parse().unwrap_or_else(|_| Utc::now())
}

fn db_err(e: impl std::fmt::Display) -> SignatureError {
    SignatureError::Database(e.to_string())
}
