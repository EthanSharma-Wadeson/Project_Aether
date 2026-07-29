//! Final demonstration report.

use aether_core::settlement::SettlementStatus;

use crate::enterprise_demo::scenario::{ScenarioKind, ScenarioOutcome};

pub struct DemoSummary {
    pub enterprise_id: String,
    pub agent_id: String,
    pub provider_id: String,
    pub capability_action: String,
    pub capability_max_spend: u64,
    pub escrow_status: Option<String>,
    pub escrow_principal: Option<u64>,
    pub settlement_status: Option<String>,
    pub external_ref: Option<String>,
    pub hard_finality: bool,
    pub soft_finality: bool,
}

impl DemoSummary {
    pub fn render(&self) -> String {
        let hard = if self.hard_finality { "TRUE" } else { "FALSE" };
        let soft = if self.soft_finality { "TRUE" } else { "FALSE" };
        format!(
            r#"
═══════════════════════════════════════════════════════════════
  ENTERPRISE SPEND CONTROL — DEMONSTRATION SUMMARY
═══════════════════════════════════════════════════════════════

Identity:
    Enterprise:  {enterprise_id}
    Agent:       {agent_id}
    Provider:    {provider_id}
    Verified:    PROTO-0 (Active identities)

Capability:
    Action:      {capability_action}
    Max spend:   {capability_max_spend} AETHER_TEST
    Authority:   PROTO-0 (delegated grant)

Escrow:
    Status:      {escrow_status}
    Principal:   {escrow_principal}
    Rules:       PROTO-2

Settlement:
    Status:      {settlement_status}
    External ref:{external_ref}
    Evidence:    PROTO-4 (mock enterprise.ledger.v0)

Finality:
    Soft (PROTO-2):  {soft}
    Hard (PROTO-4):  {hard}

Authority Source:
    Identity & capability → PROTO-0
    Economic rules        → PROTO-2
    Settlement evidence   → PROTO-4 (adapter is evidence only)

═══════════════════════════════════════════════════════════════
"#,
            enterprise_id = self.enterprise_id,
            agent_id = self.agent_id,
            provider_id = self.provider_id,
            capability_action = self.capability_action,
            capability_max_spend = self.capability_max_spend,
            escrow_status = self
                .escrow_status
                .as_deref()
                .unwrap_or("(not reached)"),
            escrow_principal = self
                .escrow_principal
                .map(|p| p.to_string())
                .unwrap_or_else(|| "(n/a)".into()),
            settlement_status = self
                .settlement_status
                .as_deref()
                .unwrap_or("(not reached)"),
            external_ref = self
                .external_ref
                .as_deref()
                .unwrap_or("(none)"),
            soft = soft,
            hard = hard,
        )
    }
}

pub fn render_scenario_header(kind: ScenarioKind) {
    println!();
    println!("{}", "═".repeat(63));
    println!("  {}", kind.title());
    println!("  {}", kind.description());
    println!("{}", "═".repeat(63));
    println!();
}

pub fn render_outcome(outcome: &ScenarioOutcome) {
    for stage in &outcome.stages_completed {
        println!("[{}] {}", *stage as u8, stage.label());
    }

    if outcome.success {
        println!();
        println!("Result: SUCCESS");
        if outcome.kind == ScenarioKind::HappyPath {
            println!("Hard finality: TRUE");
        }
    } else if let (Some(stage), Some(proto), Some(detail)) = (
        outcome.failure_stage,
        outcome.failure_protocol,
        &outcome.failure_detail,
    ) {
        println!();
        println!("Result: REJECTED at stage [{}] {}", stage as u8, stage.label());
        println!("Rejected by: {proto}");
        println!("Reason: {detail}");
        let expected = outcome.kind != ScenarioKind::HappyPath;
        if expected {
            println!("Hard finality: FALSE (expected for negative scenario)");
        } else {
            println!("Hard finality: FALSE");
        }
    }

    println!("{}", outcome.summary.render());
}

pub fn status_label(s: SettlementStatus) -> String {
    s.as_str().to_string()
}
