//! Control Plane ↔ aether-treasury observation + write façade.
//!
//! Phase 19: read-only observation.
//! Phase 21: ledger governance writes via `write` (no Apply, no rails, no protocol).

pub mod adapter;
pub mod errors;
pub mod models;
pub mod ops;
pub mod queries;
pub mod write;

pub use adapter::TreasuryAdapter;
pub use errors::TreasuryCpError;
pub use write::TreasuryWriteService;
