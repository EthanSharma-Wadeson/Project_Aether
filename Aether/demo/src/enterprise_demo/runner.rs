//! Scenario execution — drives the full enterprise spend-control lifecycle.

use aether_core::error::Error;
use aether_core::escrow::model::EscrowFundingV0;
use aether_core::escrow::state::EscrowV0;
use aether_core::escrow::terms::EscrowTermsV0;
use aether_core::escrow::transition::{
    create_escrow, fund_escrow, release_escrow, sign_fund, sign_receipt, sign_release,
    sign_terms_dual, submit_receipt,
};
use aether_core::settlement::{
    advance_mock_status, finalize_settlement, intent_from_escrow, query_settlement,
    request_settlement, submit_settlement, EconomicOutcome, SettlementBindingV0,
    SettlementStatus,
};

use crate::enterprise_demo::harness::{protocol_layer, DemoConfig, EnterpriseWorld, PROTOCOL_VERSION, SCHEMA_VERSION};
use crate::enterprise_demo::reporting::{status_label, DemoSummary};
use crate::enterprise_demo::scenario::{DemoStage, ScenarioKind, ScenarioOutcome};

struct RunState {
    stages: Vec<DemoStage>,
    escrow_id: Option<[u8; 32]>,
    terms: Option<EscrowTermsV0>,
    binding: Option<SettlementBindingV0>,
    cached_report: Option<aether_core::settlement::SettlementReportV0>,
}

impl RunState {
    fn push(&mut self, stage: DemoStage) {
        self.stages.push(stage);
    }

    fn fail(
        self,
        kind: ScenarioKind,
        stage: DemoStage,
        err: Error,
        world: &EnterpriseWorld,
        config: &DemoConfig,
    ) -> ScenarioOutcome {
        let proto = protocol_layer(&err);
        let summary = build_summary(world, config, &self, false);
        ScenarioOutcome {
            kind,
            success: false,
            stages_completed: self.stages,
            failure_stage: Some(stage),
            failure_protocol: Some(proto),
            failure_detail: Some(format!("{err:?}")),
            summary,
            escrow_id: self.escrow_id,
            settlement_binding_id: self.binding.as_ref().map(|b| b.binding_id),
        }
    }

    fn succeed(
        mut self,
        kind: ScenarioKind,
        world: &EnterpriseWorld,
        config: &DemoConfig,
    ) -> ScenarioOutcome {
        self.stages.push(DemoStage::Complete);
        let summary = build_summary(world, config, &self, true);
        ScenarioOutcome {
            kind,
            success: true,
            stages_completed: self.stages,
            failure_stage: None,
            failure_protocol: None,
            failure_detail: None,
            summary,
            escrow_id: self.escrow_id,
            settlement_binding_id: self.binding.as_ref().map(|b| b.binding_id),
        }
    }
}

fn build_summary(
    world: &EnterpriseWorld,
    config: &DemoConfig,
    state: &RunState,
    _success: bool,
) -> DemoSummary {
    let escrow = state.escrow_id.and_then(|id| world.escrows.get(&id));
    DemoSummary {
        enterprise_id: world.enterprise.identity.derived_agent_id(),
        agent_id: world.agent.identity.derived_agent_id(),
        provider_id: world.provider.identity.derived_agent_id(),
        capability_action: "settlement.settle (delegated)".into(),
        capability_max_spend: config.agent_max_spend,
        escrow_status: escrow.map(|r| r.escrow.status.as_str().to_string()),
        escrow_principal: escrow.map(|r| r.escrow.terms.principal_amount),
        settlement_status: state
            .binding
            .as_ref()
            .map(|b| status_label(b.settlement_status)),
        external_ref: state
            .binding
            .as_ref()
            .and_then(|b| b.external_settlement_ref.clone()),
        hard_finality: escrow
            .map(|r| r.escrow.finality.hard_settlement_placeholder)
            .unwrap_or(false),
        soft_finality: escrow
            .map(|r| r.escrow.finality.finalized)
            .unwrap_or(false),
    }
}

fn sample_terms(world: &EnterpriseWorld, config: &DemoConfig) -> EscrowTermsV0 {
    EscrowTermsV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        payer: world.enterprise.identity.derived_agent_id(),
        provider: world.provider.identity.derived_agent_id(),
        asset: "AETHER_TEST".into(),
        principal_amount: config.principal,
        max_protocol_fee: config.max_fee,
        claim_type: "enterprise.compute.task".into(),
        required_result_code: "ok".into(),
        accept_output_commitment: false,
        expected_output_commitment: None,
        fund_before: config.fund_before,
        receipt_before: config.receipt_before,
        dispute_window: config.dispute_window,
        terms_version: 1,
    }
}

