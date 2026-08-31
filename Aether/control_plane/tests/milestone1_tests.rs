use std::sync::Arc;

use aether_control_plane::auth::jwt::{
    issue_access_token, issue_refresh_token, verify_access_token, verify_refresh_token, Claims,
    JWT_AUDIENCE_API, JWT_ISSUER, TOKEN_TYPE_ACCESS,
};
use aether_control_plane::build_app;
use aether_control_plane::config::Config;
use aether_control_plane::db::operators::{create_operator, find_by_username, OperatorRole};
use aether_control_plane::db::Db;
use aether_control_plane::protocol::state::ProtocolState;
use aether_control_plane::routes::{api_router, auth_router, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use jsonwebtoken::{encode, EncodingKey, Header};
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

async fn setup_app() -> (Arc<AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
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

fn cookie_value(set_cookie: &str, name: &str) -> Option<String> {
    set_cookie.split(';').next().and_then(|part| {
        let mut it = part.splitn(2, '=');
        let n = it.next()?.trim();
        let v = it.next()?.trim();
        if n == name {
            Some(v.to_string())
        } else {
            None
        }
    })
}

fn cookie_from_headers(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    for value in headers.get_all(header::SET_COOKIE) {
        let s = value.to_str().ok()?;
        if let Some(v) = cookie_value(s, name) {
            return Some(v);
        }
    }
    None
}

#[tokio::test]
async fn login_succeeds_with_valid_credentials() {
    let (state, _dir) = setup_app().await;
    let app = auth_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "10.0.0.1")
                .body(Body::from(r#"{"username":"admin","password":"admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn login_fails_with_invalid_password() {
    let (state, _dir) = setup_app().await;
    let app = auth_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "10.0.0.2")
                .body(Body::from(r#"{"username":"admin","password":"wrong"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "invalid credentials");
}

#[tokio::test]
async fn login_unknown_user_returns_generic_error() {
    let (state, _dir) = setup_app().await;
    let app = auth_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "10.0.0.3")
                .body(Body::from(r#"{"username":"nosuch","password":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "invalid credentials");
}

#[tokio::test]
async fn repeated_failed_attempts_trigger_rate_limit() {
    let (state, _dir) = setup_app().await;
    let app = auth_router(state);

    for i in 0..5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "10.0.0.50")
                    .body(Body::from(r#"{"username":"admin","password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "attempt {i} should be 401"
        );
    }

    let limited = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "10.0.0.50")
                .body(Body::from(r#"{"username":"admin","password":"wrong"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = limited.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "too many login attempts");
}

#[tokio::test]
async fn successful_login_resets_rate_limit() {
    let (state, _dir) = setup_app().await;
    let app = auth_router(state);

    for _ in 0..4 {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "10.0.0.60")
                    .body(Body::from(r#"{"username":"admin","password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let ok = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "10.0.0.60")
                .body(Body::from(r#"{"username":"admin","password":"admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);

    // After reset, five more failures are allowed before 429.
    for i in 0..5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "10.0.0.60")
                    .body(Body::from(r#"{"username":"admin","password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "post-reset attempt {i}"
        );
    }
}

#[tokio::test]
async fn expired_token_rejected() {
    let claims = Claims {
        sub: "op1".into(),
        username: "admin".into(),
        role: "admin".into(),
        iss: JWT_ISSUER.into(),
        aud: JWT_AUDIENCE_API.into(),
        typ: TOKEN_TYPE_ACCESS.into(),
        jti: None,
        iat: 1,
        exp: 2,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(b"secret"),
    )
    .unwrap();
    let err = verify_access_token("secret", &token);
    assert!(err.is_err());
}

#[tokio::test]
async fn jwt_rejects_wrong_issuer() {
    let claims = Claims {
        sub: "op1".into(),
        username: "admin".into(),
        role: "admin".into(),
        iss: "evil-issuer".into(),
        aud: JWT_AUDIENCE_API.into(),
        typ: TOKEN_TYPE_ACCESS.into(),
        jti: None,
        iat: chrono::Utc::now().timestamp(),
        exp: chrono::Utc::now().timestamp() + 900,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(b"secret"),
    )
    .unwrap();
    assert!(verify_access_token("secret", &token).is_err());
}

#[tokio::test]
async fn jwt_rejects_wrong_audience() {
    let claims = Claims {
        sub: "op1".into(),
        username: "admin".into(),
        role: "admin".into(),
        iss: JWT_ISSUER.into(),
        aud: "wrong-audience".into(),
        typ: TOKEN_TYPE_ACCESS.into(),
        jti: None,
        iat: chrono::Utc::now().timestamp(),
        exp: chrono::Utc::now().timestamp() + 900,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(b"secret"),
    )
    .unwrap();
    assert!(verify_access_token("secret", &token).is_err());
}

#[tokio::test]
async fn access_token_cannot_be_used_as_refresh_token() {
    let (state, _dir) = setup_app().await;
    let access = issue_access_token(
        &state.config.jwt_secret,
        "admin-id",
        "admin",
        OperatorRole::Admin,
        900,
    )
    .unwrap();
    assert!(verify_refresh_token(&state.config.jwt_secret, &access).is_err());

    let app = auth_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .header("cookie", format!("cp_refresh_token={access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_token_cannot_access_api_routes() {
    let (state, _dir) = setup_app().await;
    let refresh = issue_refresh_token(
        &state.config.jwt_secret,
        "admin-id",
        "admin",
        OperatorRole::Admin,
        900,
    )
    .unwrap();

    let app = api_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state,
        aether_control_plane::auth::middleware::auth_middleware,
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/agents")
                .header("authorization", format!("Bearer {refresh}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_token_rotation_works_and_old_token_rejected() {
    let (state, _dir) = setup_app().await;
    let app = auth_router(state.clone());

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "10.0.0.70")
                .body(Body::from(r#"{"username":"admin","password":"admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let refresh_a =
        cookie_from_headers(login.headers(), "cp_refresh_token").expect("refresh cookie");

    let refresh_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .header("cookie", format!("cp_refresh_token={refresh_a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let refresh_status = refresh_resp.status();
    let refresh_headers = refresh_resp.headers().clone();
    let refresh_body = refresh_resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        refresh_status,
        StatusCode::OK,
        "body={}",
        String::from_utf8_lossy(&refresh_body)
    );
    let refresh_b = cookie_from_headers(&refresh_headers, "cp_refresh_token").expect("refresh b");
    assert_ne!(refresh_a, refresh_b);

    // Old refresh token must be rejected (and reuse logged).
    let reuse = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .header("cookie", format!("cp_refresh_token={refresh_a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reuse.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_route_requires_auth() {
    let (state, _dir) = setup_app().await;
    let app = api_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state,
        aether_control_plane::auth::middleware::auth_middleware,
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn agents_endpoint_returns_observatory_data() {
    let (state, _dir) = setup_app().await;
    let token = issue_access_token(
        &state.config.jwt_secret,
        "admin-id",
        "admin",
        OperatorRole::Admin,
        900,
    )
    .unwrap();

    let app = api_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state,
        aether_control_plane::auth::middleware::auth_middleware,
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/agents")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let agents = json["agents"].as_array().expect("agents array");
    assert_eq!(agents.len(), 3);
}

#[tokio::test]
async fn dashboard_matches_protocol_escrow_state() {
    let protocol = ProtocolState::bootstrap().expect("bootstrap");
    let escrows = aether_control_plane::protocol::proto2::list_escrows(&protocol);
    assert_eq!(escrows.len(), 1);
    let escrow = &escrows[0];
    assert_eq!(escrow.status, "released");
    assert!(escrow.soft_finality);
    assert!(escrow.hard_finality);
}

#[tokio::test]
async fn protocol_write_routes_remain_absent() {
    let (state, _dir) = setup_app().await;
    let token = issue_access_token(
        &state.config.jwt_secret,
        "admin-id",
        "admin",
        OperatorRole::Admin,
        900,
    )
    .unwrap();
    let app = api_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state,
        aether_control_plane::auth::middleware::auth_middleware,
    ));

    for (method, uri, body) in [
        ("POST", "/capabilities/revoke", r#"{"capability_id":"00"}"#),
        ("POST", "/capabilities/grant", r#"{}"#),
        ("POST", "/protocol/apply-policy", r#"{}"#),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {uri}");
    }
}

#[tokio::test]
async fn viewer_role_stored_correctly() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roles.db");
    let db = Db::connect(db_path.to_str().unwrap()).await.unwrap();
    db.migrate().await.unwrap();
    create_operator(db.pool(), "viewer1", "viewer1", OperatorRole::Viewer)
        .await
        .unwrap();
    let op = find_by_username(db.pool(), "viewer1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(op.role, OperatorRole::Viewer);
}

#[tokio::test]
async fn security_headers_present_on_responses() {
    let (state, _dir) = setup_app().await;
    let app = build_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "10.0.0.80")
                .body(Body::from(r#"{"username":"admin","password":"admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
    assert_eq!(
        headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
    );
    assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
    assert!(headers.contains_key(header::REFERRER_POLICY));
}

#[tokio::test]
async fn issued_access_token_has_expected_claims() {
    let token = issue_access_token("secret", "op1", "admin", OperatorRole::Admin, 900).unwrap();
    let claims = verify_access_token("secret", &token).unwrap();
    assert_eq!(claims.iss, JWT_ISSUER);
    assert_eq!(claims.aud, JWT_AUDIENCE_API);
    assert_eq!(claims.typ, TOKEN_TYPE_ACCESS);
}
