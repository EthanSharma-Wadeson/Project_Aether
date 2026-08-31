//! Milestone 2 Phase 2 — Policy Template Workflow tests.
//! Templates are Control Plane records only (no PROTO-0).

use std::sync::Arc;

use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

fn test_config(db_path: &str) -> Config {
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
        allowed_origins: vec!["https://control.example.com".into()],
        signer: aether_control_plane::config::SignerConfig::default(),
        secure_cookies: false,
        treasury_db_path: format!("{}.treasury.db", db_path.trim_end_matches(".db")),
        treasury_organisation_id: "org-default".into(),
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

async fn setup_app() -> (Arc<AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("phase2.db");
    let config = test_config(db_path.to_str().unwrap());
    let db = Db::connect(&config.db_path).await.expect("db");
    db.migrate().await.expect("migrate");
    db.seed_default_operators(&config).await.expect("seed");
    let protocol = ProtocolState::bootstrap().expect("bootstrap");
    let signing =
        aether_control_plane::signer::build_signing_gateway(&config, &db).expect("signer");
    let treasury = aether_control_plane::treasury::TreasuryAdapter::connect(&config)
        .await
        .expect("treasury");
    let treasury_write = aether_control_plane::treasury::TreasuryWriteService::new(db.pool().clone());
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

async fn token_for(state: &AppState, username: &str) -> (String, String) {
    let op = find_by_username(state.db.pool(), username)
        .await
        .unwrap()
        .unwrap();
    let token =
        issue_access_token(&state.config.jwt_secret, &op.id, &op.username, op.role, 900).unwrap();
    (token, op.id)
}

fn csrf(state: &AppState, operator_id: &str) -> String {
    state.csrf_store.issue(operator_id).unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn mutate(
    router: &axum::Router,
    method: &str,
    uri: &str,
    token: &str,
    csrf_token: Option<&str>,
    origin: Option<&str>,
    body: &str,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json");
    if let Some(csrf) = csrf_token {
        builder = builder.header("x-csrf-token", csrf);
    }
    if let Some(origin) = origin {
        builder = builder.header(header::ORIGIN, origin);
    }
    router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn unauthenticated_policy_access_rejected() {
    let (state, _dir) = setup_app().await;
    let router = app(state);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/policies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn viewer_cannot_create_policy() {
    let (state, _dir) = setup_app().await;
    let (token, op_id) = token_for(&state, "viewer").await;
    let csrf_token = csrf(&state, &op_id);
    let router = app(state);
    let response = mutate(
        &router,
        "POST",
        "/policies",
        &token,
        Some(&csrf_token),
        Some("https://control.example.com"),
        r#"{"name":"v","policy_type":"t","policy_data":{}}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn operator_can_create_draft_but_not_approve() {
    let (state, _dir) = setup_app().await;
    let (token, op_id) = token_for(&state, "operator").await;
    let csrf_token = csrf(&state, &op_id);
    let router = app(state.clone());

    let create = mutate(
        &router,
        "POST",
        "/policies",
        &token,
        Some(&csrf_token),
        Some("https://control.example.com"),
        r#"{"name":"spend-limit","policy_type":"capability_constraints","policy_data":{"max_spend":100}}"#,
    )
    .await;
    assert_eq!(create.status(), StatusCode::OK);
    let created = json_body(create).await;
    let id = created["policy"]["id"].as_str().unwrap().to_string();
    assert_eq!(created["policy"]["status"], "draft");
    assert_eq!(created["policy"]["version"], 1);

    let csrf2 = csrf(&state, &op_id);
    let submit = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/submit"),
        &token,
        Some(&csrf2),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(submit.status(), StatusCode::OK);

    let csrf3 = csrf(&state, &op_id);
    let approve = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/approve"),
        &token,
        Some(&csrf3),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(approve.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_approve_other_operators_policy() {
    let (state, _dir) = setup_app().await;
    let (op_token, op_id) = token_for(&state, "operator").await;
    let (admin_token, admin_id) = token_for(&state, "admin").await;
    let router = app(state.clone());

    let csrf_op = csrf(&state, &op_id);
    let create = mutate(
        &router,
        "POST",
        "/policies",
        &op_token,
        Some(&csrf_op),
        Some("https://control.example.com"),
        r#"{"name":"approve-me","policy_type":"capability_constraints","policy_data":{"actions":["pay"]}}"#,
    )
    .await;
    assert_eq!(create.status(), StatusCode::OK);
    let id = json_body(create).await["policy"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let csrf_op2 = csrf(&state, &op_id);
    let submit = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/submit"),
        &op_token,
        Some(&csrf_op2),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(submit.status(), StatusCode::OK);

    let csrf_admin = csrf(&state, &admin_id);
    let approve = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/approve"),
        &admin_token,
        Some(&csrf_admin),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(approve.status(), StatusCode::OK);
    let body = json_body(approve).await;
    assert_eq!(body["policy"]["status"], "approved");
    assert_eq!(body["protocol_active"], false);

    let audits = state.mutation_audit.list_recent(20).await.unwrap();
    assert!(audits.iter().any(|a| a.action == "POLICY_APPROVED"));
}

#[tokio::test]
async fn rejection_and_archive_workflow() {
    let (state, _dir) = setup_app().await;
    let (op_token, op_id) = token_for(&state, "operator").await;
    let (admin_token, admin_id) = token_for(&state, "admin").await;
    let router = app(state.clone());

    let csrf_op = csrf(&state, &op_id);
    let create = mutate(
        &router,
        "POST",
        "/policies",
        &op_token,
        Some(&csrf_op),
        Some("https://control.example.com"),
        r#"{"name":"reject-me","policy_type":"t","policy_data":{}}"#,
    )
    .await;
    let id = json_body(create).await["policy"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let csrf_op2 = csrf(&state, &op_id);
    assert_eq!(
        mutate(
            &router,
            "POST",
            &format!("/policies/{id}/submit"),
            &op_token,
            Some(&csrf_op2),
            Some("https://control.example.com"),
            "{}",
        )
        .await
        .status(),
        StatusCode::OK
    );

    let csrf_admin = csrf(&state, &admin_id);
    let reject = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/reject"),
        &admin_token,
        Some(&csrf_admin),
        Some("https://control.example.com"),
        r#"{"reason":"too broad"}"#,
    )
    .await;
    assert_eq!(reject.status(), StatusCode::OK);
    assert_eq!(json_body(reject).await["policy"]["status"], "rejected");

    let csrf_admin2 = csrf(&state, &admin_id);
    let archive = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/archive"),
        &admin_token,
        Some(&csrf_admin2),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(archive.status(), StatusCode::OK);
    assert_eq!(json_body(archive).await["policy"]["status"], "archived");

    // Archived cannot be edited
    let csrf_op3 = csrf(&state, &op_id);
    let edit = mutate(
        &router,
        "PUT",
        &format!("/policies/{id}"),
        &op_token,
        Some(&csrf_op3),
        Some("https://control.example.com"),
        r#"{"name":"nope"}"#,
    )
    .await;
    assert_eq!(edit.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn csrf_required_on_policy_create() {
    let (state, _dir) = setup_app().await;
    let (token, _op_id) = token_for(&state, "operator").await;
    let router = app(state);
    let response = mutate(
        &router,
        "POST",
        "/policies",
        &token,
        None,
        Some("https://control.example.com"),
        r#"{"name":"x","policy_type":"t","policy_data":{}}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unknown_origin_rejected_on_policy_create() {
    let (state, _dir) = setup_app().await;
    let (token, op_id) = token_for(&state, "operator").await;
    let csrf_token = csrf(&state, &op_id);
    let router = app(state);
    let response = mutate(
        &router,
        "POST",
        "/policies",
        &token,
        Some(&csrf_token),
        Some("https://evil.example"),
        r#"{"name":"x","policy_type":"t","policy_data":{}}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn draft_update_bumps_version_and_audits() {
    let (state, _dir) = setup_app().await;
    let (token, op_id) = token_for(&state, "operator").await;
    let router = app(state.clone());

    let csrf1 = csrf(&state, &op_id);
    let create = mutate(
        &router,
        "POST",
        "/policies",
        &token,
        Some(&csrf1),
        Some("https://control.example.com"),
        r#"{"name":"v1","policy_type":"t","policy_data":{"a":1}}"#,
    )
    .await;
    let id = json_body(create).await["policy"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let csrf2 = csrf(&state, &op_id);
    let update = mutate(
        &router,
        "PUT",
        &format!("/policies/{id}"),
        &token,
        Some(&csrf2),
        Some("https://control.example.com"),
        r#"{"name":"v2","policy_data":{"a":2}}"#,
    )
    .await;
    assert_eq!(update.status(), StatusCode::OK);
    let body = json_body(update).await;
    assert_eq!(body["policy"]["version"], 2);
    assert_eq!(body["policy"]["name"], "v2");

    let audits = state.mutation_audit.list_recent(20).await.unwrap();
    assert!(audits.iter().any(|a| a.action == "POLICY_CREATED"));
    assert!(audits.iter().any(|a| a.action == "POLICY_UPDATED"));
    assert!(audits.iter().all(|a| {
        matches!(
            a.protocol_result,
            aether_control_plane::audit::mutation::ProtocolResult::NotApplicable
        )
    }));
}

#[tokio::test]
async fn admin_cannot_approve_own_policy() {
    let (state, _dir) = setup_app().await;
    let (admin_token, admin_id) = token_for(&state, "admin").await;
    let router = app(state.clone());

    let csrf1 = csrf(&state, &admin_id);
    let create = mutate(
        &router,
        "POST",
        "/policies",
        &admin_token,
        Some(&csrf1),
        Some("https://control.example.com"),
        r#"{"name":"self","policy_type":"t","policy_data":{}}"#,
    )
    .await;
    let id = json_body(create).await["policy"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let csrf2 = csrf(&state, &admin_id);
    assert_eq!(
        mutate(
            &router,
            "POST",
            &format!("/policies/{id}/submit"),
            &admin_token,
            Some(&csrf2),
            Some("https://control.example.com"),
            "{}",
        )
        .await
        .status(),
        StatusCode::OK
    );

    let csrf3 = csrf(&state, &admin_id);
    let approve = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/approve"),
        &admin_token,
        Some(&csrf3),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(approve.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn viewer_lists_only_approved_policies() {
    let (state, _dir) = setup_app().await;
    let (op_token, op_id) = token_for(&state, "operator").await;
    let (admin_token, admin_id) = token_for(&state, "admin").await;
    let (viewer_token, _) = token_for(&state, "viewer").await;
    let router = app(state.clone());

    let csrf_op = csrf(&state, &op_id);
    let create = mutate(
        &router,
        "POST",
        "/policies",
        &op_token,
        Some(&csrf_op),
        Some("https://control.example.com"),
        r#"{"name":"visible","policy_type":"t","policy_data":{}}"#,
    )
    .await;
    let id = json_body(create).await["policy"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let csrf_op2 = csrf(&state, &op_id);
    mutate(
        &router,
        "POST",
        &format!("/policies/{id}/submit"),
        &op_token,
        Some(&csrf_op2),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    let csrf_admin = csrf(&state, &admin_id);
    mutate(
        &router,
        "POST",
        &format!("/policies/{id}/approve"),
        &admin_token,
        Some(&csrf_admin),
        Some("https://control.example.com"),
        "{}",
    )
    .await;

    // Draft that viewer must not see
    let csrf_op3 = csrf(&state, &op_id);
    mutate(
        &router,
        "POST",
        "/policies",
        &op_token,
        Some(&csrf_op3),
        Some("https://control.example.com"),
        r#"{"name":"hidden-draft","policy_type":"t","policy_data":{}}"#,
    )
    .await;

    let list = router
        .oneshot(
            Request::builder()
                .uri("/policies")
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let body = json_body(list).await;
    let policies = body["policies"].as_array().unwrap();
    assert!(policies.iter().all(|p| p["status"] == "approved"));
    assert!(policies.iter().any(|p| p["name"] == "visible"));
    assert!(!policies.iter().any(|p| p["name"] == "hidden-draft"));
}
