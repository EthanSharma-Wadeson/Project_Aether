//! Enterprise development signer — server-held Ed25519 identity.
//!
//! Key material stays inside this module. Business logic must never touch
//! `SigningKey`. Future KMS/HSM/Vault backends replace this type behind
//! the same `Signer` trait.

use std::sync::Arc;

use chrono::Utc;
use ed25519_dalek::{Signature, Signer as DalekSigner, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::config::SignerConfig;
use crate::signer::errors::SignerError;
use crate::signer::operation::{GovernanceOperation, SignedGovernanceOperation};
use crate::signer::traits::Signer;

/// Opaque key material — never exported beyond this module.
struct EnterpriseKeyMaterial {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl EnterpriseKeyMaterial {
    fn from_seed(seed: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    fn from_seed_hex(hex_seed: &str) -> Result<Self, SignerError> {
        let bytes = hex::decode(hex_seed.trim())
            .map_err(|e| SignerError::Config(format!("CP_SIGNER_SEED_HEX invalid hex: {e}")))?;
        if bytes.len() != 32 {
            return Err(SignerError::Config(
                "CP_SIGNER_SEED_HEX must be 32 bytes (64 hex chars)".into(),
            ));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        Ok(Self::from_seed(seed))
    }

    /// Derive a deterministic seed from identity string (dev only).
    fn from_identity_derivation(identity: &str) -> Self {
        let digest = Sha256::digest(format!("aether-cp-enterprise-signer:{identity}").as_bytes());
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&digest);
        Self::from_seed(seed)
    }
}

/// Server-held enterprise signer (MVP / development).
///
/// Configuration:
/// - `CP_SIGNER_MODE=enterprise`
/// - `CP_SIGNER_IDENTITY=enterprise-default`
/// - `CP_SIGNER_SEED_HEX` (optional) — deterministic key for tests/dev
///
/// If no seed is provided, a deterministic key is derived from the identity
/// string so restarts remain stable without requiring production key files.
pub struct EnterpriseSigner {
    identity: String,
    keys: EnterpriseKeyMaterial,
}

impl EnterpriseSigner {
    pub fn from_config(config: &SignerConfig) -> Result<Self, SignerError> {
        if config.mode != "enterprise" {
            return Err(SignerError::UnsupportedMode(config.mode.clone()));
        }
        let identity = if config.identity.trim().is_empty() {
            "enterprise-default".to_string()
        } else {
            config.identity.clone()
        };

        let keys = if let Some(seed_hex) = &config.seed_hex {
            EnterpriseKeyMaterial::from_seed_hex(seed_hex)?
        } else if config.ephemeral {
            EnterpriseKeyMaterial::generate()
        } else {
            // Stable dev default — not a production key custody model.
            EnterpriseKeyMaterial::from_identity_derivation(&identity)
        };

        Ok(Self { identity, keys })
    }

    pub fn shared(config: &SignerConfig) -> Result<Arc<Self>, SignerError> {
        Ok(Arc::new(Self::from_config(config)?))
    }

    /// Hex-encoded public key for diagnostics (never the private key).
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.keys.verifying_key.as_bytes())
    }
}

impl Signer for EnterpriseSigner {
    fn signer_identity(&self) -> &str {
        &self.identity
    }

    fn sign_operation(
        &self,
        operation: &GovernanceOperation,
    ) -> Result<SignedGovernanceOperation, SignerError> {
        if operation.action.trim().is_empty() {
            return Err(SignerError::Sign("action must not be empty".into()));
        }
        // Test-only hook: identity `__fail_sign__` forces signing failure (Phase 4A tests).
        if self.identity == "__fail_sign__" {
            return Err(SignerError::Sign("signer unavailable".into()));
        }
        let bytes = operation
            .canonical_bytes()
            .map_err(|e| SignerError::Sign(format!("canonical serialize: {e}")))?;
        let signature: Signature = self.keys.signing_key.sign(&bytes);
        Ok(SignedGovernanceOperation {
            operation: operation.clone(),
            signer_identity: self.identity.clone(),
            signature: hex::encode(signature.to_bytes()),
            signed_at: Utc::now(),
        })
    }

    fn verify_signature(
        &self,
        operation: &GovernanceOperation,
        signature_hex: &str,
    ) -> Result<(), SignerError> {
        let bytes = operation
            .canonical_bytes()
            .map_err(|e| SignerError::Verify(format!("canonical serialize: {e}")))?;
        self.verify_message(&bytes, signature_hex)
    }

    fn sign_message(&self, message: &[u8]) -> Result<String, SignerError> {
        if self.identity == "__fail_sign__" {
            return Err(SignerError::Sign("signer unavailable".into()));
        }
        let signature: Signature = self.keys.signing_key.sign(message);
        Ok(hex::encode(signature.to_bytes()))
    }

    fn verify_message(&self, message: &[u8], signature_hex: &str) -> Result<(), SignerError> {
        let sig_bytes = hex::decode(signature_hex.trim())
            .map_err(|e| SignerError::Verify(format!("invalid signature hex: {e}")))?;
        let signature = Signature::from_slice(&sig_bytes)
            .map_err(|e| SignerError::Verify(format!("invalid signature length: {e}")))?;
        self.keys
            .verifying_key
            .verify(message, &signature)
            .map_err(|_| SignerError::Verify("signature mismatch".into()))
    }
}
