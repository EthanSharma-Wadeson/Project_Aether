//! Phase 33 — Agent governance evaluation / benchmark framework (lab only).

pub mod metrics;
pub mod runner;
pub mod scenarios;
pub mod score;

pub use metrics::{BenchmarkMetric, MultiProviderComparison};
pub use runner::GovernanceBenchmarkRunner;
pub use scenarios::{expected_bucket, catalog, ScenarioAgentKind, ScenarioDef, ScenarioExpectation};
pub use score::{GovernanceScoreReport, score_from_metrics};
