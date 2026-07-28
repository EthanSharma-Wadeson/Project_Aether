//! Settlement signature, capability, escrow, and evidence verification.

#![allow(clippy::too_many_arguments)]

use crate::capability::grant::CapabilityGrant;
use crate::capability::model::CapabilityStore;
use crate::crypto::sha256;
use crate::crypto::verify::{verify_signed_message, SignedMessage};
use crate::error::{Error, Result};
use crate::escrow::state::EscrowStatus;
use crate::escrow::store::EscrowStore;
use crate::identity::registry::IdentityRegistry;
use crate::settlement::adapter::{MockSettlementAdapterV0, SettlementAdapterV0};
use crate::settlement::binding::{SettlementAccountBindingV0, SettlementBindingV0};
use crate::settlement::model::{EconomicOutcome, SettlementStatus};
use crate::settlement::report::SettlementReportV0;
use crate::settlement::store::SettlementStore;
use crate::types::{ActionRequest, Authorisation};
use crate::verifier::authorise::authorise_action;

pub fn require_capability(
    registry: &IdentityRegistry,
    store: &CapabilityStore,
    agent_id: &str,
    grant: Option<&CapabilityGrant>,
    action: &str,
    spend: Option<u64>,
    asset: Option<&str>,
    now: u64,
) -> Result<()> {
    let decision = authorise_action(
        registry,
        store,
        agent_id,
        grant,
        &ActionRequest {
            action: action.into(),
            spend,
            asset: asset.map(str::to_string),
            counterparty: None,
        },
        now,
    );
    match decision {
        Authorisation::Authorised => Ok(()),
        Authorisation::Rejected(reason) => match reason {
            crate::types::RejectReason::IdentityFrozen
            | crate::types::RejectReason::IdentityRevoked => Err(Error::IdentityNotActive),
            crate::types::RejectReason::CapabilityRevoked => Err(Error::CapabilityRevoked),
            crate::types::RejectReason::IdentityOnlyBypass => Err(Error::CapabilityDenied),
            _ => Err(Error::CapabilityDenied),
        },
    }
}

pub fn verify_signed_body(
    signed: &SignedMessage,
    pubkey: &[u8],
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
) -> Result<()> {
    verify_signed_message(
        signed,
        pubkey,
        message_type,
        protocol_version,
        schema_version,
    )
}

pub fn settleable_escrow_status(status: EscrowStatus, outcome: EconomicOutcome) -> Result<()> {
    match (status, outcome) {
        (
            EscrowStatus::Released | EscrowStatus::ResolvedReleased,
            EconomicOutcome::ReleaseToProvider,
        ) => Ok(()),
        (
            EscrowStatus::Refunded | EscrowStatus::ResolvedRefunded,
            EconomicOutcome::RefundToPayer,
        ) => Ok(()),
        _ => Err(Error::InvalidEscrowStatus),
    }
}

pub fn escrow_terminal_commitment(
    escrow_store: &EscrowStore,
    escrow_id: &[u8; 32],
) -> Result<[u8; 32]> {
    let record = escrow_store.get(escrow_id).ok_or(Error::EscrowNotFound)?;
    Ok(sha256(&record.escrow.encode()?))
}

pub fn validate_account_binding_live(
    store: &SettlementStore,
    binding_id: &[u8; 32],
    expected_agent: &str,
    expected_provider: &str,
    now: u64,
) -> Result<SettlementAccountBindingV0> {
    let acct = store
        .get_account(binding_id)
        .ok_or(Error::AccountBindingNotFound)?
        .clone();
    acct.verify_id()?;
    if acct.agent_id != expected_agent {
        return Err(Error::AccountBindingMismatch);
    }
    if acct.settlement_provider != expected_provider {
        return Err(Error::SettlementProviderMismatch);
    }
    if now < acct.valid_after || now > acct.valid_before {
        return Err(Error::MalformedObject("account binding expired"));
    }
    Ok(acct)
}

