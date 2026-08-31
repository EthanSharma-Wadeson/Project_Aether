use thiserror::Error;

pub type Result<T> = std::result::Result<T, TreasuryError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TreasuryError {
    #[error("negative amount rejected")]
    NegativeAmount,
    #[error("negative balance rejected for account {account}")]
    NegativeBalance { account: String },
    #[error("asset mismatch: expected {expected}, got {got}")]
    AssetMismatch { expected: String, got: String },
    #[error("unsupported or disabled asset: {0}")]
    UnsupportedAsset(String),
    #[error("treasury closed: {0}")]
    TreasuryClosed(String),
    #[error("treasury frozen: {0}")]
    TreasuryFrozen(String),
    #[error("treasury not found: {0}")]
    TreasuryNotFound(String),
    #[error("allocation not found: {0}")]
    AllocationNotFound(String),
    #[error("allocation expired: {0}")]
    AllocationExpired(String),
    #[error("allocation not active: {0}")]
    AllocationNotActive(String),
    #[error("insufficient available funds")]
    InsufficientFunds,
    #[error("insufficient allocation remaining")]
    InsufficientAllocation,
    #[error("duplicate reservation / idempotency key: {0}")]
    DuplicateIdempotency(String),
    #[error("reservation not found: {0}")]
    ReservationNotFound(String),
    #[error("reservation not active: {0}")]
    ReservationNotActive(String),
    #[error("journal batch unbalanced")]
    UnbalancedBatch,
    #[error("immutability violation: journal entries cannot be updated or deleted")]
    ImmutabilityViolation,
    #[error("invalid hierarchy: {0}")]
    InvalidHierarchy(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("database error: {0}")]
    Db(String),
}

impl From<sqlx::Error> for TreasuryError {
    fn from(value: sqlx::Error) -> Self {
        // Unique constraint → duplicate idempotency
        let msg = value.to_string();
        if msg.contains("UNIQUE") || msg.contains("unique") {
            return TreasuryError::DuplicateIdempotency(msg);
        }
        TreasuryError::Db(msg)
    }
}
