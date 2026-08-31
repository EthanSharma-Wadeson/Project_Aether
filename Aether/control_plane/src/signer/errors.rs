//! Signer errors — never leak private key material.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignerError {
    #[error("signer configuration error: {0}")]
    Config(String),
    #[error("signing failed: {0}")]
    Sign(String),
    #[error("signature verification failed: {0}")]
    Verify(String),
    #[error("unsupported signer mode: {0}")]
    UnsupportedMode(String),
    #[error("audit error: {0}")]
    Audit(String),
}

impl From<SignerError> for crate::error::Error {
    fn from(value: SignerError) -> Self {
        match value {
            SignerError::Config(msg) | SignerError::UnsupportedMode(msg) => {
                crate::error::Error::Config(msg)
            }
            SignerError::Verify(msg) | SignerError::Sign(msg) => {
                crate::error::Error::Forbidden(format!("signer: {msg}"))
            }
            SignerError::Audit(msg) => crate::error::Error::Config(format!("signer audit: {msg}")),
        }
    }
}
