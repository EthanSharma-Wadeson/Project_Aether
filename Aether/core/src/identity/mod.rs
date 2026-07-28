//! Identity: AgentId derivation, identity objects, in-memory registry.

pub mod agent_id;
pub mod agent_identity;
pub mod registry;

pub use agent_id::derive_agent_id;
pub use agent_identity::{AgentIdentityV0, IdentityBundle};
pub use registry::IdentityRegistry;
