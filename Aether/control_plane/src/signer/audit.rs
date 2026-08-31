//! Signing audit — every signature attempt is recorded.
//!
//! No `protocol_result` — signing is CP-local attestation only (Phase 3).

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::signer::errors::SignerError;
use crate::signer::operation::GovernanceOperation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignOutcome {
    Created,
    Completed,
    Failed,
}

impl SignOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SignerAuditRecord {
    pub id: String,
    pub action: String,
    pub request_id: String,
    pub operation_id: String,
    pub signer_identity: String,
    pub operator_id: String,
    pub policy_id: Option<String>,
    pub outcome: SignOutcome,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct SignerAuditService {
    pool: SqlitePool,
}

impl SignerAuditService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record_created(
        &self,
        op: &GovernanceOperation,
        signer_identity: &str,
    ) -> Result<SignerAuditRecord, SignerError> {
        self.insert(
            "SIGN_REQUEST_CREATED",
            op,
            signer_identity,
            SignOutcome::Created,
            None,
        )
        .await
    }

    pub async fn record_completed(
        &self,
        op: &GovernanceOperation,
        signer_identity: &str,
    ) -> Result<SignerAuditRecord, SignerError> {
        self.insert(
            "SIGN_REQUEST_COMPLETED",
            op,
            signer_identity,
            SignOutcome::Completed,
            None,
        )
        .await
    }

    pub async fn record_failed(
        &self,
        op: &GovernanceOperation,
        signer_identity: &str,
        reason: &str,
    ) -> Result<SignerAuditRecord, SignerError> {
        self.insert(
            "SIGN_REQUEST_FAILED",
            op,
            signer_identity,
            SignOutcome::Failed,
            Some(reason),
        )
        .await
    }

    async fn insert(
        &self,
        action: &str,
        op: &GovernanceOperation,
        signer_identity: &str,
        outcome: SignOutcome,
        failure_reason: Option<&str>,
    ) -> Result<SignerAuditRecord, SignerError> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO signer_audit (
                id, action, request_id, operation_id, signer_identity,
                operator_id, policy_id, outcome, failure_reason, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(action)
        .bind(&op.request_id)
        .bind(&op.operation_id)
        .bind(signer_identity)
        .bind(&op.operator_id)
        .bind(&op.policy_id)
        .bind(outcome.as_str())
        .bind(failure_reason)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| SignerError::Audit(e.to_string()))?;

        // Mirror into general audit timeline for operators.
        let _ = crate::db::audit::append(
            &self.pool,
            Some(&op.operator_id),
            action,
            op.policy_id.as_deref().or(Some(op.operation_id.as_str())),
            Some(serde_json::json!({
                "request_id": op.request_id,
                "operation_id": op.operation_id,
                "signer_identity": signer_identity,
                "outcome": outcome.as_str(),
                "failure_reason": failure_reason,
            })),
        )
        .await;

        Ok(SignerAuditRecord {
            id,
            action: action.into(),
            request_id: op.request_id.clone(),
            operation_id: op.operation_id.clone(),
            signer_identity: signer_identity.into(),
            operator_id: op.operator_id.clone(),
            policy_id: op.policy_id.clone(),
            outcome,
            failure_reason: failure_reason.map(str::to_string),
            created_at,
        })
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<SignerAuditRecord>, SignerError> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
                String,
                Option<String>,
                String,
            ),
        >(
            r#"
            SELECT id, action, request_id, operation_id, signer_identity,
                   operator_id, policy_id, outcome, failure_reason, created_at
            FROM signer_audit
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SignerError::Audit(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    action,
                    request_id,
                    operation_id,
                    signer_identity,
                    operator_id,
                    policy_id,
                    outcome,
                    failure_reason,
                    created_at,
                )| {
                    SignerAuditRecord {
                        id,
                        action,
                        request_id,
                        operation_id,
                        signer_identity,
                        operator_id,
                        policy_id,
                        outcome: match outcome.as_str() {
                            "completed" => SignOutcome::Completed,
                            "failed" => SignOutcome::Failed,
                            _ => SignOutcome::Created,
                        },
                        failure_reason,
                        created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
                    }
                },
            )
            .collect())
    }
}
