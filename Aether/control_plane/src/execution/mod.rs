//! Governance execution pipeline — Phase 4A dry-run only.
//!
//! Completes the Operator → Auth → RBAC → Policy → Signer → (simulated) PROTO-0
//! path without mutating protocol state. Live Apply is Phase 4B.

pub mod errors;
pub mod executor;
pub mod hash;
pub mod planner;
pub mod simulate;
pub mod types;

pub use errors::ExecutionError;
pub use executor::ExecutionExecutor;
pub use hash::{
    compute_execution_hash, execution_hash_for_policy, execution_parameters_from_policy,
    ExecutionBindingInput,
};
pub use planner::ExecutionPlanner;
pub use simulate::{
    protocol_observation_fingerprint, simulate_capability_grant, simulate_capability_revoke,
    simulate_policy_apply,
};
pub use types::{
    DryRunReport, ExecutionMode, ExecutionPlan, ProtocolOperationKind, SimulationOutcome,
    ValidationStep,
};
