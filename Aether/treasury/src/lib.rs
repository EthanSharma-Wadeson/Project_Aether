//! Aether Treasury Foundation — Phase 17.
//!
//! Internal organisation money-control domain: hierarchy, allocations,
//! reservations, and an append-only double-entry journal.
//!
//! **Non-goals (enforced by absence):** HTTP APIs, custody, payments, FX,
//! ERP connectors, Apply integration, `aether-core` / PROTO mutations.

#![deny(unsafe_code)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

pub mod db;
pub mod engine;
pub mod error;
pub mod models;
pub mod types;

pub use engine::TreasuryEngine;
pub use engine::{AssetBalance, ReservationOpsCounts, TreasurySecurityMetrics};
pub use engine::{
    DEFAULT_RESERVATION_TTL_SECS, MAX_RESERVATION_TTL_SECS, ORPHAN_RESERVATION_AGE_SECS,
};
pub use error::{Result, TreasuryError};
pub use models::*;
pub use types::*;
