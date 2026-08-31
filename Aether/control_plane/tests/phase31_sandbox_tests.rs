//! Phase 31 — Agent runtime sandbox lifecycle tests.

use std::sync::Arc;

use aether_control_plane::agents::runtime::models::AgentStatus;
use aether_control_plane::agents::runtime::registry as agent_reg;
use aether_control_plane::agents::sandbox::models::SandboxTaskRequest;
use aether_control_plane::agents::sandbox::SandboxRuntime;
use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::auth::middleware::AuthContext;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::{find_by_username, OperatorRole};
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
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
    let db_path = dir.path().join("phase31.db");
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

async fn ctx(state: &AppState) -> AuthContext {
    let op = find_by_username(state.db.pool(), "operator")
        .await
        .unwrap()
        .unwrap();
    AuthContext {
        operator_id: op.id,
        username: op.username,
        role: OperatorRole::Operator,
    }
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

async fn get_json(router: &axum::Router, path: &str, bearer: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .header("Authorization", format!("Bearer {bearer}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Fixture allocation for E2 spend-scoped research tool (test-only; not sandbox module).
async fn fixture_alloc(state: &AppState, agent_id: &str) {
    let eng = state.treasury.engine();
    let _ = eng
        .register_asset(AssetId::new("AETHER_TEST"), 0, "test")
        .await;
    let org = state.treasury.organisation_id().to_string();
    let tid = eng.list_treasuries(&org).await.unwrap()[0]
        .treasury_id
        .clone();
    let ik = format!("p31-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let _ = eng
        .fund(
            &tid,
            AssetId::new("AETHER_TEST"),
            Amount(10_000),
            "p31",
            &ik,
        )
        .await;
    if eng
        .list_allocations_for_organisation(&org)
        .await
        .unwrap()
        .iter()
        .any(|a| a.agent_id == agent_id)
    {
        return;
    }
    let _ = eng
        .create_allocation(
            &tid,
            agent_id,
            AssetId::new("AETHER_TEST"),
            Amount(100),
            Amount(0),
            None,
            "p31-a",
            &format!("p31-a-{ik}"),
        )
        .await
        .unwrap();
}

async fn boot(state: &AppState) {
    let c = ctx(state).await;
    SandboxRuntime::bootstrap(&state.protocol, &state.treasury, &state.db, &c)
        .await
        .unwrap();
}

#[tokio::test]
async fn allow_valid_tool_request() {
    let (state, _dir) = setup("org-default").await;
    boot(&state).await;
    let a = SandboxRuntime::get_scenario(&state.db, "org-default", "agent_a")
        .await
        .unwrap();
    fixture_alloc(&state, &a.agent_id).await;

    let c = ctx(&state).await;
    let result = SandboxRuntime::run_task(
        &state.protocol,
        &state.treasury,
        &state.db,
        &c,
        SandboxTaskRequest {
            scenario: Some("agent_a".into()),
            agent_id: None,
            tool_id: "research.search".into(),
            parameters: json!({"q": "aether"}),
            request_id: Some("sbx-allow-1".into()),
            amount_minor: Some(10),
            asset_id: Some("AETHER_TEST".into()),
        },
        "hint".into(),
    )
    .await
    .unwrap();
    assert_eq!(result.decision, "ALLOW");
    assert_eq!(result.simulated_result.as_str(), "SUCCESS");
    assert!(!result.external_execution);
    assert!(!result.treasury_mutated);
}

#[tokio::test]
async fn deny_missing_capability() {
    let (state, _dir) = setup("org-default").await;
    boot(&state).await;
    let c = ctx(&state).await;
    let result = SandboxRuntime::run_task(
        &state.protocol,
        &state.treasury,
        &state.db,
        &c,
        SandboxTaskRequest {
            scenario: Some("agent_b".into()),
            agent_id: None,
            tool_id: "purchase.subscription".into(),
            parameters: json!({}),
            request_id: Some("sbx-deny-cap".into()),
            amount_minor: None,
            asset_id: None,
        },
        "hint".into(),
    )
    .await
    .unwrap();
    assert_eq!(result.decision, "DENY");
    assert_eq!(result.simulated_result.as_str(), "STOPPED");
}

#[tokio::test]
async fn deny_expired_session() {
    let (state, _dir) = setup("org-default").await;
    boot(&state).await;
    let a = SandboxRuntime::get_scenario(&state.db, "org-default", "agent_a")
        .await
        .unwrap();
    sqlx::query("UPDATE runtime_sessions SET expires_at = ? WHERE session_id = ?")
        .bind((Utc::now() - Duration::hours(1)).to_rfc3339())
        .bind(&a.session_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let c = ctx(&state).await;
    let result = SandboxRuntime::run_task(
        &state.protocol,
        &state.treasury,
        &state.db,
        &c,
        SandboxTaskRequest {
            scenario: Some("agent_a".into()),
            agent_id: None,
            tool_id: "research.search".into(),
            parameters: json!({}),
            request_id: Some("sbx-exp".into()),
            amount_minor: Some(1),
            asset_id: Some("AETHER_TEST".into()),
        },
        "hint".into(),
    )
    .await
    .unwrap();
    assert_eq!(result.decision, "DENY");
    assert!(result.reason.contains("SESSION_EXPIRED"));
    assert_eq!(result.simulated_result.as_str(), "STOPPED");
}

#[tokio::test]
async fn deny_frozen_agent() {
    let (state, _dir) = setup("org-default").await;
    boot(&state).await;
    let a = SandboxRuntime::get_scenario(&state.db, "org-default", "agent_a")
        .await
        .unwrap();
    agent_reg::set_agent_status(
        state.db.pool(),
        "org-default",
        &a.agent_id,
        AgentStatus::Frozen,
    )
    .await
    .unwrap();
    let c = ctx(&state).await;
    let result = SandboxRuntime::run_task(
        &state.protocol,
        &state.treasury,
        &state.db,
        &c,
        SandboxTaskRequest {
            scenario: Some("agent_a".into()),
            agent_id: None,
            tool_id: "research.search".into(),
            parameters: json!({}),
            request_id: Some("sbx-frozen".into()),
            amount_minor: Some(1),
            asset_id: Some("AETHER_TEST".into()),
        },
        "hint".into(),
    )
    .await
    .unwrap();
    assert_eq!(result.decision, "DENY");
    assert_eq!(result.simulated_result.as_str(), "STOPPED");
}

#[tokio::test]
async fn review_high_risk_tool() {
    let (state, _dir) = setup("org-default").await;
    boot(&state).await;
    let a = SandboxRuntime::get_scenario(&state.db, "org-default", "agent_a")
        .await
        .unwrap();
    fixture_alloc(&state, &a.agent_id).await;
    let c = ctx(&state).await;
    let result = SandboxRuntime::run_task(
        &state.protocol,
        &state.treasury,
        &state.db,
        &c,
        SandboxTaskRequest {
            scenario: Some("agent_a".into()),
            agent_id: None,
            tool_id: "deploy.production".into(),
            parameters: json!({}),
            request_id: Some("sbx-review".into()),
            amount_minor: Some(10),
            asset_id: Some("AETHER_TEST".into()),
        },
        "hint".into(),
    )
    .await
    .unwrap();
    assert_eq!(result.decision, "REQUIRES_REVIEW");
    assert_eq!(result.simulated_result.as_str(), "REVIEW_PENDING");
    assert!(result.review_event_id.is_some());
}

#[tokio::test]
async fn audit_reconstruction() {
    let (state, _dir) = setup("org-default").await;
    boot(&state).await;
    let a = SandboxRuntime::get_scenario(&state.db, "org-default", "agent_a")
        .await
        .unwrap();
    fixture_alloc(&state, &a.agent_id).await;
    let c = ctx(&state).await;
    let result = SandboxRuntime::run_task(
        &state.protocol,
        &state.treasury,
        &state.db,
        &c,
        SandboxTaskRequest {
            scenario: Some("agent_a".into()),
            agent_id: None,
            tool_id: "research.search".into(),
            parameters: json!({"q": 1}),
            request_id: Some("sbx-recon-1".into()),
            amount_minor: Some(5),
            asset_id: Some("AETHER_TEST".into()),
        },
        "hint".into(),
    )
    .await
    .unwrap();

    let rec = SandboxRuntime::reconstruct(&state.db, "org-default", "sbx-recon-1")
        .await
        .unwrap();
    assert_eq!(rec.agent_id, result.agent_id);
    assert_eq!(rec.session_id, result.session_id);
    assert_eq!(rec.tool_id, "research.search");
    assert_eq!(rec.decision, result.decision);
    assert_eq!(rec.reason, result.reason);

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let (status, body) = get_json(&router, "/sandbox/reconstruct/sbx-recon-1", &t).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["agent"], result.agent_id);
    assert_eq!(body["tool"], "research.search");
    assert_eq!(body["decision"], result.decision);
}

#[tokio::test]
async fn http_bootstrap_and_run() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/sandbox/bootstrap",
        &t,
        Some(&csrf(&state, &oid)),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["agents"].as_array().unwrap().len(), 2);
}
