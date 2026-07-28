//! Aether v0 reference implementation — PROTO-0.
//!
//! Local deterministic identity + capability enforcement.
//! No networking, settlement, channels, or escrow.

pub mod capability;
pub mod cbor;
pub mod crypto;
pub mod error;
pub mod identity;
pub mod permission;
pub mod types;
pub mod verifier;

pub use error::{Error, Result};
pub use types::{
    ActionRequest, ActionSelector, AgentId, AgentStatus, Authorisation, RateLimit, RejectReason,
    SubjectRef,
};
pub use verifier::authorise::authorise_action;
