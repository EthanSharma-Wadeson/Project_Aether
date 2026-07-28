//! Escrow signature and authority verification.

#![allow(clippy::too_many_arguments)]

use crate::capability::grant::CapabilityGrant;
use crate::capability::model::CapabilityStore;
use crate::crypto::verify::{verify_signed_message, SignedMessage};
use crate::error::{Error, Result};
use crate::escrow::model::{DualSignedCancel, DualSignedTerms};
use crate::escrow::receipt::SettlementReceiptV0;
use crate::escrow::terms::EscrowTermsV0;
use crate::identity::registry::IdentityRegistry;
use crate::types::{ActionRequest, Authorisation};
use crate::verifier::authorise::authorise_action;

pub fn verify_dual_signed_terms(
    dual: &DualSignedTerms,
    pubkey_payer: &[u8],
    pubkey_provider: &[u8],
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
) -> Result<EscrowTermsV0> {
    if dual.sig_payer.body != dual.body || dual.sig_provider.body != dual.body {
        return Err(Error::InvalidSignature);
    }
    if dual.sig_payer.body != dual.sig_provider.body {
        return Err(Error::UnilateralUpdate);
    }

    verify_signed_message(
        &dual.sig_payer,
        pubkey_payer,
        message_type,
        protocol_version,
        schema_version,
    )?;
    verify_signed_message(
        &dual.sig_provider,
        pubkey_provider,
        message_type,
        protocol_version,
        schema_version,
    )?;

    let terms = EscrowTermsV0::decode(&dual.body)?;
    if terms.encode()? != dual.body {
        return Err(Error::MalformedObject("non-canonical escrow terms"));
    }
    terms.validate()?;
    Ok(terms)
}

pub fn verify_dual_signed_cancel(
    dual: &DualSignedCancel,
    pubkey_payer: &[u8],
    pubkey_provider: &[u8],
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
) -> Result<()> {
    if dual.sig_payer.body != dual.body || dual.sig_provider.body != dual.body {
        return Err(Error::InvalidSignature);
    }
    verify_signed_message(
        &dual.sig_payer,
        pubkey_payer,
        message_type,
        protocol_version,
        schema_version,
    )?;
    verify_signed_message(
        &dual.sig_provider,
        pubkey_provider,
        message_type,
        protocol_version,
        schema_version,
    )?;
    Ok(())
}

pub fn verify_signed_funding(
    signed: &SignedMessage,
    pubkey_payer: &[u8],
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
) -> Result<()> {
    verify_signed_message(
        signed,
        pubkey_payer,
        message_type,
        protocol_version,
        schema_version,
    )
}

pub fn verify_signed_receipt(
    signed: &SignedMessage,
    pubkey_provider: &[u8],
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
) -> Result<SettlementReceiptV0> {
    verify_signed_message(
        signed,
        pubkey_provider,
        message_type,
        protocol_version,
        schema_version,
    )?;
    let receipt = SettlementReceiptV0::decode(&signed.body)?;
    if receipt.encode()? != signed.body {
        return Err(Error::MalformedObject("non-canonical receipt"));
    }
    receipt.verify_id()?;
    Ok(receipt)
}

pub fn require_capability(
    registry: &IdentityRegistry,
    store: &CapabilityStore,
    agent_id: &str,
    grant: &CapabilityGrant,
    action: &str,
    now: u64,
) -> Result<()> {
    let decision = authorise_action(
        registry,
        store,
        agent_id,
        Some(grant),
        &ActionRequest {
            action: action.into(),
            spend: None,
            asset: None,
            counterparty: None,
        },
        now,
    );
    match decision {
        Authorisation::Authorised => Ok(()),
        Authorisation::Rejected(_) => Err(Error::CapabilityDenied),
    }
}

pub fn require_active_participant(
    registry: &IdentityRegistry,
    payer: &str,
    provider: &str,
    agent_id: &str,
) -> Result<()> {
    if agent_id != payer && agent_id != provider {
        return Err(Error::ParticipantMismatch);
    }
    let entry = registry.get(agent_id).ok_or(Error::IdentityNotFound)?;
    if entry.status != crate::types::AgentStatus::Active {
        return Err(Error::IdentityNotActive);
    }
    Ok(())
}

pub fn party_keys(
    registry: &IdentityRegistry,
    payer: &str,
    provider: &str,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let p = registry.get(payer).ok_or(Error::IdentityNotFound)?;
    let r = registry.get(provider).ok_or(Error::IdentityNotFound)?;
    Ok((
        p.identity.operational_public_key.clone(),
        r.identity.operational_public_key.clone(),
    ))
}
