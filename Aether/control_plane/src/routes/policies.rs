//! Policy template HTTP routes — Control Plane DB only (no PROTO-0).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::json;

use crate::auth::middleware::AuthContext;
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::models::policies::{CreatePolicyRequest, RejectPolicyRequest, UpdatePolicyRequest};
use crate::policies::PolicyTemplateService;
use crate::routes::AppState;
use crate::security::csrf::{csrf_cookie, CsrfStore};
use crate::security::write_guard::guard_mutation;

fn service(state: &AppState) -> PolicyTemplateService<'_> {
    PolicyTemplateService::new(state.db.pool(), &state.mutation_audit)
}

pub async fn list_policies(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
) -> Result<Json<serde_json::Value>> {
    let policies = service(&state).list(&ctx).await?;
    Ok(Json(json!({
        "policies": policies,
        "note": "Approved templates are not protocol-active until a future apply phase.",
    })))
}

pub async fn get_policy(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let (policy, versions, audit_events) = service(&state).history(&ctx, &id).await?;
    Ok(Json(json!({
        "policy": policy,
        "versions": versions,
        "audit_events": audit_events,
        "protocol_active": false,
        "note": "Approved policies are Control Plane records only until a future controlled application phase.",
    })))
}

pub async fn create_policy(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
    Json(body): Json<CreatePolicyRequest>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        false,
    )?;
    let policy = service(&state).create(&ctx, &request_id, body).await?;
    Ok(Json(json!({ "policy": policy })))
}

pub async fn update_policy(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
    Json(body): Json<UpdatePolicyRequest>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        false,
    )?;
    let policy = service(&state).update(&ctx, &request_id, &id, body).await?;
    Ok(Json(json!({ "policy": policy })))
}

pub async fn submit_policy(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        false,
    )?;
    let policy = service(&state).submit(&ctx, &request_id, &id).await?;
    Ok(Json(json!({ "policy": policy })))
}

pub async fn approve_policy(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        true,
    )?;
    let policy = service(&state).approve(&ctx, &request_id, &id).await?;
    Ok(Json(json!({
        "policy": policy,
        "protocol_active": false,
        "note": "Approval does not apply the policy to the protocol.",
    })))
}

pub async fn reject_policy(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
    Json(body): Json<RejectPolicyRequest>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        true,
    )?;
    let policy = service(&state).reject(&ctx, &request_id, &id, body).await?;
    Ok(Json(json!({ "policy": policy })))
}

pub async fn archive_policy(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        true,
    )?;
    let policy = service(&state).archive(&ctx, &request_id, &id).await?;
    Ok(Json(json!({ "policy": policy })))
}

/// Dry-run governance execution — no protocol mutation.
pub async fn dry_run_policy(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request_id: Option<axum::Extension<RequestId>>,
) -> Result<Json<serde_json::Value>> {
    let request_id = guard_mutation(
        &state,
        &ctx,
        &headers,
        request_id.as_ref().map(|e| &e.0),
        false,
    )?;

    let policy = service(&state).get(&ctx, &id).await?;
    let executor = crate::execution::ExecutionExecutor::new(
        state.db.pool(),
        state.protocol.as_ref(),
        state.signing.as_ref(),
    );
    let report = executor.dry_run(&ctx, &request_id, &policy).await?;

    Ok(Json(json!({
        "report": report,
        "protocol_mutated": false,
        "apply_enabled": false,
        "note": "Dry-run validates the governance pipeline without modifying protocol state.",
    })))
}

/// Issue a fresh CSRF synchronizer token for authenticated operators.
pub async fn issue_csrf(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
) -> Result<axum::response::Response> {
    use axum::response::IntoResponse;
    use axum_extra::extract::cookie::CookieJar;

    let token = state.csrf_store.issue(&ctx.operator_id)?;
    let jar = CookieJar::new().add(csrf_cookie(token.clone(), state.config.secure_cookies));
    Ok((
        jar,
        Json(json!({
            "csrf_token": token,
            "header": "X-CSRF-Token",
        })),
    )
        .into_response())
}

/// Helper for tests / login path.
pub fn issue_csrf_for(store: &CsrfStore, operator_id: &str) -> Result<String> {
    store.issue(operator_id)
}
