use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::errors::RuntimeAgentError;
use super::models::{
    CreateSessionRequest, L3Credential, RuntimeSession, SessionStatus,
};
use super::registry;

const DEFAULT_TTL_SECS: i64 = 1_800;
const MAX_TTL_SECS: i64 = 86_400;

pub async fn create_session(
    pool: &SqlitePool,
    organisation_id: &str,
    req: &CreateSessionRequest,
) -> Result<(RuntimeSession, L3Credential), RuntimeAgentError> {
    if registry::org_is_frozen(pool, organisation_id).await? {
        return Err(RuntimeAgentError::OrganisationFrozen);
    }
    let agent = registry::get_agent(pool, organisation_id, &req.agent_id)
        .await?
        .ok_or(RuntimeAgentError::AgentNotFound)?;
    if !agent.status.can_create_session() {
        return Err(RuntimeAgentError::AgentNotSessionable(
            agent.status.as_str().into(),
        ));
    }

    let ttl = req
        .ttl_secs
        .map(|t| t as i64)
        .unwrap_or(DEFAULT_TTL_SECS)
        .clamp(1, MAX_TTL_SECS);
    let issued_at = Utc::now();
    let expires_at = issued_at + Duration::seconds(ttl);
    let session_id = format!("rt-sess-{}", Uuid::new_v4());
    let credential_id = format!("rt-cred-{}", Uuid::new_v4());
    let jti = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO runtime_sessions
          (session_id, agent_id, organisation_id, issued_at, expires_at,
           request_counter, status, credential_id)
        VALUES (?, ?, ?, ?, ?, 0, ?, ?)
        "#,
    )
    .bind(&session_id)
    .bind(&agent.agent_id)
    .bind(organisation_id)
    .bind(issued_at.to_rfc3339())
    .bind(expires_at.to_rfc3339())
    .bind(SessionStatus::Active.as_str())
    .bind(&credential_id)
    .execute(pool)
    .await?;

    let session = RuntimeSession {
        session_id: session_id.clone(),
        agent_id: agent.agent_id.clone(),
        organisation_id: organisation_id.to_string(),
        issued_at,
        expires_at,
        request_counter: 0,
        status: SessionStatus::Active,
        credential_id: credential_id.clone(),
    };
    let cred = L3Credential {
        credential_id,
        agent_id: agent.agent_id,
        organisation_id: organisation_id.to_string(),
        session_id,
        expires_at,
        jti,
    };
    Ok((session, cred))
}

pub async fn get_session(
    pool: &SqlitePool,
    organisation_id: &str,
    session_id: &str,
) -> Result<Option<RuntimeSession>, RuntimeAgentError> {
    let row: Option<(String, String, String, String, String, i64, String, String)> =
        sqlx::query_as(
            r#"
            SELECT session_id, agent_id, organisation_id, issued_at, expires_at,
                   request_counter, status, credential_id
            FROM runtime_sessions
            WHERE session_id = ? AND organisation_id = ?
            "#,
        )
        .bind(session_id)
        .bind(organisation_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(map_session).transpose()?)
}

pub async fn revoke_session(
    pool: &SqlitePool,
    organisation_id: &str,
    session_id: &str,
) -> Result<RuntimeSession, RuntimeAgentError> {
    let _session = get_session(pool, organisation_id, session_id)
        .await?
        .ok_or(RuntimeAgentError::SessionNotFound)?;
    sqlx::query(
        "UPDATE runtime_sessions SET status = ? WHERE session_id = ? AND organisation_id = ?",
    )
    .bind(SessionStatus::Revoked.as_str())
    .bind(session_id)
    .bind(organisation_id)
    .execute(pool)
    .await?;
    get_session(pool, organisation_id, session_id)
        .await?
        .ok_or(RuntimeAgentError::SessionNotFound)
}

pub async fn mark_expired_if_needed(
    pool: &SqlitePool,
    organisation_id: &str,
    session_id: &str,
) -> Result<RuntimeSession, RuntimeAgentError> {
    let mut session = get_session(pool, organisation_id, session_id)
        .await?
        .ok_or(RuntimeAgentError::SessionNotFound)?;
    if session.status == SessionStatus::Revoked {
        return Ok(session);
    }
    if session.expires_at <= Utc::now() && session.status != SessionStatus::Expired {
        sqlx::query(
            "UPDATE runtime_sessions SET status = ? WHERE session_id = ? AND organisation_id = ?",
        )
        .bind(SessionStatus::Expired.as_str())
        .bind(session_id)
        .bind(organisation_id)
        .execute(pool)
        .await?;
        session.status = SessionStatus::Expired;
    }
    Ok(session)
}

