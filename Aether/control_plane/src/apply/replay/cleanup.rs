//! Replay sweeper — transitions stale `reserved` / `executing` rows to `stuck`.
//!
//! No automatic retries, no PROTO-0 interaction, never assumes success.

use chrono::{Duration as ChronoDuration, Utc};

use super::model::ReplayError;
use super::store::ReplayStore;

/// Move `reserved` rows older than the store reserve timeout to `stuck`.
///
/// Returns the number of rows transitioned.
pub async fn sweep_reserved_timeouts(store: &ReplayStore) -> Result<u64, ReplayError> {
    let timeout = store.reserve_timeout();
    let cutoff = Utc::now()
        - ChronoDuration::from_std(timeout).unwrap_or_else(|_| ChronoDuration::minutes(5));
    let cutoff_str = cutoff.to_rfc3339();
    let now = Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        UPDATE signed_operation_replay
        SET status = 'stuck',
            updated_at = ?,
            terminal_reason = COALESCE(terminal_reason, 'reserve_timeout'),
            finalised_at = COALESCE(finalised_at, ?)
        WHERE status = 'reserved'
          AND reserved_at < ?
        "#,
    )
    .bind(&now)
    .bind(&now)
    .bind(&cutoff_str)
    .execute(store.pool())
    .await
    .map_err(|e| ReplayError::Database(e.to_string()))?;

    Ok(result.rows_affected())
}

/// Move `executing` rows older than the execution timeout to `stuck`.
///
/// Uses `updated_at` (set when entering executing). Never marks executed.
pub async fn sweep_executing_timeouts(store: &ReplayStore) -> Result<u64, ReplayError> {
    let timeout = store.reserve_timeout(); // same TTL bound for Phase 11; configurable later
    let cutoff = Utc::now()
        - ChronoDuration::from_std(timeout).unwrap_or_else(|_| ChronoDuration::minutes(5));
    let cutoff_str = cutoff.to_rfc3339();
    let now = Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        UPDATE signed_operation_replay
        SET status = 'stuck',
            updated_at = ?,
            terminal_reason = COALESCE(terminal_reason, 'execution_timeout'),
            finalised_at = COALESCE(finalised_at, ?)
        WHERE status = 'executing'
          AND updated_at < ?
        "#,
    )
    .bind(&now)
    .bind(&now)
    .bind(&cutoff_str)
    .execute(store.pool())
    .await
    .map_err(|e| ReplayError::Database(e.to_string()))?;

    Ok(result.rows_affected())
}
