//! Escrow state transitions.

#![allow(clippy::too_many_arguments)]

use ed25519_dalek::SigningKey;

use crate::capability::grant::CapabilityGrant;
use crate::capability::model::CapabilityStore;
use crate::crypto::signing::{sign_body, DOMAIN_TAG};
use crate::crypto::verify::SignedMessage;
use crate::error::{Error, Result};
use crate::escrow::fee::{default_fee_quotes, BalanceLedger, FeeLedgerEntryV0};
use crate::escrow::model::{
    DualSignedCancel, DualSignedTerms, EscrowCancelV0, EscrowFundingV0, EscrowRefundV0,
    EscrowReleaseV0, ACTION_CANCEL, ACTION_CREATE, ACTION_FUND, ACTION_REFUND, ACTION_RELEASE,
    ACTION_SUBMIT_RECEIPT, MSG_ESCROW_CANCEL, MSG_ESCROW_CREATE, MSG_ESCROW_FUND,
    MSG_ESCROW_REFUND, MSG_ESCROW_RELEASE, MSG_ESCROW_SUBMIT_RECEIPT,
};
use crate::escrow::receipt::SettlementReceiptV0;
use crate::escrow::state::{EconomicFinalityViewV0, EscrowStatus, EscrowV0};
use crate::escrow::store::{EscrowRecord, EscrowStore};
use crate::escrow::terms::EscrowTermsV0;
use crate::escrow::verify::{
    party_keys, require_active_participant, require_capability, verify_dual_signed_cancel,
    verify_dual_signed_terms, verify_signed_funding, verify_signed_receipt,
};
use crate::identity::registry::IdentityRegistry;

fn signed_message(
    signing_key: &SigningKey,
    agent_id: &str,
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
    body: &[u8],
) -> SignedMessage {
    let (_, signature) = sign_body(
        signing_key,
        DOMAIN_TAG,
        protocol_version,
        schema_version,
        message_type,
        body,
    );
    SignedMessage {
        protocol_version,
        schema_version,
        message_type: message_type.into(),
        body: body.to_vec(),
        signer_key_id: agent_id.into(),
        signature: signature.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    }
}

pub fn sign_terms_dual(
    terms: &EscrowTermsV0,
    payer_key: &SigningKey,
    provider_key: &SigningKey,
    payer_id: &str,
    provider_id: &str,
) -> Result<DualSignedTerms> {
    let body = terms.encode()?;
    Ok(DualSignedTerms {
        body: body.clone(),
        sig_payer: signed_message(
            payer_key,
            payer_id,
            MSG_ESCROW_CREATE,
            terms.protocol_version,
            terms.schema_version,
            &body,
        ),
        sig_provider: signed_message(
            provider_key,
            provider_id,
            MSG_ESCROW_CREATE,
            terms.protocol_version,
            terms.schema_version,
            &body,
        ),
    })
}

pub fn sign_cancel_dual(
    cancel: &EscrowCancelV0,
    payer_key: &SigningKey,
    provider_key: &SigningKey,
    payer_id: &str,
    provider_id: &str,
) -> Result<DualSignedCancel> {
    let body = cancel.encode()?;
    Ok(DualSignedCancel {
        body: body.clone(),
        sig_payer: signed_message(
            payer_key,
            payer_id,
            MSG_ESCROW_CANCEL,
            cancel.protocol_version,
            cancel.schema_version,
            &body,
        ),
        sig_provider: signed_message(
            provider_key,
            provider_id,
            MSG_ESCROW_CANCEL,
            cancel.protocol_version,
            cancel.schema_version,
            &body,
        ),
    })
}

pub fn sign_fund(
    funding: &EscrowFundingV0,
    payer_key: &SigningKey,
    payer_id: &str,
    protocol_version: u32,
    schema_version: u32,
) -> Result<SignedMessage> {
    let body = funding.encode()?;
    Ok(signed_message(
        payer_key,
        payer_id,
        MSG_ESCROW_FUND,
        protocol_version,
        schema_version,
        &body,
    ))
}

pub fn sign_receipt(
    receipt: &SettlementReceiptV0,
    provider_key: &SigningKey,
    provider_id: &str,
) -> Result<SignedMessage> {
    let body = receipt.encode()?;
    Ok(signed_message(
        provider_key,
        provider_id,
        MSG_ESCROW_SUBMIT_RECEIPT,
        receipt.protocol_version,
        receipt.schema_version,
        &body,
    ))
}

