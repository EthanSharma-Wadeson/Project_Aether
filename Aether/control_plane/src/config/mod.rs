//! Control Plane configuration.

mod production;

pub use production::{
    is_production_mode, production_config_check, production_config_check_enforced,
};

use std::path::PathBuf;

use crate::security::origin::parse_allowed_origins;

#[derive(Clone, Debug)]
pub struct SignerConfig {
    /// `enterprise` (default) — server-held Ed25519 behind the Signer trait.
    pub mode: String,
    /// Audit / identity label (never a private key).
    pub identity: String,
    /// Optional 32-byte seed as hex for deterministic keys (tests/dev).
    pub seed_hex: Option<String>,
    /// If true, generate a fresh ephemeral key (not recommended for production).
    pub ephemeral: bool,
}

impl Default for SignerConfig {
    fn default() -> Self {
        Self {
            mode: "enterprise".into(),
            identity: "enterprise-default".into(),
            seed_hex: None,
            ephemeral: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub db_path: String,
    pub jwt_secret: String,
    pub access_token_ttl_secs: i64,
    pub refresh_token_ttl_secs: i64,
    pub frontend_dir: Option<PathBuf>,
    pub log_level: String,
    pub bootstrap_demo: bool,
    /// Exact Origin allowlist for mutating requests. Empty = localhost-only + missing Origin.
    pub allowed_origins: Vec<String>,
    pub signer: SignerConfig,
    /// When true, auth/CSRF cookies are marked `Secure` (HTTPS-only).
    pub secure_cookies: bool,
    /// Path to Treasury SQLite DB (separate from CP). Empty → derive from `db_path`.
    pub treasury_db_path: String,
    /// Organisation tenancy bound to this Control Plane deployment.
    pub treasury_organisation_id: String,
    /// Seed demo treasury hierarchy when empty (lab only).
    pub treasury_bootstrap_demo: bool,
    /// Default reservation TTL seconds when omitted (Phase 22.5).
    pub treasury_reservation_default_ttl_secs: i64,
    /// Max reservation TTL from now.
    pub treasury_reservation_max_ttl_secs: i64,
    /// Orphan active reservation age (no expires_at / legacy).
    pub treasury_reservation_orphan_age_secs: i64,
    /// Background sweeper interval (0 = disabled loop; manual sweep still ok).
    pub treasury_sweeper_interval_secs: u64,
    /// Enable background reservation sweeper on startup.
    pub treasury_sweeper_enabled: bool,
    /// Mutations left in `executing` longer than this are reconciled to failed.
    pub treasury_mutation_executing_timeout_secs: i64,
    /// When true, admin treasury mutations require MFA verified signal (pilot boundary).
    pub admin_mfa_required: bool,
}

impl Config {
    pub fn from_env() -> crate::error::Result<Self> {
        Ok(Self {
            host: std::env::var("CP_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: std::env::var("CP_PORT")
                .unwrap_or_else(|_| "3001".into())
                .parse()
                .map_err(|e| crate::error::Error::Config(format!("CP_PORT: {e}")))?,
            db_path: std::env::var("CP_DB_PATH").unwrap_or_else(|_| "./control_plane.db".into()),
            jwt_secret: std::env::var("CP_JWT_SECRET")
                .map_err(|_| crate::error::Error::Config("CP_JWT_SECRET is required".into()))?,
            access_token_ttl_secs: 900,
            refresh_token_ttl_secs: 604_800,
            frontend_dir: std::env::var("CP_FRONTEND_DIR").ok().map(PathBuf::from),
            log_level: std::env::var("CP_LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            bootstrap_demo: std::env::var("CP_BOOTSTRAP_DEMO")
                .map(|v| v != "0" && v != "false")
                .unwrap_or(true),
            allowed_origins: std::env::var("CP_ALLOWED_ORIGINS")
                .map(|s| parse_allowed_origins(&s))
                .unwrap_or_default(),
            signer: SignerConfig {
                mode: std::env::var("CP_SIGNER_MODE").unwrap_or_else(|_| "enterprise".into()),
                identity: std::env::var("CP_SIGNER_IDENTITY")
                    .unwrap_or_else(|_| "enterprise-default".into()),
                seed_hex: std::env::var("CP_SIGNER_SEED_HEX").ok(),
                ephemeral: std::env::var("CP_SIGNER_EPHEMERAL")
                    .map(|v| v == "1" || v == "true")
                    .unwrap_or(false),
            },
            secure_cookies: std::env::var("CP_SECURE_COOKIES")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            treasury_db_path: std::env::var("CP_TREASURY_DB_PATH").unwrap_or_default(),
            treasury_organisation_id: std::env::var("CP_TREASURY_ORG_ID")
                .unwrap_or_else(|_| "org-default".into()),
            treasury_bootstrap_demo: std::env::var("CP_TREASURY_BOOTSTRAP_DEMO")
                .map(|v| v != "0" && v != "false")
                .unwrap_or(true),
            treasury_reservation_default_ttl_secs: std::env::var("CP_TW_RESERVATION_DEFAULT_TTL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(900),
            treasury_reservation_max_ttl_secs: std::env::var("CP_TW_RESERVATION_MAX_TTL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86_400),
            treasury_reservation_orphan_age_secs: std::env::var("CP_TW_RESERVATION_ORPHAN_AGE_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86_400),
            treasury_sweeper_interval_secs: std::env::var("CP_TW_SWEEPER_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            treasury_sweeper_enabled: std::env::var("CP_TW_SWEEPER_ENABLED")
                .map(|v| v != "0" && v != "false")
                .unwrap_or(true),
            treasury_mutation_executing_timeout_secs: std::env::var(
                "CP_TW_MUTATION_EXECUTING_TIMEOUT_SECS",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300),
            admin_mfa_required: std::env::var("CP_ADMIN_MFA_REQUIRED")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        })
    }
}
