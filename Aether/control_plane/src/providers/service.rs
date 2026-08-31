//! Provider adapter orchestration — propose → map → session → gateway/E2 → sandbox simulate.

use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::agents::runtime::{registry as agent_registry, sessions};
use crate::agents::sandbox::executor;
use crate::auth::middleware::AuthContext;
use crate::db::Db;
use crate::protocol::state::ProtocolState;
use crate::tools::runtime::ToolGatewayService;
use crate::treasury::TreasuryAdapter;

use super::audit;
use super::errors::ProviderAdapterError;
use super::external::transport::LabHttpTransport;
use super::governors::{self, SessionGovernorConfig};
use super::mapper;
use super::mocks;
use super::models::{
    GovernedProviderResult, ProviderReasonHttpRequest, ProviderReasonRequest,
};
use super::secrets::SecretRef;

pub struct ProviderAdapterService;

impl ProviderAdapterService {
    /// Lab: mock or external provider reasons, then Aether governs. Sandbox only.
    pub async fn reason_and_govern(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        ctx: &AuthContext,
        http: ProviderReasonHttpRequest,
        request_id_hint: String,
    ) -> Result<GovernedProviderResult, ProviderAdapterError> {
        Self::reason_and_govern_with_transport(
            protocol,
            treasury,
            db,
            ctx,
            http,
            request_id_hint,
            None,
        )
        .await
    }

    /// Same as [`reason_and_govern`] with optional scripted HTTP transport (tests / isolated lab).
    pub async fn reason_and_govern_with_transport(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        ctx: &AuthContext,
        http: ProviderReasonHttpRequest,
        request_id_hint: String,
        transport: Option<Arc<dyn LabHttpTransport>>,
    ) -> Result<GovernedProviderResult, ProviderAdapterError> {
        let org = treasury.organisation_id().to_string();
        let correlation_id = http
            .correlation_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("corr-{}", Uuid::new_v4()));
        let provider_call_id = format!("pvc-{}", Uuid::new_v4());

        // Trusted identity from operator request — never from model.
        let agent = agent_registry::get_agent(db.pool(), &org, &http.agent_id)
            .await?
            .ok_or(crate::agents::runtime::errors::RuntimeAgentError::AgentNotFound)?;
        if !agent.status.can_act() {
            return Err(
                crate::agents::runtime::errors::RuntimeAgentError::AgentNotActable(
                    agent.status.as_str().into(),
                )
                .into(),
            );
        }

        let session = sessions::mark_expired_if_needed(db.pool(), &org, &http.session_id).await?;
        if session.agent_id != http.agent_id {
            return Err(ProviderAdapterError::BadRequest(
                "session does not belong to agent".into(),
            ));
        }

        let gov_config = SessionGovernorConfig::default();
        let governor = governors::load_or_init(db.pool(), &org, &session, &gov_config).await?;
        governor.precheck(&session)?;

        let secret_ref = match http.secret_ref.as_deref() {
            Some(h) => Some(SecretRef::parse(h, &org)?),
            None => None,
        };

        let (profile, provider) =
            mocks::select_provider(&http.mock_profile, transport).await?;
        let reason_req = ProviderReasonRequest {
            prompt: http.prompt.clone(),
            correlation_id: correlation_id.clone(),
            secret_ref,
            provider_call_id: provider_call_id.clone(),
        };
        let response = provider.reason(&reason_req).await?;
        debug_assert!(!response.tools_executed);

        audit::record_provider_reasoned(
            db,
            &ctx.operator_id,
            &correlation_id,
            &provider_call_id,
            provider.model_id(),
            &http.agent_id,
            &org,
            &response.reasoning_summary,
        )
        .await?;

        let intent = response
            .proposed_intents
            .first()
            .ok_or(ProviderAdapterError::EmptyProposals)?;

        let request_id = if request_id_hint.trim().is_empty() {
            format!("prov-{}", Uuid::new_v4())
        } else {
            request_id_hint
        };

        let mapped = match mapper::map_proposed_intent(
            intent,
            &http.agent_id,
            &http.session_id,
            profile,
            request_id.clone(),
            http.amount_minor,
            http.asset_id.clone(),
        ) {
            Ok(m) => m,
            Err(e) => {
                let reason = e.to_string();
                audit::record_provider_proposal_rejected(
                    db,
                    &ctx.operator_id,
                    &correlation_id,
                    &http.agent_id,
                    &reason,
                )
                .await?;
                let gov =
                    governors::record_outcome(db.pool(), &org, &http.session_id, "DENY").await?;
                let result = GovernedProviderResult {
                    correlation_id,
                    provider_call_id,
                    provider_kind: response.metadata.provider_kind.as_str().into(),
                    model_id: response.metadata.model_id.clone(),
                    agent_id: http.agent_id.clone(),
                    organisation_id: org,
                    session_id: http.session_id.clone(),
                    tool_id: Some(intent.tool_id.clone()),
                    decision: "DENY".into(),
                    reason,
                    simulated_result: Some("STOPPED".into()),
                    review_event_id: None,
                    governor: gov.snapshot(),
                    evidence: json!({
                        "rejection": "injection_or_unsafe_proposal",
                        "provider_has_authority": false,
                    }),
                    external_execution: false,
                    apply_invoked: false,
                    proto0_mutated: false,
                    treasury_mutated: false,
                    tools_executed: false,
                    provider_has_authority: false,
                };
                audit::record_provider_governed(db, &ctx.operator_id, &result).await?;
                return Ok(result);
            }
        };

        let decision = ToolGatewayService::evaluate(
            protocol,
            treasury,
            db,
            ctx,
            &org,
            mapped.http,
            request_id.clone(),
        )
        .await?;

        let exec = executor::apply_decision(db.pool(), &org, &decision).await?;
        let gov = governors::record_outcome(
            db.pool(),
            &org,
            &http.session_id,
            decision.decision.as_str(),
        )
        .await?;

        let result = GovernedProviderResult {
            correlation_id,
            provider_call_id,
            provider_kind: response.metadata.provider_kind.as_str().into(),
            model_id: response.metadata.model_id,
            agent_id: decision.agent_id.clone(),
            organisation_id: org,
            session_id: decision.session_id.clone(),
            tool_id: Some(decision.tool_id.clone()),
            decision: decision.decision.as_str().into(),
            reason: decision.reason.clone(),
            simulated_result: Some(exec.simulated_result.as_str().into()),
            review_event_id: exec.review_event_id,
            governor: gov.snapshot(),
            evidence: json!({
                "gateway": decision.evidence,
                "injection_flags": mapped.injection_flags,
                "reasoning_summary": response.reasoning_summary,
                "provider_has_authority": false,
            }),
            external_execution: false,
            apply_invoked: false,
            proto0_mutated: false,
            treasury_mutated: false,
            tools_executed: false,
            provider_has_authority: false,
        };

        audit::record_provider_governed(db, &ctx.operator_id, &result).await?;
        Ok(result)
    }
}
