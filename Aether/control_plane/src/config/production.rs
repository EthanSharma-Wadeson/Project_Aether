//! Production deployment gate — fail closed on unsafe defaults.

use crate::config::Config;
use crate::error::{Error, Result};

const DEFAULT_PASSWORDS: &[&str] = &["admin", "viewer", "operator", "password", "changeme"];
const WEAK_JWT_SECRETS: &[&str] = &[
    "test-secret-key",
    "secret",
    "changeme",
    "jwt-secret",
    "aether",
];

/// Returns true when this process must enforce production safety checks.
pub fn is_production_mode() -> bool {
    match std::env::var("CP_ENV") {
        Ok(v) => {
            let v = v.to_ascii_lowercase();
            v == "production" || v == "prod"
        }
        Err(_) => std::env::var("CP_PRODUCTION")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
    }
}

/// Fail-closed validation for production (and optional forced checks).
///
/// Call after `Config::from_env()`. Does not enable Apply or load production keys
/// into business logic — it only refuses unsafe deployment posture.
pub fn production_config_check(config: &Config) -> Result<()> {
    production_config_check_enforced(config, is_production_mode() || force_checks())
}

/// Same checks as [`production_config_check`], with an explicit enforce flag (for tests).
pub fn production_config_check_enforced(config: &Config, enforce: bool) -> Result<()> {
    if !enforce {
        return Ok(());
    }

    let mut failures: Vec<String> = Vec::new();

    // --- JWT ---
    if config.jwt_secret.len() < 32 {
        failures.push("CP_JWT_SECRET must be at least 32 characters in production".into());
    }
    if WEAK_JWT_SECRETS
        .iter()
        .any(|w| config.jwt_secret.eq_ignore_ascii_case(w))
    {
        failures.push("CP_JWT_SECRET must not use a known weak/default value".into());
    }

    // --- Origins ---
    if config.allowed_origins.is_empty() {
        failures.push("CP_ALLOWED_ORIGINS must be set explicitly in production".into());
    }
    for origin in &config.allowed_origins {
        if origin.contains('*') {
            failures.push(format!("wildcard origin not allowed: {origin}"));
        }
        if origin.starts_with("http://") && !is_loopback_http(origin) {
            failures.push(format!(
                "non-localhost http:// origin not allowed in production: {origin}"
            ));
        }
    }

    // --- Passwords (seed operators) ---
    check_password_env("CP_ADMIN_PASSWORD", &mut failures);
    check_password_env("CP_VIEWER_PASSWORD", &mut failures);
    check_password_env("CP_OPERATOR_PASSWORD", &mut failures);

    // --- Signer ---
    if config.signer.ephemeral {
        failures.push("CP_SIGNER_EPHEMERAL must be false in production".into());
    }
    match &config.signer.seed_hex {
        None => failures.push(
            "CP_SIGNER_SEED_HEX is required in production (identity-derived keys forbidden)".into(),
        ),
        Some(seed) => {
            if hex::decode(seed.trim()).map(|b| b.len()).unwrap_or(0) != 32 {
                failures.push("CP_SIGNER_SEED_HEX must be 32 bytes (64 hex chars)".into());
            }
        }
    }
    if config.signer.mode.trim().is_empty() {
        failures.push("CP_SIGNER_MODE must be set".into());
    }
    if config.signer.identity.trim().is_empty() || config.signer.identity == "enterprise-default" {
        // Identity label alone is not enough — seed is the hard requirement.
        // Flag default identity as a warning-level failure for explicitness.
        failures.push(
            "CP_SIGNER_IDENTITY must be set to a non-default production identity label".into(),
        );
    }

    // --- Cookies ---
    if !config.secure_cookies {
        failures.push(
            "CP_SECURE_COOKIES must be true (1/true) in production (HTTPS-only cookies)".into(),
        );
    }

    // --- Demo bootstrap ---
    if config.bootstrap_demo {
        failures
            .push("CP_BOOTSTRAP_DEMO must be false in production (set CP_BOOTSTRAP_DEMO=0)".into());
    }

    // --- Apply enablement (C8) — production must never request Apply ---
    if crate::apply::enablement::env_requests_apply_enabled() {
        failures.push(
            "CP_APPLY_ENABLED must not be true in production (Apply remains disabled)".into(),
        );
    }
    // Runtime switch must remain false regardless of env.
    if crate::apply::apply_enabled() {
        failures.push("apply_enabled() must be false in production".into());
    }

    if failures.is_empty() {
        tracing::info!("production_config_check passed");
        Ok(())
    } else {
        Err(Error::Config(format!(
            "production_config_check failed (fail closed):\n  - {}",
            failures.join("\n  - ")
        )))
    }
}

