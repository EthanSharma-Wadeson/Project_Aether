//! Aether Control Plane — Enterprise Agent Governance Console.

pub mod apply;
pub mod apply_treasury_sync;
pub mod agents;
pub mod audit;
pub mod auth;
pub mod config;
pub mod db;
pub mod enforcement;
pub mod error;
pub mod execution;
pub mod middleware;
pub mod models;
pub mod policies;
pub mod protocol;
pub mod providers;
pub mod routes;
pub mod security;
pub mod signer;
pub mod tools;
pub mod treasury;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::{HeaderValue, Method};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::audit::mutation::MutationAuditService;
use crate::auth::middleware::auth_middleware;
use crate::auth::rate_limit::InMemoryLoginRateLimiter;
use crate::auth::security_headers::security_headers;
use crate::config::{production_config_check, Config};
use crate::db::Db;
use crate::middleware::request_id::request_id_middleware;
use crate::protocol::state::ProtocolState;
use crate::routes::{api_router, auth_router, AppState};
use crate::security::csrf::CsrfStore;
use crate::signer::build_signing_gateway;

pub async fn run(config: Config) -> error::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .init();

    // Fail closed on unsafe production posture (no Apply; config hardening only).
    production_config_check(&config)?;
    crate::apply::enablement::validate_apply_enablement_config(&config)?;

    let db = Db::connect(&config.db_path).await?;
    db.migrate().await?;
    db.seed_default_operators(&config).await?;

    // C6 — startup scan for stuck/reserved/executing Apply operations (no PROTO-0 retry).
    {
        let replay = crate::apply::replay::ReplayStore::new(db.pool().clone());
        match crate::apply::replay::startup_scan(&replay).await {
            Ok(report) => {
                tracing::info!(
                    reserved = report.reserved.len(),
                    executing = report.executing.len(),
                    stuck = report.stuck.len(),
                    reserved_timed_out = report.reserved_timed_out,
                    executing_timed_out = report.executing_timed_out,
                    "apply reconcile startup scan complete"
                );
            }
            Err(e) => {
                tracing::warn!(error = %e, "apply reconcile startup scan failed (non-fatal)");
            }
        }
    }

    let signing = build_signing_gateway(&config, &db)?;
    tracing::info!(
        signer_mode = %config.signer.mode,
        signer_identity = %signing.signer_identity(),
        "signer gateway ready (no PROTO-0 coupling)"
    );

    let protocol = ProtocolState::bootstrap()?;
    let treasury = crate::treasury::TreasuryAdapter::connect(&config)
        .await
        .map_err(|e| error::Error::Config(format!("treasury adapter: {e}")))?;
    let treasury_write = crate::treasury::TreasuryWriteService::new(db.pool().clone());
    let state = Arc::new(AppState {
        config: config.clone(),
        mutation_audit: MutationAuditService::new(db.pool().clone()),
        db,
        protocol,
        rate_limiter: InMemoryLoginRateLimiter::shared(),
        csrf_store: CsrfStore::shared(),
        signing,
        treasury,
        treasury_write,
    });

    // Phase 22.5 — startup treasury reconcile (stuck executing) + optional sweeper loop
    {
        let svc = crate::treasury::ops::reconcile::ReconcileService::new(state.db.pool().clone());
        match svc.scan(&state.treasury, &state.config).await {
            Ok(report) => {
                tracing::info!(
                    stuck_executing = report.stuck_executing_mutations.len(),
                    due_reservations = report.expired_reservations.len(),
                    stuck_reservations = report.stuck_reservations.len(),
                    "treasury ops reconcile scan complete"
                );
            }
            Err(e) => tracing::warn!(error = %e, "treasury ops reconcile scan failed (non-fatal)"),
        }
        let _ = crate::treasury::ops::sweeper::run_sweeper_once(
            &state.treasury,
            state.db.pool(),
            &state.config,
        )
        .await;
        crate::treasury::ops::sweeper::spawn_sweeper_loop(
            state.treasury.clone(),
            state.db.pool().clone(),
            state.config.clone(),
        );
    }

    let api = api_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        auth_middleware,
    ));

    let mut app = Router::new()
        .merge(auth_router(state.clone()))
        .nest("/api", api);

    if let Some(frontend_dir) = &config.frontend_dir {
        let index = ServeFile::new(frontend_dir.join("index.html"));
        let assets = ServeDir::new(frontend_dir);
        app = app.nest_service("/assets", assets).fallback_service(index);
    }

    app = app
        .layer(axum::middleware::from_fn(request_id_middleware))
        .layer(axum::middleware::from_fn(security_headers))
        .layer(TraceLayer::new_for_http())
        .layer(build_cors_layer(&config));

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e: std::net::AddrParseError| error::Error::Config(e.to_string()))?;
    tracing::info!("control plane listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(error::Error::Io)?;
    axum::serve(listener, app).await.map_err(error::Error::Io)?;

    Ok(())
}

/// Restrictive CORS: configured allowlist, or localhost-only for local/dev.
/// Never uses `CorsLayer::permissive()` in production paths.
fn build_cors_layer(config: &Config) -> CorsLayer {
    let origins: Vec<HeaderValue> = if config.allowed_origins.is_empty() {
        [
            "http://127.0.0.1:5173",
            "http://localhost:5173",
            "http://127.0.0.1:3001",
            "http://localhost:3001",
        ]
        .into_iter()
        .filter_map(|s| s.parse().ok())
        .collect()
    } else {
        config
            .allowed_origins
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::HeaderName::from_static("x-csrf-token"),
            axum::http::HeaderName::from_static("x-request-id"),
        ])
        .allow_credentials(true)
}

/// Build the same router used in production (for integration tests).
pub fn build_app(state: Arc<AppState>) -> Router {
    let api = api_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        auth_middleware,
    ));
    Router::new()
        .merge(auth_router(state))
        .nest("/api", api)
        .layer(axum::middleware::from_fn(request_id_middleware))
        .layer(axum::middleware::from_fn(security_headers))
}
