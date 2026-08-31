//! Phase 34 — Lab-only external provider adapters (Claude/Anthropic).
//!
//! Production provider mode is forbidden. Keys resolve only via SecretRef → env.
//! Model output is proposal-only; Aether retains all authority.

pub mod anthropic;
pub mod demo;
pub mod intent_extract;
pub mod transport;

pub use anthropic::AnthropicLabProvider;
pub use demo::run_agent_a_external_demo;
pub use intent_extract::extract_proposed_intents;
pub use transport::{LabHttpTransport, ReqwestLabTransport, ScriptedLabTransport};
