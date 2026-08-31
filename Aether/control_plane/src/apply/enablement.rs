//! Apply enablement posture — fail-closed configuration (Phase 11).
//!
//! Production Apply remains disabled. Environment variables alone cannot enable
//! live mutations. This module validates that posture and provides early guards.

use crate::apply::apply_enabled;
use crate::apply::execution::errors::{ExecutionPipelineError, ExecutionPipelineErrorCode};
use crate::apply::execution::model::{ApplyExecutionPhase, ExecuteApplyRequest};
use crate::config::Config;
use crate::error::{Error, Result};

/// Acknowledgement token required before any future enablement attempt is even considered.
pub const APPLY_ENABLEMENT_ACK_TOKEN: &str = "I_UNDERSTAND_APPLY_ENABLEMENT_RISKS";

/// Whether the process environment is requesting Apply enablement.
///
/// This does **not** enable mutations. [`crate::apply::apply_enabled`] remains the
/// sole runtime switch and stays `false` until a future authorised gate.
pub fn env_requests_apply_enabled() -> bool {
    match std::env::var("CP_APPLY_ENABLED") {
        Ok(v) => v == "1" || v.eq_ignore_ascii_case("true"),
        Err(_) => false,
    }
}

fn env_ack_present() -> bool {
    std::env::var("CP_APPLY_ENABLEMENT_ACK")
        .map(|v| v == APPLY_ENABLEMENT_ACK_TOKEN)
        .unwrap_or(false)
}

/// Fail-closed enablement checks for Config load / production gate.
///
/// Rules:
/// - Default is disabled.
/// - Production must never start with `CP_APPLY_ENABLED=true`.
/// - Non-production requesting enablement without the ack token is rejected
///   (prevents accidental enablement experiments).
/// - Even with ack, runtime `apply_enabled()` remains hard-false until a
///   future phase flips the implementation.
pub fn validate_apply_enablement_config(config: &Config) -> Result<()> {
    let _ = config; // reserved for future config.apply_enabled field wiring
    let requested = env_requests_apply_enabled();
    if !requested {
        return Ok(());
    }

    if crate::config::is_production_mode() {
        return Err(Error::Config(
            "CP_APPLY_ENABLED must not be true in production (Apply enablement blocked)".into(),
        ));
    }

    if !env_ack_present() {
        return Err(Error::Config(format!(
            "CP_APPLY_ENABLED=true requires CP_APPLY_ENABLEMENT_ACK={APPLY_ENABLEMENT_ACK_TOKEN}"
        )));
    }

    // Explicit request + ack in non-prod is recorded but still does not enable
    // runtime mutations — Phase 11 keeps apply_enabled() hard-false.
    tracing::warn!(
        "CP_APPLY_ENABLED requested with ack; runtime apply_enabled() remains false until final enablement gate"
    );
    Ok(())
}

/// C8 — early pipeline entry guard.
///
/// While Apply is disabled, orchestration may proceed for readiness testing
/// (mutations still impossible). When enabled, `confirm: true` is mandatory (C9).
pub fn guard_pipeline_entry(
    request: &ExecuteApplyRequest,
) -> std::result::Result<(), ExecutionPipelineError> {
    if apply_enabled() && !request.confirm {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::ConfirmRequired,
            "confirm: true is required when Apply is enabled",
            ApplyExecutionPhase::Created,
            None,
        ));
    }
    Ok(())
}

/// Runtime Apply remains disabled (Phase 11 invariant).
pub fn runtime_apply_is_disabled() -> bool {
    !apply_enabled()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SignerConfig;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn base_config() -> Config {
        Config {
            host: "127.0.0.1".into(),
            port: 3001,
            db_path: "./t.db".into(),
            jwt_secret: "a-sufficiently-long-test-jwt-secret-key!!!!".into(),
            access_token_ttl_secs: 900,
            refresh_token_ttl_secs: 604_800,
            frontend_dir: None,
            log_level: "info".into(),
            bootstrap_demo: true,
            allowed_origins: vec![],
            signer: SignerConfig::default(),
            secure_cookies: false,
            treasury_db_path: "./t.treasury.db".into(),
            treasury_organisation_id: "org-default".into(),
            treasury_bootstrap_demo: false,
        treasury_reservation_default_ttl_secs: 900,
                treasury_reservation_max_ttl_secs: 86_400,
                treasury_reservation_orphan_age_secs: 86_400,
                treasury_sweeper_interval_secs: 60,
                treasury_sweeper_enabled: false,
                treasury_mutation_executing_timeout_secs: 300,
                admin_mfa_required: false,
        }
    }

    #[test]
    fn default_enablement_is_disabled() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("CP_APPLY_ENABLED");
        std::env::remove_var("CP_APPLY_ENABLEMENT_ACK");
        assert!(!env_requests_apply_enabled());
        assert!(!apply_enabled());
        assert!(validate_apply_enablement_config(&base_config()).is_ok());
    }

    #[test]
    fn requested_without_ack_rejected() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CP_APPLY_ENABLED", "true");
        std::env::remove_var("CP_APPLY_ENABLEMENT_ACK");
        std::env::remove_var("CP_ENV");
        std::env::remove_var("CP_PRODUCTION");
        let err = validate_apply_enablement_config(&base_config()).unwrap_err();
        assert!(err.to_string().contains("CP_APPLY_ENABLEMENT_ACK"));
        std::env::remove_var("CP_APPLY_ENABLED");
    }

    #[test]
    fn production_rejects_apply_enabled_request() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CP_ENV", "production");
        std::env::set_var("CP_APPLY_ENABLED", "true");
        std::env::set_var("CP_APPLY_ENABLEMENT_ACK", APPLY_ENABLEMENT_ACK_TOKEN);
        let err = validate_apply_enablement_config(&base_config()).unwrap_err();
        assert!(err.to_string().contains("production"));
        std::env::remove_var("CP_ENV");
        std::env::remove_var("CP_APPLY_ENABLED");
        std::env::remove_var("CP_APPLY_ENABLEMENT_ACK");
    }
}
