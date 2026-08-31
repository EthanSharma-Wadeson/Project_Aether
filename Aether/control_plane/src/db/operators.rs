use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatorRole {
    Admin,
    Operator,
    Viewer,
}

impl OperatorRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Operator => "operator",
            Self::Viewer => "viewer",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "admin" => Ok(Self::Admin),
            "operator" => Ok(Self::Operator),
            "viewer" => Ok(Self::Viewer),
            _ => Err(Error::BadRequest(format!("unknown role: {s}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Operator {
    pub id: String,
    pub username: String,
    pub role: OperatorRole,
    pub created_at: DateTime<Utc>,
}

pub async fn find_by_username(pool: &SqlitePool, username: &str) -> Result<Option<Operator>> {
    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id, username, role, created_at FROM operators WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, username, role, created_at)| Operator {
        id,
        username,
        role: OperatorRole::parse(&role).unwrap_or(OperatorRole::Viewer),
        created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
    }))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Operator>> {
    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id, username, role, created_at FROM operators WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, username, role, created_at)| Operator {
        id,
        username,
        role: OperatorRole::parse(&role).unwrap_or(OperatorRole::Viewer),
        created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
    }))
}

pub async fn password_hash_for_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<String>> {
    let row =
        sqlx::query_as::<_, (String,)>("SELECT password_hash FROM operators WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(h,)| h))
}

pub async fn create_operator(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    role: OperatorRole,
) -> Result<Operator> {
    let hash = bcrypt::hash(password, 12).map_err(|e| Error::Auth(e.to_string()))?;
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO operators (id, username, password_hash, role, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(username)
    .bind(&hash)
    .bind(role.as_str())
    .bind(&created_at)
    .execute(pool)
    .await?;

    Ok(Operator {
        id,
        username: username.into(),
        role,
        created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
    })
}
