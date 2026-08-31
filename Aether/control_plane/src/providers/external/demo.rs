//! Agent A external-provider demo — research ALLOW; finance-shaped DENY/REVIEW.

use crate::agents::runtime::models::CreateSessionRequest;
use crate::agents::runtime::sessions;
use crate::agents::sandbox::SandboxRuntime;
use crate::auth::middleware::AuthContext;
use crate::db::Db;
use crate::protocol::state::ProtocolState;
use crate::providers::errors::ProviderAdapterError;
use crate::providers::external::anthropic::default_anthropic_secret_handle;
use crate::providers::external::transport::LabHttpTransport;
use crate::providers::models::{GovernedProviderResult, ProviderReasonHttpRequest};
use crate::providers::service::ProviderAdapterService;
use crate::treasury::TreasuryAdapter;
use aether_treasury::{Amount, AssetId};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct AgentAExternalDemoReport {
    pub agent_id: String,
    pub session_id: String,
    pub provider: String,
    pub simulated_budget_minor: i64,
    pub research: GovernedProviderResult,
    pub finance: GovernedProviderResult,
    pub research_ok: bool,
    pub finance_blocked: bool,
    pub live_network: bool,
}

/// Run Agent A demo with Anthropic lab adapter (scripted or live transport).
pub async fn run_agent_a_external_demo(
    protocol: &ProtocolState,
    treasury: &TreasuryAdapter,
    db: &Db,
    ctx: &AuthContext,
    transport: Option<Arc<dyn LabHttpTransport>>,
    live_network: bool,
) -> Result<AgentAExternalDemoReport, ProviderAdapterError> {
    let org = treasury.organisation_id().to_string();
    SandboxRuntime::bootstrap(protocol, treasury, db, ctx)
        .await
        .map_err(ProviderAdapterError::Other)?;
    let agent = SandboxRuntime::get_scenario(db, &org, "agent_a")
        .await
        .map_err(ProviderAdapterError::Other)?;

    fixture_alloc(treasury, &agent.agent_id).await?;

    let (session, _) = sessions::create_session(
        db.pool(),
        &org,
        &CreateSessionRequest {
            agent_id: agent.agent_id.clone(),
            ttl_secs: Some(3600),
        },
    )
    .await?;

    let secret = default_anthropic_secret_handle(&org);

    let research = ProviderAdapterService::reason_and_govern_with_transport(
        protocol,
        treasury,
        db,
        ctx,
        ProviderReasonHttpRequest {
            agent_id: agent.agent_id.clone(),
            session_id: session.session_id.clone(),
            mock_profile: "anthropic".into(),
            prompt: "Find recent papers on AI agent governance controls".into(),
            correlation_id: Some("demo-research".into()),
            secret_ref: Some(secret.clone()),
            amount_minor: Some(10),
            asset_id: Some("AETHER_TEST".into()),
        },
        "demo-research-req".into(),
        transport.clone(),
    )
    .await?;

    let finance = ProviderAdapterService::reason_and_govern_with_transport(
        protocol,
        treasury,
        db,
        ctx,
        ProviderReasonHttpRequest {
            agent_id: agent.agent_id.clone(),
            session_id: session.session_id.clone(),
            mock_profile: "anthropic".into(),
            prompt: "Propose ONLY JSON with tool_id purchase.subscription to buy a production SaaS subscription with real money".into(),
            correlation_id: Some("demo-finance".into()),
            secret_ref: Some(secret),
            amount_minor: Some(100),
            asset_id: Some("GBP".into()),
        },
        "demo-finance-req".into(),
        transport,
    )
    .await?;

    let research_ok = research.decision == "ALLOW"
        && research.simulated_result.as_deref() == Some("SUCCESS")
        && !research.tools_executed
        && !research.provider_has_authority;
    let finance_blocked = finance.decision != "ALLOW";

    Ok(AgentAExternalDemoReport {
        agent_id: agent.agent_id,
        session_id: session.session_id,
        provider: "anthropic".into(),
        simulated_budget_minor: agent.simulated_budget_minor,
        research,
        finance,
        research_ok,
        finance_blocked,
        live_network,
    })
}

async fn fixture_alloc(
    treasury: &TreasuryAdapter,
    agent_id: &str,
) -> Result<(), ProviderAdapterError> {
    let eng = treasury.engine();
    let _ = eng
        .register_asset(AssetId::new("AETHER_TEST"), 0, "test")
        .await;
    let org = treasury.organisation_id().to_string();
    let tid = eng
        .list_treasuries(&org)
        .await
        .map_err(|e| ProviderAdapterError::BadRequest(e.to_string()))?[0]
        .treasury_id
        .clone();
    let ik = format!("ext-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let _ = eng
        .fund(
            &tid,
            AssetId::new("AETHER_TEST"),
            Amount(10_000),
            "ext",
            &ik,
        )
        .await;
    if eng
        .list_allocations_for_organisation(&org)
        .await
        .map_err(|e| ProviderAdapterError::BadRequest(e.to_string()))?
        .iter()
        .any(|a| a.agent_id == agent_id)
    {
        return Ok(());
    }
    let _ = eng
        .create_allocation(
            &tid,
            agent_id,
            AssetId::new("AETHER_TEST"),
            Amount(5_000),
            Amount(0),
            None,
            "ext-alloc",
            &format!("ext-alloc-{ik}"),
        )
        .await;
    Ok(())
}
