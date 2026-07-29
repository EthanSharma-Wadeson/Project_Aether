//! Command-line interface for the enterprise demonstrator.

use crate::enterprise_demo::reporting::render_outcome;
use crate::enterprise_demo::runner::{config_for, run_all, run_scenario};
use crate::enterprise_demo::scenario::{DemoStage, ScenarioKind, ScenarioOutcome};

const BANNER: &str = r#"
╔═══════════════════════════════════════════════════════════════╗
║     AETHER — Enterprise Agent Spend Control Demonstrator      ║
║     Proof-of-value · local simulation · no production rails   ║
╚═══════════════════════════════════════════════════════════════╝
"#;

pub fn run(args: Vec<String>) {
    println!("{BANNER}");

    let outcomes = match args.first().map(|s| s.as_str()) {
        None | Some("all") => run_all(),
        Some(arg) => {
            if arg == "--help" || arg == "-h" {
                print_usage();
                return;
            }
            let kind = ScenarioKind::parse(arg).unwrap_or_else(|| {
                eprintln!("Unknown scenario: {arg}");
                print_usage();
                std::process::exit(1);
            });
            vec![run_scenario(kind, &config_for(kind))]
        }
    };

    let mut passed = 0usize;
    let mut failed = 0usize;
    for outcome in &outcomes {
        render_outcome(outcome);
        if scenario_met_expectation(outcome) {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    println!();
    println!("{}", "─".repeat(63));
    println!("  Scenarios run: {}", outcomes.len());
    println!("  Expected outcomes met: {passed}");
    if failed > 0 {
        println!("  UNEXPECTED FAILURES: {failed}");
        std::process::exit(1);
    }
    println!();
    println!("Demonstration complete. No protocol code was modified.");
    println!("Next: review ENTERPRISE_DEMO.md and ENTERPRISE_DEMO_RESULTS.md");
}

fn scenario_met_expectation(outcome: &ScenarioOutcome) -> bool {
    match outcome.kind {
        ScenarioKind::HappyPath => outcome.success,
        ScenarioKind::SpendPolicyExceeded => {
            !outcome.success
                && outcome.failure_stage == Some(DemoStage::SettlementRequested)
                && outcome.failure_protocol == Some("PROTO-0")
        }
        ScenarioKind::AdapterReversal => {
            !outcome.success
                && outcome.failure_stage == Some(DemoStage::HardSettlementVerified)
                && !outcome.summary.hard_finality
        }
        ScenarioKind::ReceiptReplay => {
            !outcome.success
                && outcome.failure_stage == Some(DemoStage::ReceiptAccepted)
                && outcome.failure_protocol == Some("PROTO-2")
                && outcome
                    .failure_detail
                    .as_deref()
                    .is_some_and(|d| d == "ReceiptReplay" || d == "InvalidEscrowStatus")
        }
    }
}

fn print_usage() {
    eprintln!(
        r#"Usage: enterprise-demo [SCENARIO]

Scenarios:
  all                      Run all four scenarios (default)
  happy-path | 1           Scenario 1 — full success path
  spend-policy-exceeded | 2  Scenario 2 — PROTO-0 spend cap rejection
  adapter-reversal | 3       Scenario 3 — adapter reverse blocks hard finality
  receipt-replay | 4         Scenario 4 — PROTO-2 receipt replay rejection

Examples:
  cargo run -p aether-enterprise-demo
  cargo run -p aether-enterprise-demo -- happy-path
"#
    );
}
