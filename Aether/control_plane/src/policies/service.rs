//! Policy template service — CP-local lifecycle only.
//!
//! Does NOT call PROTO-0, sign, or mutate agents.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::apply::canonical_json::canonicalize_json;
use crate::audit::mutation::{MutationAuditDraft, MutationAuditService, ProtocolResult};
use crate::auth::middleware::AuthContext;
use crate::db::operators::OperatorRole;
use crate::db::{audit, policies as policy_db};
use crate::error::{Error, Result};
use crate::models::policies::{
    CreatePolicyRequest, PolicyStatus, PolicyTemplate, PolicyTemplateVersion, RejectPolicyRequest,
    UpdatePolicyRequest,
};

pub struct PolicyTemplateService<'a> {
    pool: &'a SqlitePool,
    mutation_audit: &'a MutationAuditService,
}

impl<'a> PolicyTemplateService<'a> {
    pub fn new(pool: &'a SqlitePool, mutation_audit: &'a MutationAuditService) -> Self {
        Self {
            pool,
            mutation_audit,
        }
    }

    pub async fn list(&self, ctx: &AuthContext) -> Result<Vec<PolicyTemplate>> {
        match ctx.role {
            OperatorRole::Viewer => policy_db::list_policies(self.pool, true, None).await,
            OperatorRole::Operator | OperatorRole::Admin => {
                policy_db::list_policies(self.pool, false, None).await
            }
        }
    }

    pub async fn get(&self, ctx: &AuthContext, id: &str) -> Result<PolicyTemplate> {
        let policy = policy_db::get_policy(self.pool, id)
            .await?
            .ok_or_else(|| Error::NotFound("policy".into()))?;
        self.authorize_read(ctx, &policy)?;
        Ok(policy)
    }

    pub async fn history(
        &self,
        ctx: &AuthContext,
        id: &str,
    ) -> Result<(
        PolicyTemplate,
        Vec<PolicyTemplateVersion>,
        Vec<crate::models::audit::AuditEvent>,
    )> {
        let policy = self.get(ctx, id).await?;
        let versions = policy_db::list_versions(self.pool, id).await?;
        let events = audit::list(self.pool, 200).await?;
        let related: Vec<_> = events
            .into_iter()
            .filter(|e| e.target.as_deref() == Some(id) || e.action.starts_with("POLICY_"))
            .filter(|e| e.target.as_deref() == Some(id))
            .collect();
        Ok((policy, versions, related))
    }

    pub async fn create(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        req: CreatePolicyRequest,
    ) -> Result<PolicyTemplate> {
        require_writer(ctx)?;
        validate_name(&req.name)?;
        validate_policy_type(&req.policy_type)?;

        let id = Uuid::new_v4().to_string();
        let description = req.description.clone().unwrap_or_default();
        let hash = compute_policy_hash(
            &req.name,
            &description,
            req.target_agent_id.as_deref(),
            &req.policy_type,
            &req.policy_data,
            1,
        );
        let payload = serde_json::to_vec(&req)?;

        let policy = policy_db::insert_policy(
            self.pool,
            &id,
            &req.name,
            &description,
            req.target_agent_id.as_deref(),
            &req.policy_type,
            &req.policy_data,
            &ctx.operator_id,
            1,
            &hash,
        )
        .await?;

        policy_db::insert_version(self.pool, &policy, &ctx.operator_id, "POLICY_CREATED").await?;
        self.record_lifecycle(ctx, request_id, "POLICY_CREATED", &policy, &payload)
            .await?;
        Ok(policy)
    }

