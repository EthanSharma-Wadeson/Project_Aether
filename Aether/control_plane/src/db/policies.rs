//! Policy template persistence — Control Plane SQLite only.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::models::policies::{PolicyStatus, PolicyTemplate, PolicyTemplateVersion};

fn parse_ts(s: &str) -> DateTime<Utc> {
    s.parse().unwrap_or_else(|_| Utc::now())
}

fn map_policy_row(row: sqlx::sqlite::SqliteRow) -> Result<PolicyTemplate> {
    let status: String = row.try_get("status")?;
    let policy_data: String = row.try_get("policy_data")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;
    let submitted_at: Option<String> = row.try_get("submitted_at")?;
    let approved_at: Option<String> = row.try_get("approved_at")?;
    let rejected_at: Option<String> = row.try_get("rejected_at")?;

    Ok(PolicyTemplate {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        target_agent_id: row.try_get("target_agent_id")?,
        policy_type: row.try_get("policy_type")?,
        policy_data: serde_json::from_str(&policy_data).unwrap_or(Value::Null),
        status: PolicyStatus::parse(&status)?,
        created_by: row.try_get("created_by")?,
        created_by_username: row.try_get("created_by_username")?,
        created_at: parse_ts(&created_at),
        updated_at: parse_ts(&updated_at),
        submitted_by: row.try_get("submitted_by")?,
        submitted_at: submitted_at.map(|s| parse_ts(&s)),
        approved_by: row.try_get("approved_by")?,
        approved_at: approved_at.map(|s| parse_ts(&s)),
        rejected_by: row.try_get("rejected_by")?,
        rejected_at: rejected_at.map(|s| parse_ts(&s)),
        rejection_reason: row.try_get("rejection_reason")?,
        version: row.try_get("version")?,
        hash: row.try_get("hash")?,
    })
}

const SELECT_POLICY: &str = r#"
SELECT p.id, p.name, p.description, p.target_agent_id, p.policy_type, p.policy_data,
       p.status, p.created_by, o.username AS created_by_username,
       p.created_at, p.updated_at, p.submitted_by, p.submitted_at,
       p.approved_by, p.approved_at, p.rejected_by, p.rejected_at,
       p.rejection_reason, p.version, p.hash
FROM policy_templates p
LEFT JOIN operators o ON o.id = p.created_by
"#;

#[allow(clippy::too_many_arguments)]
pub async fn insert_policy(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: &str,
    target_agent_id: Option<&str>,
    policy_type: &str,
    policy_data: &Value,
    created_by: &str,
    version: i64,
    hash: &str,
) -> Result<PolicyTemplate> {
    let now = Utc::now().to_rfc3339();
    let data = policy_data.to_string();
    sqlx::query(
        r#"
        INSERT INTO policy_templates (
            id, name, description, target_agent_id, policy_type, policy_data,
            status, created_by, created_at, updated_at,
            submitted_by, submitted_at, approved_by, approved_at,
            rejected_by, rejected_at, rejection_reason, version, hash
        ) VALUES (?, ?, ?, ?, ?, ?, 'draft', ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, ?)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(target_agent_id)
    .bind(policy_type)
    .bind(&data)
    .bind(created_by)
    .bind(&now)
    .bind(&now)
    .bind(version)
    .bind(hash)
    .execute(pool)
    .await?;

    get_policy(pool, id)
        .await?
        .ok_or_else(|| Error::NotFound("policy".into()))
}

pub async fn get_policy(pool: &SqlitePool, id: &str) -> Result<Option<PolicyTemplate>> {
    let row = sqlx::query(&format!("{SELECT_POLICY} WHERE p.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(map_policy_row).transpose()
}

pub async fn list_policies(
    pool: &SqlitePool,
    approved_only: bool,
    created_by: Option<&str>,
) -> Result<Vec<PolicyTemplate>> {
    let rows = if approved_only {
        sqlx::query(&format!(
            "{SELECT_POLICY} WHERE p.status = 'approved' ORDER BY p.updated_at DESC"
        ))
        .fetch_all(pool)
        .await?
    } else if let Some(owner) = created_by {
        sqlx::query(&format!(
            "{SELECT_POLICY} WHERE p.created_by = ? ORDER BY p.updated_at DESC"
        ))
        .bind(owner)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(&format!("{SELECT_POLICY} ORDER BY p.updated_at DESC"))
            .fetch_all(pool)
            .await?
    };

    rows.into_iter().map(map_policy_row).collect()
}

#[allow(clippy::too_many_arguments)]
pub async fn update_policy_content(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: &str,
    target_agent_id: Option<&str>,
    policy_type: &str,
    policy_data: &Value,
    version: i64,
    hash: &str,
    status: PolicyStatus,
) -> Result<PolicyTemplate> {
    let now = Utc::now().to_rfc3339();
    let data = policy_data.to_string();
    let result = sqlx::query(
        r#"
        UPDATE policy_templates SET
            name = ?, description = ?, target_agent_id = ?, policy_type = ?,
            policy_data = ?, version = ?, hash = ?, status = ?, updated_at = ?,
            submitted_by = NULL, submitted_at = NULL,
            approved_by = NULL, approved_at = NULL,
            rejected_by = NULL, rejected_at = NULL, rejection_reason = NULL
        WHERE id = ? AND status IN ('draft', 'rejected')
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(target_agent_id)
    .bind(policy_type)
    .bind(&data)
    .bind(version)
    .bind(hash)
    .bind(status.as_str())
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(Error::Forbidden(
            "cannot update policy: must be draft or rejected (concurrent transition?)".into(),
        ));
    }
    get_policy(pool, id)
        .await?
        .ok_or_else(|| Error::NotFound("policy".into()))
}

pub async fn set_pending_review(
    pool: &SqlitePool,
    id: &str,
    submitted_by: &str,
) -> Result<PolicyTemplate> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"
        UPDATE policy_templates SET
            status = 'pending_review',
            submitted_by = ?,
            submitted_at = ?,
            updated_at = ?,
            rejected_by = NULL,
            rejected_at = NULL,
            rejection_reason = NULL
        WHERE id = ? AND status = 'draft'
        "#,
    )
    .bind(submitted_by)
    .bind(&now)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(Error::BadRequest(
            "policy must be in draft status to submit".into(),
        ));
    }
    get_policy(pool, id)
        .await?
        .ok_or_else(|| Error::NotFound("policy".into()))
}

