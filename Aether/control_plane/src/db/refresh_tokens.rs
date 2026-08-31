use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{Error, Result};

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

pub async fn store_refresh_token(
    pool: &SqlitePool,
    operator_id: &str,
    token: &str,
    ttl_secs: i64,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let expires_at = created_at + Duration::seconds(ttl_secs);
    sqlx::query(
        "INSERT INTO refresh_tokens (id, operator_id, token_hash, expires_at, created_at, revoked) VALUES (?, ?, ?, ?, ?, 0)",
    )
    .bind(&id)
    .bind(operator_id)
    .bind(hash_token(token))
    .bind(expires_at.to_rfc3339())
    .bind(created_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

/// Outcome of inspecting a presented refresh token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshLookup {
    /// Active, non-expired token for this operator.
    Valid { operator_id: String },
    /// Token exists but was already revoked (possible theft / reuse).
    Reused { operator_id: String },
    /// Unknown, expired, or malformed presentation.
    Invalid,
}

pub async fn lookup_refresh_token(pool: &SqlitePool, token: &str) -> Result<RefreshLookup> {
    let hash = hash_token(token);
    let row = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT operator_id, expires_at, revoked FROM refresh_tokens WHERE token_hash = ?",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await?;

    let Some((operator_id, expires_at, revoked)) = row else {
        return Ok(RefreshLookup::Invalid);
    };

    if revoked != 0 {
        return Ok(RefreshLookup::Reused { operator_id });
    }

    let expires_at: DateTime<Utc> = expires_at
        .parse()
        .map_err(|_| Error::Auth("invalid refresh token expiry".into()))?;
    if Utc::now() > expires_at {
        return Ok(RefreshLookup::Invalid);
    }
    Ok(RefreshLookup::Valid { operator_id })
}

pub async fn validate_refresh_token(pool: &SqlitePool, token: &str) -> Result<String> {
    match lookup_refresh_token(pool, token).await? {
        RefreshLookup::Valid { operator_id } => Ok(operator_id),
        RefreshLookup::Reused { .. } => Err(Error::Auth("refresh token reuse detected".into())),
        RefreshLookup::Invalid => Err(Error::Auth("invalid refresh token".into())),
    }
}

pub async fn revoke_refresh_token(pool: &SqlitePool, token: &str) -> Result<()> {
    sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE token_hash = ?")
        .bind(hash_token(token))
        .execute(pool)
        .await?;
    Ok(())
}

/// Revoke every refresh token for an operator (reuse-compromise response).
pub async fn revoke_all_for_operator(pool: &SqlitePool, operator_id: &str) -> Result<u64> {
    let result =
        sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE operator_id = ? AND revoked = 0")
            .bind(operator_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

/// Rotate: revoke `old_token`, persist `new_token`. Caller must issue `new_token`.
pub async fn rotate_refresh_token(
    pool: &SqlitePool,
    operator_id: &str,
    old_token: &str,
    new_token: &str,
    ttl_secs: i64,
) -> Result<()> {
    // Ensure old token still belongs to this operator and is active.
    match lookup_refresh_token(pool, old_token).await? {
        RefreshLookup::Valid { operator_id: oid } if oid == operator_id => {}
        RefreshLookup::Reused { .. } => {
            return Err(Error::Auth("refresh token reuse detected".into()));
        }
        _ => return Err(Error::Auth("invalid refresh token".into())),
    }

    revoke_refresh_token(pool, old_token).await?;
    store_refresh_token(pool, operator_id, new_token, ttl_secs).await?;
    Ok(())
}
