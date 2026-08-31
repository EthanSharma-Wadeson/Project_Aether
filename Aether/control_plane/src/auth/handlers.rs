use std::sync::Arc;

use axum::extract::ConnectInfo;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde_json::json;

use crate::auth::jwt::{issue_access_token, issue_refresh_token, verify_refresh_token};
use crate::auth::rate_limit::LoginRateLimitStore;
use crate::db::audit;
use crate::db::operators::{find_by_username, password_hash_for_username};
use crate::db::refresh_tokens::{self, RefreshLookup};
use crate::error::{Error, Result};
use crate::models::auth::{LoginRequest, LoginResponse, OperatorProfile};
use crate::routes::AppState;
use crate::security::csrf::csrf_cookie;

const REFRESH_COOKIE: &str = "cp_refresh_token";
const INVALID_CREDENTIALS: &str = "invalid credentials";

fn client_ip(headers: &HeaderMap, connect: Option<&ConnectInfo<std::net::SocketAddr>>) -> String {
    if let Some(xff) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return xff.to_string();
    }
    if let Some(real) = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return real.to_string();
    }
    connect
        .map(|c| c.0.ip().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn refresh_cookie(token: String, secure: bool) -> Cookie<'static> {
    let mut builder = Cookie::build((REFRESH_COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/auth");
    if secure {
        builder = builder.secure(true);
    }
    builder.build()
}

fn clear_refresh_cookie(secure: bool) -> Cookie<'static> {
    let mut cleared = Cookie::build((REFRESH_COOKIE, ""))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/auth");
    if secure {
        cleared = cleared.secure(true);
    }
    let mut cleared = cleared.build();
    cleared.make_removal();
    cleared
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    headers: HeaderMap,
    connect: Option<ConnectInfo<std::net::SocketAddr>>,
    Json(body): Json<LoginRequest>,
) -> Result<Response> {
    let ip = client_ip(&headers, connect.as_ref());
    let username = body.username.trim().to_string();

    state.rate_limiter.check_allowed(&ip, &username)?;

    // Always return the same error string — do not reveal username existence.
    let auth_failed = || Error::Auth(INVALID_CREDENTIALS.into());

    let hash = match password_hash_for_username(state.db.pool(), &username).await? {
        Some(h) => h,
        None => {
            state.rate_limiter.record_failure(&ip, &username);
            audit::append(
                state.db.pool(),
                None,
                "auth.login.failed",
                Some(&username),
                Some(json!({ "reason": "unknown_user", "ip": ip })),
            )
            .await?;
            return Err(auth_failed());
        }
    };

    let valid = bcrypt::verify(&body.password, &hash).map_err(|e| Error::Auth(e.to_string()))?;
    if !valid {
        state.rate_limiter.record_failure(&ip, &username);
        audit::append(
            state.db.pool(),
            None,
            "auth.login.failed",
            Some(&username),
            Some(json!({ "reason": "bad_password", "ip": ip })),
        )
        .await?;
        return Err(auth_failed());
    }

    let operator = find_by_username(state.db.pool(), &username)
        .await?
        .ok_or_else(auth_failed)?;

    state.rate_limiter.reset(&ip, &username);

    let access_token = issue_access_token(
        &state.config.jwt_secret,
        &operator.id,
        &operator.username,
        operator.role,
        state.config.access_token_ttl_secs,
    )?;

    let refresh_token = issue_refresh_token(
        &state.config.jwt_secret,
        &operator.id,
        &operator.username,
        operator.role,
        state.config.refresh_token_ttl_secs,
    )?;
    refresh_tokens::store_refresh_token(
        state.db.pool(),
        &operator.id,
        &refresh_token,
        state.config.refresh_token_ttl_secs,
    )
    .await?;

    audit::append(
        state.db.pool(),
        Some(&operator.id),
        "auth.login.success",
        Some(&operator.username),
        Some(json!({ "ip": ip })),
    )
    .await?;

    let csrf_token = state.csrf_store.issue(&operator.id)?;

    let body = LoginResponse {
        access_token,
        token_type: "Bearer".into(),
        expires_in: state.config.access_token_ttl_secs,
        role: operator.role.as_str().into(),
        csrf_token: csrf_token.clone(),
    };

    let secure = state.config.secure_cookies;
    Ok((
        jar.add(refresh_cookie(refresh_token, secure))
            .add(csrf_cookie(csrf_token, secure)),
        Json(body),
    )
        .into_response())
}

pub async fn refresh(State(state): State<Arc<AppState>>, jar: CookieJar) -> Result<Response> {
    let token = jar
        .get(REFRESH_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| Error::Auth("missing refresh token".into()))?;

    // Reject access tokens presented as refresh cookies (type separation).
    if crate::auth::jwt::verify_access_token(&state.config.jwt_secret, &token).is_ok() {
        audit::append(
            state.db.pool(),
            None,
            "auth.refresh.rejected_access_token",
            None,
            None,
        )
        .await?;
        return Err(Error::Auth("invalid refresh token".into()));
    }

    let claims = verify_refresh_token(&state.config.jwt_secret, &token)
        .map_err(|_| Error::Auth("invalid refresh token".into()))?;

    match refresh_tokens::lookup_refresh_token(state.db.pool(), &token).await? {
        RefreshLookup::Reused { operator_id } => {
            let revoked =
                refresh_tokens::revoke_all_for_operator(state.db.pool(), &operator_id).await?;
            audit::append(
                state.db.pool(),
                Some(&operator_id),
                "auth.refresh.reuse_detected",
                Some(&claims.username),
                Some(json!({ "revoked_tokens": revoked })),
            )
            .await?;
            return Err(Error::Auth("refresh token reuse detected".into()));
        }
        RefreshLookup::Invalid => {
            audit::append(
                state.db.pool(),
                Some(&claims.sub),
                "auth.refresh.invalid",
                Some(&claims.username),
                None,
            )
            .await?;
            return Err(Error::Auth("invalid refresh token".into()));
        }
        RefreshLookup::Valid { operator_id } => {
            if operator_id != claims.sub {
                return Err(Error::Auth("invalid refresh token".into()));
            }
        }
    }

    let operator = crate::db::operators::find_by_id(state.db.pool(), &claims.sub)
        .await?
        .ok_or_else(|| Error::Auth("invalid refresh token".into()))?;

    let access_token = issue_access_token(
        &state.config.jwt_secret,
        &operator.id,
        &operator.username,
        operator.role,
        state.config.access_token_ttl_secs,
    )?;

    let new_refresh = issue_refresh_token(
        &state.config.jwt_secret,
        &operator.id,
        &operator.username,
        operator.role,
        state.config.refresh_token_ttl_secs,
    )?;

    refresh_tokens::rotate_refresh_token(
        state.db.pool(),
        &operator.id,
        &token,
        &new_refresh,
        state.config.refresh_token_ttl_secs,
    )
    .await?;

    audit::append(
        state.db.pool(),
        Some(&operator.id),
        "auth.refresh.rotated",
        Some(&operator.username),
        None,
    )
    .await?;

    let csrf_token = state.csrf_store.issue(&operator.id)?;

    let body = LoginResponse {
        access_token,
        token_type: "Bearer".into(),
        expires_in: state.config.access_token_ttl_secs,
        role: operator.role.as_str().into(),
        csrf_token: csrf_token.clone(),
    };

    let secure = state.config.secure_cookies;
    Ok((
        jar.add(refresh_cookie(new_refresh, secure))
            .add(csrf_cookie(csrf_token, secure)),
        Json(body),
    )
        .into_response())
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse> {
    if let Some(cookie) = jar.get(REFRESH_COOKIE) {
        refresh_tokens::revoke_refresh_token(state.db.pool(), cookie.value()).await?;
        audit::append(state.db.pool(), None, "auth.logout", None, None).await?;
    }
    Ok((
        jar.remove(clear_refresh_cookie(state.config.secure_cookies)),
        StatusCode::NO_CONTENT,
    ))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    ctx: axum::Extension<crate::auth::middleware::AuthContext>,
) -> Result<Json<OperatorProfile>> {
    let operator = crate::db::operators::find_by_id(state.db.pool(), &ctx.operator_id)
        .await?
        .ok_or_else(|| Error::NotFound("operator".into()))?;
    Ok(Json(OperatorProfile {
        id: operator.id,
        username: operator.username,
        role: operator.role.as_str().into(),
    }))
}
