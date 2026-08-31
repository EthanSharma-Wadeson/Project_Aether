//! Signing boundary abstraction (Milestone 2 Phase 3).
//!
//! The signer authenticates an already-authorized governance operation.
//! It is NOT the authority and does NOT call PROTO-0.

pub mod audit;
pub mod enterprise_signer;
pub mod errors;
pub mod gateway;
pub mod operation;
pub mod traits;

pub use audit::{SignOutcome, SignerAuditRecord, SignerAuditService};
pub use enterprise_signer::EnterpriseSigner;
pub use errors::SignerError;
pub use gateway::{SharedSigningGateway, SigningGateway};
pub use operation::{GovernanceOperation, SignedGovernanceOperation};
pub use traits::{SharedSigner, Signer};

use crate::config::{Config, SignerConfig};
use crate::db::Db;

/// Build the configured signer + audited gateway. Never exposes private keys.
pub fn build_signing_gateway(
    config: &Config,
    db: &Db,
) -> Result<SharedSigningGateway, SignerError> {
    build_signing_gateway_with(&config.signer, db)
}

pub fn build_signing_gateway_with(
    signer_config: &SignerConfig,
    db: &Db,
) -> Result<SharedSigningGateway, SignerError> {
    let audit = SignerAuditService::new(db.pool().clone());
    let signer: SharedSigner = match signer_config.mode.as_str() {
        "enterprise" | "dev" | "" => EnterpriseSigner::shared(signer_config)?,
        other => {
            return Err(SignerError::UnsupportedMode(other.into()));
        }
    };
    Ok(std::sync::Arc::new(SigningGateway::new(signer, audit)))
}
