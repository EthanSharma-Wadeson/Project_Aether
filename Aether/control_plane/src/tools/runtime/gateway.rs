use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::agents::runtime::errors::RuntimeAgentError;
use crate::agents::runtime::{registry as agent_registry, sessions as agent_sessions};
use crate::auth::middleware::AuthContext;
use crate::db::Db;
use crate::enforcement::{EnforcementRequest, EnforcementService};
use crate::protocol::state::ProtocolState;
use crate::treasury::TreasuryAdapter;

use super::audit;
use super::errors::ToolGatewayError;
use super::models::{
    CreateToolRequest, GatewayDecision, GatewayEvidence, GatewayOutcome, ToolEvaluateHttpRequest,
    ToolRecord, ToolRequest, ToolStatus,
};
use super::policy;
use super::registry;

/// Lab tool gateway — decision only (`tools_executed` always false).
pub struct ToolGatewayService;

impl ToolGatewayService {
    pub async fn create_tool(
        db: &Db,
        org: &str,
        ctx: &AuthContext,
        req: CreateToolRequest,
    ) -> Result<ToolRecord, ToolGatewayError> {
        registry::create_tool(db.pool(), org, &ctx.operator_id, &req).await
    }

    pub async fn disable_tool(
        db: &Db,
        org: &str,
        tool_id: &str,
    ) -> Result<ToolRecord, ToolGatewayError> {
        registry::set_status(db.pool(), org, tool_id, ToolStatus::Disabled).await
    }

    pub async fn list_tools(db: &Db, org: &str) -> Result<Vec<ToolRecord>, ToolGatewayError> {
        registry::list_tools(db.pool(), org).await
    }

    pub async fn evaluate(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        ctx: &AuthContext,
        org: &str,
        http: ToolEvaluateHttpRequest,
        request_id_hint: String,
    ) -> Result<GatewayDecision, ToolGatewayError> {
        let request_id = http
            .request_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(request_id_hint);

        if load_replay(db, org, &request_id).await?.is_some() {
            return Err(ToolGatewayError::RequestReplay);
        }

        let parameters_hash = hash_parameters(&http.parameters);
        let tool_req = ToolRequest {
            request_id: request_id.clone(),
            agent_id: http.agent_id.clone(),
            session_id: http.session_id.clone(),
            tool_id: http.tool_id.clone(),
            parameters_hash: parameters_hash.clone(),
            created_at: Utc::now(),
        };

        let agent = match agent_registry::get_agent(db.pool(), org, &http.agent_id).await {
            Ok(Some(a)) => a,
            Ok(None) => return Err(ToolGatewayError::AgentNotFound),
            Err(e) => return Err(ToolGatewayError::Other(e.into_cp())),
        };
        if !agent.status.can_act() {
            let d = deny_decision(
                &tool_req,
                org,
                "AGENT_NOT_ACTABLE",
                format!("agent status {}", agent.status.as_str()),
                Some(agent.status.as_str()),
                None,
                None,
                &parameters_hash,
            );
            return finalize(db, ctx, org, d).await;
        }

        let session =
            match agent_sessions::mark_expired_if_needed(db.pool(), org, &http.session_id).await {
                Ok(s) => s,
                Err(RuntimeAgentError::SessionNotFound) => {
                    return Err(ToolGatewayError::SessionNotFound);
                }
                Err(e) => return Err(ToolGatewayError::Other(e.into_cp())),
            };
        if session.agent_id != http.agent_id || session.organisation_id != org {
            return Err(ToolGatewayError::OrganisationMismatch);
        }
        if session.status == crate::agents::runtime::models::SessionStatus::Expired
            || session.expires_at <= Utc::now()
        {
            let d = deny_decision(
                &tool_req,
                org,
                "SESSION_EXPIRED",
                "runtime session expired".into(),
                Some(agent.status.as_str()),
                Some("EXPIRED"),
                None,
                &parameters_hash,
            );
            return finalize(db, ctx, org, d).await;
        }
        if !session.status.is_usable() {
            let d = deny_decision(
                &tool_req,
                org,
                "SESSION_NOT_USABLE",
                format!("session status {}", session.status.as_str()),
                Some(agent.status.as_str()),
                Some(session.status.as_str()),
                None,
                &parameters_hash,
            );
            return finalize(db, ctx, org, d).await;
        }

        let tool = match registry::get_tool(db.pool(), org, &http.tool_id).await? {
            Some(t) => t,
            None => {
                let d = deny_decision(
                    &tool_req,
                    org,
                    policy::deny_reason_unknown(),
                    "tool not registered for organisation".into(),
                    Some(agent.status.as_str()),
                    Some(session.status.as_str()),
                    None,
                    &parameters_hash,
                );
                return finalize(db, ctx, org, d).await;
            }
        };

        if tool.status == ToolStatus::Disabled {
            let d = deny_decision(
                &tool_req,
                org,
                policy::deny_reason_for_disabled(),
                "tool is DISABLED".into(),
                Some(agent.status.as_str()),
                Some(session.status.as_str()),
                Some(&tool),
                &parameters_hash,
            );
            return finalize(db, ctx, org, d).await;
        }

        let enf = EnforcementRequest {
            agent_id: http.agent_id.clone(),
            organisation_id: Some(org.to_string()),
            action: tool.required_capability.clone(),
            asset_id: http.asset_id.clone(),
            amount_minor: http.amount_minor,
            policy_id: None,
            force_review: false,
        };
        let e2 = EnforcementService::evaluate(
            protocol,
            treasury,
            db,
            ctx,
            enf,
            request_id.clone(),
        )
        .await?;

        let outcome = policy::compose_decision(tool.status, tool.risk_level, e2.decision);
        let reason = match outcome {
            GatewayOutcome::Allow => "identity, session, registry, and E2 ALLOW".into(),
            GatewayOutcome::Deny => e2
                .reason
                .clone()
                .or_else(|| e2.deny_reason.map(|r| r.as_str().to_string()))
                .unwrap_or_else(|| "E2 DENY".into()),
            GatewayOutcome::RequiresReview => {
                if tool.risk_level.forces_review() || tool.status == ToolStatus::Restricted {
                    format!(
                        "tool risk/status requires review ({}/{})",
                        tool.risk_level.as_str(),
                        tool.status.as_str()
                    )
                } else {
                    e2.reason
                        .unwrap_or_else(|| "E2 REQUIRES_REVIEW".into())
                }
            }
        };

        let decision = GatewayDecision {
            decision: outcome,
            decision_code: outcome.as_str().to_string(),
            reason,
            request_id: request_id.clone(),
            agent_id: http.agent_id,
            tool_id: http.tool_id,
            session_id: http.session_id,
            evidence: GatewayEvidence {
                agent_status: Some(agent.status.as_str().to_string()),
                session_status: Some(session.status.as_str().to_string()),
                tool_status: Some(tool.status.as_str().to_string()),
                risk_level: Some(tool.risk_level.as_str().to_string()),
                required_capability: Some(tool.required_capability),
                parameters_hash,
                e2_decision: Some(e2.decision_code),
                e2_deny_reason: e2.deny_reason.map(|r| r.as_str().to_string()),
                organisation_id: org.to_string(),
            },
            tools_executed: false,
            apply_invoked: false,
            proto0_mutated: false,
            treasury_mutated: false,
        };

        let _ = agent_sessions::bump_request_counter(db.pool(), org, &decision.session_id).await;
        finalize(db, ctx, org, decision).await
    }
}