fn force_checks() -> bool {
    std::env::var("CP_FORCE_PRODUCTION_CHECKS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn check_password_env(var: &str, failures: &mut Vec<String>) {
    match std::env::var(var) {
        Err(_) => failures.push(format!("{var} must be set explicitly in production")),
        Ok(pw) => {
            if pw.trim().is_empty() {
                failures.push(format!("{var} must not be empty"));
            }
            if DEFAULT_PASSWORDS.iter().any(|d| pw.eq_ignore_ascii_case(d)) {
                failures.push(format!("{var} must not use a default password"));
            }
            if pw.len() < 12 {
                failures.push(format!("{var} must be at least 12 characters"));
            }
        }
    }
}

fn is_loopback_http(origin: &str) -> bool {
    origin.starts_with("http://127.0.0.1") || origin.starts_with("http://localhost")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SignerConfig;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn base_config() -> Config {
        Config {
            host: "0.0.0.0".into(),
            port: 443,
            db_path: "./prod.db".into(),
            jwt_secret: "a-sufficiently-long-production-jwt-secret-key!!".into(),
            access_token_ttl_secs: 900,
            refresh_token_ttl_secs: 604_800,
            frontend_dir: None,
            log_level: "info".into(),
            bootstrap_demo: false,
            allowed_origins: vec!["https://control.example.com".into()],
            signer: SignerConfig {
                mode: "enterprise".into(),
                identity: "prod-enterprise-1".into(),
                seed_hex: Some(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
                ),
                ephemeral: false,
            },
            secure_cookies: true,
            treasury_db_path: "./prod.treasury.db".into(),
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

    fn with_prod_passwords<F: FnOnce() -> T, T>(admin: &str, f: F) -> T {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CP_ADMIN_PASSWORD", admin);
        std::env::set_var("CP_VIEWER_PASSWORD", "complex-viewer-pass-99");
        std::env::set_var("CP_OPERATOR_PASSWORD", "complex-operator-pass-99");
        let out = f();
        std::env::remove_var("CP_ADMIN_PASSWORD");
        std::env::remove_var("CP_VIEWER_PASSWORD");
        std::env::remove_var("CP_OPERATOR_PASSWORD");
        out
    }

    #[test]
    fn production_check_passes_with_safe_config() {
        with_prod_passwords("complex-admin-pass-99", || {
            assert!(production_config_check_enforced(&base_config(), true).is_ok());
        });
    }

    #[test]
    fn production_check_rejects_default_password() {
        with_prod_passwords("admin", || {
            let err = production_config_check_enforced(&base_config(), true).unwrap_err();
            assert!(err.to_string().contains("CP_ADMIN_PASSWORD"));
        });
    }

    #[test]
    fn production_check_rejects_missing_origins() {
        with_prod_passwords("complex-admin-pass-99", || {
            let mut cfg = base_config();
            cfg.allowed_origins.clear();
            let err = production_config_check_enforced(&cfg, true).unwrap_err();
            assert!(err.to_string().contains("CP_ALLOWED_ORIGINS"));
        });
    }

    #[test]
    fn production_check_rejects_missing_signer_seed() {
        with_prod_passwords("complex-admin-pass-99", || {
            let mut cfg = base_config();
            cfg.signer.seed_hex = None;
            let err = production_config_check_enforced(&cfg, true).unwrap_err();
            assert!(err.to_string().contains("CP_SIGNER_SEED_HEX"));
        });
    }

    #[test]
    fn production_check_rejects_insecure_cookies() {
        with_prod_passwords("complex-admin-pass-99", || {
            let mut cfg = base_config();
            cfg.secure_cookies = false;
            let err = production_config_check_enforced(&cfg, true).unwrap_err();
            assert!(err.to_string().contains("CP_SECURE_COOKIES"));
        });
    }

    #[test]
    fn non_production_skips_checks() {
        let mut cfg = base_config();
        cfg.secure_cookies = false;
        cfg.allowed_origins.clear();
        assert!(production_config_check_enforced(&cfg, false).is_ok());
    }

    #[test]
    fn production_check_rejects_apply_enabled_env() {
        with_prod_passwords("complex-admin-pass-99", || {
            std::env::set_var("CP_APPLY_ENABLED", "true");
            let err = production_config_check_enforced(&base_config(), true).unwrap_err();
            assert!(err.to_string().contains("CP_APPLY_ENABLED"));
            std::env::remove_var("CP_APPLY_ENABLED");
        });
    }
}
