//! Phase 24 — Apply ↔ Treasury sync observation (read-only drift reporting).

use std::sync::Arc;

use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
use aether_treasury::{Amount, AssetId, TreasuryKind};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
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
    let db_path = dir.path().join("phase24.db");
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

async fn ensure_aether_test_funded(state: &AppState) -> String {
    let eng = state.treasury.engine();
    let _ = eng
        .register_asset(AssetId::new("AETHER_TEST"), 0, "test")
        .await;
    let org = state.treasury.organisation_id().to_string();
    let list = eng.list_treasuries(&org).await.unwrap();
    let tid = list[0].treasury_id.clone();
    let _ = eng
        .fund(
            &tid,
            AssetId::new("AETHER_TEST"),
            Amount(1_000_000),
            "p24-fund",
            &format!("p24-fund-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0)),
        )
        .await;
    tid
}

fn agent_subject(state: &AppState) -> String {
    // Autonomous agent (delegated spend grant in bootstrap).
    state.protocol.index.agent_ids[1].clone()
}

fn findings_kinds(body: &Value) -> Vec<String> {
    body["findings"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|f| f["kind"].as_str().unwrap_or("").to_string())
        .collect()
}

#[tokio::test]
async fn missing_allocation_detected() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "viewer").await;
    let agent = agent_subject(&state);
    let (status, body) = get(
        &router,
        &format!("/sync/treasury-capabilities/{agent}"),
        Some(&t),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["observation_only"], true);
    assert_eq!(body["enforcement"], false);
    assert_eq!(body["auto_repair"], false);
    assert_eq!(body["status_code"], "DRIFT_DETECTED");
    let kinds = findings_kinds(&body);
    assert!(
        kinds.iter().any(|k| k == "capability_without_allocation"),
        "{kinds:?}"
    );
}

#[tokio::test]
async fn excess_capability_requires_review() {
    let (state, _dir) = setup("org-default").await;
    let tid = ensure_aether_test_funded(&state).await;
    let agent = agent_subject(&state);
    // Cap max_spend is 1000 for agent; allocate only 100.
    state
        .treasury
        .engine()
        .create_allocation(
            &tid,
            &agent,
            AssetId::new("AETHER_TEST"),
            Amount(100),
            Amount(0),
            None,
            "p24-excess",
            "p24-excess-ik",
        )
        .await
        .unwrap();

    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let (status, body) = get(
        &router,
        &format!("/sync/treasury-capabilities/{agent}"),
        Some(&t),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status_code"], "REQUIRES_REVIEW");
    let kinds = findings_kinds(&body);
    assert!(
        kinds
            .iter()
            .any(|k| k == "capability_limit_exceeds_allocation"),
        "{kinds:?}"
    );
}

#[tokio::test]
async fn expired_allocation_with_active_capability() {
    let (state, _dir) = setup("org-default").await;
    let tid = ensure_aether_test_funded(&state).await;
    let agent = agent_subject(&state);
    let past = Utc::now() - Duration::hours(1);
    let (alloc, _) = state
        .treasury
        .engine()
        .create_allocation(
            &tid,
            &agent,
            AssetId::new("AETHER_TEST"),
            Amount(5_000),
            Amount(0),
            Some(past),
            "p24-exp",
            "p24-exp-ik",
        )
        .await
        .unwrap();
    // Ensure status reflects expiry.
    let _ = state
        .treasury
        .engine()
        .expire_allocation_if_needed(&alloc.allocation_id)
        .await
        .unwrap();

    let router = app(state.clone());
    let t = token(&state, "admin").await;
    let (status, body) = get(
        &router,
        &format!("/sync/treasury-capabilities/{agent}"),
        Some(&t),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status_code"], "REQUIRES_REVIEW");
    let kinds = findings_kinds(&body);
    assert!(
        kinds
            .iter()
            .any(|k| k == "expired_allocation_active_capability"
                || k == "capability_without_allocation"),
        "{kinds:?}"
    );
}

#[tokio::test]
async fn matching_state_sync_ok() {
    let (state, _dir) = setup("org-default").await;
    let tid = ensure_aether_test_funded(&state).await;
    let agent = agent_subject(&state);
    // Match or exceed agent max_spend (1000).
    state
        .treasury
        .engine()
        .create_allocation(
            &tid,
            &agent,
            AssetId::new("AETHER_TEST"),
            Amount(5_000),
            Amount(0),
            None,
            "p24-match",
            "p24-match-ik",
        )
        .await
        .unwrap();

    let router = app(state.clone());
    let t = token(&state, "viewer").await;
    let (status, body) = get(
        &router,
        &format!("/sync/treasury-capabilities/{agent}"),
        Some(&t),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status_code"], "SYNC_OK", "{body}");
    assert!(body["findings"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn concurrent_reads_stable() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "viewer").await;
    let path = "/sync/treasury-capabilities";
    let (a, b) = tokio::join!(
        get(&router, path, Some(&t)),
        get(&router, path, Some(&t))
    );
    assert_eq!(a.0, StatusCode::OK);
    assert_eq!(b.0, StatusCode::OK);
    assert_eq!(a.1["status_code"], b.1["status_code"]);
    assert_eq!(a.1["observation_only"], true);
    assert_eq!(b.1["enforcement"], false);
}

#[tokio::test]
async fn cross_org_isolation() {
    let (state, _dir) = setup("org-default").await;
    let eng = state.treasury.engine();
    let _ = eng
        .register_asset(AssetId::new("AETHER_TEST"), 0, "test")
        .await;
    let foreign = eng
        .create_treasury("org-other", None, TreasuryKind::Organisation, "Other")
        .await
        .unwrap();
    eng.fund(
        &foreign.treasury_id,
        AssetId::new("AETHER_TEST"),
        Amount(50_000),
        "fx-fund",
        "fx-fund-ik",
    )
    .await
    .unwrap();
    let agent = agent_subject(&state);
    eng.create_allocation(
        &foreign.treasury_id,
        &agent,
        AssetId::new("AETHER_TEST"),
        Amount(50_000),
        Amount(0),
        None,
        "fx-alloc",
        "fx-alloc-ik",
    )
    .await
    .unwrap();

    let router = app(state.clone());
    let t = token(&state, "admin").await;
    let (status, body) = get(
        &router,
        &format!("/sync/treasury-capabilities/{agent}"),
        Some(&t),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Foreign allocation must not back our org observation → still missing alloc.
    assert_eq!(body["organisation_id"], "org-default");
    assert_eq!(body["status_code"], "DRIFT_DETECTED");
    let kinds = findings_kinds(&body);
    assert!(
        kinds.iter().any(|k| k == "capability_without_allocation"),
        "{kinds:?}"
    );
    // No finding should cite foreign allocation ids from org-other.
    for f in body["findings"].as_array().unwrap() {
        if let Some(aid) = f["allocation_id"].as_str() {
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT organisation_id FROM allocations WHERE allocation_id = ?",
            )
            .bind(aid)
            .fetch_optional(state.treasury.engine().db().pool())
            .await
            .unwrap();
            if let Some((org,)) = row {
                assert_ne!(org, "org-other");
            }
        }
    }
}
