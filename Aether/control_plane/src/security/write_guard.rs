//! Combined security checks for mutating Control Plane routes.

use axum::http::HeaderMap;
use uuid::Uuid;

use crate::auth::middleware::AuthContext;
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::routes::AppState;
use crate::security::csrf::validate_request_csrf;
use crate::security::origin::check_origin;

/// Enforce Origin + CSRF for a mutating request. Returns the request id.
pub fn guard_mutation(
    state: &AppState,
    ctx: &AuthContext,
    headers: &HeaderMap,
    request_id: Option<&RequestId>,
    consume_csrf: bool,
) -> Result<String> {
    check_origin(headers, &state.config.allowed_origins)?;
    validate_request_csrf(&state.csrf_store, &ctx.operator_id, headers, consume_csrf)?;
    Ok(request_id
        .map(|r| r.0.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string()))
}
