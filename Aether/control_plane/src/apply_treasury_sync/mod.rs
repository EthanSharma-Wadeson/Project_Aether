//! Phase 24 — Apply ↔ Treasury sync **observation** (read-only drift reporting).
//!
//! No enforcement, no Apply execution, no PROTO-0 mutation, no treasury writes,
//! no automatic repair.

pub mod audit;
pub mod compare;
pub mod models;
pub mod queries;
pub mod service;

pub use models::{DriftFinding, DriftKind, SyncObservationReport, SyncStatus};
pub use service::SyncObservationService;
