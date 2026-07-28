//! PROTO-0 error types.

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
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
