use thiserror::Error;

use super::models::DenyReason;

#[derive(Debug, Error)]
pub enum EnforcementError {
    #[error("invalid enforcement request: {0}")]
    InvalidRequest(String),
    #[error("treasury observation failed: {0}")]
    Treasury(String),
    #[error("policy load failed: {0}")]
    Policy(String),
    #[error(transparent)]
    Other(#[from] crate::error::Error),
}

impl EnforcementError {
    pub fn deny_reason(&self) -> Option<DenyReason> {
        match self {
            Self::InvalidRequest(_) => Some(DenyReason::InvalidRequest),
            _ => None,
        }
    }
}
