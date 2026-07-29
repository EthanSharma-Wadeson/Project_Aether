//! PROTO-3 — Evidence-based reputation layer.
//!
//! Read-only indexer that derives `ReputationEventV0` from signed PROTO-0/1/2/4 artifacts
//! and computes deterministic `AgentMetricsV0`. PROTO-3 never mutates authority layers.

pub mod derive;
pub mod metrics;
pub mod model;
pub mod query;
pub mod store;

pub use derive::ReputationIndexer;
pub use metrics::AgentMetricsV0;
pub use model::{
    Attribution, EventType, EvidenceRefV0, EvidenceSourceProto, ReputationEventV0,
    REPUTATION_PROTOCOL_VERSION, REPUTATION_SCHEMA_VERSION,
};
pub use query::{ReputationQueryResultV0, ReputationQueryV0};
pub use store::ReputationStore;
