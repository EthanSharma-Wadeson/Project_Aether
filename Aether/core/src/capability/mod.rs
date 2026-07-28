//! Capability grants, delegation bounds, and revocation store.

pub mod delegation;
pub mod grant;
pub mod model;

pub use delegation::{is_subset_actions, narrows_constraints};
pub use grant::{grant_capability, verify_capability_grant, CapabilityGrant};
pub use model::{CapabilityRecord, CapabilityStore, CapabilityV0, MSG_CAPABILITY_GRANT};