fn create_and_fund_escrow(
    world: &mut EnterpriseWorld,
    config: &DemoConfig,
    now: u64,
) -> Result<(EscrowTermsV0, [u8; 32]), (DemoStage, Error)> {
    let terms = sample_terms(world, config);
    let escrow_id = terms.escrow_id().map_err(|e| (DemoStage::EscrowCreated, e))?;
    let ent_id = world.enterprise.identity.derived_agent_id();
    let prov_id = world.provider.identity.derived_agent_id();
    let dual = sign_terms_dual(
        &terms,
        &world.enterprise.signing_key,
        &world.provider.signing_key,
        &ent_id,
        &prov_id,
    )
    .map_err(|e| (DemoStage::EscrowCreated, e))?;
    create_escrow(
        &mut world.escrows,
        &world.registry,
        &world.caps,
        &dual,
        &world.enterprise_grant,
        &world.provider_grant,
        now,
    )
    .map_err(|e| (DemoStage::EscrowCreated, e))?;

    let record = world.escrows.get(&escrow_id).ok_or((DemoStage::EscrowFunded, Error::EscrowNotFound))?;
    let funding = EscrowFundingV0 {
        escrow_id,
        payer: ent_id.clone(),
        amount: config.principal,
        fee_reservation: record.fund_quote.quoted_fee,
        logical_time: now + 1,
    };
    let signed = sign_fund(
        &funding,
        &world.enterprise.signing_key,
        &ent_id,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
    )
    .map_err(|e| (DemoStage::EscrowFunded, e))?;
    fund_escrow(
        &mut world.escrows,
        &world.registry,
        &world.caps,
        &mut world.ledger,
        &signed,
        &world.enterprise_grant,
        now + 1,
    )
    .map_err(|e| (DemoStage::EscrowFunded, e))?;
    Ok((terms, escrow_id))
}

fn submit_receipt_once(
    world: &mut EnterpriseWorld,
    terms: &EscrowTermsV0,
    escrow_id: [u8; 32],
    nonce: u64,
    now: u64,
) -> Result<EscrowV0, Error> {
    let receipt = aether_core::escrow::receipt::SettlementReceiptV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        receipt_id: [0u8; 32],
        escrow_id,
        terms_version: terms.terms_version,
        payer: terms.payer.clone(),
        provider: terms.provider.clone(),
        claim_type: terms.claim_type.clone(),
        result_code: terms.required_result_code.clone(),
        output_commitment: None,
        claimed_amount: terms.principal_amount,
        logical_time: now,
        receipt_nonce: nonce,
    }
    .with_computed_id()?;
    let prov_id = world.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &world.provider.signing_key, &prov_id)?;
    submit_receipt(
        &mut world.escrows,
        &world.registry,
        &world.caps,
        &escrow_id,
        &signed,
        &world.provider_grant,
        now,
    )
}

fn release_escrow_terminal(world: &mut EnterpriseWorld, escrow_id: [u8; 32], now: u64) -> Result<(), Error> {
    let record = world.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;
    let receipt_id = record.escrow.receipt_id.ok_or(Error::InvalidReceipt)?;
    let ent_id = world.enterprise.identity.derived_agent_id();
    let release = aether_core::escrow::model::EscrowReleaseV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        receipt_id,
        actor: ent_id.clone(),
        logical_time: now,
    };
    let signed = sign_release(&release, &world.enterprise.signing_key)?;
    release_escrow(
        &mut world.escrows,
        &world.registry,
        &world.caps,
        &mut world.ledger,
        &signed,
        &world.enterprise_grant,
        &ent_id,
        now,
    )?;
    Ok(())
}

fn request_settle(
    world: &mut EnterpriseWorld,
    escrow_id: [u8; 32],
    now: u64,
    as_agent: bool,
) -> Result<SettlementBindingV0, Error> {
    let intent = intent_from_escrow(
        &world.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        aether_core::settlement::PROVIDER_ENTERPRISE_LEDGER_V0,
        world.enterprise_account.binding_id,
        Some(world.provider_account.binding_id),
    )?;
    if as_agent {
        let agent_id = world.agent.identity.derived_agent_id();
        request_settlement(
            &mut world.settlements,
            &world.escrows,
            &world.registry,
            &world.caps,
            &intent,
            &agent_id,
            &world.agent.signing_key,
            Some(&world.agent_delegate_grant),
            now,
        )
    } else {
        let ent_id = world.enterprise.identity.derived_agent_id();
        request_settlement(
            &mut world.settlements,
            &world.escrows,
            &world.registry,
            &world.caps,
            &intent,
            &ent_id,
            &world.enterprise.signing_key,
            Some(&world.enterprise_grant),
            now,
        )
    }
}

