//! Phase 28 — Lab agent registry & runtime identity (CP-local).
//!
//! No provider APIs, tool execution, Apply, PROTO-0 mutation, treasury writes,
//! or agent wallets.

pub mod audit;
pub mod errors;
pub mod models;
pub mod registry;
pub mod service;
pub mod sessions;

pub use models::{
    AgentProviderType, AgentStatus, CreateAgentRequest, CreateSessionRequest, L3Credential,
    RuntimeAgent, RuntimeEvaluateRequest, RuntimeSession, SessionStatus,
};
pub use service::AgentRuntimeService;