pub fn sign_release(release: &EscrowReleaseV0, actor_key: &SigningKey) -> Result<SignedMessage> {
    let body = release.encode()?;
    Ok(signed_message(
        actor_key,
        &release.actor,
        MSG_ESCROW_RELEASE,
        release.protocol_version,
        release.schema_version,
        &body,
    ))
}

pub fn sign_refund(refund: &EscrowRefundV0, actor_key: &SigningKey) -> Result<SignedMessage> {
    let body = refund.encode()?;
    Ok(signed_message(
        actor_key,
        &refund.actor,
        MSG_ESCROW_REFUND,
        refund.protocol_version,
        refund.schema_version,
        &body,
    ))
}

pub(crate) fn append_fee_ledger(
    record: &mut EscrowRecord,
    escrow_id: [u8; 32],
    agent_id: &str,
    operation: &str,
    amount: u64,
    now: u64,
) -> Result<()> {
    let remaining = record
        .escrow
        .fee_reserved
        .checked_sub(record.escrow.fee_consumed)
        .and_then(|v| v.checked_sub(record.escrow.fee_refunded))
        .ok_or(Error::FeeBudgetExceeded)?;
    if amount > remaining {
        return Err(Error::FeeBudgetExceeded);
    }
    let new_remaining = remaining - amount;
    record.fee_ledger.push(FeeLedgerEntryV0 {
        escrow_id,
        agent_id: agent_id.into(),
        operation: operation.into(),
        amount,
        logical_time: now,
        remaining_budget: new_remaining,
    });
    record.escrow.fee_consumed = record
        .escrow
        .fee_consumed
        .checked_add(amount)
        .ok_or(Error::FeeBudgetExceeded)?;
    Ok(())
}

/// Create escrow from dual-signed terms.
pub fn create_escrow(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    dual: &DualSignedTerms,
    grant_payer: &CapabilityGrant,
    grant_provider: &CapabilityGrant,
    now: u64,
) -> Result<EscrowV0> {
    let preview = EscrowTermsV0::decode(&dual.body)?;
    let (pk_p, pk_r) = party_keys(registry, &preview.payer, &preview.provider)?;
    let terms = verify_dual_signed_terms(
        dual,
        &pk_p,
        &pk_r,
        MSG_ESCROW_CREATE,
        preview.protocol_version,
        preview.schema_version,
    )?;

    require_active_participant(registry, &terms.payer, &terms.provider, &terms.payer)?;
    require_active_participant(registry, &terms.payer, &terms.provider, &terms.provider)?;
    require_capability(
        registry,
        caps,
        &terms.payer,
        grant_payer,
        ACTION_CREATE,
        now,
    )?;
    require_capability(
        registry,
        caps,
        &terms.provider,
        grant_provider,
        ACTION_CREATE,
        now,
    )?;

    let escrow_id = terms.escrow_id()?;
    if store.get(&escrow_id).is_some() {
        return Err(Error::EscrowAlreadyExists);
    }

    let (fund_quote, release_quote) = default_fee_quotes(
        terms.protocol_version,
        terms.schema_version,
        &terms.asset,
        terms.max_protocol_fee,
        terms.fund_before,
    );
    if fund_quote.quoted_fee + release_quote.quoted_fee > terms.max_protocol_fee {
        return Err(Error::FeeBudgetExceeded);
    }

    let escrow = EscrowV0 {
        protocol_version: terms.protocol_version,
        schema_version: terms.schema_version,
        escrow_id,
        terms: terms.clone(),
        status: EscrowStatus::Proposed,
        funded_amount: 0,
        fee_reserved: 0,
        fee_consumed: 0,
        fee_refunded: 0,
        receipt_id: None,
        created_at: now,
        funded_at: None,
        receipt_submitted_at: None,
        finalized_at: None,
        finality: EconomicFinalityViewV0::for_status(EscrowStatus::Proposed, false, None),
    };

    store.insert_record(
        escrow_id,
        EscrowRecord {
            escrow: escrow.clone(),
            fund_quote,
            release_quote,
            fee_ledger: Vec::new(),
            last_receipt_nonce: 0,
            bound_receipt: None,
        },
    );
    Ok(escrow)
}

