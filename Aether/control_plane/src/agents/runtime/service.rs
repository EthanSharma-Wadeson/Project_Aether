use crate::auth::middleware::AuthContext;
use crate::db::Db;
use crate::enforcement::{EnforcementRequest, EnforcementService};
use crate::protocol::state::ProtocolState;
use crate::treasury::TreasuryAdapter;

use super::audit;
use super::errors::RuntimeAgentError;
use super::models::{
    AgentStatus, CreateAgentRequest, CreateSessionRequest, L3Credential, RuntimeAgent,
    RuntimeEvaluateRequest, RuntimeEvaluateResponse, RuntimeSession, SessionStatus,
};
use super::{registry, sessions};

/// Lab agent registry + session identity service.
pub struct AgentRuntimeService;

impl AgentRuntimeService {
    pub fn organisation_id(treasury: &TreasuryAdapter) -> &str {
        treasury.organisation_id()
    }

    pub async fn create_agent(
        db: &Db,
        org: &str,
        ctx: &AuthContext,
        req: CreateAgentRequest,
        request_id: &str,
    ) -> Result<RuntimeAgent, RuntimeAgentError> {
        let agent = registry::create_agent(db.pool(), org, &ctx.operator_id, &req).await?;
        audit::record(
            db,
            &ctx.operator_id,
            "AGENT_CREATED",
            &agent.agent_id,
            org,
            request_id,
            Some(serde_json::json!({
                "display_name": agent.display_name,
                "provider_type": agent.provider_type,
            })),
        )
        .await?;
        Ok(agent)
    }

    pub async fn list_agents(db: &Db, org: &str) -> Result<Vec<RuntimeAgent>, RuntimeAgentError> {
        registry::list_agents(db.pool(), org).await
    }

    pub async fn get_agent(
        db: &Db,
        org: &str,
        agent_id: &str,
    ) -> Result<RuntimeAgent, RuntimeAgentError> {
        registry::get_agent(db.pool(), org, agent_id)
            .await?
            .ok_or(RuntimeAgentError::AgentNotFound)
    }

    pub async fn freeze_agent(
        db: &Db,
        org: &str,
        ctx: &AuthContext,
        agent_id: &str,
        request_id: &str,
    ) -> Result<RuntimeAgent, RuntimeAgentError> {
        let agent =
            registry::set_agent_status(db.pool(), org, agent_id, AgentStatus::Frozen).await?;
        audit::record(
            db,
            &ctx.operator_id,
            "AGENT_FROZEN",
            agent_id,
            org,
            request_id,
            None,
        )
        .await?;
        Ok(agent)
    }

    pub async fn revoke_agent(
        db: &Db,
        org: &str,
        ctx: &AuthContext,
        agent_id: &str,
        request_id: &str,
    ) -> Result<RuntimeAgent, RuntimeAgentError> {
        let agent =
            registry::set_agent_status(db.pool(), org, agent_id, AgentStatus::Revoked).await?;
        audit::record(
            db,
            &ctx.operator_id,
            "AGENT_REVOKED",
            agent_id,
            org,
            request_id,
            None,
        )
        .await?;
        Ok(agent)
    }

    pub async fn create_session(
        db: &Db,
        org: &str,
        ctx: &AuthContext,
        req: CreateSessionRequest,
        request_id: &str,
    ) -> Result<(RuntimeSession, L3Credential), RuntimeAgentError> {
        let (session, cred) = sessions::create_session(db.pool(), org, &req).await?;
        audit::record(
            db,
            &ctx.operator_id,
            "SESSION_CREATED",
            &session.agent_id,
            org,
            request_id,
            Some(serde_json::json!({
                "session_id": session.session_id,
                "expires_at": session.expires_at,
                "credential_id": session.credential_id,
            })),
        )
        .await?;
        Ok((session, cred))
    }

    pub async fn revoke_session(
        db: &Db,
        org: &str,
        ctx: &AuthContext,
        session_id: &str,
        request_id: &str,
    ) -> Result<RuntimeSession, RuntimeAgentError> {
        let session = sessions::revoke_session(db.pool(), org, session_id).await?;
        audit::record(
            db,
            &ctx.operator_id,
            "SESSION_REVOKED",
            &session.agent_id,
            org,
            request_id,
            Some(serde_json::json!({ "session_id": session_id })),
        )
        .await?;
        Ok(session)
    }

    pub async fn expire_session_check(
        db: &Db,
        org: &str,
        ctx: &AuthContext,
        session_id: &str,
        request_id: &str,
    ) -> Result<RuntimeSession, RuntimeAgentError> {
        let before = sessions::get_session(db.pool(), org, session_id)
            .await?
            .ok_or(RuntimeAgentError::SessionNotFound)?;
        let session = sessions::mark_expired_if_needed(db.pool(), org, session_id).await?;
        if before.status != SessionStatus::Expired && session.status == SessionStatus::Expired {
            audit::record(
                db,
                &ctx.operator_id,
                "SESSION_EXPIRED",
                &session.agent_id,
                org,
                request_id,
                Some(serde_json::json!({ "session_id": session_id })),
            )
            .await?;
        }
        Ok(session)
    }

    /// Identity validation → E2 evaluate. No tool execution.
    pub async fn evaluate_request(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        ctx: &AuthContext,
        org: &str,
        req: RuntimeEvaluateRequest,
        request_id: String,
    ) -> Result<RuntimeEvaluateResponse, RuntimeAgentError> {
        let session =
            sessions::validate_credential(db.pool(), org, &req.credential, true).await?;
        sessions::bump_request_counter(db.pool(), org, &session.session_id).await?;

        let enf_req = EnforcementRequest {
            agent_id: session.agent_id.clone(),
            organisation_id: Some(org.to_string()),
            action: req.action,
            asset_id: req.asset_id,
            amount_minor: req.amount_minor,
            policy_id: req.policy_id,
            force_review: req.force_review,
        };

        let decision = EnforcementService::evaluate(
            protocol,
            treasury,
            db,
            ctx,
            enf_req,
            request_id.clone(),
        )
        .await?;

        let enforcement = serde_json::to_value(&decision)
            .map_err(|e| RuntimeAgentError::BadRequest(e.to_string()))?;

        Ok(RuntimeEvaluateResponse {
            identity_valid: true,
            agent_id: session.agent_id,
            organisation_id: org.to_string(),
            session_id: session.session_id,
            enforcement,
            tools_executed: false,
            apply_invoked: false,
            proto0_mutated: false,
            treasury_mutated: false,
            request_id,
        })
    }
}
