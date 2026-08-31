use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::audit;
use crate::error::Result as CpResult;

use super::errors::TreasuryWriteError;
use super::models::TreasuryOperation;

pub async fn append_mutation_audit(
    pool: &SqlitePool,
    organisation_id: &str,
    mutation_id: Option<&str>,
    request_id: &str,
    actor: &str,
    operation: TreasuryOperation,
    target: Option<&str>,
    approval_state: Option<&str>,
    journal_reference: Option<&str>,
    outcome: &str,
    metadata: serde_json::Value,
) -> Result<(), TreasuryWriteError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO treasury_mutation_audit (
          id, mutation_id, organisation_id, request_id, actor, operation, target,
          approval_state, journal_reference, outcome, metadata_json, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(mutation_id)
    .bind(organisation_id)
    .bind(request_id)
    .bind(actor)
    .bind(operation.as_str())
    .bind(target)
    .bind(approval_state)
    .bind(journal_reference)
    .bind(outcome)
    .bind(metadata.to_string())
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;
    Ok(())
}

pub async fn append_cp_audit(
    pool: &SqlitePool,
    actor: &str,
    action: &str,
    target: Option<&str>,
    request_id: &str,
    extra: serde_json::Value,
) -> CpResult<()> {
    let mut meta = extra;
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("request_id".into(), json!(request_id));
    }
    audit::append(pool, Some(actor), action, target, Some(meta)).await?;
    Ok(())
}

pub async fn list_mutation_audit(
    pool: &SqlitePool,
    organisation_id: &str,
    limit: i64,
) -> Result<Vec<serde_json::Value>, TreasuryWriteError> {
    let rows = sqlx::query_as::<_, (String, Option<String>, String, String, String, Option<String>, Option<String>, Option<String>, String, String, String)>(
        r#"
        SELECT id, mutation_id, request_id, actor, operation, target, approval_state,
               journal_reference, outcome, metadata_json, created_at
        FROM treasury_mutation_audit
        WHERE organisation_id = ?
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(organisation_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                mutation_id,
                request_id,
                actor,
                operation,
                target,
                approval_state,
                journal_reference,
                outcome,
                metadata_json,
                created_at,
            )| {
                json!({
                    "id": id,
                    "mutation_id": mutation_id,
                    "request_id": request_id,
                    "actor": actor,
                    "operation": operation,
                    "target": target,
                    "approval_state": approval_state,
                    "journal_reference": journal_reference,
                    "outcome": outcome,
                    "metadata": serde_json::from_str::<serde_json::Value>(&metadata_json).unwrap_or(json!({})),
                    "created_at": created_at,
                })
            },
        )
        .collect())
}
