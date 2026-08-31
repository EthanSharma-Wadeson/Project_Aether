//! Signer trait — Control Plane business logic depends only on this.

use crate::signer::errors::SignerError;
use crate::signer::operation::{GovernanceOperation, SignedGovernanceOperation};

/// Cryptographic authenticator for already-authorized governance operations.
///
/// The signer authenticates; it is **not** the authority. PROTO-0 (Phase 4)
/// remains the final authority for protocol state transitions.
///
/// Implementations must never expose raw private key material through this
/// interface. Future backends (Vault, AWS KMS, Azure Key Vault, GCP KMS, HSM,
/// dedicated signing service) plug in behind the same trait.
pub trait Signer: Send + Sync {
    /// Stable identity string for audit (`enterprise-default`, `kms:alias/...`, …).
    fn signer_identity(&self) -> &str;

    /// Sign a governance operation. Does not call PROTO-0 or mutate state.
    fn sign_operation(
        &self,
        operation: &GovernanceOperation,
    ) -> Result<SignedGovernanceOperation, SignerError>;

    /// Verify a signature over a governance operation.
    fn verify_signature(
        &self,
        operation: &GovernanceOperation,
        signature_hex: &str,
    ) -> Result<(), SignerError>;

    /// Sign arbitrary message bytes (Apply sign body, etc.). Never exposes key material.
    fn sign_message(&self, message: &[u8]) -> Result<String, SignerError>;

    /// Verify a hex signature over arbitrary message bytes.
    fn verify_message(&self, message: &[u8], signature_hex: &str) -> Result<(), SignerError>;
}

/// Shared signer handle for AppState.
pub type SharedSigner = std::sync::Arc<dyn Signer>;
