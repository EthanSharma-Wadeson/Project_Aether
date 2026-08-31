//! Durable Apply approval store — authorisation binding only.

use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use super::errors::ApprovalError;
use super::model::{
    ApplyApproval, ApprovalBinding, ApprovalStatus, CreateApprovalRequest, DEFAULT_APPROVAL_TTL,
};
use crate::apply::attestation::{
    AttestationBinding, AttestationError, AttestationErrorCode, AttestationStore,
};

/// SQLite-backed Apply approval store.
#[derive(Clone)]
pub struct ApprovalStore {
    pool: SqlitePool,
}

impl ApprovalStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Create an active approval bound to a valid dry-run attestation.
    pub async fn create_approval(
        &self,
        request: CreateApprovalRequest,
    ) -> Result<ApplyApproval, ApprovalError> {
        validate_create_inputs(&request)?;

        let attestations = AttestationStore::new(self.pool.clone());
        let attestation = attestations
            .get_valid_attestation(&AttestationBinding {
                dry_run_id: request.dry_run_id.clone(),
                execution_hash: request.execution_hash.clone(),
                policy_version: request.policy_version,
                operation_intent: request.operation_intent.clone(),
            })
            .await
            .map_err(map_attestation_error)?;

        if attestation.policy_id != request.policy_id {
            return Err(ApprovalError::PolicyIdMismatch);
        }

        // SoD: approver must not be the dry-run operator.
        if attestation.created_by == request.approved_by {
            return Err(ApprovalError::SeparationOfDuties {
                message: "approver must not be the dry-run operator".into(),
            });
        }

        let approval_id = request
            .approval_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();
        let ttl = request.ttl.unwrap_or(DEFAULT_APPROVAL_TTL);
        let expires_at = now + ttl;

        let result = sqlx::query(
            r#"
            INSERT INTO apply_approvals (
                approval_id, dry_run_id, execution_hash, policy_id, policy_version,
                operation_intent, approved_by, approved_at, expires_at, status,
                consumed_at, cancelled_at, audit_reference, request_id, approver_role,
                consumed_by_operation_id, cancelled_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, ?, ?, ?, NULL, NULL)
            "#,
        )
        .bind(&approval_id)
        .bind(&request.dry_run_id)
        .bind(&request.execution_hash)
        .bind(&request.policy_id)
        .bind(request.policy_version)
        .bind(&request.operation_intent)
        .bind(&request.approved_by)
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .bind(ApprovalStatus::Active.as_str())
        .bind(&request.audit_reference)
        .bind(&request.request_id)
        .bind(&request.approver_role)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(ApplyApproval {
                approval_id,
                dry_run_id: request.dry_run_id,
                execution_hash: request.execution_hash,
                policy_id: request.policy_id,
                policy_version: request.policy_version,
                operation_intent: request.operation_intent,
                approved_by: request.approved_by,
                approved_at: now,
                expires_at,
                status: ApprovalStatus::Active,
                consumed_at: None,
                cancelled_at: None,
                audit_reference: request.audit_reference,
                request_id: request.request_id,
                approver_role: request.approver_role,
                consumed_by_operation_id: None,
                cancelled_by: None,
            }),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                let msg = db.message().to_string();
                if msg.contains("idx_apply_approval_active")
                    || msg.contains("policy_id")
                    || msg.contains("dry_run_id")
                {
                    Err(ApprovalError::ActiveBindingExists)
                } else {
                    Err(ApprovalError::DuplicateApprovalId { approval_id })
                }
            }
            Err(e) => Err(ApprovalError::Database(e.to_string())),
        }
    }

    /// Load approval by id (any status). Lazy-expires Active past `expires_at`.
    pub async fn get_approval(
        &self,
        approval_id: &str,
    ) -> Result<Option<ApplyApproval>, ApprovalError> {
        let mut record = match fetch(self.pool(), approval_id).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        if record.status == ApprovalStatus::Active && Utc::now() >= record.expires_at {
            self.mark_expired(approval_id).await?;
            record =
                fetch(self.pool(), approval_id)
                    .await?
                    .ok_or_else(|| ApprovalError::NotFound {
                        approval_id: approval_id.to_string(),
                    })?;
        }
        Ok(Some(record))
    }

    /// Validate an active approval against a binding.
    pub async fn validate_approval(
        &self,
        binding: &ApprovalBinding,
    ) -> Result<ApplyApproval, ApprovalError> {
        let record = self
            .get_approval(&binding.approval_id)
            .await?
            .ok_or_else(|| ApprovalError::NotFound {
                approval_id: binding.approval_id.clone(),
            })?;

        if record.dry_run_id != binding.dry_run_id {
            return Err(ApprovalError::IntentMismatch);
        }
        if record.execution_hash != binding.execution_hash {
            return Err(ApprovalError::ExecutionHashMismatch);
        }
        if record.policy_version != binding.policy_version {
            return Err(ApprovalError::PolicyVersionMismatch {
                expected: binding.policy_version,
                actual: record.policy_version,
            });
        }

        match record.status {
            ApprovalStatus::Active => Ok(record),
            ApprovalStatus::Expired => Err(ApprovalError::Expired {
                approval_id: binding.approval_id.clone(),
            }),
            ApprovalStatus::Consumed => Err(ApprovalError::Consumed {
                approval_id: binding.approval_id.clone(),
            }),
            ApprovalStatus::Cancelled => Err(ApprovalError::Cancelled {
                approval_id: binding.approval_id.clone(),
            }),
        }
    }

    /// Consume an active approval (single-use). Concurrent consume: exactly one wins.
    pub async fn consume_approval(
        &self,
        approval_id: &str,
        consumed_by_operation_id: Option<&str>,
    ) -> Result<ApplyApproval, ApprovalError> {
        // Ensure lazy expiry applied first.
        let current =
            self.get_approval(approval_id)
                .await?
                .ok_or_else(|| ApprovalError::NotFound {
                    approval_id: approval_id.to_string(),
                })?;

        match current.status {
            ApprovalStatus::Active => {}
            ApprovalStatus::Expired => {
                return Err(ApprovalError::Expired {
                    approval_id: approval_id.to_string(),
                });
            }
            ApprovalStatus::Consumed => {
                return Err(ApprovalError::Consumed {
                    approval_id: approval_id.to_string(),
                });
            }
            ApprovalStatus::Cancelled => {
                return Err(ApprovalError::Cancelled {
                    approval_id: approval_id.to_string(),
                });
            }
        }

        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query(
            r#"
            UPDATE apply_approvals
            SET status = 'consumed',
                consumed_at = ?,
                consumed_by_operation_id = COALESCE(?, consumed_by_operation_id)
            WHERE approval_id = ? AND status = 'active'
            "#,
        )
        .bind(&now)
        .bind(consumed_by_operation_id)
        .bind(approval_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApprovalError::Database(e.to_string()))?
        .rows_affected();

        if rows == 0 {
            let again =
                self.get_approval(approval_id)
                    .await?
                    .ok_or_else(|| ApprovalError::NotFound {
                        approval_id: approval_id.to_string(),
                    })?;
            return Err(match again.status {
                ApprovalStatus::Consumed => ApprovalError::Consumed {
                    approval_id: approval_id.to_string(),
                },
                ApprovalStatus::Cancelled => ApprovalError::Cancelled {
                    approval_id: approval_id.to_string(),
                },
                ApprovalStatus::Expired => ApprovalError::Expired {
                    approval_id: approval_id.to_string(),
                },
                ApprovalStatus::Active => ApprovalError::InvalidTransition {
                    from: ApprovalStatus::Active,
                    to: ApprovalStatus::Consumed,
                },
            });
        }

        self.get_approval(approval_id)
            .await?
            .ok_or_else(|| ApprovalError::NotFound {
                approval_id: approval_id.to_string(),
            })
    }

    /// Cancel an active approval.
    pub async fn cancel_approval(
        &self,
        approval_id: &str,
        cancelled_by: &str,
    ) -> Result<ApplyApproval, ApprovalError> {
        let current =
            self.get_approval(approval_id)
                .await?
                .ok_or_else(|| ApprovalError::NotFound {
                    approval_id: approval_id.to_string(),
                })?;

        match current.status {
            ApprovalStatus::Active => {}
            ApprovalStatus::Expired => {
                return Err(ApprovalError::InvalidTransition {
                    from: ApprovalStatus::Expired,
                    to: ApprovalStatus::Cancelled,
                });
            }
            ApprovalStatus::Consumed => {
                return Err(ApprovalError::InvalidTransition {
                    from: ApprovalStatus::Consumed,
                    to: ApprovalStatus::Cancelled,
                });
            }
            ApprovalStatus::Cancelled => return Ok(current),
        }

        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query(
            r#"
            UPDATE apply_approvals
            SET status = 'cancelled',
                cancelled_at = ?,
                cancelled_by = ?
            WHERE approval_id = ? AND status = 'active'
            "#,
        )
        .bind(&now)
        .bind(cancelled_by)
        .bind(approval_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApprovalError::Database(e.to_string()))?
        .rows_affected();

        if rows == 0 {
            let again =
                self.get_approval(approval_id)
                    .await?
                    .ok_or_else(|| ApprovalError::NotFound {
                        approval_id: approval_id.to_string(),
                    })?;
            return Err(ApprovalError::InvalidTransition {
                from: again.status,
                to: ApprovalStatus::Cancelled,
            });
        }

        self.get_approval(approval_id)
            .await?
            .ok_or_else(|| ApprovalError::NotFound {
                approval_id: approval_id.to_string(),
            })
    }

    async fn mark_expired(&self, approval_id: &str) -> Result<(), ApprovalError> {
        sqlx::query(
            r#"
            UPDATE apply_approvals
            SET status = 'expired'
            WHERE approval_id = ? AND status = 'active'
            "#,
        )
        .bind(approval_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApprovalError::Database(e.to_string()))?;
        Ok(())
    }
}