/// Fund escrow from payer-signed funding intent.
pub fn fund_escrow(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    ledger: &mut BalanceLedger,
    signed_funding: &SignedMessage,
    grant_payer: &CapabilityGrant,
    now: u64,
) -> Result<EscrowV0> {
    let funding = EscrowFundingV0::decode(&signed_funding.body)?;
    let record = store
        .get(&funding.escrow_id)
        .ok_or(Error::EscrowNotFound)?
        .clone();

    if record.escrow.status != EscrowStatus::Proposed {
        return Err(Error::InvalidEscrowStatus);
    }
    if now > record.escrow.terms.fund_before {
        return Err(Error::InvalidEscrowStatus);
    }
    if funding.payer != record.escrow.terms.payer {
        return Err(Error::ParticipantMismatch);
    }
    if funding.amount != record.escrow.terms.principal_amount {
        return Err(Error::InvalidEscrowTerms);
    }
    if funding.fee_reservation > record.escrow.terms.max_protocol_fee {
        return Err(Error::FeeBudgetExceeded);
    }
    if funding.fee_reservation != record.fund_quote.quoted_fee {
        return Err(Error::FeeBudgetExceeded);
    }

    let (pk_p, _) = party_keys(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
    )?;
    verify_signed_funding(
        signed_funding,
        &pk_p,
        MSG_ESCROW_FUND,
        record.escrow.protocol_version,
        record.escrow.schema_version,
    )?;

    require_active_participant(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
        &funding.payer,
    )?;
    require_capability(
        registry,
        caps,
        &funding.payer,
        grant_payer,
        ACTION_FUND,
        now,
    )?;

    let total_debit = funding
        .amount
        .checked_add(funding.fee_reservation)
        .ok_or(Error::EscrowValueConservation)?;
    ledger.debit(&funding.payer, total_debit)?;

    let mut record = record;
    record.escrow.status = EscrowStatus::Funded;
    record.escrow.funded_amount = funding.amount;
    record.escrow.fee_reserved = funding.fee_reservation;
    record.escrow.funded_at = Some(now);
    record.escrow.finality = EconomicFinalityViewV0::for_status(EscrowStatus::Funded, true, None);

    let escrow = record.escrow.clone();
    store.insert_record(funding.escrow_id, record);
    Ok(escrow)
}

/// Submit provider-signed settlement receipt.
pub fn submit_receipt(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    escrow_id: &[u8; 32],
    signed_receipt: &SignedMessage,
    grant_provider: &CapabilityGrant,
    now: u64,
) -> Result<EscrowV0> {
    let record = store.get(escrow_id).ok_or(Error::EscrowNotFound)?.clone();

    if record.escrow.status != EscrowStatus::Funded {
        return Err(Error::InvalidEscrowStatus);
    }
    if now > record.escrow.terms.receipt_before {
        return Err(Error::InvalidReceipt);
    }

    let (_, pk_r) = party_keys(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
    )?;
    let receipt = verify_signed_receipt(
        signed_receipt,
        &pk_r,
        MSG_ESCROW_SUBMIT_RECEIPT,
        record.escrow.protocol_version,
        record.escrow.schema_version,
    )?;

    require_active_participant(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
        &record.escrow.terms.provider,
    )?;
    require_capability(
        registry,
        caps,
        &record.escrow.terms.provider,
        grant_provider,
        ACTION_SUBMIT_RECEIPT,
        now,
    )?;

    validate_receipt_against_terms(&receipt, &record)?;

    if receipt.receipt_nonce <= record.last_receipt_nonce {
        return Err(Error::ReceiptReplay);
    }

    let dispute_deadline = now.checked_add(record.escrow.terms.dispute_window);
    let mut record = record;
    record.escrow.status = EscrowStatus::ReceiptAccepted;
    record.escrow.receipt_id = Some(receipt.receipt_id);
    record.escrow.receipt_submitted_at = Some(now);
    record.escrow.finality =
        EconomicFinalityViewV0::for_status(EscrowStatus::ReceiptAccepted, true, dispute_deadline);
    record.last_receipt_nonce = receipt.receipt_nonce;
    record.bound_receipt = Some(receipt);

    let escrow = record.escrow.clone();
    store.insert_record(*escrow_id, record);
    Ok(escrow)
}

