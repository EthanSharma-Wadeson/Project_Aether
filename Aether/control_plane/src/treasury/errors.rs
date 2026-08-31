use crate::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum TreasuryCpError {
    #[error("treasury not found")]
    NotFound,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("treasury engine: {0}")]
    Engine(String),
}

impl From<TreasuryCpError> for Error {
    fn from(value: TreasuryCpError) -> Self {
        match value {
            TreasuryCpError::NotFound => Error::NotFound("treasury not found".into()),
            TreasuryCpError::Forbidden(m) => Error::Forbidden(m),
            TreasuryCpError::Engine(m) => Error::BadRequest(m),
        }
    }
}

impl From<aether_treasury::TreasuryError> for TreasuryCpError {
    fn from(value: aether_treasury::TreasuryError) -> Self {
        match value {
            aether_treasury::TreasuryError::TreasuryNotFound(_) => TreasuryCpError::NotFound,
            other => TreasuryCpError::Engine(other.to_string()),
        }
    }
}
