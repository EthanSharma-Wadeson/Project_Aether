use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use super::errors::ToolGatewayError;
use super::models::{CreateToolRequest, RiskLevel, ToolRecord, ToolStatus};

pub async fn create_tool(
    pool: &SqlitePool,
    organisation_id: &str,
    created_by: &str,
    req: &CreateToolRequest,
) -> Result<ToolRecord, ToolGatewayError> {
    if req.tool_id.trim().is_empty() {
        return Err(ToolGatewayError::BadRequest("tool_id required".into()));
    }
    if req.name.trim().is_empty() {
        return Err(ToolGatewayError::BadRequest("name required".into()));
    }
    if req.required_capability.trim().is_empty() {
        return Err(ToolGatewayError::BadRequest(
            "required_capability required".into(),
        ));
    }
    if get_tool(pool, organisation_id, &req.tool_id).await?.is_some() {
        return Err(ToolGatewayError::DuplicateTool);
    }

    let created_at = Utc::now();
    let description = req.description.clone().unwrap_or_default();
    let res = sqlx::query(
        r#"
        INSERT INTO runtime_tools
          (tool_id, organisation_id, name, description, risk_level,
           required_capability, status, created_at, created_by)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(req.tool_id.trim())
    .bind(organisation_id)
    .bind(req.name.trim())
    .bind(&description)
    .bind(req.risk_level.as_str())
    .bind(req.required_capability.trim())
    .bind(ToolStatus::Active.as_str())
    .bind(created_at.to_rfc3339())
    .bind(created_by)
    .execute(pool)
    .await;

    if let Err(sqlx::Error::Database(ref d)) = res {
        if d.is_unique_violation() || d.message().contains("UNIQUE") {
            return Err(ToolGatewayError::DuplicateTool);
        }
    }
    res?;

    get_tool(pool, organisation_id, req.tool_id.trim())
        .await?
        .ok_or(ToolGatewayError::ToolNotFound)
}

pub async fn get_tool(
    pool: &SqlitePool,
    organisation_id: &str,
    tool_id: &str,
) -> Result<Option<ToolRecord>, ToolGatewayError> {
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT tool_id, organisation_id, name, description, risk_level,
               required_capability, status, created_at, created_by
        FROM runtime_tools
        WHERE organisation_id = ? AND tool_id = ?
        "#,
    )
    .bind(organisation_id)
    .bind(tool_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(map_tool).transpose()?)
}

pub async fn set_status(
    pool: &SqlitePool,
    organisation_id: &str,
    tool_id: &str,
    status: ToolStatus,
) -> Result<ToolRecord, ToolGatewayError> {
    let _ = get_tool(pool, organisation_id, tool_id)
        .await?
        .ok_or(ToolGatewayError::ToolNotFound)?;
    sqlx::query(
        "UPDATE runtime_tools SET status = ? WHERE organisation_id = ? AND tool_id = ?",
    )
    .bind(status.as_str())
    .bind(organisation_id)
    .bind(tool_id)
    .execute(pool)
    .await?;
    get_tool(pool, organisation_id, tool_id)
        .await?
        .ok_or(ToolGatewayError::ToolNotFound)
}

pub async fn list_tools(
    pool: &SqlitePool,
    organisation_id: &str,
) -> Result<Vec<ToolRecord>, ToolGatewayError> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT tool_id, organisation_id, name, description, risk_level,
               required_capability, status, created_at, created_by
        FROM runtime_tools
        WHERE organisation_id = ?
        ORDER BY tool_id
        "#,
    )
    .bind(organisation_id)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for r in rows {
        out.push(map_tool(r)?);
    }
    Ok(out)
}

fn map_tool(
    row: (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ),
) -> Result<ToolRecord, ToolGatewayError> {
    let (
        tool_id,
        organisation_id,
        name,
        description,
        risk_level,
        required_capability,
        status,
        created_at,
        created_by,
    ) = row;
    let risk_level = RiskLevel::parse(&risk_level).ok_or_else(|| {
        ToolGatewayError::BadRequest(format!("invalid risk_level: {risk_level}"))
    })?;
    let status = ToolStatus::parse(&status)
        .ok_or_else(|| ToolGatewayError::BadRequest(format!("invalid tool status: {status}")))?;
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| ToolGatewayError::BadRequest(e.to_string()))?;
    Ok(ToolRecord {
        tool_id,
        organisation_id,
        name,
        description,
        risk_level,
        required_capability,
        status,
        created_at,
        created_by,
    })
}