fn validate_receipt_against_terms(
    receipt: &SettlementReceiptV0,
    record: &EscrowRecord,
) -> Result<()> {
    let terms = &record.escrow.terms;
    if receipt.escrow_id != record.escrow.escrow_id {
        return Err(Error::InvalidReceipt);
    }
    if receipt.terms_version != terms.terms_version {
        return Err(Error::InvalidReceipt);
    }
    if receipt.payer != terms.payer || receipt.provider != terms.provider {
        return Err(Error::InvalidReceipt);
    }
    if receipt.claim_type != terms.claim_type {
        return Err(Error::InvalidReceipt);
    }
    if receipt.result_code != terms.required_result_code {
        return Err(Error::InvalidReceipt);
    }
    if receipt.claimed_amount > terms.principal_amount {
        return Err(Error::InvalidReceipt);
    }
    if terms.accept_output_commitment
        && receipt.output_commitment != terms.expected_output_commitment
    {
        return Err(Error::InvalidReceipt);
    }
    Ok(())
}

/// Release escrow after dispute window with bound receipt.
pub fn release_escrow(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    ledger: &mut BalanceLedger,
    signed_release: &SignedMessage,
    grant: &CapabilityGrant,
    actor_id: &str,
    now: u64,
) -> Result<EscrowV0> {
    let release = EscrowReleaseV0::decode(&signed_release.body)?;

    let record = store
        .get(&release.escrow_id)
        .ok_or(Error::EscrowNotFound)?
        .clone();

    if record.escrow.status != EscrowStatus::ReceiptAccepted {
        return Err(Error::InvalidEscrowStatus);
    }
    if actor_id != record.escrow.terms.payer && actor_id != record.escrow.terms.provider {
        return Err(Error::UntrustedTerminalOperation);
    }
    if release.actor != actor_id {
        return Err(Error::ParticipantMismatch);
    }

    let (pk_p, pk_r) = party_keys(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
    )?;
    let pk = if actor_id == record.escrow.terms.payer {
        &pk_p
    } else {
        &pk_r
    };
    verify_signed_message_release(signed_release, pk, &record)?;

    require_active_participant(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
        actor_id,
    )?;
    require_capability(registry, caps, actor_id, grant, ACTION_RELEASE, now)?;

    if let Some(deadline) = record.escrow.finality.dispute_deadline {
        if now < deadline {
            return Err(Error::DisputeWindowOpen);
        }
    }

    let bound_id = record.escrow.receipt_id.ok_or(Error::InvalidReceipt)?;
    if release.receipt_id != bound_id {
        return Err(Error::InvalidReceipt);
    }

    let fee = record.release_quote.quoted_fee;
    let provider_amount = record
        .escrow
        .funded_amount
        .checked_sub(fee)
        .ok_or(Error::FeeBudgetExceeded)?;

    let payer_id = record.escrow.terms.payer.clone();
    let provider_id = record.escrow.terms.provider.clone();
    let escrow_id = release.escrow_id;

    let mut record = record;
    append_fee_ledger(&mut record, escrow_id, &payer_id, ACTION_RELEASE, fee, now)?;
    ledger.consume_fee(fee)?;
    ledger.credit(&provider_id, provider_amount)?;

    record.escrow.status = EscrowStatus::Released;
    record.escrow.funded_amount = 0;
    record.escrow.finalized_at = Some(now);
    record.escrow.finality = EconomicFinalityViewV0::for_status(EscrowStatus::Released, true, None);

    let escrow = record.escrow.clone();
    escrow.check_conservation()?;
    store.insert_record(escrow_id, record);
    Ok(escrow)
}

fn verify_signed_message_release(
    signed: &SignedMessage,
    pubkey: &[u8],
    record: &EscrowRecord,
) -> Result<()> {
    verify_signed_funding(
        signed,
        pubkey,
        MSG_ESCROW_RELEASE,
        record.escrow.protocol_version,
        record.escrow.schema_version,
    )?;
    let release = EscrowReleaseV0::decode(&signed.body)?;
    if release.encode()? != signed.body {
        return Err(Error::MalformedObject("non-canonical release"));
    }
    Ok(())
}