pub async fn bump_request_counter(
    pool: &SqlitePool,
    organisation_id: &str,
    session_id: &str,
) -> Result<(), RuntimeAgentError> {
    sqlx::query(
        r#"
        UPDATE runtime_sessions
        SET request_counter = request_counter + 1,
            status = CASE WHEN status = 'CREATED' THEN 'ACTIVE' ELSE status END
        WHERE session_id = ? AND organisation_id = ?
        "#,
    )
    .bind(session_id)
    .bind(organisation_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Validate placeholder L3 credential against session + agent + org + replay store.
pub async fn validate_credential(
    pool: &SqlitePool,
    expected_org: &str,
    cred: &L3Credential,
    consume_jti: bool,
) -> Result<RuntimeSession, RuntimeAgentError> {
    if cred.agent_id.trim().is_empty()
        || cred.organisation_id.trim().is_empty()
        || cred.credential_id.trim().is_empty()
        || cred.session_id.trim().is_empty()
        || cred.jti.trim().is_empty()
    {
        return Err(RuntimeAgentError::InvalidCredential(
            "missing required credential fields".into(),
        ));
    }
    if cred.organisation_id != expected_org {
        return Err(RuntimeAgentError::OrganisationMismatch);
    }
    if cred.expires_at <= Utc::now() {
        return Err(RuntimeAgentError::InvalidCredential("credential expired".into()));
    }
    if registry::org_is_frozen(pool, expected_org).await? {
        return Err(RuntimeAgentError::OrganisationFrozen);
    }

    let agent = registry::get_agent(pool, expected_org, &cred.agent_id)
        .await?
        .ok_or(RuntimeAgentError::AgentNotFound)?;
    if !agent.status.can_act() {
        return Err(RuntimeAgentError::AgentNotActable(
            agent.status.as_str().into(),
        ));
    }

    let session = mark_expired_if_needed(pool, expected_org, &cred.session_id).await?;
    if session.agent_id != cred.agent_id {
        return Err(RuntimeAgentError::InvalidCredential(
            "session is not transferable between agents".into(),
        ));
    }
    if session.organisation_id != cred.organisation_id {
        return Err(RuntimeAgentError::OrganisationMismatch);
    }
    if session.credential_id != cred.credential_id {
        return Err(RuntimeAgentError::InvalidCredential(
            "credential_id does not match session".into(),
        ));
    }
    if session.status == SessionStatus::Revoked {
        return Err(RuntimeAgentError::SessionNotUsable(
            SessionStatus::Revoked.as_str().into(),
        ));
    }
    if session.status == SessionStatus::Expired || session.expires_at <= Utc::now() {
        return Err(RuntimeAgentError::SessionExpired);
    }
    if !session.status.is_usable() {
        return Err(RuntimeAgentError::SessionNotUsable(
            session.status.as_str().into(),
        ));
    }

    if consume_jti {
        let insert = sqlx::query(
            r#"
            INSERT INTO runtime_credential_jti
              (jti, credential_id, session_id, organisation_id, agent_id, consumed_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&cred.jti)
        .bind(&cred.credential_id)
        .bind(&cred.session_id)
        .bind(&cred.organisation_id)
        .bind(&cred.agent_id)
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await;
        if let Err(sqlx::Error::Database(ref d)) = insert {
            if d.message().contains("UNIQUE") || d.is_unique_violation() {
                return Err(RuntimeAgentError::CredentialReplay);
            }
        }
        insert?;
    }

    Ok(session)
}

fn map_session(
    row: (String, String, String, String, String, i64, String, String),
) -> Result<RuntimeSession, RuntimeAgentError> {
    let (session_id, agent_id, organisation_id, issued_at, expires_at, request_counter, status, credential_id) =
        row;
    let status = SessionStatus::parse(&status).ok_or_else(|| {
        RuntimeAgentError::BadRequest(format!("invalid session status: {status}"))
    })?;
    let issued_at = chrono::DateTime::parse_from_rfc3339(&issued_at)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| RuntimeAgentError::BadRequest(e.to_string()))?;
    let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| RuntimeAgentError::BadRequest(e.to_string()))?;
    Ok(RuntimeSession {
        session_id,
        agent_id,
        organisation_id,
        issued_at,
        expires_at,
        request_counter,
        status,
        credential_id,
    })
}
