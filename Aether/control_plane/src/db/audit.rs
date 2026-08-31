use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::Result;
use crate::models::audit::AuditEvent;

pub async fn append(
    pool: &SqlitePool,
    operator_id: Option<&str>,
    action: &str,
    target: Option<&str>,
    metadata: Option<Value>,
) -> Result<AuditEvent> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let metadata_json = metadata.as_ref().map(|v| v.to_string());
    sqlx::query(
        "INSERT INTO audit_log (id, operator_id, action, target, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(operator_id)
    .bind(action)
    .bind(target)
    .bind(metadata_json)
    .bind(created_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(AuditEvent {
        id,
        operator_id: operator_id.map(str::to_string),
        action: action.into(),
        target: target.map(str::to_string),
        metadata,
        created_at,
    })
}

pub async fn list(pool: &SqlitePool, limit: i64) -> Result<Vec<AuditEvent>> {
    let rows = sqlx::query_as::<_, (String, Option<String>, String, Option<String>, Option<String>, String)>(
        "SELECT id, operator_id, action, target, metadata, created_at FROM audit_log ORDER BY created_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, operator_id, action, target, metadata, created_at)| AuditEvent {
                id,
                operator_id,
                action,
                target,
                metadata: metadata.and_then(|m| serde_json::from_str(&m).ok()),
                created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
            },
        )
        .collect())
}
