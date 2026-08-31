//! Phase 12 — Apply HTTP API integration tests.
//! Apply remains disabled; no protocol mutations.

use std::sync::Arc;

use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::execution::protocol_observation_fingerprint;
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
    let db_path = dir.path().join("phase12.db");
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

async fn get_authed(router: &axum::Router, uri: &str, token: &str) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

struct PreparedApply {
    operation_id: String,
    approval_id: String,
    dry_run_id: String,
    execution_hash: String,
    fingerprint: String,
}

async fn prepare_full_chain(
    state: &AppState,
    router: &axum::Router,
    suffix: &str,
) -> PreparedApply {
    let fingerprint = protocol_observation_fingerprint(&state.protocol);
    let agent_id = state.protocol.index.agent_ids[1].clone();
    let (op_token, op_id) = token_for(state, "operator").await;
    let (admin_token, admin_id) = token_for(state, "admin").await;

    let created = mutate(
        router,
        "POST",
        "/policies",
        &op_token,
        Some(&csrf(state, &op_id)),
        Some("https://control.example.com"),
        &json!({
            "name": format!("apply-http-{suffix}"),
            "policy_type": "identity_freeze",
            "target_agent_id": agent_id,
            "policy_data": {},
        })
        .to_string(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let policy_id = json_body(created).await["policy"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    assert_eq!(
        mutate(
            router,
            "POST",
            &format!("/policies/{policy_id}/submit"),
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
            &format!("/policies/{policy_id}/approve"),
            &admin_token,
            Some(&csrf(state, &admin_id)),
            Some("https://control.example.com"),
            "{}",
        )
        .await
        .status(),
        StatusCode::OK
    );

    let dry = mutate(
        router,
        "POST",
        &format!("/policies/{policy_id}/dry-run"),
        &op_token,
        Some(&csrf(state, &op_id)),
        Some("https://control.example.com"),
        "{}",
    )
    .await;
    assert_eq!(dry.status(), StatusCode::OK);
    let dry_body = json_body(dry).await;
    assert_eq!(dry_body["report"]["executable"], true);
    let dry_run_id = dry_body["report"]["dry_run_id"]
        .as_str()
        .unwrap()
        .to_string();
    let execution_hash = dry_body["report"]["execution_hash"]
        .as_str()
        .unwrap()
        .to_string();
    let policy_version = dry_body["report"]["policy_version"].as_i64().unwrap();
    let operation_intent = dry_body["report"]["predicted_protocol_operation"]
        .as_str()
        .unwrap()
        .to_string();

    let appr_resp = mutate(
        router,
        "POST",
        "/apply-approvals",
        &admin_token,
        Some(&csrf(state, &admin_id)),
        Some("https://control.example.com"),
        &json!({
            "dry_run_id": dry_run_id,
            "execution_hash": execution_hash,
            "policy_id": policy_id,
            "policy_version": policy_version,
            "operation_intent": operation_intent,
        })
        .to_string(),
    )
    .await;
    assert_eq!(appr_resp.status(), StatusCode::OK);
    let appr_body = json_body(appr_resp).await;
    let approval_id = appr_body["approval"]["approval_id"]
        .as_str()
        .unwrap()
        .to_string();

    let prep = mutate(
        router,
        "POST",
        "/apply/prepare",
        &op_token,
        Some(&csrf(state, &op_id)),
        Some("https://control.example.com"),
        &json!({
            "approval_id": approval_id,
            "dry_run_id": dry_run_id,
            "execution_hash": execution_hash,
            "policy_id": policy_id,
            "policy_version": policy_version,
            "operation_intent": operation_intent,
            "target_agent": agent_id,
        })
        .to_string(),
    )
    .await;
    let prep_status = prep.status();
    let prep_body = json_body(prep).await;
    assert_eq!(prep_status, StatusCode::OK, "{prep_body:?}");
    let operation_id = prep_body["operation_id"].as_str().unwrap().to_string();
    assert!(!prep_body["payload_hash"].as_str().unwrap().is_empty());
    assert_eq!(prep_body["apply_enabled"], false);
    assert_eq!(prep_body["protocol_mutated"], false);

    PreparedApply {
        operation_id,
        approval_id,
        dry_run_id,
        execution_hash,
        fingerprint,
    }
}

#[tokio::test]
async fn prepare_endpoint_succeeds() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "prep").await;
    assert!(!prepared.operation_id.is_empty());
}

#[tokio::test]
async fn execute_returns_disabled_without_mutation() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "exec").await;
    let (admin_token, admin_id) = token_for(&state, "admin").await;

    let resp = mutate(
        &router,
        "POST",
        "/apply",
        &admin_token,
        Some(&csrf(&state, &admin_id)),
        Some("https://control.example.com"),
        &json!({
            "operation_id": prepared.operation_id,
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "confirm": true,
        })
        .to_string(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["apply_enabled"], false);
    assert_eq!(body["protocol_mutated"], false);
    assert_eq!(body["outcome_code"], "APPLY_EXECUTION_DISABLED");
    assert_eq!(
        protocol_observation_fingerprint(&state.protocol),
        prepared.fingerprint
    );
}

