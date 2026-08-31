//! Phase 28 — Lab agent registry & runtime identity tests.

use std::sync::Arc;

use aether_control_plane::agents::runtime::models::{
    AgentProviderType, AgentStatus, CreateAgentRequest, CreateSessionRequest,
    OrganisationRuntimeStatus, SessionStatus,
};
use aether_control_plane::agents::runtime::{registry, sessions, AgentRuntimeService};
use aether_control_plane::auth::jwt::issue_access_token;
use aether_control_plane::auth::middleware::auth_middleware;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::find_by_username;
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, AppState};
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
    let db_path = dir.path().join("phase28.db");
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

fn create_body(name: &str) -> Value {
    json!({
        "display_name": name,
        "description": "lab agent",
        "provider_type": "ANTHROPIC",
        "model_identifier": "claude-lab"
    })
}

#[tokio::test]
async fn create_agent() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (status, body) = post_json(
        &router,
        "/runtime/agents",
        &t,
        Some(&csrf(&state, &oid)),
        create_body("Claude Research Agent"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["agent"]["status"], "ACTIVE");
    assert_eq!(body["agent"]["organisation_id"], "org-default");
    assert_eq!(body["agent"]["provider_type"], "ANTHROPIC");
    assert!(body["agent"]["agent_id"].as_str().unwrap().starts_with("rt-agent-"));
}

