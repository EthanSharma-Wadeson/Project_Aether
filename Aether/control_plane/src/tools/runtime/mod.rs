//! Lab tool registry + decision-only gateway.
//!
//! No provider APIs, no tool handlers, no Apply, no PROTO-0/treasury writes.

pub mod audit;
pub mod errors;
pub mod gateway;
pub mod models;
pub mod policy;
pub mod registry;

pub use gateway::ToolGatewayService;
pub use models::{
    CreateToolRequest, GatewayDecision, GatewayOutcome, RiskLevel, ToolEvaluateHttpRequest,
    ToolRecord, ToolRequest, ToolStatus,
};