fn confirm_and_finalize(
    world: &mut EnterpriseWorld,
    binding: &SettlementBindingV0,
    now: u64,
    do_finalize: bool,
) -> Result<(SettlementBindingV0, Option<aether_core::settlement::SettlementReportV0>), Error> {
    let ent_id = world.enterprise.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut world.settlements,
        &world.escrows,
        &world.registry,
        &world.caps,
        &mut world.adapter,
        &binding.binding_id,
        &ent_id,
        Some(&world.enterprise_grant),
        now,
    )?;
    advance_mock_status(&mut world.adapter, &submitted, SettlementStatus::Accepted)?;
    let (accepted, _) = query_settlement(
        &mut world.settlements,
        &mut world.escrows,
        &world.registry,
        &world.caps,
        &mut world.adapter,
        &submitted.binding_id,
        &ent_id,
        Some(&world.enterprise_grant),
        now + 1,
    )?;
    advance_mock_status(&mut world.adapter, &accepted, SettlementStatus::Confirmed)?;
    let (confirmed, report) = query_settlement(
        &mut world.settlements,
        &mut world.escrows,
        &world.registry,
        &world.caps,
        &mut world.adapter,
        &accepted.binding_id,
        &ent_id,
        Some(&world.enterprise_grant),
        now + 2,
    )?;
    if !do_finalize {
        return Ok((confirmed, Some(report)));
    }
    let finalized = finalize_settlement(
        &mut world.settlements,
        &mut world.escrows,
        &world.registry,
        &world.caps,
        &world.adapter,
        &confirmed.binding_id,
        &ent_id,
        Some(&world.enterprise_grant),
        &report,
        now + 3,
    )?;
    Ok((finalized, Some(report)))
}

/// Run a single demonstration scenario against an existing world.
pub fn run_scenario_on_world(
    kind: ScenarioKind,
    config: &DemoConfig,
    world: &mut EnterpriseWorld,
) -> ScenarioOutcome {
    let mut state = RunState {
        stages: Vec::new(),
        escrow_id: None,
        terms: None,
        binding: None,
        cached_report: None,
    };

    state.push(DemoStage::EnterpriseRegistered);
    state.push(DemoStage::AgentVerified);
    state.push(DemoStage::CapabilityDelegated);

    let now = 50u64;

    let (terms, escrow_id) = match create_and_fund_escrow(world, config, now) {
        Ok(v) => v,
        Err((stage, e)) => return state.fail(kind, stage, e, world, config),
    };
    state.escrow_id = Some(escrow_id);
    state.terms = Some(terms.clone());
    state.push(DemoStage::EscrowCreated);
    state.push(DemoStage::EscrowFunded);

    match kind {
        ScenarioKind::ReceiptReplay => {
            if let Err(e) = submit_receipt_once(world, &terms, escrow_id, 1, now + 10) {
                return state.fail(kind, DemoStage::ReceiptAccepted, e, world, config);
            }
            state.push(DemoStage::ReceiptAccepted);
            // Replay: resubmit identical signed receipt after escrow left Funded state.
            match submit_receipt_once(world, &terms, escrow_id, 1, now + 11) {
                Ok(_) => state.fail(
                    kind,
                    DemoStage::ReceiptAccepted,
                    Error::InvalidReceipt,
                    world,
                    config,
                ),
                Err(e) if matches!(e, Error::ReceiptReplay | Error::InvalidEscrowStatus) => {
                    state.fail(kind, DemoStage::ReceiptAccepted, e, world, config)
                }
                Err(e) => state.fail(kind, DemoStage::ReceiptAccepted, e, world, config),
            }
        }

        ScenarioKind::SpendPolicyExceeded => {
            if let Err(e) = submit_receipt_once(world, &terms, escrow_id, 1, now + 10) {
                return state.fail(kind, DemoStage::ReceiptAccepted, e, world, config);
            }
            state.push(DemoStage::ReceiptAccepted);
            if let Err(e) = release_escrow_terminal(world, escrow_id, now + 120) {
                return state.fail(kind, DemoStage::ReceiptAccepted, e, world, config);
            }
            // Agent max_spend is below principal in this scenario config
            match request_settle(world, escrow_id, now + 130, true) {
                Ok(_) => state.fail(
                    kind,
                    DemoStage::SettlementRequested,
                    Error::CapabilityDenied,
                    world,
                    config,
                ),
                Err(e) => state.fail(kind, DemoStage::SettlementRequested, e, world, config),
            }
        }

        ScenarioKind::AdapterReversal | ScenarioKind::HappyPath => {
            if let Err(e) = submit_receipt_once(world, &terms, escrow_id, 1, now + 10) {
                return state.fail(kind, DemoStage::ReceiptAccepted, e, world, config);
            }
            state.push(DemoStage::ReceiptAccepted);
            if let Err(e) = release_escrow_terminal(world, escrow_id, now + 120) {
                return state.fail(kind, DemoStage::ReceiptAccepted, e, world, config);
            }

            let binding = match request_settle(world, escrow_id, now + 130, false) {
                Ok(b) => b,
                Err(e) => return state.fail(kind, DemoStage::SettlementRequested, e, world, config),
            };
            state.binding = Some(binding.clone());
            state.push(DemoStage::SettlementRequested);

            let (confirmed, report) = match confirm_and_finalize(
                world,
                &binding,
                now + 140,
                kind == ScenarioKind::HappyPath,
            ) {
                Ok(v) => v,
                Err(e) => {
                    let stage = if kind == ScenarioKind::AdapterReversal {
                        DemoStage::HardSettlementVerified
                    } else {
                        DemoStage::AdapterConfirmed
                    };
                    return state.fail(kind, stage, e, world, config);
                }
            };
            state.binding = Some(confirmed.clone());
            state.push(DemoStage::AdapterConfirmed);

            if kind == ScenarioKind::AdapterReversal {
                let report = report.expect("cached report");
                state.cached_report = Some(report.clone());
                let ext = confirmed
                    .external_settlement_ref
                    .as_ref()
                    .expect("external ref");
                world.adapter.set_reversed(ext).expect("reverse");
                let ent_id = world.enterprise.identity.derived_agent_id();
                match finalize_settlement(
                    &mut world.settlements,
                    &mut world.escrows,
                    &world.registry,
                    &world.caps,
                    &world.adapter,
                    &confirmed.binding_id,
                    &ent_id,
                    Some(&world.enterprise_grant),
                    &report,
                    now + 200,
                ) {
                    Ok(_) => {
                        return state.fail(
                            kind,
                            DemoStage::HardSettlementVerified,
                            Error::InvalidSettlementEvidence,
                            world,
                            config,
                        );
                    }
                    Err(e) => {
                        let hard = world
                            .escrows
                            .get(&escrow_id)
                            .map(|r| r.escrow.finality.hard_settlement_placeholder)
                            .unwrap_or(false);
                        if hard {
                            return state.fail(
                                kind,
                                DemoStage::HardSettlementVerified,
                                Error::InvalidSettlementEvidence,
                                world,
                                config,
                            );
                        }
                        return state.fail(kind, DemoStage::HardSettlementVerified, e, world, config);
                    }
                }
            }

            state.push(DemoStage::HardSettlementVerified);
            state.succeed(kind, world, config)
        }
    }
}

