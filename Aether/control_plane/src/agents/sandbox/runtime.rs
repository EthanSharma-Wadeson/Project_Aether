use chrono::{DateTime, Utc};

use crate::agents::runtime::models::{
    AgentProviderType, CreateAgentRequest, CreateSessionRequest,
};
use crate::agents::runtime::{registry as agent_registry, sessions};
use crate::auth::middleware::AuthContext;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::protocol::state::ProtocolState;
use crate::tools::runtime::models::{CreateToolRequest, RiskLevel, ToolStatus};
use crate::tools::runtime::{registry as tool_registry, ToolGatewayService};
use crate::treasury::TreasuryAdapter;

use super::audit;
use super::executor;
use super::models::{
    AuditReconstruction, SandboxAgent, SandboxAgentStatus, SandboxRunResult, SandboxRuntimeType,
    SandboxTaskRequest,
};
use super::planner;

const BUDGET_ONE_POUND_MINOR: i64 = 100;

/// Orchestrates sandbox lifecycle: bootstrap scenarios + run tasks.
pub struct SandboxRuntime;

impl SandboxRuntime {
    /// Seed Agent A (research) + Agent B (finance) with sessions, tools, simulated £1.
    pub async fn bootstrap(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        ctx: &AuthContext,
    ) -> Result<Vec<SandboxAgent>> {
        let org = treasury.organisation_id().to_string();
        let proto_a = protocol.index.agent_ids.get(1).cloned().ok_or_else(|| {
            Error::BadRequest("bootstrap protocol agent missing for Agent A".into())
        })?;
        // Distinct display; reuse same proto subject for B would collide agent_id PK —
        // Agent B uses a dedicated runtime id (E2 will DENY missing PROTO-0 — expected for finance deny/review demos).
        let agent_b_id = format!("rt-sandbox-finance-{}", &org);

        ensure_tools(db, &org, &ctx.operator_id).await?;

        let a = upsert_scenario_agent(
            db,
            &org,
            &ctx.operator_id,
            "agent_a",
            &proto_a,
            "Research assistant",
            AgentProviderType::Anthropic,
            SandboxRuntimeType::ResearchAssistant,
        )
        .await?;

        // £1 is recorded as simulated_budget_minor on sandbox_agents only.
        // Treasury allocations for E2 demos are created by tests/fixtures — not here
        // (Phase 31: no treasury writes from sandbox runtime).

        let b = upsert_scenario_agent(
            db,
            &org,
            &ctx.operator_id,
            "agent_b",
            &agent_b_id,
            "Finance assistant",
            AgentProviderType::Google,
            SandboxRuntimeType::FinanceAssistant,
        )
        .await?;

        Ok(vec![a, b])
    }

    pub async fn run_task(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        ctx: &AuthContext,
        task: SandboxTaskRequest,
        request_id_hint: String,
    ) -> Result<SandboxRunResult> {
        let org = treasury.organisation_id().to_string();
        let agent = resolve_agent(db, &org, &task).await?;
        let planned = planner::plan_tool_request(
            &agent,
            &task.tool_id,
            task.parameters,
            task.request_id.or(Some(request_id_hint)),
            task.amount_minor,
            task.asset_id.or_else(|| Some("AETHER_TEST".into())),
        );

        let decision = ToolGatewayService::evaluate(
            protocol,
            treasury,
            db,
            ctx,
            &org,
            planned.http,
            planned.request_id.clone(),
        )
        .await
        .map_err(|e| e.into_cp())?;

        let exec = executor::apply_decision(db.pool(), &org, &decision)
            .await
            .map_err(Error::Database)?;

        let result = SandboxRunResult {
            request_id: decision.request_id.clone(),
            agent_id: decision.agent_id.clone(),
            organisation_id: org.clone(),
            session_id: decision.session_id.clone(),
            tool_id: decision.tool_id.clone(),
            decision: decision.decision_code.clone(),
            reason: decision.reason.clone(),
            simulated_result: exec.simulated_result,
            review_event_id: exec.review_event_id.clone(),
            evidence: serde_json::to_value(&decision.evidence)?,
            external_execution: false,
            apply_invoked: false,
            proto0_mutated: false,
            treasury_mutated: false,
        };

        persist_outcome(db, &result).await?;
        audit::record_sandbox_run(db, &ctx.operator_id, &result).await?;
        Ok(result)
    }