async fn finalize(
    db: &Db,
    ctx: &AuthContext,
    org: &str,
    decision: GatewayDecision,
) -> Result<GatewayDecision, ToolGatewayError> {
    audit::record_requested(db, &ctx.operator_id, &decision).await?;
    audit::record_outcome(db, &ctx.operator_id, &decision).await?;
    store_replay(db, org, &decision).await?;
    debug_assert!(!decision.tools_executed);
    debug_assert!(!decision.apply_invoked);
    Ok(decision)
}

fn deny_decision(
    req: &ToolRequest,
    org: &str,
    code: &str,
    reason: String,
    agent_status: Option<&str>,
    session_status: Option<&str>,
    tool: Option<&ToolRecord>,
    parameters_hash: &str,
) -> GatewayDecision {
    GatewayDecision {
        decision: GatewayOutcome::Deny,
        decision_code: GatewayOutcome::Deny.as_str().to_string(),
        reason: format!("{code}: {reason}"),
        request_id: req.request_id.clone(),
        agent_id: req.agent_id.clone(),
        tool_id: req.tool_id.clone(),
        session_id: req.session_id.clone(),
        evidence: GatewayEvidence {
            agent_status: agent_status.map(str::to_string),
            session_status: session_status.map(str::to_string),
            tool_status: tool.map(|t| t.status.as_str().to_string()),
            risk_level: tool.map(|t| t.risk_level.as_str().to_string()),
            required_capability: tool.map(|t| t.required_capability.clone()),
            parameters_hash: parameters_hash.to_string(),
            e2_decision: None,
            e2_deny_reason: Some(code.to_string()),
            organisation_id: org.to_string(),
        },
        tools_executed: false,
        apply_invoked: false,
        proto0_mutated: false,
        treasury_mutated: false,
    }
}

fn hash_parameters(params: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(params).unwrap_or_else(|_| "{}".into());
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

async fn store_replay(
    db: &Db,
    org: &str,
    decision: &GatewayDecision,
) -> Result<(), ToolGatewayError> {
    let json = serde_json::to_string(decision)
        .map_err(|e| ToolGatewayError::BadRequest(e.to_string()))?;
    let res = sqlx::query(
        r#"
        INSERT INTO tool_request_replay
          (request_id, organisation_id, agent_id, session_id, tool_id,
           parameters_hash, decision_code, decision_json, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&decision.request_id)
    .bind(org)
    .bind(&decision.agent_id)
    .bind(&decision.session_id)
    .bind(&decision.tool_id)
    .bind(&decision.evidence.parameters_hash)
    .bind(&decision.decision_code)
    .bind(&json)
    .bind(Utc::now().to_rfc3339())
    .execute(db.pool())
    .await;

    if let Err(sqlx::Error::Database(ref d)) = res {
        if d.is_unique_violation() || d.message().contains("UNIQUE") {
            return Err(ToolGatewayError::RequestReplay);
        }
    }
    res?;
    Ok(())
}

async fn load_replay(
    db: &Db,
    org: &str,
    request_id: &str,
) -> Result<Option<()>, ToolGatewayError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT organisation_id FROM tool_request_replay WHERE request_id = ?",
    )
    .bind(request_id)
    .fetch_optional(db.pool())
    .await?;
    match row {
        None => Ok(None),
        Some((stored_org,)) => {
            if stored_org != org {
                return Err(ToolGatewayError::OrganisationMismatch);
            }
            Ok(Some(()))
        }
    }
}
