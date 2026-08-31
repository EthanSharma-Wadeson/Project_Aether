//! Phase 26 — Runtime spend enforcement foundation (E2 decision engine).

use std::sync::Arc;

use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
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
    let db_path = dir.path().join("phase26.db");
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

fn agent_subject(state: &AppState) -> String {
    state.protocol.index.agent_ids[1].clone()
}

async fn post_eval(
    router: &axum::Router,
    bearer: &str,
    csrf_token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/enforcement/evaluate")
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
    let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
}

async fn fund_agent_allocation(state: &AppState, agent: &str, remaining_ceiling: i64) -> String {
    let eng = state.treasury.engine();
    let _ = eng
        .register_asset(AssetId::new("AETHER_TEST"), 0, "test")
        .await;
    let org = state.treasury.organisation_id().to_string();
    let list = eng.list_treasuries(&org).await.unwrap();
    let tid = list[0].treasury_id.clone();
    let ik = format!("p26-fund-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let _ = eng
        .fund(
            &tid,
            AssetId::new("AETHER_TEST"),
            Amount(remaining_ceiling.max(10_000)),
            "p26-fund",
            &ik,
        )
        .await;
    let (alloc, _) = eng
        .create_allocation(
            &tid,
            agent,
            AssetId::new("AETHER_TEST"),
            Amount(remaining_ceiling),
            Amount(0),
            None,
            "p26-alloc",
            &format!("p26-alloc-{ik}"),
        )
        .await
        .unwrap();
    alloc.allocation_id
}

#[tokio::test]
async fn allowed_agent_action() {
    let (state, _dir) = setup("org-default").await;
    let agent = agent_subject(&state);
    fund_agent_allocation(&state, &agent, 5_000).await;

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_eval(
        &router,
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent,
            "organisation_id": "org-default",
            "action": "settlement.settle",
            "asset_id": "AETHER_TEST",
            "amount_minor": 100
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "ALLOW");
    assert_eq!(body["apply_invoked"], false);
    assert_eq!(body["treasury_mutated"], false);
    assert_eq!(body["proto0_mutated"], false);
}

#[tokio::test]
async fn missing_capability() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    // Unknown agent → no capability
    let (status, body) = post_eval(
        &router,
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": "agent-does-not-exist",
            "action": "settlement.settle",
            "asset_id": "AETHER_TEST",
            "amount_minor": 10
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    // not found before capability, or capability missing
    let reason = body["deny_reason"].as_str().unwrap_or("");
    assert!(
        reason == "AGENT_NOT_FOUND" || reason == "CAPABILITY_MISSING",
        "{body}"
    );
}

#[tokio::test]
async fn insufficient_allocation() {
    let (state, _dir) = setup("org-default").await;
    let agent = agent_subject(&state);
    fund_agent_allocation(&state, &agent, 50).await;

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_eval(
        &router,
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent,
            "action": "settlement.settle",
            "asset_id": "AETHER_TEST",
            "amount_minor": 200
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    assert_eq!(body["deny_reason"], "ALLOCATION_EXCEEDED");
}

#[tokio::test]
async fn expired_allocation() {
    let (state, _dir) = setup("org-default").await;
    let agent = agent_subject(&state);
    let eng = state.treasury.engine();
    let _ = eng
        .register_asset(AssetId::new("AETHER_TEST"), 0, "test")
        .await;
    let org = state.treasury.organisation_id().to_string();
    let tid = eng.list_treasuries(&org).await.unwrap()[0]
        .treasury_id
        .clone();
    let _ = eng
        .fund(
            &tid,
            AssetId::new("AETHER_TEST"),
            Amount(10_000),
            "p26-exp-fund",
            "p26-exp-fund-ik",
        )
        .await;
    let past = Utc::now() - Duration::hours(2);
    let (alloc, _) = eng
        .create_allocation(
            &tid,
            &agent,
            AssetId::new("AETHER_TEST"),
            Amount(5_000),
            Amount(0),
            Some(past),
            "p26-exp",
            "p26-exp-ik",
        )
        .await
        .unwrap();
    let _ = eng
        .expire_allocation_if_needed(&alloc.allocation_id)
        .await
        .unwrap();

    let router = app(state.clone());
    let t = token(&state, "admin").await;
    let oid = op_id(&state, "admin").await;
    let (status, body) = post_eval(
        &router,
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent,
            "action": "settlement.settle",
            "asset_id": "AETHER_TEST",
            "amount_minor": 100
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    assert_eq!(body["deny_reason"], "EXPIRED_ALLOCATION");
}

#[tokio::test]
async fn frozen_agent_unit_path_via_decision() {
    // Integration cannot mutate Arc<ProtocolState> freeze without Apply;
    // pure decision engine covers AGENT_FROZEN (see lib unit tests).
    use aether_control_plane::enforcement::decision::decide;
    use aether_control_plane::enforcement::models::*;
    use std::collections::HashMap;

    let mut treasuries = HashMap::new();
    treasuries.insert("tr".into(), "active".into());
    let snap = AgentAuthoritySnapshot {
        agent_id: "a".into(),
        agent_status: "frozen".into(),
        capabilities: vec![],
        allocations: vec![],
        treasury_status_by_id: treasuries,
        known_assets: vec!["AETHER_TEST".into()],
        policy: None,
    };
    let req = EnforcementRequest {
        agent_id: "a".into(),
        organisation_id: None,
        action: "settlement.settle".into(),
        asset_id: Some("AETHER_TEST".into()),
        amount_minor: Some(1),
        policy_id: None,
        force_review: false,
    };
    let d = decide(&req, &snap, "org-default", Some("rid".into()));
    assert_eq!(d.deny_reason, Some(DenyReason::AgentFrozen));
}

#[tokio::test]
async fn cross_org_isolation() {
    let (state, _dir) = setup("org-default").await;
    let agent = agent_subject(&state);
    fund_agent_allocation(&state, &agent, 5_000).await;

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_eval(
        &router,
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent,
            "organisation_id": "org-other",
            "action": "settlement.settle",
            "asset_id": "AETHER_TEST",
            "amount_minor": 100
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "DENY");
    assert_eq!(body["deny_reason"], "ORGANISATION_MISMATCH");
}

#[tokio::test]
async fn deterministic_decisions() {
    let (state, _dir) = setup("org-default").await;
    let agent = agent_subject(&state);
    fund_agent_allocation(&state, &agent, 5_000).await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let payload = json!({
        "agent_id": agent,
        "action": "settlement.settle",
        "asset_id": "AETHER_TEST",
        "amount_minor": 25
    });
    let (s1, b1) = post_eval(&router, &t, Some(&csrf(&state, &oid)), payload.clone()).await;
    let (s2, b2) = post_eval(&router, &t, Some(&csrf(&state, &oid)), payload).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(b1["decision"], b2["decision"]);
    assert_eq!(b1["deny_reason"], b2["deny_reason"]);
    assert_eq!(b1["evidence"]["allocation_id"], b2["evidence"]["allocation_id"]);
}

#[tokio::test]
async fn no_treasury_mutation_on_evaluate() {
    let (state, _dir) = setup("org-default").await;
    let agent = agent_subject(&state);
    let alloc_id = fund_agent_allocation(&state, &agent, 5_000).await;
    let before = state
        .treasury
        .engine()
        .list_allocations_for_organisation(state.treasury.organisation_id())
        .await
        .unwrap();
    let rem_before = before
        .iter()
        .find(|a| a.allocation_id == alloc_id)
        .unwrap()
        .remaining_minor;
    let journal_before = state
        .treasury
        .engine()
        .list_journal_entries(state.treasury.organisation_id())
        .await
        .unwrap()
        .len();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (_, body) = post_eval(
        &router,
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent,
            "action": "settlement.settle",
            "asset_id": "AETHER_TEST",
            "amount_minor": 100
        }),
    )
    .await;
    assert_eq!(body["decision"], "ALLOW");
    assert_eq!(body["treasury_mutated"], false);

    let after = state
        .treasury
        .engine()
        .list_allocations_for_organisation(state.treasury.organisation_id())
        .await
        .unwrap();
    let rem_after = after
        .iter()
        .find(|a| a.allocation_id == alloc_id)
        .unwrap()
        .remaining_minor;
    assert_eq!(rem_before, rem_after);
    let journal_after = state
        .treasury
        .engine()
        .list_journal_entries(state.treasury.organisation_id())
        .await
        .unwrap()
        .len();
    assert_eq!(journal_before, journal_after);
}

#[tokio::test]
async fn no_apply_invocation_flags() {
    let (state, _dir) = setup("org-default").await;
    let agent = agent_subject(&state);
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (_, body) = post_eval(
        &router,
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent,
            "action": "settlement.settle",
            "asset_id": "AETHER_TEST",
            "amount_minor": 10
        }),
    )
    .await;
    assert_eq!(body["apply_invoked"], false);
    assert_eq!(body["proto0_mutated"], false);
}

#[tokio::test]
async fn viewer_forbidden_csrf_required() {
    let (state, _dir) = setup("org-default").await;
    let agent = agent_subject(&state);
    let router = app(state.clone());
    let t = token(&state, "viewer").await;
    let oid = op_id(&state, "viewer").await;
    let (status, _) = post_eval(
        &router,
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": agent,
            "action": "settlement.settle",
            "amount_minor": 1
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let t = token(&state, "operator").await;
    let (status, _) = post_eval(
        &router,
        &t,
        None,
        json!({
            "agent_id": agent,
            "action": "settlement.settle",
            "amount_minor": 1
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
