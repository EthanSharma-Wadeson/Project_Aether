//! Demonstration scenario definitions and outcomes.

use crate::enterprise_demo::reporting::DemoSummary;

/// Which end-to-end story to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioKind {
    HappyPath,
    SpendPolicyExceeded,
    AdapterReversal,
    ReceiptReplay,
}

impl ScenarioKind {
    pub const ALL: [Self; 4] = [
        Self::HappyPath,
        Self::SpendPolicyExceeded,
        Self::AdapterReversal,
        Self::ReceiptReplay,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::HappyPath => "Scenario 1 — Happy path",
            Self::SpendPolicyExceeded => "Scenario 2 — Agent exceeds spend policy",
            Self::AdapterReversal => "Scenario 3 — Settlement adapter reverses",
            Self::ReceiptReplay => "Scenario 4 — Receipt replay attack",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::HappyPath => {
                "Enterprise delegates authority; agent completes task; verified hard settlement."
            }
            Self::SpendPolicyExceeded => {
                "Autonomous agent attempts settlement above delegated max_spend; PROTO-0 rejects."
            }
            Self::AdapterReversal => {
                "Adapter confirms then reverses before finalize; hard settlement never occurs."
            }
            Self::ReceiptReplay => {
                "Provider receipt submitted twice; PROTO-2 rejects replay."
            }
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "happy-path" | "1" => Some(Self::HappyPath),
            "spend-policy-exceeded" | "spend" | "2" => Some(Self::SpendPolicyExceeded),
            "adapter-reversal" | "reversal" | "3" => Some(Self::AdapterReversal),
            "receipt-replay" | "replay" | "4" => Some(Self::ReceiptReplay),
            "all" => None,
            _ => None,
        }
    }
}

/// Stage reached before completion or failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoStage {
    EnterpriseRegistered = 1,
    AgentVerified = 2,
    CapabilityDelegated = 3,
    EscrowCreated = 4,
    EscrowFunded = 5,
    ReceiptAccepted = 6,
    SettlementRequested = 7,
    AdapterConfirmed = 8,
    HardSettlementVerified = 9,
    Complete = 10,
}

impl DemoStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::EnterpriseRegistered => "Enterprise registered",
            Self::AgentVerified => "Agent identity verified",
            Self::CapabilityDelegated => "Capability delegated",
            Self::EscrowCreated => "Escrow created",
            Self::EscrowFunded => "Escrow funded",
            Self::ReceiptAccepted => "Receipt accepted",
            Self::SettlementRequested => "Settlement requested",
            Self::AdapterConfirmed => "Adapter confirmed",
            Self::HardSettlementVerified => "Hard settlement verified",
            Self::Complete => "Demonstration complete",
        }
    }
}

pub struct ScenarioOutcome {
    pub kind: ScenarioKind,
    pub success: bool,
    pub stages_completed: Vec<DemoStage>,
    pub failure_stage: Option<DemoStage>,
    pub failure_protocol: Option<&'static str>,
    pub failure_detail: Option<String>,
    pub summary: DemoSummary,
}
