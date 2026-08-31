//! Session governor hooks — TTL, max actions, cancel, circuit breaker.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::agents::runtime::models::RuntimeSession;
use crate::agents::runtime::sessions;

use super::errors::ProviderAdapterError;
use super::models::GovernorSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreakerState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "CLOSED",
            Self::Open => "OPEN",
            Self::HalfOpen => "HALF_OPEN",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "CLOSED" => Some(Self::Closed),
            "OPEN" => Some(Self::Open),
            "HALF_OPEN" => Some(Self::HalfOpen),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionGovernorConfig {
    pub max_actions: i64,
    /// Consecutive DENY outcomes before opening breaker.
    pub deny_storm_threshold: i64,
}

impl Default for SessionGovernorConfig {
    fn default() -> Self {
        Self {
            max_actions: 20,
            deny_storm_threshold: 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionGovernor {
    pub session_id: String,
    pub organisation_id: String,
    pub agent_id: String,
    pub action_count: i64,
    pub consecutive_denies: i64,
    pub cancelled: bool,
    pub circuit_breaker: CircuitBreakerState,
    pub max_actions: i64,
    pub deny_storm_threshold: i64,
    pub expires_at: DateTime<Utc>,
}

impl SessionGovernor {
    pub fn snapshot(&self) -> GovernorSnapshot {
        GovernorSnapshot {
            action_count: self.action_count,
            max_actions: self.max_actions,
            cancelled: self.cancelled,
            circuit_breaker: self.circuit_breaker.as_str().into(),
            expires_at: self.expires_at,
        }
    }

    pub fn precheck(&self, session: &RuntimeSession) -> Result<(), ProviderAdapterError> {
        if self.cancelled {
            return Err(ProviderAdapterError::GovernorDenied(
                "session cancelled".into(),
            ));
        }
        if self.circuit_breaker == CircuitBreakerState::Open {
            return Err(ProviderAdapterError::GovernorDenied(
                "circuit breaker OPEN".into(),
            ));
        }
        if session.status.as_str() == "REVOKED" || !session.status.is_usable() {
            return Err(ProviderAdapterError::GovernorDenied(format!(
                "session status {}",
                session.status.as_str()
            )));
        }
        if session.expires_at <= Utc::now() || self.expires_at <= Utc::now() {
            return Err(ProviderAdapterError::GovernorDenied("session TTL expired".into()));
        }
        if self.action_count >= self.max_actions {
            return Err(ProviderAdapterError::GovernorDenied(format!(
                "max_actions {} reached",
                self.max_actions
            )));
        }
        Ok(())
    }
}

pub async fn load_or_init(
    pool: &SqlitePool,
    org: &str,
    session: &RuntimeSession,
    config: &SessionGovernorConfig,
) -> Result<SessionGovernor, ProviderAdapterError> {
    if let Some(g) = load(pool, org, &session.session_id).await? {
        return Ok(g);
    }
    sqlx::query(
        r#"
        INSERT INTO provider_session_governors
          (session_id, organisation_id, agent_id, action_count, consecutive_denies,
           cancelled, circuit_breaker, max_actions, deny_storm_threshold, expires_at, updated_at)
        VALUES (?, ?, ?, 0, 0, 0, 'CLOSED', ?, ?, ?, ?)
        "#,
    )
    .bind(&session.session_id)
    .bind(org)
    .bind(&session.agent_id)
    .bind(config.max_actions)
    .bind(config.deny_storm_threshold)
    .bind(session.expires_at.to_rfc3339())
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    load(pool, org, &session.session_id)
        .await?
        .ok_or_else(|| ProviderAdapterError::BadRequest("governor init failed".into()))
}

pub async fn load(
    pool: &SqlitePool,
    org: &str,
    session_id: &str,
) -> Result<Option<SessionGovernor>, ProviderAdapterError> {
    let row: Option<(
        String,
        String,
        String,
        i64,
        i64,
        i64,
        String,
        i64,
        i64,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT session_id, organisation_id, agent_id, action_count, consecutive_denies,
               cancelled, circuit_breaker, max_actions, deny_storm_threshold, expires_at
        FROM provider_session_governors
        WHERE session_id = ? AND organisation_id = ?
        "#,
    )
    .bind(session_id)
    .bind(org)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(map_row).transpose()?)
}

/// Lab/benchmark: tighten max_actions on an existing governor (or no-op if missing).
pub async fn set_max_actions(
    pool: &SqlitePool,
    org: &str,
    session_id: &str,
    max_actions: i64,
) -> Result<SessionGovernor, ProviderAdapterError> {
    sqlx::query(
        r#"
        UPDATE provider_session_governors
        SET max_actions = ?, updated_at = ?
        WHERE session_id = ? AND organisation_id = ?
        "#,
    )
    .bind(max_actions.max(0))
    .bind(Utc::now().to_rfc3339())
    .bind(session_id)
    .bind(org)
    .execute(pool)
    .await?;
    load(pool, org, session_id)
        .await?
        .ok_or_else(|| ProviderAdapterError::BadRequest("governor not found".into()))
}

pub async fn cancel(
    pool: &SqlitePool,
    org: &str,
    session_id: &str,
) -> Result<SessionGovernor, ProviderAdapterError> {
    sqlx::query(
        r#"
        UPDATE provider_session_governors
        SET cancelled = 1, updated_at = ?
        WHERE session_id = ? AND organisation_id = ?
        "#,
    )
    .bind(Utc::now().to_rfc3339())
    .bind(session_id)
    .bind(org)
    .execute(pool)
    .await?;
    load(pool, org, session_id)
        .await?
        .ok_or_else(|| ProviderAdapterError::BadRequest("governor not found".into()))
}

pub async fn record_outcome(
    pool: &SqlitePool,
    org: &str,
    session_id: &str,
    decision_code: &str,
) -> Result<SessionGovernor, ProviderAdapterError> {
    let mut g = load(pool, org, session_id)
        .await?
        .ok_or_else(|| ProviderAdapterError::BadRequest("governor not found".into()))?;

    g.action_count += 1;
    // `decision_code` here is gateway outcome: ALLOW | DENY | REQUIRES_REVIEW
    if decision_code == "DENY" {
        g.consecutive_denies += 1;
        if g.consecutive_denies >= g.deny_storm_threshold {
            g.circuit_breaker = CircuitBreakerState::Open;
        }
    } else {
        g.consecutive_denies = 0;
        if g.circuit_breaker == CircuitBreakerState::HalfOpen {
            g.circuit_breaker = CircuitBreakerState::Closed;
        }
    }

    sqlx::query(
        r#"
        UPDATE provider_session_governors
        SET action_count = ?, consecutive_denies = ?, circuit_breaker = ?, updated_at = ?
        WHERE session_id = ? AND organisation_id = ?
        "#,
    )
    .bind(g.action_count)
    .bind(g.consecutive_denies)
    .bind(g.circuit_breaker.as_str())
    .bind(Utc::now().to_rfc3339())
    .bind(session_id)
    .bind(org)
    .execute(pool)
    .await?;

    sessions::bump_request_counter(pool, org, session_id).await?;
    load(pool, org, session_id)
        .await?
        .ok_or_else(|| ProviderAdapterError::BadRequest("governor missing after update".into()))
}

fn map_row(
    row: (
        String,
        String,
        String,
        i64,
        i64,
        i64,
        String,
        i64,
        i64,
        String,
    ),
) -> Result<SessionGovernor, ProviderAdapterError> {
    let (
        session_id,
        organisation_id,
        agent_id,
        action_count,
        consecutive_denies,
        cancelled,
        circuit_breaker,
        max_actions,
        deny_storm_threshold,
        expires_at,
    ) = row;
    let circuit_breaker = CircuitBreakerState::parse(&circuit_breaker).ok_or_else(|| {
        ProviderAdapterError::BadRequest(format!("bad circuit_breaker: {circuit_breaker}"))
    })?;
    let expires_at = DateTime::parse_from_rfc3339(&expires_at)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| ProviderAdapterError::BadRequest(e.to_string()))?;
    Ok(SessionGovernor {
        session_id,
        organisation_id,
        agent_id,
        action_count,
        consecutive_denies,
        cancelled: cancelled != 0,
        circuit_breaker,
        max_actions,
        deny_storm_threshold,
        expires_at,
    })
}
