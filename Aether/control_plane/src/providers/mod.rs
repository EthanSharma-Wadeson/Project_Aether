//! Phase 32 — Lab-only provider adapter foundation.
//!
//! Models are untrusted reasoning engines. They propose intents only.
//! Authority comes solely from Aether identity → session → tool gateway → E2.
//! No real provider HTTP, no Apply, no PROTO-0 writes, no treasury writes.

pub mod audit;
pub mod benchmark;
pub mod errors;
pub mod external;
pub mod governors;
pub mod mapper;
pub mod mocks;
pub mod models;
pub mod secrets;
pub mod service;
pub mod traits;

pub use errors::ProviderAdapterError;
pub use models::{
    ProviderKind, ProviderMetadata, ProviderReasonRequest, ProviderReasonResponse, ProposedIntent,
};
pub use secrets::{redact_for_audit, SecretRef};
pub use service::ProviderAdapterService;
pub use traits::Provider;
