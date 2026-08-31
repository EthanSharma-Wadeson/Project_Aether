//! Apply foundation modules.
//!
//! Production: `apply_enabled()` remains `false` — no live PROTO-0 mutations.
//! The mutation adapter may mutate only when enabled (test override).

pub mod approval;
pub mod attestation;
pub mod canonical_json;
pub mod enablement;
pub mod errors;
pub mod execution;
pub mod policy_mapping;
pub mod replay;
pub mod resimulation;
pub mod signature;
pub mod validation;

#[cfg(test)]
use std::cell::Cell;

use crate::apply::errors::ApplyErrorCode;

/// Whether Apply execution is enabled.
///
/// Production: always `false` until authorised post-checklist.
/// Tests may temporarily override via [`ApplyEnabledGuard`].
pub fn apply_enabled() -> bool {
    #[cfg(test)]
    {
        if let Some(v) = APPLY_ENABLED_OVERRIDE.with(|c| c.get()) {
            return v;
        }
    }
    false
}

#[cfg(test)]
thread_local! {
    static APPLY_ENABLED_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Test-only RAII override for [`apply_enabled`]. Cleared on drop.
#[cfg(test)]
pub struct ApplyEnabledGuard {
    previous: Option<bool>,
}

#[cfg(test)]
impl ApplyEnabledGuard {
    pub fn enable() -> Self {
        let previous = APPLY_ENABLED_OVERRIDE.with(|c| c.replace(Some(true)));
        Self { previous }
    }

    pub fn disable() -> Self {
        let previous = APPLY_ENABLED_OVERRIDE.with(|c| c.replace(Some(false)));
        Self { previous }
    }
}

#[cfg(test)]
impl Drop for ApplyEnabledGuard {
    fn drop(&mut self) {
        APPLY_ENABLED_OVERRIDE.with(|c| c.set(self.previous));
    }
}

/// Policy mapping failure with stable Apply error code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyMappingError {
    pub code: ApplyErrorCode,
    pub message: String,
}

impl PolicyMappingError {
    pub fn new(code: ApplyErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PolicyMappingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for PolicyMappingError {}