#[tokio::test]
async fn execute_missing_confirm_rejected() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "noconfirm").await;
    let (admin_token, admin_id) = token_for(&state, "admin").await;

    let resp = mutate(
        &router,
        "POST",
        "/apply",
        &admin_token,
        Some(&csrf(&state, &admin_id)),
        Some("https://control.example.com"),
        &json!({
            "operation_id": prepared.operation_id,
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "confirm": false,
        })
        .to_string(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert!(body["error"].as_str().unwrap().contains("CONFIRM_REQUIRED"));
}

#[tokio::test]
async fn execute_missing_csrf_rejected() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "nocsrf").await;
    let (admin_token, _) = token_for(&state, "admin").await;

    let resp = mutate(
        &router,
        "POST",
        "/apply",
        &admin_token,
        None,
        Some("https://control.example.com"),
        &json!({
            "operation_id": prepared.operation_id,
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "confirm": true,
        })
        .to_string(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn viewer_forbidden_on_execute_and_prepare() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "viewer").await;
    let (viewer_token, viewer_id) = token_for(&state, "viewer").await;

    let prep = mutate(
        &router,
        "POST",
        "/apply/prepare",
        &viewer_token,
        Some(&csrf(&state, &viewer_id)),
        Some("https://control.example.com"),
        &json!({
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "policy_id": "x",
            "policy_version": 1,
            "operation_intent": "FreezeIdentity",
        })
        .to_string(),
    )
    .await;
    assert_eq!(prep.status(), StatusCode::FORBIDDEN);

    let exec = mutate(
        &router,
        "POST",
        "/apply",
        &viewer_token,
        Some(&csrf(&state, &viewer_id)),
        Some("https://control.example.com"),
        &json!({
            "operation_id": prepared.operation_id,
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "confirm": true,
        })
        .to_string(),
    )
    .await;
    assert_eq!(exec.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn invalid_jwt_rejected() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let resp = mutate(
        &router,
        "POST",
        "/apply",
        "not-a-valid-jwt",
        Some("csrf"),
        Some("https://control.example.com"),
        &json!({
            "operation_id": "op",
            "approval_id": "ap",
            "dry_run_id": "dry",
            "execution_hash": "hash",
            "confirm": true,
        })
        .to_string(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn operation_status_endpoint() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "status").await;
    let (op_token, _) = token_for(&state, "operator").await;

    let resp = get_authed(
        &router,
        &format!("/apply/operations/{}", prepared.operation_id),
        &op_token,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["operation_id"], prepared.operation_id);
    assert_eq!(body["apply_enabled"], false);
    assert!(body["signed_operation"].is_object());
}

#[tokio::test]
async fn duplicate_execute_rejected() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "dup").await;
    let (admin_token, admin_id) = token_for(&state, "admin").await;
    let body = json!({
        "operation_id": prepared.operation_id,
        "approval_id": prepared.approval_id,
        "dry_run_id": prepared.dry_run_id,
        "execution_hash": prepared.execution_hash,
        "confirm": true,
    })
    .to_string();

    let first = mutate(
        &router,
        "POST",
        "/apply",
        &admin_token,
        Some(&csrf(&state, &admin_id)),
        Some("https://control.example.com"),
        &body,
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = mutate(
        &router,
        "POST",
        "/apply",
        &admin_token,
        Some(&csrf(&state, &admin_id)),
        Some("https://control.example.com"),
        &body,
    )
    .await;
    assert_eq!(second.status(), StatusCode::BAD_REQUEST);
    let err = json_body(second).await;
    let msg = err["error"].as_str().unwrap_or("");
    assert!(
        msg.contains("REPLAY_")
            || msg.contains("APPROVAL")
            || msg.contains("VALIDATION")
            || msg.contains("CONSUME"),
        "unexpected duplicate error: {msg}"
    );
}

#[tokio::test]
async fn admin_reconcile_list_and_abort() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "recon").await;
    let (admin_token, admin_id) = token_for(&state, "admin").await;

    // Reserve manually then mark stuck for abort path.
    let replay = aether_control_plane::apply::replay::ReplayStore::new(state.db.pool().clone());
    replay
        .reserve(aether_control_plane::apply::replay::ReserveRequest {
            operation_id: prepared.operation_id.clone(),
            execution_hash: prepared.execution_hash.clone(),
            dry_run_id: prepared.dry_run_id.clone(),
        })
        .await
        .unwrap();
    replay
        .mark_stuck(&prepared.operation_id, Some("test".into()))
        .await
        .unwrap();

    let list = get_authed(&router, "/apply/reconcile", &admin_token).await;
    assert_eq!(list.status(), StatusCode::OK);
    let list_body = json_body(list).await;
    assert_eq!(list_body["apply_enabled"], false);
    assert!(!list_body["stuck"].as_array().unwrap().is_empty());

    let one = get_authed(
        &router,
        &format!("/apply/reconcile/{}", prepared.operation_id),
        &admin_token,
    )
    .await;
    assert_eq!(one.status(), StatusCode::OK);

    let abort = mutate(
        &router,
        "POST",
        &format!("/apply/reconcile/{}/abort", prepared.operation_id),
        &admin_token,
        Some(&csrf(&state, &admin_id)),
        Some("https://control.example.com"),
        &json!({"reason": "test-abort"}).to_string(),
    )
    .await;
    assert_eq!(abort.status(), StatusCode::OK);
    let abort_body = json_body(abort).await;
    assert_eq!(abort_body["operation"]["status"], "aborted");
    assert_eq!(abort_body["protocol_mutated"], false);
}

