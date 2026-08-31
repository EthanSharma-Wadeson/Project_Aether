//! Phase 30 — Tool registry & decision gateway (no execution).

use std::sync::Arc;

use aether_control_plane::agents::runtime::models::{
    AgentProviderType, AgentStatus, CreateAgentRequest, CreateSessionRequest,
};
use aether_control_plane::agents::runtime::{registry as agent_reg, sessions};
use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
use aether_control_plane::tools::runtime::models::{CreateToolRequest, RiskLevel};
use aether_control_plane::tools::runtime::registry as tool_reg;
use aether_treasury::{Amount, AssetId};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn test_config(db_path: &str, org: &str) -> Config {
    Config {
        host: "127.0.0.1".into(),
        port: 0,
        db_path: db_path.into(),
        jwt_secret: "test-secret-key".into(),
        access_token_ttl_secs: 900,
        refresh_token_ttl_secs: 604_800,
        frontend_dir: None,
        log_level: "error".into(),
        bootstrap_demo: true,
        allowed_origins: Vec::new(),
        signer: aether_control_plane::config::SignerConfig::default(),
        secure_cookies: false,
        treasury_db_path: format!("{}.treasury.db", db_path.trim_end_matches(".db")),
        treasury_organisation_id: org.into(),
        treasury_bootstrap_demo: true,
        treasury_reservation_default_ttl_secs: 900,
        treasury_reservation_max_ttl_secs: 86_400,
        treasury_reservation_orphan_age_secs: 86_400,
        treasury_sweeper_interval_secs: 60,
        treasury_sweeper_enabled: false,
        treasury_mutation_executing_timeout_secs: 300,
        admin_mfa_required: false,
    }
}

async fn setup(org: &str) -> (Arc<AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("phase30.db");
    let config = test_config(db_path.to_str().unwrap(), org);
    let db = Db::connect(&config.db_path).await.unwrap();
    db.migrate().await.unwrap();
    db.seed_default_operators(&config).await.unwrap();
    let protocol = ProtocolState::bootstrap().unwrap();
    let signing =
        aether_control_plane::signer::build_signing_gateway(&config, &db).unwrap();
    let treasury = aether_control_plane::treasury::TreasuryAdapter::connect(&config)
        .await
        .unwrap();
    let treasury_write =
        aether_control_plane::treasury::TreasuryWriteService::new(db.pool().clone());
    let state = Arc::new(AppState {
        mutation_audit: aether_control_plane::audit::mutation::MutationAuditService::new(
            db.pool().clone(),
        ),
        config,
        db,
        protocol,
        rate_limiter: aether_control_plane::auth::rate_limit::InMemoryLoginRateLimiter::shared(),
        csrf_store: aether_control_plane::security::csrf::CsrfStore::shared(),
        signing,
        treasury,
        treasury_write,
    });
    (state, dir)
}

fn app(state: Arc<AppState>) -> axum::Router {
    api_router(state.clone()).layer(axum::middleware::from_fn_with_state(state, auth_middleware))
}

async fn token(state: &AppState, username: &str) -> String {
    let op = find_by_username(state.db.pool(), username)
        .await
        .unwrap()
        .unwrap();
    issue_access_token(&state.config.jwt_secret, &op.id, &op.username, op.role, 900).unwrap()
}

fn csrf(state: &AppState, operator_id: &str) -> String {
    state.csrf_store.issue(operator_id).unwrap()
}

async fn op_id(state: &AppState, username: &str) -> String {
    find_by_username(state.db.pool(), username)
        .await
        .unwrap()
        .unwrap()
        .id
}

