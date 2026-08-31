use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolGatewayError {
    #[error("tool not found")]
    ToolNotFound,
    #[error("duplicate tool")]
    DuplicateTool,
    #[error("tool disabled")]
    ToolDisabled,
    #[error("agent not found")]
    AgentNotFound,
    #[error("agent frozen or not actable")]
    AgentNotActable,
    #[error("session not found")]
    SessionNotFound,
    #[error("session expired")]
    SessionExpired,
    #[error("session not usable")]
    SessionNotUsable,
    #[error("organisation mismatch")]
    OrganisationMismatch,
    #[error("replayed tool request")]
    RequestReplay,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] crate::error::Error),
}

impl ToolGatewayError {
    pub fn into_cp(self) -> crate::error::Error {
        use crate::error::Error;
        match self {
            Self::ToolNotFound | Self::AgentNotFound | Self::SessionNotFound => {
                Error::NotFound(self.to_string())
            }
            Self::ToolDisabled
            | Self::AgentNotActable
            | Self::SessionExpired
            | Self::SessionNotUsable
            | Self::OrganisationMismatch
            | Self::RequestReplay => Error::Forbidden(self.to_string()),
            Self::DuplicateTool | Self::BadRequest(_) => Error::BadRequest(self.to_string()),
            Self::Db(e) => Error::Database(e),
            Self::Other(e) => e,
        }
    }
}
