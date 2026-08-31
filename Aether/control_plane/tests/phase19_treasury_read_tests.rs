//! Phase 19 — Treasury read-only observation security & integrity tests.

use std::sync::Arc;

use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
use aether_treasury::{Amount, AssetId, TreasuryEngine, TreasuryKind};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
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
    let db_path = dir.path().join("phase19.db");
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

async fn token(state: &AppState, username: &str) -> String {
    let op = find_by_username(state.db.pool(), username)
        .await
        .unwrap()
        .unwrap();
    issue_access_token(&state.config.jwt_secret, &op.id, &op.username, op.role, 900).unwrap()
}

async fn get(router: &axum::Router, path: &str, bearer: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("GET").uri(path);
    if let Some(t) = bearer {
        builder = builder.header("Authorization", format!("Bearer {t}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

#[tokio::test]
async fn viewer_operator_admin_can_list_treasury() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    for user in ["viewer", "operator", "admin"] {
        let t = token(&state, user).await;
        let (status, body) = get(&router, "/treasury", Some(&t)).await;
        assert_eq!(status, StatusCode::OK, "{user}");
        assert_eq!(body["observation_only"], true);
        assert!(!body["treasuries"].as_array().unwrap().is_empty());
    }
}

#[tokio::test]
async fn auditor_viewer_can_read_journal() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "viewer").await;
    let (_, overview) = get(&router, "/treasury", Some(&t)).await;
    let id = overview["treasuries"][0]["treasury_id"].as_str().unwrap();
    let (status, body) = get(&router, &format!("/treasury/{id}/journal"), Some(&t)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["immutable"], true);
    assert_eq!(body["read_only"], true);
}

#[tokio::test]
async fn admin_security_ok_operator_forbidden() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let admin = token(&state, "admin").await;
    let operator = token(&state, "operator").await;
    let (_, overview) = get(&router, "/treasury", Some(&admin)).await;
    let id = overview["treasuries"][0]["treasury_id"].as_str().unwrap();

    let (status, _) = get(&router, &format!("/treasury/{id}/security"), Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = get(
        &router,
        &format!("/treasury/{id}/security"),
        Some(&operator),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_and_invalid_jwt_rejected() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let (status, _) = get(&router, "/treasury", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = get(&router, "/treasury", Some("not-a-jwt")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cross_tenant_treasury_not_found() {
    let (state, dir) = setup("org-default").await;
    // Plant a foreign-org treasury in the same DB file via engine
    let foreign = state.treasury.engine();
    let _ = foreign
        .register_asset(AssetId::new("EUR"), 2, "fiat")
        .await;
    let foreign_t = foreign
        .create_treasury("org-other", None, TreasuryKind::Organisation, "Other Org")
        .await
        .unwrap();
    foreign
        .fund(
            &foreign_t.treasury_id,
            AssetId::new("EUR"),
            Amount(50),
            "x",
            "x-idem",
        )
        .await
        .unwrap();

    let router = app(state.clone());
    let t = token(&state, "admin").await;
    let (status, body) = get(
        &router,
        &format!("/treasury/{}", foreign_t.treasury_id),
        Some(&t),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let _ = dir;
}

#[tokio::test]
async fn settlement_posts_labelled_as_treasury_accounting() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "admin").await;
    let (_, overview) = get(&router, "/treasury", Some(&t)).await;
    // find dept with settlement
    let mut found = false;
    for tr in overview["treasuries"].as_array().unwrap() {
        let id = tr["treasury_id"].as_str().unwrap();
        let (status, body) = get(&router, &format!("/treasury/{id}/settlements"), Some(&t)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["label"], "treasury_settlement_post");
        if let Some(posts) = body["posts"].as_array() {
            if !posts.is_empty() {
                assert_eq!(posts[0]["accounting_truth"], "treasury_journal");
                found = true;
            }
        }
    }
    assert!(found, "expected bootstrap settlement post");
}

#[tokio::test]
async fn journal_integrity_matches_projections() {
    let (state, _dir) = setup("org-default").await;
    let org = state.treasury.organisation_id();
    let treasuries = state.treasury.engine().list_treasuries(org).await.unwrap();
    let t = &treasuries[0];
    let ok = state
        .treasury
        .engine()
        .verify_balance_matches_journal(org, &t.treasury_id, "GBP")
        .await
        .unwrap();
    assert!(ok);
}

#[tokio::test]
async fn no_treasury_mutation_http_methods() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "admin").await;
    for method in ["POST", "PUT", "DELETE", "PATCH"] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/treasury")
                    .header("Authorization", format!("Bearer {t}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            response.status() == StatusCode::METHOD_NOT_ALLOWED
                || response.status() == StatusCode::NOT_FOUND,
            "{method} => {}",
            response.status()
        );
    }
}

#[tokio::test]
async fn reservations_expose_write_note() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let (_, overview) = get(&router, "/treasury", Some(&t)).await;
    for tr in overview["treasuries"].as_array().unwrap() {
        let id = tr["treasury_id"].as_str().unwrap();
        let (status, body) = get(&router, &format!("/treasury/{id}/reservations"), Some(&t)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["read_only"], false);
    }
}

// Keep engine type referenced for compile-time coupling check in tests.
#[allow(dead_code)]
fn _engine_type(_: &TreasuryEngine) {}
