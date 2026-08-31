use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::error::Error;

use super::models::LEDGER_NOTICE;

#[derive(Debug, thiserror::Error)]
pub enum TreasuryWriteError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Unauthorized(String),
}

#[derive(Serialize)]
struct Body {
    error: String,
    code: String,
    ledger_notice: &'static str,
}

impl TreasuryWriteError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "VALIDATION",
            Self::Forbidden(m)
                if m.to_lowercase().contains("csrf") =>
            {
                "CSRF_FAILED"
            }
            Self::Forbidden(m)
                if m.contains("Separation")
                    || m.contains("SoD")
                    || m.contains("own request")
                    || m.contains("self") =>
            {
                "APPROVAL_SOD"
            }
            Self::Forbidden(_) => "FORBIDDEN",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Conflict(m) if m.to_lowercase().contains("expir") => "APPROVAL_EXPIRED",
            Self::Conflict(m) if m.contains("hash") || m.contains("PAYLOAD") => "PAYLOAD_MISMATCH",
            Self::Conflict(_) => "CONFLICT",
            Self::Unavailable(_) => "UNAVAILABLE",
            Self::Unauthorized(_) => "UNAUTHORIZED",
        }
    }
}

impl IntoResponse for TreasuryWriteError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        };
        let code = self.code().to_string();
        (
            status,
            Json(Body {
                error: self.to_string(),
                code,
                ledger_notice: LEDGER_NOTICE,
            }),
        )
            .into_response()
    }
}

impl From<Error> for TreasuryWriteError {
    fn from(value: Error) -> Self {
        match value {
            Error::NotFound(m) => Self::NotFound(m),
            Error::Forbidden(m) => Self::Forbidden(m),
            Error::Csrf(m) => Self::Forbidden(format!("csrf: {m}")),
            Error::Auth(m) => Self::Unauthorized(m),
            Error::BadRequest(m) => Self::BadRequest(m),
            Error::Database(e) => Self::Unavailable(e.to_string()),
            other => Self::BadRequest(other.to_string()),
        }
    }
}

impl From<aether_treasury::TreasuryError> for TreasuryWriteError {
    fn from(value: aether_treasury::TreasuryError) -> Self {
        use aether_treasury::TreasuryError::*;
        match value {
            TreasuryNotFound(_) | AllocationNotFound(_) | ReservationNotFound(_) => {
                Self::NotFound(value.to_string())
            }
            TreasuryFrozen(_)
            | TreasuryClosed(_)
            | InsufficientFunds
            | InsufficientAllocation
            | AllocationExpired(_)
            | AllocationNotActive(_)
            | ReservationNotActive(_)
            | DuplicateIdempotency(_) => Self::Conflict(value.to_string()),
            Db(m) => Self::Unavailable(m),
            other => Self::BadRequest(other.to_string()),
        }
    }
}

impl From<crate::treasury::errors::TreasuryCpError> for TreasuryWriteError {
    fn from(value: crate::treasury::errors::TreasuryCpError) -> Self {
        match value {
            crate::treasury::errors::TreasuryCpError::NotFound => {
                Self::NotFound("treasury not found".into())
            }
            crate::treasury::errors::TreasuryCpError::Forbidden(m) => Self::Forbidden(m),
            crate::treasury::errors::TreasuryCpError::Engine(m) => Self::BadRequest(m),
        }
    }
}
