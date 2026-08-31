//! Synchronizer CSRF token store — ready for future mutating routes.
//!
//! Complements refresh-cookie `SameSite=Strict` (see cookie helper below). Access
//! JWTs are Bearer headers, so CSRF + Origin remain required for write defence.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum_extra::extract::cookie::{Cookie, SameSite};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{Error, Result};

const DEFAULT_TTL: Duration = Duration::from_secs(2 * 60 * 60);

/// Cookie name for delivering the synchronizer token to the SPA (readable by JS).
pub const CSRF_COOKIE_NAME: &str = "cp_csrf_token";

#[derive(Debug, Clone)]
struct CsrfEntry {
    operator_id: String,
    expires_at: Instant,
}

/// In-memory synchronizer token store (swap for Redis later if needed).
#[derive(Debug, Default)]
pub struct CsrfStore {
    tokens: Mutex<HashMap<String, CsrfEntry>>,
}

pub type SharedCsrfStore = Arc<CsrfStore>;

impl CsrfStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shared() -> SharedCsrfStore {
        Arc::new(Self::new())
    }

    /// Issue a new CSRF token bound to an authenticated operator.
    pub fn issue(&self, operator_id: &str) -> Result<String> {
        let token = Uuid::new_v4().to_string();
        let mut guard = self
            .tokens
            .lock()
            .map_err(|_| Error::Auth("csrf store unavailable".into()))?;
        guard.insert(
            token.clone(),
            CsrfEntry {
                operator_id: operator_id.into(),
                expires_at: Instant::now() + DEFAULT_TTL,
            },
        );
        Ok(token)
    }

    /// Validate synchronizer token for the given operator.
    ///
    /// When `consume` is true (recommended for high-risk mutations), the token
    /// is invalidated after a successful check.
    pub fn validate(&self, operator_id: &str, token: &str, consume: bool) -> Result<()> {
        if token.is_empty() {
            return Err(Error::Csrf("missing csrf token".into()));
        }
        let mut guard = self
            .tokens
            .lock()
            .map_err(|_| Error::Auth("csrf store unavailable".into()))?;

        let Some(entry) = guard.get(token) else {
            return Err(Error::Csrf("invalid csrf token".into()));
        };
        if Instant::now() > entry.expires_at {
            guard.remove(token);
            return Err(Error::Csrf("expired csrf token".into()));
        }
        if entry.operator_id != operator_id {
            return Err(Error::Csrf("csrf token operator mismatch".into()));
        }
        if consume {
            guard.remove(token);
        }
        Ok(())
    }
}

/// Extract CSRF token from `X-CSRF-Token` header.
pub fn csrf_token_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Validate CSRF for a mutating request (foundation helper for Phase 2+).
pub fn validate_request_csrf(
    store: &CsrfStore,
    operator_id: &str,
    headers: &axum::http::HeaderMap,
    consume: bool,
) -> Result<()> {
    let token =
        csrf_token_from_headers(headers).ok_or_else(|| Error::Csrf("missing csrf token".into()))?;
    store.validate(operator_id, &token, consume)
}

/// Hash helper used when correlating CSRF-related audit metadata.
pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Build a CSRF delivery cookie compatible with refresh-cookie SameSite policy.
///
/// `HttpOnly=false` so the SPA can copy the value into `X-CSRF-Token`.
/// `SameSite=Strict` matches the refresh cookie posture (W3).
/// When `secure` is true, the cookie is HTTPS-only (`Secure` flag).
pub fn csrf_cookie(token: String, secure: bool) -> Cookie<'static> {
    let mut builder = Cookie::build((CSRF_COOKIE_NAME, token))
        .http_only(false)
        .same_site(SameSite::Strict)
        .path("/");
    if secure {
        builder = builder.secure(true);
    }
    builder.build()
}