/// Run a single demonstration scenario.
pub fn run_scenario(kind: ScenarioKind, config: &DemoConfig) -> ScenarioOutcome {
    crate::enterprise_demo::reporting::render_scenario_header(kind);
    let mut world = crate::enterprise_demo::harness::setup_world(config);
    run_scenario_on_world(kind, config, &mut world)
}

/// Bootstrap data for Control Plane observatory (happy path only).
pub struct ObservatoryBootstrap {
    pub world: EnterpriseWorld,
    pub escrow_id: Option<[u8; 32]>,
    pub settlement_binding_id: Option<[u8; 32]>,
}

/// Bootstrap world for Control Plane observatory (happy path only).
pub fn build_observatory_bootstrap() -> ObservatoryBootstrap {
    let kind = ScenarioKind::HappyPath;
    let config = config_for(kind);
    let mut world = crate::enterprise_demo::harness::setup_world(&config);
    let outcome = run_scenario_on_world(kind, &config, &mut world);
    if !outcome.success {
        panic!(
            "observatory bootstrap failed: {:?}",
            outcome.failure_detail
        );
    }
    ObservatoryBootstrap {
        world,
        escrow_id: outcome.escrow_id,
        settlement_binding_id: outcome.settlement_binding_id,
    }
}

/// Config overrides per scenario.
pub fn config_for(kind: ScenarioKind) -> DemoConfig {
    let mut config = DemoConfig::default();
    match kind {
        ScenarioKind::HappyPath => {}
        ScenarioKind::SpendPolicyExceeded => {
            config.agent_max_spend = 500;
            config.principal = 1_000;
        }
        ScenarioKind::AdapterReversal => {}
        ScenarioKind::ReceiptReplay => {}
    }
    config
}

pub fn run_all() -> Vec<ScenarioOutcome> {
    ScenarioKind::ALL
        .iter()
        .map(|&kind| run_scenario(kind, &config_for(kind)))
        .collect()
}
