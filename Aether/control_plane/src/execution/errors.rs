//! Execution / dry-run errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("dry-run failed: {0}")]
    Failed(String),
}

impl From<ExecutionError> for crate::error::Error {
    fn from(value: ExecutionError) -> Self {
        match value {
            ExecutionError::Forbidden(msg) => crate::error::Error::Forbidden(msg),
            ExecutionError::BadRequest(msg) => crate::error::Error::BadRequest(msg),
            ExecutionError::NotFound(msg) => crate::error::Error::NotFound(msg),
            ExecutionError::Failed(msg) => crate::error::Error::BadRequest(msg),
        }
    }
}