async fn post_json(
    router: &axum::Router,
    path: &str,
    bearer: &str,
    csrf_token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header("Authorization", format!("Bearer {bearer}"))
        .header("content-type", "application/json");
    if let Some(c) = csrf_token {
        builder = builder.header("x-csrf-token", c);
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn fund_proto_agent(state: &AppState, agent_id: &str) {
    let eng = state.treasury.engine();
    let _ = eng
        .register_asset(AssetId::new("AETHER_TEST"), 0, "test")
        .await;
    let org = state.treasury.organisation_id().to_string();
    let tid = eng.list_treasuries(&org).await.unwrap()[0]
        .treasury_id
        .clone();
    let ik = format!("p30-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let _ = eng
        .fund(
            &tid,
            AssetId::new("AETHER_TEST"),
            Amount(100_000),
            "p30-fund",
            &ik,
        )
        .await;
    let _ = eng
        .create_allocation(
            &tid,
            agent_id,
            AssetId::new("AETHER_TEST"),
            Amount(5_000),
            Amount(0),
            None,
            "p30-alloc",
            &format!("p30-alloc-{ik}"),
        )
        .await
        .unwrap();
}

async fn register_linked_agent_session(state: &AppState) -> (String, String) {
    let proto_id = state.protocol.index.agent_ids[1].clone();
    agent_reg::create_agent(
        state.db.pool(),
        "org-default",
        "op",
        &CreateAgentRequest {
            display_name: "Claude Research".into(),
            description: Some("lab".into()),
            provider_type: AgentProviderType::Anthropic,
            model_identifier: Some("claude-lab".into()),
            agent_id: Some(proto_id.clone()),
        },
    )
    .await
    .unwrap();
    fund_proto_agent(state, &proto_id).await;
    let (sess, _) = sessions::create_session(
        state.db.pool(),
        "org-default",
        &CreateSessionRequest {
            agent_id: proto_id.clone(),
            ttl_secs: Some(3600),
        },
    )
    .await
    .unwrap();
    (proto_id, sess.session_id)
}

#[tokio::test]
async fn create_tool() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/tools",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "tool_id": "research.search",
            "name": "Research Search",
            "description": "lab",
            "risk_level": "LOW",
            "required_capability": "research.search"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["tool"]["status"], "ACTIVE");
    assert_eq!(body["tool"]["risk_level"], "LOW");
}

#[tokio::test]
async fn duplicate_tool_rejection() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let body = json!({
        "tool_id": "research.search",
        "name": "A",
        "risk_level": "LOW",
        "required_capability": "research.search"
    });
    let (_, _) = post_json(&router, "/tools", &t, Some(&csrf(&state, &oid)), body.clone()).await;
    let (status, _) = post_json(&router, "/tools", &t, Some(&csrf(&state, &oid)), body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn disabled_tool() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    tool_reg::create_tool(
        state.db.pool(),
        "org-default",
        "op",
        &CreateToolRequest {
            tool_id: "research.search".into(),
            name: "R".into(),
            description: None,
            risk_level: RiskLevel::Low,
            required_capability: "settlement.settle".into(),
        },
    )
    .await
    .unwrap();
    tool_reg::set_status(
        state.db.pool(),
        "org-default",
        "research.search",
        aether_control_plane::tools::runtime::models::ToolStatus::Disabled,
    )
    .await
    .unwrap();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent_id,
            "session_id": session_id,
            "tool_id": "research.search",
            "parameters": {},
            "request_id": "req-disabled-1",
            "amount_minor": 10,
            "asset_id": "AETHER_TEST"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    assert!(body["reason"].as_str().unwrap().contains("DISABLED"));
    assert_eq!(body["tools_executed"], false);
}

#[tokio::test]
async fn valid_allow_path() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    tool_reg::create_tool(
        state.db.pool(),
        "org-default",
        "op",
        &CreateToolRequest {
            tool_id: "spend.settle".into(),
            name: "Settle".into(),
            description: None,
            risk_level: RiskLevel::Low,
            required_capability: "settlement.settle".into(),
        },
    )
    .await
    .unwrap();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent_id,
            "session_id": session_id,
            "tool_id": "spend.settle",
            "parameters": {"q": "x"},
            "request_id": "req-allow-1",
            "amount_minor": 100,
            "asset_id": "AETHER_TEST"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "ALLOW", "{body}");
    assert_eq!(body["tools_executed"], false);
    assert_eq!(body["apply_invoked"], false);
    assert_eq!(body["treasury_mutated"], false);
    assert!(body["evidence"]["e2_decision"].as_str().unwrap() == "ALLOW");
}

#[tokio::test]
async fn missing_capability_deny() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    tool_reg::create_tool(
        state.db.pool(),
        "org-default",
        "op",
        &CreateToolRequest {
            tool_id: "deploy.production".into(),
            name: "Deploy".into(),
            description: None,
            risk_level: RiskLevel::Critical,
            required_capability: "deploy.production".into(),
        },
    )
    .await
    .unwrap();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent_id,
            "session_id": session_id,
            "tool_id": "deploy.production",
            "parameters": {},
            "request_id": "req-cap-miss"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    assert_eq!(body["tools_executed"], false);
}

#[tokio::test]
async fn frozen_agent_deny() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    tool_reg::create_tool(
        state.db.pool(),
        "org-default",
        "op",
        &CreateToolRequest {
            tool_id: "spend.settle".into(),
            name: "S".into(),
            description: None,
            risk_level: RiskLevel::Low,
            required_capability: "settlement.settle".into(),
        },
    )
    .await
    .unwrap();
    agent_reg::set_agent_status(
        state.db.pool(),
        "org-default",
        &agent_id,
        AgentStatus::Frozen,
    )
    .await
    .unwrap();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent_id,
            "session_id": session_id,
            "tool_id": "spend.settle",
            "parameters": {},
            "request_id": "req-frozen",
            "amount_minor": 1,
            "asset_id": "AETHER_TEST"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    assert!(body["reason"].as_str().unwrap().contains("AGENT"));
}

