//! Phase 33 — Governance benchmark HTTP (lab only).

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use serde_json::{json, Value};

use crate::auth::middleware::AuthContext;
use crate::auth::roles::require_operator;
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::providers::benchmark::GovernanceBenchmarkRunner;
use crate::routes::AppState;
use crate::security::write_guard::guard_mutation;

/// POST /api/benchmark/governance/run
pub async fn run_governance_benchmark(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;
    let report = GovernanceBenchmarkRunner::run_full(
        &state.protocol,
        &state.treasury,
        &state.db,
        &ctx,
    )
    .await
    .map_err(|e| e.into_cp())?;
    Ok(Json(json!({
        "request_id": request_id,
        "report": report,
        "live_providers": false,
        "real_execution": false,
    })))
}