pub async fn set_approved(
    pool: &SqlitePool,
    id: &str,
    approved_by: &str,
) -> Result<PolicyTemplate> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"
        UPDATE policy_templates SET
            status = 'approved',
            approved_by = ?,
            approved_at = ?,
            updated_at = ?
        WHERE id = ? AND status = 'pending_review'
        "#,
    )
    .bind(approved_by)
    .bind(&now)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(Error::BadRequest(
            "policy must be pending review to approve".into(),
        ));
    }
    get_policy(pool, id)
        .await?
        .ok_or_else(|| Error::NotFound("policy".into()))
}

pub async fn set_rejected(
    pool: &SqlitePool,
    id: &str,
    rejected_by: &str,
    reason: Option<&str>,
) -> Result<PolicyTemplate> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"
        UPDATE policy_templates SET
            status = 'rejected',
            rejected_by = ?,
            rejected_at = ?,
            rejection_reason = ?,
            updated_at = ?
        WHERE id = ? AND status = 'pending_review'
        "#,
    )
    .bind(rejected_by)
    .bind(&now)
    .bind(reason)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(Error::BadRequest(
            "policy must be pending review to reject".into(),
        ));
    }
    get_policy(pool, id)
        .await?
        .ok_or_else(|| Error::NotFound("policy".into()))
}

pub async fn set_archived(pool: &SqlitePool, id: &str) -> Result<PolicyTemplate> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"
        UPDATE policy_templates SET
            status = 'archived',
            updated_at = ?
        WHERE id = ? AND status IN ('approved', 'rejected')
        "#,
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(Error::BadRequest(
            "only approved or rejected policies can be archived".into(),
        ));
    }
    get_policy(pool, id)
        .await?
        .ok_or_else(|| Error::NotFound("policy".into()))
}

pub async fn insert_version(
    pool: &SqlitePool,
    policy: &PolicyTemplate,
    changed_by: &str,
    change_action: &str,
) -> Result<PolicyTemplateVersion> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let data = policy.policy_data.to_string();
    sqlx::query(
        r#"
        INSERT INTO policy_template_versions (
            id, policy_id, version, name, description, target_agent_id,
            policy_type, policy_data, status, hash, changed_by, changed_at, change_action
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&policy.id)
    .bind(policy.version)
    .bind(&policy.name)
    .bind(&policy.description)
    .bind(&policy.target_agent_id)
    .bind(&policy.policy_type)
    .bind(&data)
    .bind(policy.status.as_str())
    .bind(&policy.hash)
    .bind(changed_by)
    .bind(now.to_rfc3339())
    .bind(change_action)
    .execute(pool)
    .await?;

    Ok(PolicyTemplateVersion {
        id,
        policy_id: policy.id.clone(),
        version: policy.version,
        name: policy.name.clone(),
        description: policy.description.clone(),
        target_agent_id: policy.target_agent_id.clone(),
        policy_type: policy.policy_type.clone(),
        policy_data: policy.policy_data.clone(),
        status: policy.status,
        hash: policy.hash.clone(),
        changed_by: changed_by.into(),
        changed_at: now,
        change_action: change_action.into(),
    })
}

pub async fn list_versions(
    pool: &SqlitePool,
    policy_id: &str,
) -> Result<Vec<PolicyTemplateVersion>> {
    let rows = sqlx::query(
        r#"
        SELECT id, policy_id, version, name, description, target_agent_id,
               policy_type, policy_data, status, hash, changed_by, changed_at, change_action
        FROM policy_template_versions
        WHERE policy_id = ?
        ORDER BY version DESC
        "#,
    )
    .bind(policy_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            let status: String = row.try_get("status")?;
            let policy_data: String = row.try_get("policy_data")?;
            let changed_at: String = row.try_get("changed_at")?;
            Ok(PolicyTemplateVersion {
                id: row.try_get("id")?,
                policy_id: row.try_get("policy_id")?,
                version: row.try_get("version")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                target_agent_id: row.try_get("target_agent_id")?,
                policy_type: row.try_get("policy_type")?,
                policy_data: serde_json::from_str(&policy_data).unwrap_or(Value::Null),
                status: PolicyStatus::parse(&status)?,
                hash: row.try_get("hash")?,
                changed_by: row.try_get("changed_by")?,
                changed_at: parse_ts(&changed_at),
                change_action: row.try_get("change_action")?,
            })
        })
        .collect()
}