pub fn validate_settlement_against_escrow(
    escrow_store: &EscrowStore,
    binding: &SettlementBindingV0,
) -> Result<()> {
    let record = escrow_store
        .get(&binding.escrow_id)
        .ok_or(Error::EscrowNotFound)?;
    let escrow = &record.escrow;

    if escrow.terms.terms_version != binding.terms_version {
        return Err(Error::InvalidSettlementEvidence);
    }
    if escrow.terms.asset != binding.asset {
        return Err(Error::SettlementAmountMismatch);
    }
    if escrow.terms.payer != binding.payer_agent_id
        || escrow.terms.provider != binding.provider_agent_id
    {
        return Err(Error::ParticipantMismatch);
    }
    if escrow.terms.principal_amount != binding.principal_amount {
        return Err(Error::SettlementAmountMismatch);
    }
    if escrow.fee_consumed != binding.fee_amount {
        return Err(Error::SettlementAmountMismatch);
    }
    if escrow.status.as_str() != binding.aether_escrow_status {
        return Err(Error::InvalidSettlementEvidence);
    }
    settleable_escrow_status(escrow.status, binding.economic_outcome)?;
    Ok(())
}

pub fn validate_destination_accounts(
    store: &SettlementStore,
    binding: &SettlementBindingV0,
    now: u64,
) -> Result<()> {
    let payer_acct = validate_account_binding_live(
        store,
        &binding.payer_account_binding_id,
        &binding.payer_agent_id,
        &binding.settlement_provider,
        now,
    )?;

    match binding.economic_outcome {
        EconomicOutcome::ReleaseToProvider => {
            let provider_id = binding
                .provider_account_binding_id
                .ok_or(Error::AccountBindingNotFound)?;
            let provider_acct = validate_account_binding_live(
                store,
                &provider_id,
                &binding.provider_agent_id,
                &binding.settlement_provider,
                now,
            )?;
            if binding.destination_binding_id != provider_acct.binding_id {
                return Err(Error::AccountBindingMismatch);
            }
            let _ = payer_acct;
        }
        EconomicOutcome::RefundToPayer | EconomicOutcome::FeeOnly => {
            if binding.destination_binding_id != payer_acct.binding_id {
                return Err(Error::AccountBindingMismatch);
            }
        }
    }
    Ok(())
}

/// Verify adapter report against binding before Confirmed/Finalized promotion.
pub fn verify_settlement_report(
    binding: &SettlementBindingV0,
    report: &SettlementReportV0,
    adapter: &dyn SettlementAdapterV0,
) -> Result<()> {
    if report.settlement_id != binding.settlement_id() {
        return Err(Error::InvalidSettlementEvidence);
    }
    if report.correlation_id != binding.correlation_id {
        return Err(Error::InvalidSettlementEvidence);
    }
    if report.adapter_identity != adapter.provider_id() {
        return Err(Error::SettlementProviderMismatch);
    }
    if report.adapter_identity != binding.settlement_provider {
        return Err(Error::SettlementProviderMismatch);
    }
    let Some(ext) = &binding.external_settlement_ref else {
        return Err(Error::InvalidSettlementEvidence);
    };
    if &report.external_reference != ext {
        return Err(Error::InvalidSettlementEvidence);
    }
    if report.amount != binding.principal_amount {
        return Err(Error::SettlementAmountMismatch);
    }
    match report.status {
        SettlementStatus::Confirmed | SettlementStatus::Finalized | SettlementStatus::Accepted => {}
        SettlementStatus::Failed => {}
        _ => return Err(Error::InvalidSettlementStatus),
    }

    // Confirmed/Finalized require stub proof token matching deterministic mock proof.
    if matches!(
        report.status,
        SettlementStatus::Confirmed | SettlementStatus::Finalized
    ) {
        let Some(token) = report.proof_token else {
            return Err(Error::InvalidSettlementEvidence);
        };
        let expected =
            MockSettlementAdapterV0::proof_for(&binding.settlement_id(), ext, report.amount);
        if token != expected {
            return Err(Error::InvalidSettlementEvidence);
        }
    }
    Ok(())
}

pub fn assert_canonical_account_body(
    signed_body: &[u8],
    binding: &SettlementAccountBindingV0,
) -> Result<()> {
    if binding.encode()? != signed_body {
        return Err(Error::MalformedObject("non-canonical account binding"));
    }
    Ok(())
}

pub fn assert_canonical_settlement_body(
    signed_body: &[u8],
    binding: &SettlementBindingV0,
) -> Result<()> {
    if binding.encode()? != signed_body {
        return Err(Error::MalformedObject("non-canonical settlement binding"));
    }
    Ok(())
}
