//! Milestone 2 Phase 4A — Dry-run execution pipeline tests.
//! No protocol mutations. No Apply.

use std::sync::Arc;

use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::audit;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::execution::{
    protocol_observation_fingerprint, simulate_capability_grant, simulate_capability_revoke,
};
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
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
    let db_path = dir.path().join("phase4a.db");
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

async fn approve_policy(
    state: &AppState,
    router: &axum::Router,
    name: &str,
    target_agent_id: Option<&str>,
    policy_data: Value,
    policy_type: &str,
) -> String {
    let (op_token, op_id) = token_for(state, "operator").await;
    let (admin_token, admin_id) = token_for(state, "admin").await;

    let mut body = json!({
        "name": name,
        "policy_type": policy_type,
        "policy_data": policy_data,
    });
    if let Some(t) = target_agent_id {
        body["target_agent_id"] = json!(t);
    }

    let created = mutate(
        router,
        "POST",
        "/policies",
        &op_token,
        Some(&csrf(state, &op_id)),
        Some("https://control.example.com"),
        &body.to_string(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let id = json_body(created).await["policy"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    assert_eq!(
        mutate(
            router,
            "POST",
            &format!("/policies/{id}/submit"),
            &op_token,
            Some(&csrf(state, &op_id)),
            Some("https://control.example.com"),
            "{}",
        )
        .await
        .status(),
        StatusCode::OK
    );

    assert_eq!(
        mutate(
            router,
            "POST",
            &format!("/policies/{id}/approve"),
            &admin_token,
            Some(&csrf(state, &admin_id)),
            Some("https://control.example.com"),
            "{}",
        )
        .await
        .status(),
        StatusCode::OK
    );
    id
}

#[tokio::test]
async fn dry_run_succeeds() {
    let (state, _dir) = setup_app().await;
    let agent_id = state.protocol.index.agent_ids[0].clone();
    let router = app(state.clone());
    let id = approve_policy(
        &state,
        &router,
        "dry-ok",
        Some(&agent_id),
        json!({"actions": ["transfer"], "max_spend": 100}),
        "capability_constraints",
    )
    .await;

    let before = protocol_observation_fingerprint(&state.protocol);
    let (op_token, op_id) = token_for(&state, "operator").await;
    let response = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/dry-run"),
        &op_token,
        Some(&csrf(&state, &op_id)),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["protocol_mutated"], false);
    assert_eq!(body["apply_enabled"], false);
    assert_eq!(body["report"]["executable"], true);
    assert_eq!(body["report"]["execution_mode"], "dry_run");
    assert!(
        body["report"]["execution_hash"]
            .as_str()
            .is_some_and(|h| h.len() == 64),
        "content-addressed execution_hash required"
    );
    assert!(body["report"]["dry_run_id"].as_str().is_some());
    assert_eq!(
        body["report"]["predicted_protocol_operation"],
        "CapabilityGrant"
    );
    assert!(body["report"]["signer_identity"].as_str().is_some());
    assert_eq!(
        protocol_observation_fingerprint(&state.protocol),
        before,
        "protocol must be unchanged"
    );

    let events = audit::list(state.db.pool(), 50).await.unwrap();
    assert!(events.iter().any(|e| e.action == "DRY_RUN_REQUESTED"));
    assert!(events.iter().any(|e| e.action == "DRY_RUN_COMPLETED"));

    let dry_run_id = body["report"]["dry_run_id"].as_str().unwrap();
    let attestation =
        aether_control_plane::apply::attestation::AttestationStore::new(state.db.pool().clone())
            .get(dry_run_id)
            .await
            .unwrap()
            .expect("executable dry-run must persist attestation");
    assert_eq!(
        attestation.status,
        aether_control_plane::apply::attestation::AttestationStatus::Executable
    );
    assert_eq!(
        attestation.execution_hash,
        body["report"]["execution_hash"].as_str().unwrap()
    );
}

#[tokio::test]
async fn dry_run_blocked_missing_target() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let id = approve_policy(
        &state,
        &router,
        "dry-block",
        None,
        json!({"actions": ["transfer"]}),
        "capability_constraints",
    )
    .await;

    let (op_token, op_id) = token_for(&state, "operator").await;
    let response = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/dry-run"),
        &op_token,
        Some(&csrf(&state, &op_id)),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["report"]["executable"], false);
    assert!(!body["report"]["blocking_errors"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn unauthorised_viewer_rejected() {
    let (state, _dir) = setup_app().await;
    let agent_id = state.protocol.index.agent_ids[0].clone();
    let router = app(state.clone());
    let id = approve_policy(
        &state,
        &router,
        "dry-viewer",
        Some(&agent_id),
        json!({"actions": ["transfer"]}),
        "capability_constraints",
    )
    .await;

    let (viewer_token, viewer_id) = token_for(&state, "viewer").await;
    let response = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/dry-run"),
        &viewer_token,
        Some(&csrf(&state, &viewer_id)),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_csrf_rejected() {
    let (state, _dir) = setup_app().await;
    let agent_id = state.protocol.index.agent_ids[0].clone();
    let router = app(state.clone());
    let id = approve_policy(
        &state,
        &router,
        "dry-csrf",
        Some(&agent_id),
        json!({"actions": ["pay"]}),
        "capability_constraints",
    )
    .await;

    let (op_token, _) = token_for(&state, "operator").await;
    let response = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/dry-run"),
        &op_token,
        None,
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn invalid_origin_rejected() {
    let (state, _dir) = setup_app().await;
    let agent_id = state.protocol.index.agent_ids[0].clone();
    let router = app(state.clone());
    let id = approve_policy(
        &state,
        &router,
        "dry-origin",
        Some(&agent_id),
        json!({"actions": ["pay"]}),
        "capability_constraints",
    )
    .await;

    let (op_token, op_id) = token_for(&state, "operator").await;
    let response = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/dry-run"),
        &op_token,
        Some(&csrf(&state, &op_id)),
        Some("https://evil.example"),
        "{}",
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn approved_policy_required() {
    let (state, _dir) = setup_app().await;
    let (op_token, op_id) = token_for(&state, "operator").await;
    let router = app(state.clone());

    let created = mutate(
        &router,
        "POST",
        "/policies",
        &op_token,
        Some(&csrf(&state, &op_id)),
        Some("https://control.example.com"),
        r#"{"name":"draft-only","policy_type":"capability_constraints","policy_data":{"actions":["x"]}}"#,
    )
    .await;
    let id = json_body(created).await["policy"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/dry-run"),
        &op_token,
        Some(&csrf(&state, &op_id)),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["report"]["executable"], false);
    let errors = body["report"]["blocking_errors"].as_array().unwrap();
    assert!(errors
        .iter()
        .any(|e| e.as_str().unwrap().contains("Approved")));
}

#[tokio::test]
async fn invalid_signer_blocks_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("fail-signer.db");
    let mut config = test_config(db_path.to_str().unwrap());
    config.signer.identity = "__fail_sign__".into();
    let db = Db::connect(&config.db_path).await.unwrap();
    db.migrate().await.unwrap();
    db.seed_default_operators(&config).await.unwrap();
    let protocol = ProtocolState::bootstrap().unwrap();
    let signing = aether_control_plane::signer::build_signing_gateway(&config, &db).unwrap();
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
    let agent_id = state.protocol.index.agent_ids[0].clone();
    let router = app(state.clone());
    let id = approve_policy(
        &state,
        &router,
        "dry-fail-sign",
        Some(&agent_id),
        json!({"actions": ["transfer"]}),
        "capability_constraints",
    )
    .await;

    let (op_token, op_id) = token_for(&state, "operator").await;
    let response = mutate(
        &router,
        "POST",
        &format!("/policies/{id}/dry-run"),
        &op_token,
        Some(&csrf(&state, &op_id)),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["report"]["executable"], false);
    assert!(body["report"]["blocking_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e.as_str().unwrap().to_lowercase().contains("sign")));
}

#[tokio::test]
async fn protocol_simulation_rejection() {
    let (state, _dir) = setup_app().await;
    let outcome = simulate_capability_grant(
        &state.protocol,
        Some("agent-does-not-exist"),
        &json!({"actions": ["transfer"]}),
    );
    assert!(!outcome.executable);
    assert!(outcome.reason.unwrap().contains("not found"));

    let fake_cap = "00".repeat(32);
    let outcome = simulate_capability_revoke(&state.protocol, Some(&fake_cap), None);
    assert!(!outcome.executable);
}

#[tokio::test]
async fn no_protocol_mutation_occurred() {
    let (state, _dir) = setup_app().await;
    let agent_id = state.protocol.index.agent_ids[0].clone();
    let caps_before = state.protocol.index.capability_ids.len();
    let agents_before = state.protocol.index.agent_ids.clone();
    let router = app(state.clone());
    let id = approve_policy(
        &state,
        &router,
        "dry-immutable",
        Some(&agent_id),
        json!({"actions": ["transfer"]}),
        "capability_constraints",
    )
    .await;

    let (op_token, op_id) = token_for(&state, "operator").await;
    for _ in 0..3 {
        let _ = mutate(
            &router,
            "POST",
            &format!("/policies/{id}/dry-run"),
            &op_token,
            Some(&csrf(&state, &op_id)),
            Some("https://control.example.com"),
            "{}",
        )
        .await;
    }

    assert_eq!(state.protocol.index.capability_ids.len(), caps_before);
    assert_eq!(state.protocol.index.agent_ids, agents_before);
}