fn validate_create_inputs(request: &CreateApprovalRequest) -> Result<(), ApprovalError> {
    if request.dry_run_id.trim().is_empty() {
        return Err(ApprovalError::MissingDryRunId);
    }
    if request.execution_hash.trim().is_empty() || request.execution_hash.len() != 64 {
        return Err(ApprovalError::MissingExecutionHash);
    }
    if request.policy_version < 1 {
        return Err(ApprovalError::InvalidPolicyVersion);
    }
    if request.policy_id.trim().is_empty() {
        return Err(ApprovalError::Database("policy_id required".into()));
    }
    if request.operation_intent.trim().is_empty() {
        return Err(ApprovalError::Database("operation_intent required".into()));
    }
    Ok(())
}

fn map_attestation_error(err: AttestationError) -> ApprovalError {
    match err.code() {
        AttestationErrorCode::NotFound => ApprovalError::AttestationNotFound {
            dry_run_id: match &err {
                AttestationError::NotFound { dry_run_id } => dry_run_id.clone(),
                _ => String::new(),
            },
        },
        AttestationErrorCode::Expired => ApprovalError::AttestationExpired {
            dry_run_id: match &err {
                AttestationError::Expired { dry_run_id } => dry_run_id.clone(),
                _ => String::new(),
            },
        },
        AttestationErrorCode::Invalidated => ApprovalError::AttestationInvalid {
            dry_run_id: match &err {
                AttestationError::Invalidated { dry_run_id } => dry_run_id.clone(),
                _ => String::new(),
            },
        },
        AttestationErrorCode::ExecutionHashMismatch => ApprovalError::ExecutionHashMismatch,
        AttestationErrorCode::PolicyVersionMismatch => match err {
            AttestationError::PolicyVersionMismatch { expected, actual } => {
                ApprovalError::PolicyVersionMismatch { expected, actual }
            }
            _ => ApprovalError::PolicyVersionMismatch {
                expected: 0,
                actual: 0,
            },
        },
        AttestationErrorCode::IntentMismatch => ApprovalError::IntentMismatch,
        _ => ApprovalError::AttestationInvalid {
            dry_run_id: err.to_string(),
        },
    }
}