#[tokio::test]
async fn duplicate_rejection() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let c = csrf(&state, &oid);
    let (_, first) = post_json(
        &router,
        "/runtime/agents",
        &t,
        Some(&c),
        create_body("Dup Agent"),
    )
    .await;
    let id = first["agent"]["agent_id"].as_str().unwrap();
    let (status, body) = post_json(
        &router,
        "/runtime/agents",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "agent_id": id,
            "display_name": "Other Name",
            "provider_type": "GOOGLE"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn cross_org_isolation() {
    let (state, _dir) = setup("org-default").await;
    // Plant foreign-org agent in same DB
    registry::create_agent(
        state.db.pool(),
        "org-other",
        "seed",
        &CreateAgentRequest {
            display_name: "Foreign".into(),
            description: None,
            provider_type: AgentProviderType::Google,
            model_identifier: Some("gemini".into()),
            agent_id: Some("rt-agent-foreign".into()),
        },
    )
    .await
    .unwrap();

    let router = app(state.clone());
    let t = token(&state, "admin").await;
    let (status, body) = get_json(&router, "/runtime/agents", &t).await;
    assert_eq!(status, StatusCode::OK);
    let agents = body["agents"].as_array().unwrap();
    assert!(agents
        .iter()
        .all(|a| a["organisation_id"] == "org-default"));
    assert!(agents
        .iter()
        .all(|a| a["agent_id"] != "rt-agent-foreign"));

    let (status, _) = get_json(&router, "/runtime/agents/rt-agent-foreign", &t).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn freeze_behaviour() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (_, created) = post_json(
        &router,
        "/runtime/agents",
        &t,
        Some(&csrf(&state, &oid)),
        create_body("Freeze Me"),
    )
    .await;
    let id = created["agent"]["agent_id"].as_str().unwrap();
    let (status, body) = post_json(
        &router,
        &format!("/runtime/agents/{id}/freeze"),
        &t,
        Some(&csrf(&state, &oid)),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["agent"]["status"], "FROZEN");

    let (status, body) = post_json(
        &router,
        "/runtime/sessions",
        &t,
        Some(&csrf(&state, &oid)),
        json!({ "agent_id": id }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
async fn revoke_behaviour() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (_, created) = post_json(
        &router,
        "/runtime/agents",
        &t,
        Some(&csrf(&state, &oid)),
        create_body("Revoke Me"),
    )
    .await;
    let id = created["agent"]["agent_id"].as_str().unwrap();
    let (status, body) = post_json(
        &router,
        &format!("/runtime/agents/{id}/revoke"),
        &t,
        Some(&csrf(&state, &oid)),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["agent"]["status"], "REVOKED");
    let (status, _) = post_json(
        &router,
        "/runtime/sessions",
        &t,
        Some(&csrf(&state, &oid)),
        json!({ "agent_id": id }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn create_session_ok() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (_, created) = post_json(
        &router,
        "/runtime/agents",
        &t,
        Some(&csrf(&state, &oid)),
        create_body("Session Agent"),
    )
    .await;
    let id = created["agent"]["agent_id"].as_str().unwrap();
    let (status, body) = post_json(
        &router,
        "/runtime/sessions",
        &t,
        Some(&csrf(&state, &oid)),
        json!({ "agent_id": id, "ttl_secs": 1800 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["session"]["status"], "ACTIVE");
    assert_eq!(body["credential"]["agent_id"], id);
    assert_eq!(body["credential"]["organisation_id"], "org-default");
}

#[tokio::test]
async fn session_expiry() {
    let (state, _dir) = setup("org-default").await;
    let agent = registry::create_agent(
        state.db.pool(),
        "org-default",
        "op",
        &CreateAgentRequest {
            display_name: "Exp".into(),
            description: None,
            provider_type: AgentProviderType::Local,
            model_identifier: None,
            agent_id: None,
        },
    )
    .await
    .unwrap();
    let (session, _cred) = sessions::create_session(
        state.db.pool(),
        "org-default",
        &CreateSessionRequest {
            agent_id: agent.agent_id.clone(),
            ttl_secs: Some(3600),
        },
    )
    .await
    .unwrap();
    sqlx::query("UPDATE runtime_sessions SET expires_at = ? WHERE session_id = ?")
        .bind((Utc::now() - Duration::minutes(1)).to_rfc3339())
        .bind(&session.session_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let updated = sessions::mark_expired_if_needed(
        state.db.pool(),
        "org-default",
        &session.session_id,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, SessionStatus::Expired);
}

#[tokio::test]
async fn session_revoke() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (_, created) = post_json(
        &router,
        "/runtime/agents",
        &t,
        Some(&csrf(&state, &oid)),
        create_body("RevSess"),
    )
    .await;
    let id = created["agent"]["agent_id"].as_str().unwrap();
    let (_, sess) = post_json(
        &router,
        "/runtime/sessions",
        &t,
        Some(&csrf(&state, &oid)),
        json!({ "agent_id": id }),
    )
    .await;
    let sid = sess["session"]["session_id"].as_str().unwrap();
    let (status, body) = post_json(
        &router,
        &format!("/runtime/sessions/{sid}/revoke"),
        &t,
        Some(&csrf(&state, &oid)),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["session"]["status"], "REVOKED");
}

#[tokio::test]
async fn frozen_agent_rejection_on_validate() {
    let (state, _dir) = setup("org-default").await;
    let agent = registry::create_agent(
        state.db.pool(),
        "org-default",
        "op",
        &CreateAgentRequest {
            display_name: "F2".into(),
            description: None,
            provider_type: AgentProviderType::OpenAi,
            model_identifier: None,
            agent_id: None,
        },
    )
    .await
    .unwrap();
    let (_session, cred) = sessions::create_session(
        state.db.pool(),
        "org-default",
        &CreateSessionRequest {
            agent_id: agent.agent_id.clone(),
            ttl_secs: Some(600),
        },
    )
    .await
    .unwrap();
    registry::set_agent_status(
        state.db.pool(),
        "org-default",
        &agent.agent_id,
        AgentStatus::Frozen,
    )
    .await
    .unwrap();
    let err = sessions::validate_credential(state.db.pool(), "org-default", &cred, true)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("act") || err.to_string().contains("FROZEN"));
}

#[tokio::test]
async fn invalid_credential() {
    let (state, _dir) = setup("org-default").await;
    let agent = registry::create_agent(
        state.db.pool(),
        "org-default",
        "op",
        &CreateAgentRequest {
            display_name: "IC".into(),
            description: None,
            provider_type: AgentProviderType::Enterprise,
            model_identifier: None,
            agent_id: None,
        },
    )
    .await
    .unwrap();
    let (session, mut cred) = sessions::create_session(
        state.db.pool(),
        "org-default",
        &CreateSessionRequest {
            agent_id: agent.agent_id,
            ttl_secs: Some(600),
        },
    )
    .await
    .unwrap();
    cred.credential_id = "wrong-cred".into();
    let err = sessions::validate_credential(state.db.pool(), "org-default", &cred, true)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("credential"));
    let _ = session;
}

#[tokio::test]
async fn organisation_mismatch() {
    let (state, _dir) = setup("org-default").await;
    let agent = registry::create_agent(
        state.db.pool(),
        "org-default",
        "op",
        &CreateAgentRequest {
            display_name: "OM".into(),
            description: None,
            provider_type: AgentProviderType::Anthropic,
            model_identifier: None,
            agent_id: None,
        },
    )
    .await
    .unwrap();
    let (_s, mut cred) = sessions::create_session(
        state.db.pool(),
        "org-default",
        &CreateSessionRequest {
            agent_id: agent.agent_id,
            ttl_secs: Some(600),
        },
    )
    .await
    .unwrap();
    cred.organisation_id = "org-other".into();
    let err = sessions::validate_credential(state.db.pool(), "org-default", &cred, true)
        .await
        .unwrap_err();
    assert!(err.to_string().to_ascii_lowercase().contains("organisation")
        || err.to_string().contains("mismatch"));
}

#[tokio::test]
async fn replayed_credential_jti() {
    let (state, _dir) = setup("org-default").await;
    let agent = registry::create_agent(
        state.db.pool(),
        "org-default",
        "op",
        &CreateAgentRequest {
            display_name: "Replay".into(),
            description: None,
            provider_type: AgentProviderType::Anthropic,
            model_identifier: None,
            agent_id: None,
        },
    )
    .await
    .unwrap();
    let (_s, cred) = sessions::create_session(
        state.db.pool(),
        "org-default",
        &CreateSessionRequest {
            agent_id: agent.agent_id,
            ttl_secs: Some(600),
        },
    )
    .await
    .unwrap();
    sessions::validate_credential(state.db.pool(), "org-default", &cred, true)
        .await
        .unwrap();
    let err = sessions::validate_credential(state.db.pool(), "org-default", &cred, true)
        .await
        .unwrap_err();
    assert!(err.to_string().to_ascii_lowercase().contains("replay"));
}

#[tokio::test]
async fn deterministic_lookup() {
    let (state, _dir) = setup("org-default").await;
    let a = registry::create_agent(
        state.db.pool(),
        "org-default",
        "op",
        &CreateAgentRequest {
            display_name: "Lookup".into(),
            description: Some("x".into()),
            provider_type: AgentProviderType::Google,
            model_identifier: Some("gemini-lab".into()),
            agent_id: Some("rt-agent-lookup-1".into()),
        },
    )
    .await
    .unwrap();
    let b = AgentRuntimeService::get_agent(&state.db, "org-default", "rt-agent-lookup-1")
        .await
        .unwrap();
    let c = AgentRuntimeService::get_agent(&state.db, "org-default", "rt-agent-lookup-1")
        .await
        .unwrap();
    assert_eq!(a.agent_id, b.agent_id);
    assert_eq!(b.agent_id, c.agent_id);
    assert_eq!(b.display_name, c.display_name);
    assert_eq!(b.provider_type, AgentProviderType::Google);
}

#[tokio::test]
async fn identity_then_e2_boundary_no_tools() {
    let (state, _dir) = setup("org-default").await;
    let router = app(state.clone());
    let t = token(&state, "operator").await;
    let oid = op_id(&state, "operator").await;
    let (_, created) = post_json(
        &router,
        "/runtime/agents",
        &t,
        Some(&csrf(&state, &oid)),
        create_body("E2 Boundary"),
    )
    .await;
    let id = created["agent"]["agent_id"].as_str().unwrap();
    let (_, sess) = post_json(
        &router,
        "/runtime/sessions",
        &t,
        Some(&csrf(&state, &oid)),
        json!({ "agent_id": id }),
    )
    .await;
    let cred = &sess["credential"];
    let (status, body) = post_json(
        &router,
        "/runtime/evaluate",
        &t,
        Some(&csrf(&state, &oid)),
        json!({
            "credential": cred,
            "action": "research.search",
            "force_review": false
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["identity_valid"], true);
    assert_eq!(body["tools_executed"], false);
    assert_eq!(body["apply_invoked"], false);
    assert_eq!(body["proto0_mutated"], false);
    assert_eq!(body["treasury_mutated"], false);
    // E2 may DENY (no PROTO-0 cap for rt-agent-*) — identity layer still passed.
    assert!(body["enforcement"]["decision"].is_string());
}

#[tokio::test]
async fn frozen_org_denies_actions() {
    let (state, _dir) = setup("org-default").await;
    let agent = registry::create_agent(
        state.db.pool(),
        "org-default",
        "op",
        &CreateAgentRequest {
            display_name: "OrgFreeze".into(),
            description: None,
            provider_type: AgentProviderType::Local,
            model_identifier: None,
            agent_id: None,
        },
    )
    .await
    .unwrap();
    registry::set_org_status(
        state.db.pool(),
        "org-default",
        OrganisationRuntimeStatus::Frozen,
    )
    .await
    .unwrap();
    let err = sessions::create_session(
        state.db.pool(),
        "org-default",
        &CreateSessionRequest {
            agent_id: agent.agent_id,
            ttl_secs: Some(60),
        },
    )
    .await
    .unwrap_err();
    assert!(err.to_string().to_ascii_lowercase().contains("frozen"));
}
