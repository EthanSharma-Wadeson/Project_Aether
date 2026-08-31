//! Admin MFA enforcement boundary (pilot requirement; not HSM/KMS).
//!
//! When `CP_ADMIN_MFA_REQUIRED=true`, admin treasury mutating routes require
//! either JWT claim `mfa_verified=true` (future IdP mapping) or header
//! `X-Aether-MFA-Verified: true` (lab/pilot bridge until IdP MFA is wired).
//!
//! Production roadmap: enforce via SSO/OIDC ACR values; never store TOTP secrets
//! in Control Plane. HSM/KMS remains a separate signing roadmap item.

use axum::http::HeaderMap;

use crate::auth::middleware::AuthContext;
use crate::db::operators::OperatorRole;
use crate::treasury::write::errors::TreasuryWriteError;

pub const MFA_HEADER: &str = "x-aether-mfa-verified";

pub fn enforce_admin_mfa_if_required(
    required: bool,
    ctx: &AuthContext,
    headers: &HeaderMap,
) -> Result<(), TreasuryWriteError> {
    if !required {
        return Ok(());
    }
    if ctx.role != OperatorRole::Admin {
        return Ok(());
    }
    let header_ok = headers
        .get(MFA_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    // JWT claim reserved for future IdP integration (claims JSON not on AuthContext today).
    if header_ok {
        return Ok(());
    }
    Err(TreasuryWriteError::Forbidden(
        "admin MFA required for treasury mutations (set X-Aether-MFA-Verified or disable CP_ADMIN_MFA_REQUIRED)".into(),
    ))
}