    pub async fn update(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        id: &str,
        req: UpdatePolicyRequest,
    ) -> Result<PolicyTemplate> {
        require_writer(ctx)?;
        let existing = policy_db::get_policy(self.pool, id)
            .await?
            .ok_or_else(|| Error::NotFound("policy".into()))?;

        match existing.status {
            PolicyStatus::Draft | PolicyStatus::Rejected => {}
            PolicyStatus::Approved | PolicyStatus::Archived | PolicyStatus::PendingReview => {
                return Err(Error::Forbidden(format!(
                    "cannot edit policy in {} status",
                    existing.status.as_str()
                )));
            }
        }

        if existing.created_by != ctx.operator_id && ctx.role != OperatorRole::Admin {
            return Err(Error::Forbidden("can only edit own drafts".into()));
        }

        let name = req.name.unwrap_or(existing.name.clone());
        validate_name(&name)?;
        let description = req.description.unwrap_or(existing.description.clone());
        let target_agent_id = req
            .target_agent_id
            .or_else(|| existing.target_agent_id.clone());
        let policy_type = req.policy_type.unwrap_or(existing.policy_type.clone());
        validate_policy_type(&policy_type)?;
        let policy_data = req.policy_data.unwrap_or(existing.policy_data.clone());
        let version = existing.version + 1;
        let hash = compute_policy_hash(
            &name,
            &description,
            target_agent_id.as_deref(),
            &policy_type,
            &policy_data,
            version,
        );
        let payload = serde_json::to_vec(&json!({
            "name": name,
            "description": description,
            "target_agent_id": target_agent_id,
            "policy_type": policy_type,
            "policy_data": policy_data,
            "version": version,
        }))?;

        let policy = policy_db::update_policy_content(
            self.pool,
            id,
            &name,
            &description,
            target_agent_id.as_deref(),
            &policy_type,
            &policy_data,
            version,
            &hash,
            PolicyStatus::Draft,
        )
        .await?;

        policy_db::insert_version(self.pool, &policy, &ctx.operator_id, "POLICY_UPDATED").await?;
        self.record_lifecycle(ctx, request_id, "POLICY_UPDATED", &policy, &payload)
            .await?;
        Ok(policy)
    }

    pub async fn submit(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        id: &str,
    ) -> Result<PolicyTemplate> {
        require_writer(ctx)?;
        let existing = policy_db::get_policy(self.pool, id)
            .await?
            .ok_or_else(|| Error::NotFound("policy".into()))?;

        if existing.created_by != ctx.operator_id && ctx.role != OperatorRole::Admin {
            return Err(Error::Forbidden("can only submit own drafts".into()));
        }
        if existing.status != PolicyStatus::Draft {
            return Err(Error::BadRequest(
                "only draft policies can be submitted".into(),
            ));
        }

        let policy = policy_db::set_pending_review(self.pool, id, &ctx.operator_id).await?;
        policy_db::insert_version(self.pool, &policy, &ctx.operator_id, "POLICY_SUBMITTED").await?;
        let payload = serde_json::to_vec(&json!({ "policy_id": id, "version": policy.version }))?;
        self.record_lifecycle(ctx, request_id, "POLICY_SUBMITTED", &policy, &payload)
            .await?;
        Ok(policy)
    }

    pub async fn approve(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        id: &str,
    ) -> Result<PolicyTemplate> {
        crate::auth::roles::require_admin(ctx)?;
        let existing = policy_db::get_policy(self.pool, id)
            .await?
            .ok_or_else(|| Error::NotFound("policy".into()))?;

        if existing.status != PolicyStatus::PendingReview {
            return Err(Error::BadRequest(
                "only pending review policies can be approved".into(),
            ));
        }
        if existing.created_by == ctx.operator_id {
            return Err(Error::Forbidden(
                "cannot approve own policy (separation of duties)".into(),
            ));
        }
        if existing
            .submitted_by
            .as_deref()
            .is_some_and(|s| s == ctx.operator_id)
        {
            return Err(Error::Forbidden(
                "cannot approve a policy you submitted".into(),
            ));
        }

        let policy = policy_db::set_approved(self.pool, id, &ctx.operator_id).await?;
        policy_db::insert_version(self.pool, &policy, &ctx.operator_id, "POLICY_APPROVED").await?;
        let payload = serde_json::to_vec(&json!({ "policy_id": id, "version": policy.version }))?;
        self.record_lifecycle(ctx, request_id, "POLICY_APPROVED", &policy, &payload)
            .await?;
        Ok(policy)
    }

