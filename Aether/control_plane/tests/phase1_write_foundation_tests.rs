//! Milestone 2 Phase 1 — Secure Write Foundation tests.
//! No write routes; foundations only.

use std::sync::Arc;

use aether_control_plane::audit::mutation::{
    payload_hash, MutationAuditDraft, MutationAuditService, ProtocolResult,
};
use aether_control_plane::auth::middleware::AuthContext;
use aether_control_plane::auth::roles::{require_admin, require_operator, require_role};
use aether_control_plane::build_app;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::OperatorRole;
use aether_control_plane::db::Db;
use aether_control_plane::middleware::request_id::{request_id_from_extensions, REQUEST_ID_HEADER};
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::AppState;
use aether_control_plane::security::csrf::{
    csrf_cookie, validate_request_csrf, CsrfStore, CSRF_COOKIE_NAME,
};
use aether_control_plane::security::origin::{check_origin, parse_allowed_origins};
use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum_extra::extract::cookie::SameSite;
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
        allowed_origins: Vec::new(),
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

async fn setup_state() -> (Arc<AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("phase1.db");
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
        mutation_audit: MutationAuditService::new(db.pool().clone()),
        config,
        db,
        protocol,
        rate_limiter: aether_control_plane::auth::rate_limit::InMemoryLoginRateLimiter::shared(),
        csrf_store: CsrfStore::shared(),
        signing,
        treasury,
        treasury_write,
    });
    (state, dir)
}

fn auth_ctx(role: OperatorRole) -> AuthContext {
    AuthContext {
        operator_id: "op-1".into(),
        username: "tester".into(),
        role,
    }
}

// --- Authorization ---

#[test]
fn viewer_rejected_by_operator_middleware() {
    assert!(require_operator(&auth_ctx(OperatorRole::Viewer)).is_err());
    assert!(require_role(&auth_ctx(OperatorRole::Viewer), OperatorRole::Operator).is_err());
}

#[test]
fn operator_accepted_by_operator_middleware() {
    assert!(require_operator(&auth_ctx(OperatorRole::Operator)).is_ok());
}

#[test]
fn admin_accepted_where_appropriate() {
    assert!(require_operator(&auth_ctx(OperatorRole::Admin)).is_ok());
    assert!(require_admin(&auth_ctx(OperatorRole::Admin)).is_ok());
    assert!(require_admin(&auth_ctx(OperatorRole::Operator)).is_err());
}

// --- CSRF ---

#[test]
fn csrf_missing_token_rejected() {
    let store = CsrfStore::new();
    let headers = HeaderMap::new();
    let err = validate_request_csrf(&store, "op-1", &headers, false).unwrap_err();
    assert!(err.to_string().contains("missing"));
}

#[test]
fn csrf_invalid_token_rejected() {
    let store = CsrfStore::new();
    let mut headers = HeaderMap::new();
    headers.insert("x-csrf-token", "not-a-real-token".parse().unwrap());
    let err = validate_request_csrf(&store, "op-1", &headers, false).unwrap_err();
    assert!(err.to_string().contains("invalid"));
}

#[test]
fn csrf_valid_token_accepted() {
    let store = CsrfStore::new();
    let token = store.issue("op-1").expect("issue");
    let mut headers = HeaderMap::new();
    headers.insert("x-csrf-token", token.parse().unwrap());
    assert!(validate_request_csrf(&store, "op-1", &headers, false).is_ok());
}

#[test]
fn csrf_cookie_uses_samesite_strict() {
    let cookie = csrf_cookie("tok".into(), false);
    assert_eq!(cookie.name(), CSRF_COOKIE_NAME);
    assert_eq!(cookie.same_site(), Some(SameSite::Strict));
    assert!(!cookie.http_only().unwrap_or(true));
}

// --- Origin ---

#[test]
fn origin_allowed_accepted() {
    let allowed = parse_allowed_origins("https://control.example.com,https://cp.internal");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ORIGIN,
        "https://control.example.com".parse().unwrap(),
    );
    assert!(check_origin(&headers, &allowed).is_ok());
}

#[test]
fn origin_unknown_rejected() {
    let allowed = parse_allowed_origins("https://control.example.com");
    let mut headers = HeaderMap::new();
    headers.insert(header::ORIGIN, "https://evil.example".parse().unwrap());
    assert!(check_origin(&headers, &allowed).is_err());
}

#[test]
fn origin_localhost_ok_when_allowlist_empty() {
    let mut headers = HeaderMap::new();
    headers.insert(header::ORIGIN, "http://localhost:5173".parse().unwrap());
    assert!(check_origin(&headers, &[]).is_ok());
}

#[test]
fn origin_missing_allowed_for_non_browser() {
    let headers = HeaderMap::new();
    assert!(check_origin(&headers, &["https://control.example.com".into()]).is_ok());
}

// --- Request identity ---

#[tokio::test]
async fn request_id_generated_and_available_downstream() {
    let (state, _dir) = setup_state().await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/system/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    // Unauthenticated health under /api still goes through auth — expect 401,
    // but request-id middleware runs outermost and must still set the header.
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::OK
            || response.status() == StatusCode::FORBIDDEN
    );
    let rid = response
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .expect("X-Request-ID present");
    assert!(!rid.is_empty());
}

#[tokio::test]
async fn request_id_propagated_when_client_supplies() {
    let (state, _dir) = setup_state().await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/login")
                .method("POST")
                .header(&REQUEST_ID_HEADER, "a1b2c3d4")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"viewer","password":"viewer"}"#))
                .unwrap(),
        )
        .await
        .expect("response");

    let rid = response
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .expect("X-Request-ID");
    assert_eq!(rid, "a1b2c3d4");
}

#[test]
fn request_id_from_extensions_fallback() {
    let ext = axum::http::Extensions::new();
    let id = request_id_from_extensions(&ext);
    assert!(!id.is_empty());
}

// --- Mutation audit ---

#[tokio::test]
async fn mutation_audit_accepts_future_mutation_records() {
    let (state, _dir) = setup_state().await;
    let draft = MutationAuditDraft::new(
        "req-phase1",
        "op-1",
        OperatorRole::Operator,
        "synthetic.phase1_probe",
        br#"{"probe":true}"#,
    )
    .target("capability:none")
    .approved_now()
    .signer_identity("signer:not-wired")
    .protocol_result(ProtocolResult::NotApplicable)
    .failure_reason("phase1 foundation only — no protocol write");

    let record = state
        .mutation_audit
        .record(draft)
        .await
        .expect("record mutation audit");

    assert_eq!(record.request_id, "req-phase1");
    assert_eq!(record.operator_id, "op-1");
    assert_eq!(record.role, "operator");
    assert_eq!(record.action, "synthetic.phase1_probe");
    assert_eq!(record.target.as_deref(), Some("capability:none"));
    assert!(record.approved_at.is_some());
    assert_eq!(record.signer_identity.as_deref(), Some("signer:not-wired"));
    assert_eq!(record.protocol_result, ProtocolResult::NotApplicable);
    assert!(record.failure_reason.is_some());
    assert!(!record.payload_hash.is_empty());

    let recent = state.mutation_audit.list_recent(10).await.expect("list");
    assert!(recent.iter().any(|r| r.id == record.id));
}

#[test]
fn payload_hash_generation_works() {
    let a = payload_hash(br#"{"a":1}"#);
    let b = payload_hash(br#"{"a":1}"#);
    let c = payload_hash(br#"{"a":2}"#);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.len(), 64); // sha256 hex
}
