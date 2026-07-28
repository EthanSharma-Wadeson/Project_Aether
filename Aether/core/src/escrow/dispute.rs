//! Trusted-local escrow dispute resolution.

#![allow(clippy::too_many_arguments)]

use crate::capability::grant::CapabilityGrant;
use crate::capability::model::CapabilityStore;
use crate::error::{Error, Result};
use crate::escrow::fee::BalanceLedger;
use crate::escrow::model::{ACTION_DISPUTE, ACTION_RELEASE, ACTION_RESOLVE};
use crate::escrow::receipt::SettlementEvidenceV0;
use crate::escrow::state::{EconomicFinalityViewV0, EscrowStatus, EscrowV0};
use crate::escrow::store::EscrowStore;
use crate::escrow::transition::append_fee_ledger;
use crate::escrow::verify::{
    party_keys, require_active_participant, require_capability, verify_signed_receipt,
};
use crate::identity::registry::IdentityRegistry;

#[derive(Debug, Clone)]
pub struct DisputeEvidenceV0 {
    pub escrow_id: [u8; 32],
    pub evidence: SettlementEvidenceV0,
    pub logical_time_raised: u64,
}

/// Raise dispute from Funded or ReceiptAccepted with admissible evidence.
pub fn raise_dispute(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    evidence: &DisputeEvidenceV0,
    raiser_id: &str,
    grant: &CapabilityGrant,
    now: u64,
) -> Result<EscrowV0> {
    let record = store
        .get(&evidence.escrow_id)
        .ok_or(Error::EscrowNotFound)?
        .clone();

    if !matches!(
        record.escrow.status,
        EscrowStatus::Funded | EscrowStatus::ReceiptAccepted
    ) {
        return Err(Error::UnauthorizedTransition);
    }

    if raiser_id != record.escrow.terms.payer && raiser_id != record.escrow.terms.provider {
        return Err(Error::ParticipantMismatch);
    }

    if evidence.evidence.escrow_id != evidence.escrow_id {
        return Err(Error::InvalidEscrowEvidence);
    }

    let (_, pk_r) = party_keys(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
    )?;

    // Evidence must be structurally valid even if receipt may be challenged at resolve.
    let receipt = verify_signed_receipt(
        &evidence.evidence.receipt,
        &pk_r,
        "escrow.submit_receipt",
        record.escrow.protocol_version,
        record.escrow.schema_version,
    )
    .map_err(|_| Error::InvalidEscrowEvidence)?;

    if receipt.escrow_id != evidence.escrow_id {
        return Err(Error::InvalidEscrowEvidence);
    }

    require_active_participant(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
        raiser_id,
    )?;
    require_capability(registry, caps, raiser_id, grant, ACTION_DISPUTE, now)?;

    let dispute_deadline = now.checked_add(record.escrow.terms.dispute_window);
    let mut record = record;
    record.escrow.status = EscrowStatus::Disputed;
    record.escrow.finality =
        EconomicFinalityViewV0::for_status(EscrowStatus::Disputed, false, dispute_deadline);
    let _ = evidence.logical_time_raised;

    let escrow = record.escrow.clone();
    store.insert_record(evidence.escrow_id, record);
    Ok(escrow)
}

/// Resolve dispute deterministically: valid bound receipt → release; else refund.
pub fn resolve_dispute(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    ledger: &mut BalanceLedger,
    escrow_id: &[u8; 32],
    evidence: Option<&SettlementEvidenceV0>,
    resolver_id: &str,
    grant: &CapabilityGrant,
    now: u64,
) -> Result<EscrowV0> {
    let record = store.get(escrow_id).ok_or(Error::EscrowNotFound)?.clone();

    if record.escrow.status != EscrowStatus::Disputed {
        return Err(Error::InvalidEscrowStatus);
    }

    require_capability(registry, caps, resolver_id, grant, ACTION_RESOLVE, now)?;

    let (_, pk_r) = party_keys(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
    )?;

    let receipt_valid = if let Some(ev) = evidence {
        if ev.escrow_id != *escrow_id {
            return Err(Error::InvalidEscrowEvidence);
        }
        verify_signed_receipt(
            &ev.receipt,
            &pk_r,
            "escrow.submit_receipt",
            record.escrow.protocol_version,
            record.escrow.schema_version,
        )
        .ok()
        .filter(|r| {
            r.escrow_id == *escrow_id
                && r.terms_version == record.escrow.terms.terms_version
                && r.claim_type == record.escrow.terms.claim_type
                && r.result_code == record.escrow.terms.required_result_code
        })
    } else {
        None
    };

    let mut record = record;
    if let Some(receipt) = receipt_valid {
        let fee = record.release_quote.quoted_fee;
        let provider_amount = record
            .escrow
            .funded_amount
            .checked_sub(fee)
            .ok_or(Error::FeeBudgetExceeded)?;

        let payer_id = record.escrow.terms.payer.clone();
        let provider_id = record.escrow.terms.provider.clone();

        append_fee_ledger(&mut record, *escrow_id, &payer_id, ACTION_RELEASE, fee, now)?;
        ledger.consume_fee(fee)?;
        ledger.credit(&provider_id, provider_amount)?;

        record.escrow.status = EscrowStatus::ResolvedReleased;
        record.escrow.receipt_id = Some(receipt.receipt_id);
        record.escrow.funded_amount = 0;
        record.escrow.finalized_at = Some(now);
        record.escrow.finality =
            EconomicFinalityViewV0::for_status(EscrowStatus::ResolvedReleased, true, None);
    } else {
        let principal = record.escrow.funded_amount;
        let fee_refund = record
            .escrow
            .fee_reserved
            .checked_sub(record.escrow.fee_consumed)
            .ok_or(Error::EscrowValueConservation)?;

        ledger.credit(&record.escrow.terms.payer, principal)?;
        if fee_refund > 0 {
            ledger.credit(&record.escrow.terms.payer, fee_refund)?;
            record.escrow.fee_refunded = fee_refund;
        }

        record.escrow.status = EscrowStatus::ResolvedRefunded;
        record.escrow.funded_amount = 0;
        record.escrow.fee_reserved = record.escrow.fee_consumed + record.escrow.fee_refunded;
        record.escrow.finalized_at = Some(now);
        record.escrow.finality =
            EconomicFinalityViewV0::for_status(EscrowStatus::ResolvedRefunded, true, None);
    }

    let escrow = record.escrow.clone();
    escrow.check_conservation()?;
    store.insert_record(*escrow_id, record);
    Ok(escrow)
}
