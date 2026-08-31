//! Phase 31 — Agent runtime sandbox (simulated governance loop).
//!
//! No external models, no real tool execution, no Apply, no PROTO-0/treasury writes.

pub mod audit;
pub mod executor;
pub mod models;
pub mod planner;
pub mod runtime;

pub use models::{SandboxAgent, SandboxRunResult, SimulatedResult};
pub use runtime::SandboxRuntime;