/// Cooperative refund from Funded (payer only).
pub fn refund_escrow(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    ledger: &mut BalanceLedger,
    signed_refund: &SignedMessage,
    grant: &CapabilityGrant,
    now: u64,
) -> Result<EscrowV0> {
    let refund = EscrowRefundV0::decode(&signed_refund.body)?;
    let record = store
        .get(&refund.escrow_id)
        .ok_or(Error::EscrowNotFound)?
        .clone();

    match record.escrow.status {
        EscrowStatus::Funded => {
            if refund.actor != record.escrow.terms.payer {
                return Err(Error::UntrustedTerminalOperation);
            }
        }
        EscrowStatus::Expired => {
            if refund.actor != record.escrow.terms.payer {
                return Err(Error::UntrustedTerminalOperation);
            }
        }
        _ => return Err(Error::InvalidEscrowStatus),
    }

    let (pk_p, _) = party_keys(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
    )?;
    verify_signed_funding(
        signed_refund,
        &pk_p,
        MSG_ESCROW_REFUND,
        record.escrow.protocol_version,
        record.escrow.schema_version,
    )?;
    if refund.encode()? != signed_refund.body {
        return Err(Error::MalformedObject("non-canonical refund"));
    }

    require_active_participant(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
        &refund.actor,
    )?;
    require_capability(registry, caps, &refund.actor, grant, ACTION_REFUND, now)?;

    complete_refund(store, ledger, record, now)
}

/// Timeout path: Funded → Expired (either party with escrow.refund capability).
pub fn expire_for_refund(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    escrow_id: &[u8; 32],
    actor_id: &str,
    grant: &CapabilityGrant,
    now: u64,
) -> Result<EscrowV0> {
    let record = store.get(escrow_id).ok_or(Error::EscrowNotFound)?.clone();

    if record.escrow.status != EscrowStatus::Funded {
        return Err(Error::InvalidEscrowStatus);
    }
    if now <= record.escrow.terms.receipt_before {
        return Err(Error::InvalidEscrowStatus);
    }

    require_active_participant(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
        actor_id,
    )?;
    require_capability(registry, caps, actor_id, grant, ACTION_REFUND, now)?;

    let mut record = record;
    record.escrow.status = EscrowStatus::Expired;
    record.escrow.finality = EconomicFinalityViewV0::for_status(EscrowStatus::Expired, false, None);

    let escrow = record.escrow.clone();
    store.insert_record(*escrow_id, record);
    Ok(escrow)
}

fn complete_refund(
    store: &mut EscrowStore,
    ledger: &mut BalanceLedger,
    mut record: EscrowRecord,
    now: u64,
) -> Result<EscrowV0> {
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

    record.escrow.status = EscrowStatus::Refunded;
    record.escrow.funded_amount = 0;
    record.escrow.fee_reserved = record.escrow.fee_consumed + record.escrow.fee_refunded;
    record.escrow.finalized_at = Some(now);
    record.escrow.finality = EconomicFinalityViewV0::for_status(EscrowStatus::Refunded, true, None);

    let escrow = record.escrow.clone();
    escrow.check_conservation()?;
    store.insert_record(record.escrow.escrow_id, record);
    Ok(escrow)
}

/// Cancel unfunded escrow with dual-signed cancel.
pub fn cancel_escrow(
    store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    dual: &DualSignedCancel,
    grant_payer: &CapabilityGrant,
    grant_provider: &CapabilityGrant,
    now: u64,
) -> Result<EscrowV0> {
    let cancel = EscrowCancelV0::decode(&dual.body)?;
    let record = store
        .get(&cancel.escrow_id)
        .ok_or(Error::EscrowNotFound)?
        .clone();

    if record.escrow.status != EscrowStatus::Proposed {
        return Err(Error::InvalidEscrowStatus);
    }

    let (pk_p, pk_r) = party_keys(
        registry,
        &record.escrow.terms.payer,
        &record.escrow.terms.provider,
    )?;
    verify_dual_signed_cancel(
        dual,
        &pk_p,
        &pk_r,
        MSG_ESCROW_CANCEL,
        cancel.protocol_version,
        cancel.schema_version,
    )?;
    if cancel.encode()? != dual.body {
        return Err(Error::MalformedObject("non-canonical cancel"));
    }

    require_capability(
        registry,
        caps,
        &record.escrow.terms.payer,
        grant_payer,
        ACTION_CANCEL,
        now,
    )?;
    require_capability(
        registry,
        caps,
        &record.escrow.terms.provider,
        grant_provider,
        ACTION_CANCEL,
        now,
    )?;

    let mut record = record;
    record.escrow.status = EscrowStatus::Cancelled;
    record.escrow.finalized_at = Some(now);
    record.escrow.finality =
        EconomicFinalityViewV0::for_status(EscrowStatus::Cancelled, true, None);

    let escrow = record.escrow.clone();
    store.insert_record(cancel.escrow_id, record);
    Ok(escrow)
}
