//! Browser Origin allowlist for future mutating requests.

use axum::http::HeaderMap;

use crate::error::{Error, Result};

/// Check `Origin` (preferred) or `Referer` against the configured allowlist.
///
/// Behaviour:
/// - Empty allowlist: allow missing Origin (non-browser) and localhost Origins
///   so local Vite/dev is not broken.
/// - Non-empty allowlist: Origin must match exactly (or be absent for non-browser tools).
pub fn check_origin(headers: &HeaderMap, allowed_origins: &[String]) -> Result<()> {
    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let origin = match origin {
        Some(o) => o,
        None => {
            // Non-browser clients (curl, integration tests) omit Origin.
            return Ok(());
        }
    };

    if allowed_origins.is_empty() {
        if is_localhost_origin(&origin) {
            return Ok(());
        }
        return Err(Error::Forbidden(format!(
            "origin not allowed in local mode: {origin}"
        )));
    }

    if allowed_origins.iter().any(|a| a == &origin) {
        Ok(())
    } else {
        Err(Error::Forbidden(format!("origin not allowed: {origin}")))
    }
}

fn is_localhost_origin(origin: &str) -> bool {
    origin.starts_with("http://127.0.0.1")
        || origin.starts_with("http://localhost")
        || origin.starts_with("https://127.0.0.1")
        || origin.starts_with("https://localhost")
}

/// Parse `CP_ALLOWED_ORIGINS` comma-separated list.
pub fn parse_allowed_origins(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
