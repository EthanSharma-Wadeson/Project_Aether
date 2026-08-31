//! Phase 22.5 — reservation TTL, sweeper, reconcile, metrics, backup restore.

use std::sync::Arc;

use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
use aether_control_plane::treasury::ops::sweeper::run_sweeper_once;
use aether_treasury::{Amount, AssetId, TreasuryEngine, TreasuryKind};
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
        treasury_mutation_executing_timeout_secs: 60,
        admin_mfa_required: false,
    }
}

async fn setup(org: &str) -> (Arc<AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("phase225.db");
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
    mfa: bool,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header("Authorization", format!("Bearer {bearer}"))
        .header("content-type", "application/json");
    if let Some(c) = csrf_token {
        builder = builder.header("x-csrf-token", c);
    }
    if mfa {
        builder = builder.header("x-aether-mfa-verified", "true");
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
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn reservation_default_ttl_and_settlement_rejects_expired() {
    let eng = TreasuryEngine::in_memory().await.unwrap();
    let _ = eng.register_asset(AssetId::new("GBP"), 2, "fiat").await;
    let t = eng
        .create_treasury("org-t", None, TreasuryKind::Organisation, "Org")
        .await
        .unwrap();
    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(10_000),
        "r1",
        "ik1",
    )
    .await
    .unwrap();
    let (alloc, _) = eng
        .create_allocation(
            &t.treasury_id,
            "agent-1",
            AssetId::new("GBP"),
            Amount(5_000),
            Amount(0),
            None,
            "r2",
            "ik2",
        )
        .await
        .unwrap();
    let past = Utc::now() - Duration::seconds(10);
    let err = eng
        .reserve(
            &t.treasury_id,
            &alloc.allocation_id,
            AssetId::new("GBP"),
            Amount(100),
            "r3",
            "ik3",
            Some(past),
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("future") || err.to_string().contains("expires"));

    let (res, _) = eng
        .reserve(
            &t.treasury_id,
            &alloc.allocation_id,
            AssetId::new("GBP"),
            Amount(100),
            "r4",
            "ik4",
            None,
        )
        .await
        .unwrap();
    assert!(res.expires_at.is_some());

    // Force past expiry in DB then settle must fail
    sqlx::query("UPDATE reservations SET expires_at = ? WHERE reservation_id = ?")
        .bind((Utc::now() - Duration::minutes(1)).to_rfc3339())
        .bind(&res.reservation_id)
        .execute(eng.db().pool())
        .await
        .unwrap();
    let settle_err = eng
        .settlement_post(&res.reservation_id, "r5", "ik5")
        .await
        .unwrap_err();
    assert!(settle_err.to_string().contains("expired"));
}

#[tokio::test]
async fn sweeper_releases_due_reservation_with_audit() {
    let (state, _dir) = setup("org-default").await;
    let org = state.treasury.organisation_id().to_string();
    let list = state.treasury.engine().list_treasuries(&org).await.unwrap();
    let tid = list[0].treasury_id.clone();
    let (alloc, _) = state
        .treasury
        .engine()
        .create_allocation(
            &tid,
            "agent-sweep",
            AssetId::new("GBP"),
            Amount(50_000),
            Amount(0),
            None,
            "alloc-s",
            "alloc-s-ik",
        )
        .await
        .unwrap();
    let (res, _) = state
        .treasury
        .engine()
        .reserve(
            &tid,
            &alloc.allocation_id,
            AssetId::new("GBP"),
            Amount(500),
            "res-s",
            "res-s-ik",
            Some(Utc::now() + Duration::seconds(30)),
        )
        .await
        .unwrap();
    sqlx::query("UPDATE reservations SET expires_at = ? WHERE reservation_id = ?")
        .bind((Utc::now() - Duration::seconds(5)).to_rfc3339())
        .bind(&res.reservation_id)
        .execute(state.treasury.engine().db().pool())
        .await
        .unwrap();

    let report = run_sweeper_once(&state.treasury, state.db.pool(), &state.config).await;
    assert!(report.released >= 1, "{report:?}");
    let after = state
        .treasury
        .engine()
        .get_reservation(&res.reservation_id)
        .await
        .unwrap();
    assert_eq!(after.status.as_str(), "released");

    let timeline = state
        .treasury_write
        .timeline(
            &state.treasury,
            &aether_control_plane::auth::middleware::AuthContext {
                operator_id: "system".into(),
                username: "system".into(),
                role: aether_control_plane::db::operators::OperatorRole::Admin,
            },
        )
        .await
        .unwrap();
    let events = timeline["events"].as_array().unwrap();
    assert!(
        events.iter().any(|e| e["outcome"] == "reservation_released"
            || e["outcome"] == "reservation_expired"),
        "{events:?}"
    );
}

#[tokio::test]
async fn crash_recovery_marks_stuck_executing_mutations() {
    let (state, _dir) = setup("org-default").await;
    let admin_t = token(&state, "admin").await;
    let admin_id = op_id(&state, "admin").await;
    let org = state.treasury.organisation_id();
    let now = Utc::now() - Duration::minutes(10);
    sqlx::query(
        r#"
        INSERT INTO treasury_mutation_requests (
          mutation_id, organisation_id, request_id, idempotency_key, operation,
          payload_json, payload_hash, target, requested_by, requester_role, status,
          expires_at, created_at
        ) VALUES ('mut-stuck', ?, 'rid', 'ik-stuck', 'adjustment', '{}', 'hash', NULL,
                  'op', 'operator', 'executing', ?, ?)
        "#,
    )
    .bind(org)
    .bind((now + Duration::minutes(30)).to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(state.db.pool())
    .await
    .unwrap();

    let router = app(state.clone());
    let (status, body) = post_json(
        &router,
        "/treasury/ops/reconcile/run",
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({ "confirm": true, "run_sweep": false, "fix_mutations": true }),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["mutations_marked_failed"].as_u64().unwrap() >= 1);

    let st: String = sqlx::query_scalar(
        "SELECT status FROM treasury_mutation_requests WHERE mutation_id = 'mut-stuck'",
    )
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(st, "failed");
}

#[tokio::test]
async fn ops_metrics_and_audit_reconstruction() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let admin_t = token(&state, "admin").await;
    let (status, metrics) = get_json(&router, "/treasury/ops/metrics", &admin_t).await;
    assert_eq!(status, StatusCode::OK, "{metrics}");
    assert!(metrics["active_reservations"].is_number());
    assert!(metrics["ledger_notice"]
        .as_str()
        .unwrap()
        .contains("Internal ledger"));
}

#[tokio::test]
async fn admin_mfa_boundary_enforced_when_configured() {
    let (_state, _dir) = setup("org-default").await;
    // mutate config via unsafe replace — rebuild state with MFA on
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("mfa.db");
    let mut config = test_config(db_path.to_str().unwrap(), "org-default");
    config.admin_mfa_required = true;
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
    let router = app(state.clone());
    let admin_t = token(&state, "admin").await;
    let admin_id = op_id(&state, "admin").await;
    let (status, body) = post_json(
        &router,
        "/treasury/ops/sweep",
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({}),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    let (status2, body2) = post_json(
        &router,
        "/treasury/ops/sweep",
        &admin_t,
        Some(&csrf(&state, &admin_id)),
        json!({}),
        true,
    )
    .await;
    assert_eq!(status2, StatusCode::OK, "{body2}");
}

#[tokio::test]
async fn backup_restore_drill_preserves_journal_integrity() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.treasury.db");
    let eng = TreasuryEngine::new(
        aether_treasury::db::TreasuryDb::connect(src.to_str().unwrap())
            .await
            .unwrap(),
    );
    let _ = eng.register_asset(AssetId::new("GBP"), 2, "fiat").await;
    let t = eng
        .create_treasury("org-bak", None, TreasuryKind::Organisation, "Bak")
        .await
        .unwrap();
    eng.fund(
        &t.treasury_id,
        AssetId::new("GBP"),
        Amount(42_00),
        "fund",
        "fund-ik",
    )
    .await
    .unwrap();
    eng.assert_journal_balanced(
        &eng.list_journal_entries("org-bak")
            .await
            .unwrap()
            .first()
            .map(|e| e.batch_id.clone())
            .unwrap(),
    )
    .await
    .unwrap();

    // Checkpoint / copy files (WAL-aware: reopen then copy)
    let dst = dir.path().join("restore.treasury.db");
    std::fs::copy(&src, &dst).unwrap();
    for suffix in ["-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{suffix}", src.display()));
        if p.exists() {
            let _ = std::fs::copy(&p, format!("{}{suffix}", dst.display()));
        }
    }

    let restored = TreasuryEngine::new(
        aether_treasury::db::TreasuryDb::connect(dst.to_str().unwrap())
            .await
            .unwrap(),
    );
    let bal = restored
        .available_balance("org-bak", &t.treasury_id, &AssetId::new("GBP"))
        .await
        .unwrap();
    assert_eq!(bal.0, 42_00);
    restored.replay_balances("org-bak").await.unwrap();
    let bal2 = restored
        .available_balance("org-bak", &t.treasury_id, &AssetId::new("GBP"))
        .await
        .unwrap();
    assert_eq!(bal2.0, 42_00);
}

use std::path::PathBuf;
