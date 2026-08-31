use std::sync::Arc;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::jwt::{role_from_claims, verify_access_token};
use crate::db::operators::OperatorRole;
use crate::error::Error;
use crate::routes::AppState;

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub operator_id: String,
    pub username: String,
    pub role: OperatorRole,
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Error> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Error::Auth("missing authorization header".into()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| Error::Auth("invalid authorization scheme".into()))?;

    // Refresh tokens must not authorize API access (token type separation).
    if crate::auth::jwt::verify_refresh_token(&state.config.jwt_secret, token).is_ok() {
        return Err(Error::Auth("refresh token cannot access API".into()));
    }

    let claims = verify_access_token(&state.config.jwt_secret, token)
        .map_err(|e| Error::Auth(format!("invalid access token: {e}")))?;
    let role = role_from_claims(&claims).map_err(|e| Error::Auth(e.to_string()))?;

    req.extensions_mut().insert(AuthContext {
        operator_id: claims.sub,
        username: claims.username,
        role,
    });

    Ok(next.run(req).await)
}

pub fn require_admin(ctx: &AuthContext) -> Result<(), Error> {
    crate::auth::roles::require_admin(ctx)
}
