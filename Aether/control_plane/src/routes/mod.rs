use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use crate::audit::mutation::MutationAuditService;
use crate::auth::handlers::{login, logout, me, refresh};
use crate::auth::rate_limit::SharedLoginRateLimiter;
use crate::config::Config;
use crate::db::Db;
use crate::protocol::state::ProtocolState;
use crate::security::csrf::SharedCsrfStore;
use crate::signer::SharedSigningGateway;

pub mod agents;
pub mod apply;
pub mod apply_approvals;
pub mod audit;
pub mod benchmark;
pub mod capabilities;
pub mod enforcement;
pub mod escrows;
pub mod policies;
pub mod providers;
pub mod reputation;
pub mod runtime_agents;
pub mod sandbox;
pub mod settlements;
pub mod sync;
pub mod system;
pub mod tools;
pub mod treasury;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Db,
    pub protocol: Arc<ProtocolState>,
    pub rate_limiter: SharedLoginRateLimiter,
    pub csrf_store: SharedCsrfStore,
    pub mutation_audit: MutationAuditService,
    /// Audited signer gateway — Phase 3. Does not call PROTO-0.
    pub signing: SharedSigningGateway,
    /// Read-only treasury observation adapter (Phase 19).
    pub treasury: crate::treasury::TreasuryAdapter,
    /// Phase 21 — ledger governance write service (no rails / Apply).
    pub treasury_write: crate::treasury::TreasuryWriteService,
}

pub fn auth_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .with_state(state)
}

pub fn api_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/agents", get(agents::list_agents))
        .route("/agents/:id", get(agents::get_agent))
        .route("/agents/:id/capabilities", get(agents::agent_capabilities))
        .route("/agents/:id/escrows", get(agents::agent_escrows))
        .route("/agents/:id/settlements", get(agents::agent_settlements))
        .route("/agents/:id/reputation", get(agents::agent_reputation))
        .route("/agents/:id/events", get(agents::agent_events))
        .route("/capabilities", get(capabilities::list_capabilities))
        .route("/escrows", get(escrows::list_escrows))
        .route("/settlements", get(settlements::list_settlements))
        .route("/reputation/:agent_id", get(reputation::get_reputation))
        .route("/audit", get(audit::list_audit))
        .route("/system", get(system::health))
        .route("/system/health", get(system::health))
        .route("/me", get(me))
        .route("/csrf-token", get(policies::issue_csrf))
        .route(
            "/policies",
            get(policies::list_policies).post(policies::create_policy),
        )
        .route(
            "/policies/:id",
            get(policies::get_policy).put(policies::update_policy),
        )
        .route("/policies/:id/submit", post(policies::submit_policy))
        .route("/policies/:id/approve", post(policies::approve_policy))
        .route("/policies/:id/reject", post(policies::reject_policy))
        .route("/policies/:id/archive", post(policies::archive_policy))
        .route("/policies/:id/dry-run", post(policies::dry_run_policy))
        .route("/apply-approvals", post(apply_approvals::create_approval))
        .route(
            "/apply-approvals/:id",
            get(apply_approvals::get_approval).delete(apply_approvals::cancel_approval),
        )
        .route("/apply/prepare", post(apply::prepare_apply))
        .route("/apply", post(apply::execute_apply_http))
        .route("/apply/operations/:operation_id", get(apply::get_operation))
        .route("/apply/reconcile", get(apply::reconcile_list))
        .route("/apply/reconcile/:operation_id", get(apply::reconcile_get))
        .route(
            "/apply/reconcile/:operation_id/abort",
            post(apply::reconcile_abort),
        )
        .route("/treasury", get(treasury::list_treasury))
        .route(
            "/treasury/mutations",
            get(treasury::list_mutations).post(treasury::create_mutation),
        )
        .route("/treasury/mutations/timeline", get(treasury::mutation_timeline))
        .route("/treasury/ops/metrics", get(treasury::ops_metrics))
        .route("/treasury/ops/reconcile", get(treasury::ops_reconcile_scan))
        .route("/treasury/ops/reconcile/run", post(treasury::ops_reconcile_run))
        .route(
            "/treasury/ops/reconcile/reservations/:reservation_id/force-release",
            post(treasury::ops_force_release),
        )
        .route("/treasury/ops/sweep", post(treasury::ops_sweep))
        .route(
            "/treasury/mutations/:mutation_id",
            get(treasury::get_mutation),
        )
        .route(
            "/treasury/mutations/:mutation_id/approve",
            post(treasury::approve_mutation),
        )
        .route(
            "/treasury/mutations/:mutation_id/execute",
            post(treasury::execute_mutation),
        )
        .route(
            "/treasury/mutations/:mutation_id/cancel",
            post(treasury::cancel_mutation),
        )
        .route(
            "/treasury/reservations/:reservation_id/release",
            post(treasury::release_reservation),
        )
        .route(
            "/treasury/reservations/:reservation_id/settlement-post",
            post(treasury::settlement_post),
        )
        .route(
            "/treasury/:id/reservations",
            get(treasury::treasury_reservations).post(treasury::create_reservation),
        )
        .route("/treasury/:id/freeze", post(treasury::freeze_treasury))
        .route("/treasury/:id", get(treasury::get_treasury))
        .route(
            "/treasury/:id/allocations",
            get(treasury::treasury_allocations),
        )
        .route("/treasury/:id/journal", get(treasury::treasury_journal))
        .route(
            "/treasury/:id/settlements",
            get(treasury::treasury_settlements),
        )
        .route("/treasury/:id/security", get(treasury::treasury_security))
        .route(
            "/sync/treasury-capabilities",
            get(sync::observe_org),
        )
        .route(
            "/sync/treasury-capabilities/:agent_id",
            get(sync::observe_agent),
        )
        .route("/enforcement/evaluate", post(enforcement::evaluate))
        .route("/runtime/agents", get(runtime_agents::list_agents).post(runtime_agents::create_agent))
        .route("/runtime/agents/:agent_id", get(runtime_agents::get_agent))
        .route(
            "/runtime/agents/:agent_id/freeze",
            post(runtime_agents::freeze_agent),
        )
        .route(
            "/runtime/agents/:agent_id/revoke",
            post(runtime_agents::revoke_agent),
        )
        .route("/runtime/sessions", post(runtime_agents::create_session))
        .route(
            "/runtime/sessions/:session_id/revoke",
            post(runtime_agents::revoke_session),
        )
        .route(
            "/runtime/sessions/:session_id/expire-check",
            post(runtime_agents::expire_session_check),
        )
        .route(
            "/runtime/evaluate",
            post(runtime_agents::evaluate_runtime_request),
        )
        .route("/tools", get(tools::list_tools).post(tools::create_tool))
        .route("/tools/evaluate", post(tools::evaluate_tool))
        .route("/tools/:tool_id/disable", post(tools::disable_tool))
        .route("/sandbox/bootstrap", post(sandbox::bootstrap))
        .route("/sandbox/run", post(sandbox::run_task))
        .route(
            "/sandbox/reconstruct/:request_id",
            get(sandbox::reconstruct),
        )
        .route("/providers/lab/reason", post(providers::lab_reason))
        .route(
            "/providers/lab/demo/agent-a-research",
            post(providers::demo_agent_a_research),
        )
        .route(
            "/providers/lab/sessions/:session_id/cancel",
            post(providers::cancel_session_governor),
        )
        .route(
            "/benchmark/governance/run",
            post(benchmark::run_governance_benchmark),
        )
        .with_state(state)
}
