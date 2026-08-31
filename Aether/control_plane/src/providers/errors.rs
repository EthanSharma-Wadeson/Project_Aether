use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderAdapterError {
    #[error("unknown mock profile: {0}")]
    UnknownMockProfile(String),
    #[error("session governor denied: {0}")]
    GovernorDenied(String),
    #[error("injection / unsafe proposal rejected: {0}")]
    UnsafeProposal(String),
    #[error("no proposed intents from provider")]
    EmptyProposals,
    #[error("secret ref invalid: {0}")]
    InvalidSecretRef(String),
    #[error("external provider forbidden: {0}")]
    ExternalProviderForbidden(String),
    #[error("provider HTTP error: {0}")]
    ProviderHttp(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Runtime(#[from] crate::agents::runtime::errors::RuntimeAgentError),
    #[error(transparent)]
    Gateway(#[from] crate::tools::runtime::errors::ToolGatewayError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] crate::error::Error),
}

impl ProviderAdapterError {
    pub fn into_cp(self) -> crate::error::Error {
        use crate::error::Error;
        match self {
            Self::UnknownMockProfile(s)
            | Self::BadRequest(s)
            | Self::InvalidSecretRef(s)
            | Self::ProviderHttp(s) => Error::BadRequest(s),
            Self::GovernorDenied(s)
            | Self::UnsafeProposal(s)
            | Self::ExternalProviderForbidden(s) => Error::Forbidden(s),
            Self::EmptyProposals => Error::BadRequest(self.to_string()),
            Self::Runtime(e) => e.into_cp(),
            Self::Gateway(e) => e.into_cp(),
            Self::Db(e) => Error::Database(e),
            Self::Other(e) => e,
        }
    }
}
