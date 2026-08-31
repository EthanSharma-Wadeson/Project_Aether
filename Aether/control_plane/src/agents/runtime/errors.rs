use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeAgentError {
    #[error("agent not found")]
    AgentNotFound,
    #[error("session not found")]
    SessionNotFound,
    #[error("duplicate agent")]
    DuplicateAgent,
    #[error("organisation mismatch")]
    OrganisationMismatch,
    #[error("organisation is frozen")]
    OrganisationFrozen,
    #[error("agent cannot create sessions (status={0})")]
    AgentNotSessionable(String),
    #[error("agent cannot act (status={0})")]
    AgentNotActable(String),
    #[error("session not usable (status={0})")]
    SessionNotUsable(String),
    #[error("session expired")]
    SessionExpired,
    #[error("invalid credential: {0}")]
    InvalidCredential(String),
    #[error("credential replay detected")]
    CredentialReplay,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] crate::error::Error),
}

impl RuntimeAgentError {
    pub fn into_cp(self) -> crate::error::Error {
        use crate::error::Error;
        match self {
            Self::AgentNotFound | Self::SessionNotFound => Error::NotFound(self.to_string()),
            Self::OrganisationMismatch
            | Self::OrganisationFrozen
            | Self::AgentNotSessionable(_)
            | Self::AgentNotActable(_)
            | Self::SessionNotUsable(_)
            | Self::SessionExpired
            | Self::InvalidCredential(_)
            | Self::CredentialReplay => Error::Forbidden(self.to_string()),
            Self::DuplicateAgent | Self::BadRequest(_) => Error::BadRequest(self.to_string()),
            Self::Db(e) => Error::Database(e),
            Self::Other(e) => e,
        }
    }
}
