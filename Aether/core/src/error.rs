//! PROTO-0 / PROTO-1 error types.

use crate::types::RejectReason;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidPublicKey,
    InvalidSignature,
    SigningContextMismatch,
    MalformedCbor,
    MalformedObject(&'static str),
    AgentIdMismatch,
    IdentityNotFound,
    IdentityAlreadyRegistered,
    IdentityNotActive,
    PermissionRootMismatch,
    InvalidRootAuthority,
    StaleRootVersion,
    NonMonotonicRootVersion,
    CapabilityNotFound,
    CapabilityRevoked,
    ParentMissing,
    Escalation(&'static str),
    ExcessiveDelegationDepth,
    InvalidDelegationDepth,
    UnexpectedAuthorisation,
    Rejected(RejectReason),
    // PROTO-1 channel
    ChannelNotFound,
    ChannelAlreadyExists,
    InvalidChannelStatus,
    UnilateralUpdate,
    SequenceStale,
    SequenceSkip,
    StateCommitmentMismatch,
    BalanceConservation,
    ParticipantMismatch,
    UnauthorizedTransition,
    InvalidClose,
    InvalidDisputeEvidence,
    CapabilityDenied,
    /// Cooperative finalize attempted before dispute window elapsed.
    DisputeWindowOpen,
    /// Terminal operation rejected (missing capability or wrong participant).
    UntrustedTerminalOperation,
    // PROTO-2 escrow
    EscrowNotFound,
    EscrowAlreadyExists,
    InvalidEscrowStatus,
    InsufficientBalance,
    EscrowValueConservation,
    InvalidReceipt,
    ReceiptReplay,
    InvalidEscrowTerms,
    FeeBudgetExceeded,
    InvalidEscrowEvidence,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
