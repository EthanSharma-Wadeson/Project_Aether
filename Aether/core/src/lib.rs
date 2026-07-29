//! Aether v0 reference implementation — PROTO-0 + PROTO-1 + PROTO-2 + PROTO-NET-0 + PROTO-4 + PROTO-3.
//!
//! Local deterministic identity, capability enforcement, bilateral channel simulation,
//! escrow/receipt settlement simulation, authenticated transport simulation,
//! settlement-binding to a mock external ledger, and evidence-based reputation indexer.
//! No live networking or payment rails.

pub mod capability;
pub mod cbor;
pub mod channel;
pub mod crypto;
pub mod error;
pub mod escrow;
pub mod identity;
pub mod network;
pub mod permission;
pub mod reputation;
pub mod settlement;
pub mod types;
pub mod verifier;

pub use error::{Error, Result};
pub use types::{
    ActionRequest, ActionSelector, AgentId, AgentStatus, Authorisation, RateLimit, RejectReason,
    SubjectRef,
};
pub use verifier::authorise::authorise_action;
