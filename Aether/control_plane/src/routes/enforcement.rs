//! Phase 26 — Runtime enforcement evaluation HTTP (decision only).

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use serde_json::{json, Value};

use crate::auth::middleware::AuthContext;
use crate::auth::roles::require_operator;
use crate::enforcement::{EnforcementRequest, EnforcementService};
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::routes::AppState;
use crate::security::write_guard::guard_mutation;

/// POST /api/enforcement/evaluate — read-only decision; no execution.
pub async fn evaluate(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(req): Json<EnforcementRequest>,
) -> Result<Json<Value>> {
    require_operator(&ctx)?;
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        rid.as_ref().map(|Extension(r)| r),
        false,
    )?;

    let decision = EnforcementService::evaluate(
        &state.protocol,
        &state.treasury,
        &state.db,
        &ctx,
        req,
        request_id,
    )
    .await?;

    Ok(Json(json!({
        "decision": decision.decision,
        "reason": decision.reason,
        "deny_reason": decision.deny_reason,
        "evidence": decision.evidence,
        "request_id": decision.request_id,
        "decision_code": decision.decision_code,
        "agent_id": decision.agent_id,
        "organisation_id": decision.organisation_id,
        "requested_action": decision.requested_action,
        "capability_checked": decision.capability_checked,
        "allocation_checked": decision.allocation_checked,
        "policy_checked": decision.policy_checked,
        "risk_result": decision.risk_result,
        "remediation": decision.remediation,
        "timestamp": decision.timestamp,
        "apply_invoked": decision.apply_invoked,
        "treasury_mutated": decision.treasury_mutated,
        "proto0_mutated": decision.proto0_mutated,
    })))
}
