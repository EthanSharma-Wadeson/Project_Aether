//! Aether v0 reference implementation — PROTO-0 + PROTO-1 + PROTO-2.
//!
//! Local deterministic identity, capability enforcement, bilateral channel simulation,
//! and escrow/receipt settlement simulation. No networking or live settlement.

pub mod capability;
pub mod cbor;
pub mod channel;
pub mod crypto;
pub mod error;
pub mod escrow;
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
