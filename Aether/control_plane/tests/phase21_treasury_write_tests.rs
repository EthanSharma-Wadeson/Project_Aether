//! Phase 21 — Treasury write façade security tests.

use std::sync::Arc;

use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
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
    let db_path = dir.path().join("phase21.db");
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
    let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
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
    let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
}

async fn org_treasury_id(state: &AppState) -> String {
    let list = state
        .treasury
        .engine()
        .list_treasuries(state.treasury.organisation_id())
        .await
        .unwrap();
    list.into_iter()
        .find(|t| t.name.contains("Organisation"))
        .unwrap()
        .treasury_id
}

#[tokio::test]
async fn viewer_cannot_create_mutation() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "viewer").await;
    let vid = op_id(&state, "viewer").await;
    let tid = org_treasury_id(&state).await;
    let (status, body) = post_json(
        &router,
        "/treasury/mutations",
        &t,
        Some(&csrf(&state, &vid)),
        json!({
            "operation": "allocation_create",
            "idempotency_key": "v1",
            "payload": {
                "treasury_id": tid,
                "agent_id": "agent-a",
                "asset_id": "GBP",
                "ceiling_minor": 1000,
                "initial_minor": 0
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
async fn csrf_required_on_mutation_request() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let tid = org_treasury_id(&state).await;
    let (status, body) = post_json(
        &router,
        "/treasury/mutations",
        &t,
        None,
        json!({
            "operation": "allocation_create",
            "idempotency_key": "c1",
            "payload": {
                "treasury_id": tid,
                "agent_id": "agent-a",
                "asset_id": "GBP",
                "ceiling_minor": 1000
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "CSRF_FAILED");
}

#[tokio::test]
async fn sod_rejects_self_approval() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let admin_t = token(&state, "admin").await;
    let admin_id = op_id(&state, "admin").await;
    let tid = org_treasury_id(&state).await;
    let (status, created) = post_json(
        &router,
        "/treasury/mutations",
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({
            "operation": "allocation_create",
            "idempotency_key": "sod-1",
            "payload": {
                "treasury_id": tid,
                "agent_id": "agent-sod",
                "asset_id": "GBP",
                "ceiling_minor": 500
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let mid = created["mutation_id"].as_str().unwrap();
    let (status, body) = post_json(
        &router,
        &format!("/treasury/mutations/{mid}/approve"),
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({ "decision": "approve" }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "APPROVAL_SOD");
}

#[tokio::test]
async fn dual_control_allocation_create_and_audit() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let op_t = token(&state, "operator").await;
    let operator_id = op_id(&state, "operator").await;
    let admin_t = token(&state, "admin").await;
    let admin_id = op_id(&state, "admin").await;
    let tid = org_treasury_id(&state).await;

    let (status, created) = post_json(
        &router,
        "/treasury/mutations",
        &op_t,
        Some(&csrf(&state, &operator_id)),
        json!({
            "operation": "allocation_create",
            "idempotency_key": "alloc-1",
            "payload": {
                "treasury_id": tid,
                "agent_id": "agent-1",
                "asset_id": "GBP",
                "ceiling_minor": 2500,
                "initial_minor": 0
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert!(created["ledger_notice"].as_str().unwrap().contains("Internal ledger"));
    let mid = created["mutation_id"].as_str().unwrap().to_string();

    // duplicate idempotency
    let (status, dup) = post_json(
        &router,
        "/treasury/mutations",
        &op_t,
        Some(&csrf(&state, &operator_id)),
        json!({
            "operation": "allocation_create",
            "idempotency_key": "alloc-1",
            "payload": {
                "treasury_id": tid,
                "agent_id": "agent-1",
                "asset_id": "GBP",
                "ceiling_minor": 2500
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{dup}");
    assert_eq!(dup["duplicate"], true);
    assert_eq!(dup["mutation_id"], mid);

    let (status, approved) = post_json(
        &router,
        &format!("/treasury/mutations/{mid}/approve"),
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({ "decision": "approve", "reason": "ok" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    assert_eq!(approved["status"], "approved");

    let (status, executed) = post_json(
        &router,
        &format!("/treasury/mutations/{mid}/execute"),
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({ "confirm": true, "idempotency_key": "exec-alloc-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{executed}");
    assert_eq!(executed["outcome"], "success");
    assert!(executed["resource"]["allocation_id"].is_string());

    // execute idempotency
    let (status, again) = post_json(
        &router,
        &format!("/treasury/mutations/{mid}/execute"),
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({ "confirm": true, "idempotency_key": "exec-alloc-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["duplicate"], true);

    let (status, timeline) = get_json(&router, "/treasury/mutations/timeline", &admin_t).await;
    assert_eq!(status, StatusCode::OK);
    assert!(timeline["events"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn freeze_admin_unfreeze_dual_control() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let op_t = token(&state, "operator").await;
    let operator_id = op_id(&state, "operator").await;
    let admin_t = token(&state, "admin").await;
    let admin_id = op_id(&state, "admin").await;
    let tid = org_treasury_id(&state).await;

    let (status, frozen) = post_json(
        &router,
        &format!("/treasury/{tid}/freeze"),
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({ "confirm": true, "idempotency_key": "freeze-1", "reason": "test" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{frozen}");

    // operator cannot freeze
    let (status, denied) = post_json(
        &router,
        &format!("/treasury/{tid}/freeze"),
        &op_t,
        Some(&csrf(&state, &operator_id)),
        json!({ "confirm": true, "idempotency_key": "freeze-op", "reason": "x" }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{denied}");

    let (status, req) = post_json(
        &router,
        "/treasury/mutations",
        &op_t,
        Some(&csrf(&state, &operator_id)),
        json!({
            "operation": "unfreeze",
            "idempotency_key": "unfreeze-1",
            "payload": { "treasury_id": tid }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{req}");
    let mid = req["mutation_id"].as_str().unwrap();
    let (status, _) = post_json(
        &router,
        &format!("/treasury/mutations/{mid}/approve"),
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({ "decision": "approve" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, exec) = post_json(
        &router,
        &format!("/treasury/mutations/{mid}/execute"),
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({ "confirm": true, "idempotency_key": "exec-unfreeze-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{exec}");
    assert_eq!(exec["resource"]["status"], "active");
}

#[tokio::test]
async fn cross_org_treasury_not_found_on_write() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let op_t = token(&state, "operator").await;
    let operator_id = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/treasury/mutations",
        &op_t,
        Some(&csrf(&state, &operator_id)),
        json!({
            "operation": "allocation_create",
            "idempotency_key": "xorg",
            "payload": {
                "treasury_id": "does-not-exist",
                "agent_id": "a",
                "asset_id": "GBP",
                "ceiling_minor": 1
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
}

#[tokio::test]
async fn concurrent_approval_only_one_succeeds() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let op_t = token(&state, "operator").await;
    let operator_id = op_id(&state, "operator").await;
    let admin_t = token(&state, "admin").await;
    let admin_id = op_id(&state, "admin").await;
    let tid = org_treasury_id(&state).await;

    let (_, created) = post_json(
        &router,
        "/treasury/mutations",
        &op_t,
        Some(&csrf(&state, &operator_id)),
        json!({
            "operation": "adjustment",
            "idempotency_key": "adj-1",
            "payload": {
                "treasury_id": tid,
                "asset_id": "GBP",
                "amount_minor": 1
            }
        }),
    )
    .await;
    let mid = created["mutation_id"].as_str().unwrap().to_string();
    let path = format!("/treasury/mutations/{mid}/approve");
    let c1 = csrf(&state, &admin_id);
    let c2 = csrf(&state, &admin_id);

    let ((s1, b1), (s2, b2)) = tokio::join!(
        post_json(
            &router,
            &path,
            &admin_t,
            Some(&c1),
            json!({ "decision": "approve" }),
        ),
        post_json(
            &router,
            &path,
            &admin_t,
            Some(&c2),
            json!({ "decision": "approve" }),
        ),
    );
    let oks = [s1, s2]
        .iter()
        .filter(|s| **s == StatusCode::OK)
        .count();
    let conflicts = [s1, s2]
        .iter()
        .filter(|s| **s == StatusCode::CONFLICT)
        .count();
    assert_eq!(oks, 1, "b1={b1} b2={b2}");
    assert_eq!(conflicts, 1, "b1={b1} b2={b2}");
}