    pub async fn reject(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        id: &str,
        req: RejectPolicyRequest,
    ) -> Result<PolicyTemplate> {
        crate::auth::roles::require_admin(ctx)?;
        let existing = policy_db::get_policy(self.pool, id)
            .await?
            .ok_or_else(|| Error::NotFound("policy".into()))?;

        if existing.status != PolicyStatus::PendingReview {
            return Err(Error::BadRequest(
                "only pending review policies can be rejected".into(),
            ));
        }
        if existing.created_by == ctx.operator_id {
            return Err(Error::Forbidden(
                "cannot reject own policy (separation of duties)".into(),
            ));
        }
        if existing
            .submitted_by
            .as_deref()
            .is_some_and(|s| s == ctx.operator_id)
        {
            return Err(Error::Forbidden(
                "cannot reject a policy you submitted".into(),
            ));
        }

        let policy =
            policy_db::set_rejected(self.pool, id, &ctx.operator_id, req.reason.as_deref()).await?;
        policy_db::insert_version(self.pool, &policy, &ctx.operator_id, "POLICY_REJECTED").await?;
        let payload = serde_json::to_vec(&json!({
            "policy_id": id,
            "version": policy.version,
            "reason": req.reason,
        }))?;
        self.record_lifecycle(ctx, request_id, "POLICY_REJECTED", &policy, &payload)
            .await?;
        Ok(policy)
    }

    pub async fn archive(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        id: &str,
    ) -> Result<PolicyTemplate> {
        crate::auth::roles::require_admin(ctx)?;
        let existing = policy_db::get_policy(self.pool, id)
            .await?
            .ok_or_else(|| Error::NotFound("policy".into()))?;

        if !matches!(
            existing.status,
            PolicyStatus::Approved | PolicyStatus::Rejected
        ) {
            return Err(Error::BadRequest(
                "only approved or rejected policies can be archived".into(),
            ));
        }

        let policy = policy_db::set_archived(self.pool, id).await?;
        policy_db::insert_version(self.pool, &policy, &ctx.operator_id, "POLICY_ARCHIVED").await?;
        let payload = serde_json::to_vec(&json!({ "policy_id": id, "version": policy.version }))?;
        self.record_lifecycle(ctx, request_id, "POLICY_ARCHIVED", &policy, &payload)
            .await?;
        Ok(policy)
    }

    fn authorize_read(&self, ctx: &AuthContext, policy: &PolicyTemplate) -> Result<()> {
        match ctx.role {
            OperatorRole::Viewer => {
                if policy.status == PolicyStatus::Approved {
                    Ok(())
                } else {
                    Err(Error::Forbidden(
                        "viewers may only read approved policies".into(),
                    ))
                }
            }
            OperatorRole::Operator | OperatorRole::Admin => Ok(()),
        }
    }

    async fn record_lifecycle(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        action: &str,
        policy: &PolicyTemplate,
        payload: &[u8],
    ) -> Result<()> {
        audit::append(
            self.pool,
            Some(&ctx.operator_id),
            action,
            Some(&policy.id),
            Some(json!({
                "version": policy.version,
                "status": policy.status.as_str(),
                "hash": policy.hash,
                "request_id": request_id,
                "role": ctx.role.as_str(),
            })),
        )
        .await?;

        let draft =
            MutationAuditDraft::new(request_id, &ctx.operator_id, ctx.role, action, payload)
                .target(format!("policy:{}", policy.id))
                .approved_now()
                .protocol_result(ProtocolResult::NotApplicable);

        self.mutation_audit.record(draft).await?;
        Ok(())
    }
}

fn require_writer(ctx: &AuthContext) -> Result<()> {
    crate::auth::roles::require_operator(ctx)
}

fn validate_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 200 {
        return Err(Error::BadRequest("name must be 1–200 characters".into()));
    }
    Ok(())
}

fn validate_policy_type(policy_type: &str) -> Result<()> {
    let trimmed = policy_type.trim();
    if trimmed.is_empty() || trimmed.len() > 100 {
        return Err(Error::BadRequest(
            "policy_type must be 1–100 characters".into(),
        ));
    }
    Ok(())
}

pub fn compute_policy_hash(
    name: &str,
    description: &str,
    target_agent_id: Option<&str>,
    policy_type: &str,
    policy_data: &Value,
    version: i64,
) -> String {
    let policy_data = canonicalize_json(policy_data).unwrap_or_else(|_| policy_data.clone());
    let mut map = serde_json::Map::new();
    map.insert("name".into(), json!(name));
    map.insert("description".into(), json!(description));
    map.insert(
        "target_agent_id".into(),
        target_agent_id.map(|s| json!(s)).unwrap_or(Value::Null),
    );
    map.insert("policy_type".into(), json!(policy_type));
    map.insert("policy_data".into(), policy_data);
    map.insert("version".into(), json!(version));
    let bytes = serde_json::to_vec(&Value::Object(map)).unwrap_or_default();
    hex::encode(Sha256::digest(bytes))
}