    pub async fn reconstruct(db: &Db, org: &str, request_id: &str) -> Result<AuditReconstruction> {
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
        )> = sqlx::query_as(
            r#"
            SELECT request_id, organisation_id, agent_id, session_id, tool_id,
                   decision_code, reason, simulated_result, review_event_id, created_at
            FROM sandbox_outcomes
            WHERE request_id = ? AND organisation_id = ?
            "#,
        )
        .bind(request_id)
        .bind(org)
        .fetch_optional(db.pool())
        .await?;

        let Some((
            request_id,
            organisation_id,
            agent_id,
            session_id,
            tool_id,
            decision,
            reason,
            simulated_result,
            review_event_id,
            created_at,
        )) = row
        else {
            return Err(Error::NotFound(format!(
                "sandbox outcome not found: {request_id}"
            )));
        };

        Ok(AuditReconstruction {
            request_id,
            agent_id,
            organisation_id,
            session_id,
            tool_id,
            decision,
            reason,
            simulated_result,
            review_event_id,
            created_at,
        })
    }

    pub async fn get_scenario(db: &Db, org: &str, scenario: &str) -> Result<SandboxAgent> {
        load_sandbox_agent(db, org, scenario)
            .await?
            .ok_or_else(|| Error::NotFound(format!("sandbox scenario not found: {scenario}")))
    }
}

async fn ensure_tools(db: &Db, org: &str, operator_id: &str) -> Result<()> {
    // research.search — lab capability bound to settlement.settle for PROTO-linked Agent A ALLOW demos
    if tool_registry::get_tool(db.pool(), org, "research.search")
        .await
        .map_err(|e| e.into_cp())?
        .is_none()
    {
        tool_registry::create_tool(
            db.pool(),
            org,
            operator_id,
            &CreateToolRequest {
                tool_id: "research.search".into(),
                name: "Research Search".into(),
                description: Some("Sandbox research tool".into()),
                risk_level: RiskLevel::Low,
                required_capability: "settlement.settle".into(),
            },
        )
        .await
        .map_err(|e| e.into_cp())?;
    }

    if tool_registry::get_tool(db.pool(), org, "purchase.subscription")
        .await
        .map_err(|e| e.into_cp())?
        .is_none()
    {
        tool_registry::create_tool(
            db.pool(),
            org,
            operator_id,
            &CreateToolRequest {
                tool_id: "purchase.subscription".into(),
                name: "Purchase Subscription".into(),
                description: Some("Sandbox finance restricted tool".into()),
                risk_level: RiskLevel::High,
                required_capability: "purchase.subscription".into(),
            },
        )
        .await
        .map_err(|e| e.into_cp())?;
        tool_registry::set_status(
            db.pool(),
            org,
            "purchase.subscription",
            ToolStatus::Restricted,
        )
        .await
        .map_err(|e| e.into_cp())?;
    }

    if tool_registry::get_tool(db.pool(), org, "deploy.production")
        .await
        .map_err(|e| e.into_cp())?
        .is_none()
    {
        // CRITICAL + capability Agent A holds → E2 ALLOW then gateway REQUIRES_REVIEW
        tool_registry::create_tool(
            db.pool(),
            org,
            operator_id,
            &CreateToolRequest {
                tool_id: "deploy.production".into(),
                name: "Deploy Production".into(),
                description: Some("High-risk sandbox tool".into()),
                risk_level: RiskLevel::Critical,
                required_capability: "settlement.settle".into(),
            },
        )
        .await
        .map_err(|e| e.into_cp())?;
    }
    Ok(())
}

