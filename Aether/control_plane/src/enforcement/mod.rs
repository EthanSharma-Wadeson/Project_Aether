//! Phase 26 — Runtime spend enforcement (E2 decision engine).
//!
//! Fail-closed ALLOW / DENY / REQUIRES_REVIEW. Read-only treasury + PROTO-0
//! observation. No Apply, no capability mutation, no treasury writes, no E4.

pub mod audit;
pub mod decision;
pub mod errors;
pub mod evaluator;
pub mod models;
pub mod rules;
pub mod service;

pub use models::{
    DenyReason, EnforcementDecision, EnforcementOutcome, EnforcementRequest, RiskResult,
};
pub use service::EnforcementService;
