//! Mutation audit foundation — no live mutations in Phase 1.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::operators::OperatorRole;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolResult {
    Success,
    Rejected,
    NotApplicable,
}

impl ProtocolResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Rejected => "rejected",
            Self::NotApplicable => "not_applicable",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "success" => Self::Success,
            "rejected" => Self::Rejected,
            _ => Self::NotApplicable,
        }
    }
}

/// Append-only mutation attempt record (CP-local foundation).
#[derive(Debug, Clone, Serialize)]
pub struct MutationAuditRecord {
    pub id: String,
    pub request_id: String,
    pub operator_id: String,
    pub role: String,
    pub action: String,
    pub target: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub signer_identity: Option<String>,
    pub protocol_result: ProtocolResult,
    pub failure_reason: Option<String>,
    pub payload_hash: String,
}

/// Builder for future write handlers.
#[derive(Debug, Clone)]
pub struct MutationAuditDraft {
    pub request_id: String,
    pub operator_id: String,
    pub role: OperatorRole,
    pub action: String,
    pub target: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub signer_identity: Option<String>,
    pub protocol_result: ProtocolResult,
    pub failure_reason: Option<String>,
    pub payload_hash: String,
}

impl MutationAuditDraft {
    pub fn new(
        request_id: impl Into<String>,
        operator_id: impl Into<String>,
        role: OperatorRole,
        action: impl Into<String>,
        payload: &[u8],
    ) -> Self {
        Self {
            request_id: request_id.into(),
            operator_id: operator_id.into(),
            role,
            action: action.into(),
            target: None,
            requested_at: Utc::now(),
            approved_at: None,
            signer_identity: None,
            protocol_result: ProtocolResult::NotApplicable,
            failure_reason: None,
            payload_hash: payload_hash(payload),
        }
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn approved_now(mut self) -> Self {
        self.approved_at = Some(Utc::now());
        self
    }

    pub fn signer_identity(mut self, id: impl Into<String>) -> Self {
        self.signer_identity = Some(id.into());
        self
    }

    pub fn protocol_result(mut self, result: ProtocolResult) -> Self {
        self.protocol_result = result;
        self
    }

    pub fn failure_reason(mut self, reason: impl Into<String>) -> Self {
        self.failure_reason = Some(reason.into());
        self
    }
}

pub fn payload_hash(payload: &[u8]) -> String {
    hex::encode(Sha256::digest(payload))
}

pub fn payload_hash_json(value: &serde_json::Value) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    Ok(payload_hash(&bytes))
}

/// Append-only mutation audit service.
#[derive(Clone)]
pub struct MutationAuditService {
    pool: SqlitePool,
}

impl MutationAuditService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record(&self, draft: MutationAuditDraft) -> Result<MutationAuditRecord> {
        let id = Uuid::new_v4().to_string();
        let requested_at = draft.requested_at.to_rfc3339();
        let approved_at = draft.approved_at.map(|t| t.to_rfc3339());

        sqlx::query(
            r#"
            INSERT INTO mutation_audit (
                id, request_id, operator_id, role, action, target,
                requested_at, approved_at, signer_identity, protocol_result,
                failure_reason, payload_hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&draft.request_id)
        .bind(&draft.operator_id)
        .bind(draft.role.as_str())
        .bind(&draft.action)
        .bind(&draft.target)
        .bind(&requested_at)
        .bind(&approved_at)
        .bind(&draft.signer_identity)
        .bind(draft.protocol_result.as_str())
        .bind(&draft.failure_reason)
        .bind(&draft.payload_hash)
        .execute(&self.pool)
        .await?;

        Ok(MutationAuditRecord {
            id,
            request_id: draft.request_id,
            operator_id: draft.operator_id,
            role: draft.role.as_str().into(),
            action: draft.action,
            target: draft.target,
            requested_at: draft.requested_at,
            approved_at: draft.approved_at,
            signer_identity: draft.signer_identity,
            protocol_result: draft.protocol_result,
            failure_reason: draft.failure_reason,
            payload_hash: draft.payload_hash,
        })
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<MutationAuditRecord>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                Option<String>,
                String,
                Option<String>,
                Option<String>,
                String,
                Option<String>,
                String,
            ),
        >(
            r#"
            SELECT id, request_id, operator_id, role, action, target,
                   requested_at, approved_at, signer_identity, protocol_result,
                   failure_reason, payload_hash
            FROM mutation_audit
            ORDER BY requested_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    request_id,
                    operator_id,
                    role,
                    action,
                    target,
                    requested_at,
                    approved_at,
                    signer_identity,
                    protocol_result,
                    failure_reason,
                    payload_hash,
                )| MutationAuditRecord {
                    id,
                    request_id,
                    operator_id,
                    role,
                    action,
                    target,
                    requested_at: requested_at.parse().unwrap_or_else(|_| Utc::now()),
                    approved_at: approved_at.and_then(|s| s.parse().ok()),
                    signer_identity,
                    protocol_result: ProtocolResult::parse(&protocol_result),
                    failure_reason,
                    payload_hash,
                },
            )
            .collect())
    }
}