async fn fetch(
    pool: &SqlitePool,
    approval_id: &str,
) -> Result<Option<ApplyApproval>, ApprovalError> {
    let row = sqlx::query(
        r#"
        SELECT approval_id, dry_run_id, execution_hash, policy_id, policy_version,
               operation_intent, approved_by, approved_at, expires_at, status,
               consumed_at, cancelled_at, audit_reference, request_id, approver_role,
               consumed_by_operation_id, cancelled_by
        FROM apply_approvals
        WHERE approval_id = ?
        "#,
    )
    .bind(approval_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApprovalError::Database(e.to_string()))?;

    row.map(map_row).transpose()
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<ApplyApproval, ApprovalError> {
    let status_str: String = row.try_get("status").map_err(db_err)?;
    let status = ApprovalStatus::parse(&status_str)
        .ok_or_else(|| ApprovalError::Database(format!("unknown status: {status_str}")))?;

    Ok(ApplyApproval {
        approval_id: row.try_get("approval_id").map_err(db_err)?,
        dry_run_id: row.try_get("dry_run_id").map_err(db_err)?,
        execution_hash: row.try_get("execution_hash").map_err(db_err)?,
        policy_id: row.try_get("policy_id").map_err(db_err)?,
        policy_version: row.try_get("policy_version").map_err(db_err)?,
        operation_intent: row.try_get("operation_intent").map_err(db_err)?,
        approved_by: row.try_get("approved_by").map_err(db_err)?,
        approved_at: parse_ts(row.try_get("approved_at").map_err(db_err)?),
        expires_at: parse_ts(row.try_get("expires_at").map_err(db_err)?),
        status,
        consumed_at: row
            .try_get::<Option<String>, _>("consumed_at")
            .map_err(db_err)?
            .map(|s| parse_ts(&s)),
        cancelled_at: row
            .try_get::<Option<String>, _>("cancelled_at")
            .map_err(db_err)?
            .map(|s| parse_ts(&s)),
        audit_reference: row.try_get("audit_reference").map_err(db_err)?,
        request_id: row.try_get("request_id").map_err(db_err)?,
        approver_role: row.try_get("approver_role").map_err(db_err)?,
        consumed_by_operation_id: row.try_get("consumed_by_operation_id").map_err(db_err)?,
        cancelled_by: row.try_get("cancelled_by").map_err(db_err)?,
    })
}

fn parse_ts(s: &str) -> chrono::DateTime<Utc> {
    s.parse().unwrap_or_else(|_| Utc::now())
}

fn db_err(e: impl std::fmt::Display) -> ApprovalError {
    ApprovalError::Database(e.to_string())
}
