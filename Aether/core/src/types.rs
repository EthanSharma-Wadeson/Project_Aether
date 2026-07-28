//! Shared PROTO-0 types.

use crate::error::{Error, Result};

pub type AgentId = String;
pub type ActionSelector = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Active,
    Frozen,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectRef {
    AgentId(AgentId),
    PublicKey(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimit {
    pub max_ops: u64,
    pub window_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    pub action: ActionSelector,
    pub spend: Option<u64>,
    pub asset: Option<String>,
    pub counterparty: Option<AgentId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    InvalidSignature,
    SigningContextMismatch,
    MalformedObject,
    AgentIdMismatch,
    IdentityFrozen,
    IdentityRevoked,
    PermissionRootMismatch,
    InvalidRootAuthority,
    StaleRootVersion,
    NonMonotonicRootVersion,
    CapabilityNotFound,
    CapabilityExpired,
    CapabilityNotYetValid,
    CapabilityRevoked,
    ParentMissing,
    ParentInvalid,
    Escalation,
    ExcessiveDelegationDepth,
    InvalidDelegationDepth,
    ActionNotPermitted,
    SpendExceeded,
    AssetNotPermitted,
    CounterpartyNotPermitted,
    RateLimitExceeded,
    MissingCapability,
    IdentityOnlyBypass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authorisation {
    Authorised,
    Rejected(RejectReason),
}

impl Authorisation {
    pub fn is_authorised(&self) -> bool {
        matches!(self, Self::Authorised)
    }

    pub fn expect_rejected(self) -> Result<RejectReason> {
        match self {
            Self::Rejected(r) => Ok(r),
            Self::Authorised => Err(Error::UnexpectedAuthorisation),
        }
    }
}
