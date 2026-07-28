//! Dual-signature and channel membership verification.

#![allow(clippy::too_many_arguments)]

use crate::capability::grant::CapabilityGrant;
use crate::capability::model::CapabilityStore;
use crate::channel::model::{DualSignedUpdate, StateUpdateV0};
use crate::crypto::verify::{verify_signed_message, SignedMessage};
use crate::error::{Error, Result};
use crate::identity::registry::IdentityRegistry;
use crate::types::{ActionRequest, Authorisation};
use crate::verifier::authorise::authorise_action;

/// Verify both envelopes authenticate identical body bytes under the two party keys.
pub fn verify_dual_signed(
    dual: &DualSignedUpdate,
    pubkey_a: &[u8],
    pubkey_b: &[u8],
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
) -> Result<StateUpdateV0> {
    if dual.sig_a.body != dual.body || dual.sig_b.body != dual.body {
        return Err(Error::InvalidSignature);
    }
    if dual.sig_a.body != dual.sig_b.body {
        return Err(Error::UnilateralUpdate);
    }

    verify_signed_message(
        &dual.sig_a,
        pubkey_a,
        message_type,
        protocol_version,
        schema_version,
    )?;
    verify_signed_message(
        &dual.sig_b,
        pubkey_b,
        message_type,
        protocol_version,
        schema_version,
    )?;

    let update = StateUpdateV0::decode(&dual.body)?;
    if update.encode()? != dual.body {
        return Err(Error::MalformedObject("non-canonical state update"));
    }
    Ok(update)
}

pub fn verify_open_pair(
    body: &[u8],
    sig_a: &SignedMessage,
    sig_b: &SignedMessage,
    pubkey_a: &[u8],
    pubkey_b: &[u8],
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
) -> Result<()> {
    if sig_a.body != body || sig_b.body != body {
        return Err(Error::InvalidSignature);
    }
    verify_signed_message(
        sig_a,
        pubkey_a,
        message_type,
        protocol_version,
        schema_version,
    )?;
    verify_signed_message(
        sig_b,
        pubkey_b,
        message_type,
        protocol_version,
        schema_version,
    )?;
    Ok(())
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
    channel_party_a: &str,
    channel_party_b: &str,
    agent_id: &str,
) -> Result<()> {
    if agent_id != channel_party_a && agent_id != channel_party_b {
        return Err(Error::ParticipantMismatch);
    }
    let entry = registry.get(agent_id).ok_or(Error::IdentityNotFound)?;
    if entry.status != crate::types::AgentStatus::Active {
        return Err(Error::IdentityNotActive);
    }
    Ok(())
}