#[tokio::test]
async fn viewer_forbidden_on_reconcile() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let (viewer_token, _) = token_for(&state, "viewer").await;
    let resp = get_authed(&router, "/apply/reconcile", &viewer_token).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn expired_approval_blocks_execute() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "expappr").await;

    sqlx::query("UPDATE apply_approvals SET expires_at = ? WHERE approval_id = ?")
        .bind((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
        .bind(&prepared.approval_id)
        .execute(state.db.pool())
        .await
        .unwrap();

    let (admin_token, admin_id) = token_for(&state, "admin").await;
    let resp = mutate(
        &router,
        "POST",
        "/apply",
        &admin_token,
        Some(&csrf(&state, &admin_id)),
        Some("https://control.example.com"),
        &json!({
            "operation_id": prepared.operation_id,
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "confirm": true,
        })
        .to_string(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        protocol_observation_fingerprint(&state.protocol),
        prepared.fingerprint
    );
}

#[tokio::test]
async fn expired_signature_blocks_execute() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "expsig").await;

    sqlx::query("UPDATE signed_operations SET expires_at = ? WHERE operation_id = ?")
        .bind((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
        .bind(&prepared.operation_id)
        .execute(state.db.pool())
        .await
        .unwrap();

    let (admin_token, admin_id) = token_for(&state, "admin").await;
    let resp = mutate(
        &router,
        "POST",
        "/apply",
        &admin_token,
        Some(&csrf(&state, &admin_id)),
        Some("https://control.example.com"),
        &json!({
            "operation_id": prepared.operation_id,
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "confirm": true,
        })
        .to_string(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stale_attestation_blocks_execute() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "staleatt").await;

    sqlx::query("UPDATE dry_run_attestations SET expires_at = ? WHERE dry_run_id = ?")
        .bind((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
        .bind(&prepared.dry_run_id)
        .execute(state.db.pool())
        .await
        .unwrap();

    let (admin_token, admin_id) = token_for(&state, "admin").await;
    let resp = mutate(
        &router,
        "POST",
        "/apply",
        &admin_token,
        Some(&csrf(&state, &admin_id)),
        Some("https://control.example.com"),
        &json!({
            "operation_id": prepared.operation_id,
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "confirm": true,
        })
        .to_string(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        protocol_observation_fingerprint(&state.protocol),
        prepared.fingerprint
    );
}

#[tokio::test]
async fn operator_cannot_execute() {
    let (state, _dir) = setup_app().await;
    let router = app(state.clone());
    let prepared = prepare_full_chain(&state, &router, "opforb").await;
    let (op_token, op_id) = token_for(&state, "operator").await;

    let resp = mutate(
        &router,
        "POST",
        "/apply",
        &op_token,
        Some(&csrf(&state, &op_id)),
        Some("https://control.example.com"),
        &json!({
            "operation_id": prepared.operation_id,
            "approval_id": prepared.approval_id,
            "dry_run_id": prepared.dry_run_id,
            "execution_hash": prepared.execution_hash,
            "confirm": true,
        })
        .to_string(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
