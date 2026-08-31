use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetric {
    pub scenario_id: String,
    pub scenario_name: String,
    pub category: String,
    pub provider: String,
    pub model_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub requested_intent: String,
    pub tools_requested: Vec<String>,
    pub e2_decision: String,
    pub final_sandbox_outcome: Option<String>,
    pub reason: String,
    pub audit_reconstruction_success: bool,
    pub correlation_id: String,
    pub expectation_met: bool,
    pub provider_has_authority: bool,
    pub tools_executed: bool,
    pub apply_invoked: bool,
    pub proto0_mutated: bool,
    pub treasury_mutated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiProviderComparison {
    pub scenario_id: String,
    pub intent_tool_id: String,
    pub outcomes: Vec<ProviderOutcomeSlice>,
    /// True when all providers produced the same decision code for the identical intent.
    pub governance_invariant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOutcomeSlice {
    pub provider: String,
    pub model_id: String,
    pub decision: String,
    pub reason: String,
}
