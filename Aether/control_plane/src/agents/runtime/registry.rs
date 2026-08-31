use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::errors::RuntimeAgentError;
use super::models::{
    AgentProviderType, AgentStatus, CreateAgentRequest, OrganisationRuntimeStatus, RuntimeAgent,
};

pub async fn ensure_org_active(pool: &SqlitePool, organisation_id: &str) -> Result<(), RuntimeAgentError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT status FROM runtime_organisations WHERE organisation_id = ?")
            .bind(organisation_id)
            .fetch_optional(pool)
            .await?;
    match row {
        None => {
            sqlx::query(
                "INSERT INTO runtime_organisations (organisation_id, status, updated_at) VALUES (?, ?, ?)",
            )
            .bind(organisation_id)
            .bind(OrganisationRuntimeStatus::Active.as_str())
            .bind(Utc::now().to_rfc3339())
            .execute(pool)
            .await?;
            Ok(())
        }
        Some((status,)) => {
            if OrganisationRuntimeStatus::parse(&status) == Some(OrganisationRuntimeStatus::Frozen)
            {
                Err(RuntimeAgentError::OrganisationFrozen)
            } else {
                Ok(())
            }
        }
    }
}

pub async fn set_org_status(
    pool: &SqlitePool,
    organisation_id: &str,
    status: OrganisationRuntimeStatus,
) -> Result<(), RuntimeAgentError> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO runtime_organisations (organisation_id, status, updated_at)
        VALUES (?, ?, ?)
        ON CONFLICT(organisation_id) DO UPDATE SET status = excluded.status, updated_at = excluded.updated_at
        "#,
    )
    .bind(organisation_id)
    .bind(status.as_str())
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn org_is_frozen(pool: &SqlitePool, organisation_id: &str) -> Result<bool, RuntimeAgentError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT status FROM runtime_organisations WHERE organisation_id = ?")
            .bind(organisation_id)
            .fetch_optional(pool)
            .await?;
    Ok(matches!(
        row.as_ref().map(|(s,)| OrganisationRuntimeStatus::parse(s)),
        Some(Some(OrganisationRuntimeStatus::Frozen))
    ))
}

pub async fn create_agent(
    pool: &SqlitePool,
    organisation_id: &str,
    created_by: &str,
    req: &CreateAgentRequest,
) -> Result<RuntimeAgent, RuntimeAgentError> {
    if organisation_id.trim().is_empty() {
        return Err(RuntimeAgentError::BadRequest(
            "organisation_id required".into(),
        ));
    }
    if req.display_name.trim().is_empty() {
        return Err(RuntimeAgentError::BadRequest(
            "display_name required".into(),
        ));
    }
    ensure_org_active(pool, organisation_id).await?;

    let agent_id = req
        .agent_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("rt-agent-{}", Uuid::new_v4()));

    if get_agent(pool, organisation_id, &agent_id).await?.is_some() {
        return Err(RuntimeAgentError::DuplicateAgent);
    }

    let created_at = Utc::now();
    let description = req.description.clone().unwrap_or_default();
    let res = sqlx::query(
        r#"
        INSERT INTO runtime_agents
          (agent_id, organisation_id, display_name, description, provider_type,
           model_identifier, status, created_at, created_by)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&agent_id)
    .bind(organisation_id)
    .bind(req.display_name.trim())
    .bind(&description)
    .bind(req.provider_type.as_str())
    .bind(&req.model_identifier)
    .bind(AgentStatus::Active.as_str())
    .bind(created_at.to_rfc3339())
    .bind(created_by)
    .execute(pool)
    .await;

    if let Err(sqlx::Error::Database(ref d)) = res {
        if d.message().contains("UNIQUE") {
            return Err(RuntimeAgentError::DuplicateAgent);
        }
    }
    res?;

    get_agent(pool, organisation_id, &agent_id)
        .await?
        .ok_or(RuntimeAgentError::AgentNotFound)
}

pub async fn get_agent(
    pool: &SqlitePool,
    organisation_id: &str,
    agent_id: &str,
) -> Result<Option<RuntimeAgent>, RuntimeAgentError> {
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT agent_id, organisation_id, display_name, description, provider_type,
               model_identifier, status, created_at, created_by
        FROM runtime_agents
        WHERE agent_id = ? AND organisation_id = ?
        "#,
    )
    .bind(agent_id)
    .bind(organisation_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(map_agent).transpose()?)
}

pub async fn get_agent_any_org(
    pool: &SqlitePool,
    agent_id: &str,
) -> Result<Option<RuntimeAgent>, RuntimeAgentError> {
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT agent_id, organisation_id, display_name, description, provider_type,
               model_identifier, status, created_at, created_by
        FROM runtime_agents WHERE agent_id = ?
        "#,
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(map_agent).transpose()?)
}

pub async fn list_agents(
    pool: &SqlitePool,
    organisation_id: &str,
) -> Result<Vec<RuntimeAgent>, RuntimeAgentError> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT agent_id, organisation_id, display_name, description, provider_type,
               model_identifier, status, created_at, created_by
        FROM runtime_agents
        WHERE organisation_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(organisation_id)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for r in rows {
        out.push(map_agent(r)?);
    }
    Ok(out)
}

pub async fn set_agent_status(
    pool: &SqlitePool,
    organisation_id: &str,
    agent_id: &str,
    status: AgentStatus,
) -> Result<RuntimeAgent, RuntimeAgentError> {
    let existing = get_agent(pool, organisation_id, agent_id)
        .await?
        .ok_or(RuntimeAgentError::AgentNotFound)?;
    sqlx::query(
        "UPDATE runtime_agents SET status = ? WHERE agent_id = ? AND organisation_id = ?",
    )
    .bind(status.as_str())
    .bind(&existing.agent_id)
    .bind(organisation_id)
    .execute(pool)
    .await?;
    get_agent(pool, organisation_id, agent_id)
        .await?
        .ok_or(RuntimeAgentError::AgentNotFound)
}

fn map_agent(
    row: (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        String,
    ),
) -> Result<RuntimeAgent, RuntimeAgentError> {
    let (
        agent_id,
        organisation_id,
        display_name,
        description,
        provider_type,
        model_identifier,
        status,
        created_at,
        created_by,
    ) = row;
    let provider_type = AgentProviderType::parse(&provider_type).ok_or_else(|| {
        RuntimeAgentError::BadRequest(format!("invalid provider_type in db: {provider_type}"))
    })?;
    let status = AgentStatus::parse(&status).ok_or_else(|| {
        RuntimeAgentError::BadRequest(format!("invalid agent status in db: {status}"))
    })?;
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| RuntimeAgentError::BadRequest(e.to_string()))?;
    Ok(RuntimeAgent {
        agent_id,
        organisation_id,
        display_name,
        description,
        provider_type,
        model_identifier,
        status,
        created_at,
        created_by,
    })
}