#[tokio::test]
async fn expired_session_deny() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    sqlx::query("UPDATE runtime_sessions SET expires_at = ? WHERE session_id = ?")
        .bind((Utc::now() - Duration::minutes(5)).to_rfc3339())
        .bind(&session_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    tool_reg::create_tool(
        state.db.pool(),
        "org-default",
        "op",
        &CreateToolRequest {
            tool_id: "spend.settle".into(),
            name: "S".into(),
            description: None,
            risk_level: RiskLevel::Low,
            required_capability: "settlement.settle".into(),
        },
    )
    .await
    .unwrap();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent_id,
            "session_id": session_id,
            "tool_id": "spend.settle",
            "parameters": {},
            "request_id": "req-exp",
            "amount_minor": 1,
            "asset_id": "AETHER_TEST"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    assert!(body["reason"].as_str().unwrap().contains("SESSION_EXPIRED"));
}

#[tokio::test]
async fn cross_org_deny() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    // Tool only in foreign org
    tool_reg::create_tool(
        state.db.pool(),
        "org-other",
        "op",
        &CreateToolRequest {
            tool_id: "secret.tool".into(),
            name: "Secret".into(),
            description: None,
            risk_level: RiskLevel::Low,
            required_capability: "settlement.settle".into(),
        },
    )
    .await
    .unwrap();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent_id,
            "session_id": session_id,
            "tool_id": "secret.tool",
            "parameters": {},
            "request_id": "req-xorg"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    assert!(body["reason"]
        .as_str()
        .unwrap()
        .contains("TOOL_NOT_REGISTERED"));
}

#[tokio::test]
async fn deterministic_decisions() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    tool_reg::create_tool(
        state.db.pool(),
        "org-default",
        "op",
        &CreateToolRequest {
            tool_id: "spend.settle".into(),
            name: "S".into(),
            description: None,
            risk_level: RiskLevel::Low,
            required_capability: "settlement.settle".into(),
        },
    )
    .await
    .unwrap();
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let payload = |rid: &str| {
        json!({
            "agent_id": agent_id,
            "session_id": session_id,
            "tool_id": "spend.settle",
            "parameters": {"n": 1},
            "request_id": rid,
            "amount_minor": 50,
            "asset_id": "AETHER_TEST"
        })
    };
    let (_, a) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        payload("det-a"),
    )
    .await;
    let (_, b) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        payload("det-b"),
    )
    .await;
    assert_eq!(a["decision"], b["decision"]);
    assert_eq!(a["decision_code"], b["decision_code"]);
}

#[tokio::test]
async fn replay_protection() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    tool_reg::create_tool(
        state.db.pool(),
        "org-default",
        "op",
        &CreateToolRequest {
            tool_id: "spend.settle".into(),
            name: "S".into(),
            description: None,
            risk_level: RiskLevel::Low,
            required_capability: "settlement.settle".into(),
        },
    )
    .await
    .unwrap();
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let body = json!({
        "agent_id": agent_id,
        "session_id": session_id,
        "tool_id": "spend.settle",
        "parameters": {},
        "request_id": "replay-same",
        "amount_minor": 10,
        "asset_id": "AETHER_TEST"
    });
    let (s1, _) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        body.clone(),
    )
    .await;
    assert_eq!(s1, StatusCode::OK);
    let (s2, body2) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        body,
    )
    .await;
    assert_eq!(s2, StatusCode::FORBIDDEN, "{body2}");
}

#[tokio::test]
async fn no_e2_bypass_and_no_execution_side_effects() {
    let (state, _dir) = setup("org-default").await;
    let (agent_id, session_id) = register_linked_agent_session(&state).await;
    tool_reg::create_tool(
        state.db.pool(),
        "org-default",
        "op",
        &CreateToolRequest {
            tool_id: "spend.settle".into(),
            name: "S".into(),
            description: None,
            risk_level: RiskLevel::Low,
            required_capability: "settlement.settle".into(),
        },
    )
    .await
    .unwrap();
    let journal_before = state
        .treasury
        .engine()
        .list_journal_entries("org-default")
        .await
        .unwrap()
        .len();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (_, body) = post_json(
        &router,
        "/tools/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent_id,
            "session_id": session_id,
            "tool_id": "spend.settle",
            "parameters": {},
            "request_id": "side-fx",
            "amount_minor": 25,
            "asset_id": "AETHER_TEST"
        }),
    )
    .await;
    assert!(body["evidence"]["e2_decision"].is_string());
    assert_eq!(body["tools_executed"], false);
    assert_eq!(body["proto0_mutated"], false);
    assert_eq!(body["treasury_mutated"], false);
    let journal_after = state
        .treasury
        .engine()
        .list_journal_entries("org-default")
        .await
        .unwrap()
        .len();
    assert_eq!(journal_before, journal_after);
}
