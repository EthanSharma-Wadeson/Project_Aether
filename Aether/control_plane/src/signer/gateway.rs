//! Audited signing gateway — preferred call path for Phase 4+.
//!
//! Records SIGN_REQUEST_* events around the trait-level sign call.
//! Does not call PROTO-0.

use std::sync::Arc;

use crate::signer::audit::SignerAuditService;
use crate::signer::errors::SignerError;
use crate::signer::operation::{GovernanceOperation, SignedGovernanceOperation};
use crate::signer::traits::{SharedSigner, Signer};

/// Wraps a `Signer` with mandatory signing audit.
#[derive(Clone)]
pub struct SigningGateway {
    signer: SharedSigner,
    audit: SignerAuditService,
}

impl SigningGateway {
    pub fn new(signer: SharedSigner, audit: SignerAuditService) -> Self {
        Self { signer, audit }
    }

    pub fn signer_identity(&self) -> &str {
        self.signer.signer_identity()
    }

    pub fn inner(&self) -> &dyn Signer {
        self.signer.as_ref()
    }

    /// Sign with full audit trail (created → completed | failed).
    pub async fn sign_operation(
        &self,
        operation: &GovernanceOperation,
    ) -> Result<SignedGovernanceOperation, SignerError> {
        let identity = self.signer.signer_identity().to_string();
        self.audit.record_created(operation, &identity).await?;

        match self.signer.sign_operation(operation) {
            Ok(signed) => {
                self.audit.record_completed(operation, &identity).await?;
                Ok(signed)
            }
            Err(err) => {
                let _ = self
                    .audit
                    .record_failed(operation, &identity, &err.to_string())
                    .await;
                Err(err)
            }
        }
    }

    pub fn verify_signature(
        &self,
        operation: &GovernanceOperation,
        signature_hex: &str,
    ) -> Result<(), SignerError> {
        self.signer.verify_signature(operation, signature_hex)
    }

    /// Sign raw message bytes (Apply sign body). Does not call PROTO-0.
    pub fn sign_message(&self, message: &[u8]) -> Result<String, SignerError> {
        self.signer.sign_message(message)
    }

    /// Verify raw message bytes against a hex signature.
    pub fn verify_message(&self, message: &[u8], signature_hex: &str) -> Result<(), SignerError> {
        self.signer.verify_message(message, signature_hex)
    }
}

pub type SharedSigningGateway = Arc<SigningGateway>;