async fn upsert_scenario_agent(
    db: &Db,
    org: &str,
    operator_id: &str,
    scenario: &str,
    agent_id: &str,
    display_name: &str,
    provider: AgentProviderType,
    runtime_type: SandboxRuntimeType,
) -> Result<SandboxAgent> {
    if let Some(existing) = load_sandbox_agent(db, org, scenario).await? {
        return Ok(existing);
    }

    if agent_registry::get_agent(db.pool(), org, agent_id)
        .await
        .map_err(|e| e.into_cp())?
        .is_none()
    {
        agent_registry::create_agent(
            db.pool(),
            org,
            operator_id,
            &CreateAgentRequest {
                display_name: display_name.into(),
                description: Some(format!("sandbox {scenario}")),
                provider_type: provider,
                model_identifier: None,
                agent_id: Some(agent_id.into()),
            },
        )
        .await
        .map_err(|e| e.into_cp())?;
    }

    let (session, _) = sessions::create_session(
        db.pool(),
        org,
        &CreateSessionRequest {
            agent_id: agent_id.into(),
            ttl_secs: Some(7200),
        },
    )
    .await
    .map_err(|e| e.into_cp())?;

    let created_at = Utc::now();
    let sandbox_key = format!("{org}:{scenario}");
    sqlx::query(
        r#"
        INSERT INTO sandbox_agents
          (sandbox_key, agent_id, organisation_id, session_id, runtime_type,
           status, scenario, simulated_budget_minor, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&sandbox_key)
    .bind(agent_id)
    .bind(org)
    .bind(&session.session_id)
    .bind(runtime_type.as_str())
    .bind(SandboxAgentStatus::Active.as_str())
    .bind(scenario)
    .bind(BUDGET_ONE_POUND_MINOR)
    .bind(created_at.to_rfc3339())
    .execute(db.pool())
    .await?;

    Ok(SandboxAgent {
        agent_id: agent_id.into(),
        organisation_id: org.into(),
        session_id: session.session_id,
        runtime_type,
        status: SandboxAgentStatus::Active,
        scenario: scenario.into(),
        simulated_budget_minor: BUDGET_ONE_POUND_MINOR,
        sandbox_key,
        created_at,
    })
}

async fn resolve_agent(
    db: &Db,
    org: &str,
    task: &SandboxTaskRequest,
) -> Result<SandboxAgent> {
    if let Some(ref sc) = task.scenario {
        return SandboxRuntime::get_scenario(db, org, sc).await;
    }
    if let Some(ref aid) = task.agent_id {
        // Find by agent_id among sandbox rows
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            i64,
            String,
        )> = sqlx::query_as(
            r#"
            SELECT sandbox_key, agent_id, organisation_id, session_id, runtime_type,
                   status, scenario, simulated_budget_minor, created_at
            FROM sandbox_agents
            WHERE organisation_id = ? AND agent_id = ?
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(org)
        .bind(aid)
        .fetch_optional(db.pool())
        .await?;
        return row
            .map(map_sandbox_row)
            .transpose()?
            .ok_or_else(|| Error::NotFound(format!("sandbox agent not found: {aid}")));
    }
    Err(Error::BadRequest(
        "scenario or agent_id required for sandbox task".into(),
    ))
}

async fn load_sandbox_agent(
    db: &Db,
    org: &str,
    scenario: &str,
) -> Result<Option<SandboxAgent>> {
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT sandbox_key, agent_id, organisation_id, session_id, runtime_type,
               status, scenario, simulated_budget_minor, created_at
        FROM sandbox_agents
        WHERE organisation_id = ? AND scenario = ?
        "#,
    )
    .bind(org)
    .bind(scenario)
    .fetch_optional(db.pool())
    .await?;
    row.map(map_sandbox_row).transpose()
}

fn map_sandbox_row(
    row: (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        String,
    ),
) -> Result<SandboxAgent> {
    let (
        sandbox_key,
        agent_id,
        organisation_id,
        session_id,
        runtime_type,
        status,
        scenario,
        simulated_budget_minor,
        created_at,
    ) = row;
    let runtime_type = SandboxRuntimeType::parse(&runtime_type)
        .ok_or_else(|| Error::BadRequest(format!("bad runtime_type: {runtime_type}")))?;
    let status = SandboxAgentStatus::parse(&status)
        .ok_or_else(|| Error::BadRequest(format!("bad sandbox status: {status}")))?;
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    Ok(SandboxAgent {
        agent_id,
        organisation_id,
        session_id,
        runtime_type,
        status,
        scenario,
        simulated_budget_minor,
        sandbox_key,
        created_at,
    })
}

async fn persist_outcome(db: &Db, result: &SandboxRunResult) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO sandbox_outcomes
          (request_id, organisation_id, agent_id, session_id, tool_id,
           decision_code, reason, simulated_result, review_event_id, evidence_json, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&result.request_id)
    .bind(&result.organisation_id)
    .bind(&result.agent_id)
    .bind(&result.session_id)
    .bind(&result.tool_id)
    .bind(&result.decision)
    .bind(&result.reason)
    .bind(result.simulated_result.as_str())
    .bind(&result.review_event_id)
    .bind(result.evidence.to_string())
    .bind(Utc::now().to_rfc3339())
    .execute(db.pool())
    .await?;
    Ok(())
}
