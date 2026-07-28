//! Channel state transitions (open / activate / update / close / abort).

#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;

use crate::capability::grant::CapabilityGrant;
use crate::capability::model::CapabilityStore;
use crate::cbor::{bytes, encode_value, map, text, u32_value, u64_value};
use crate::channel::model::{
    ChannelOpenMaterialV0, ChannelStatus, ChannelV0, DualSignedUpdate, FinalityViewV0, ReceiptV0,
    StateUpdateV0, ACTION_ACTIVATE, ACTION_CLOSE, ACTION_OPEN, ACTION_UPDATE, MSG_CHANNEL_ACTIVATE,
    MSG_CHANNEL_CLOSE, MSG_CHANNEL_OPEN, MSG_CHANNEL_UPDATE,
};
use crate::channel::state::ChannelStateV0;
use crate::channel::verify::{
    require_active_participant, require_capability, verify_dual_signed, verify_open_pair,
};
use crate::crypto::sha256;
use crate::crypto::verify::SignedMessage;
use crate::error::{Error, Result};
use crate::identity::registry::IdentityRegistry;

#[derive(Debug, Default)]
pub struct ChannelStore {
    pub(crate) channels: HashMap<[u8; 32], ChannelRecord>,
}

#[derive(Debug, Clone)]
pub struct ChannelRecord {
    pub channel: ChannelV0,
    pub latest_state: Option<ChannelStateV0>,
    pub open_material: ChannelOpenMaterialV0,
}

impl ChannelStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, channel_id: &[u8; 32]) -> Option<&ChannelRecord> {
        self.channels.get(channel_id)
    }

    /// Internal mutation only — channel state must change via transition functions.
    pub(crate) fn insert_record(&mut self, channel_id: [u8; 32], record: ChannelRecord) {
        self.channels.insert(channel_id, record);
    }
}

fn party_keys(
    registry: &IdentityRegistry,
    party_a: &str,
    party_b: &str,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let a = registry.get(party_a).ok_or(Error::IdentityNotFound)?;
    let b = registry.get(party_b).ok_or(Error::IdentityNotFound)?;
    Ok((
        a.identity.operational_public_key.clone(),
        b.identity.operational_public_key.clone(),
    ))
}

fn make_receipt(
    channel: &ChannelV0,
    action: &str,
    sequence: u64,
    state_commitment: [u8; 32],
    previous: Option<[u8; 32]>,
    logical_time: u64,
    accepted: bool,
    sig_a: &[u8],
    sig_b: &[u8],
) -> Result<ReceiptV0> {
    let body = map(vec![
        (
            text("protocol_version"),
            u32_value(channel.protocol_version),
        ),
        (text("schema_version"), u32_value(channel.schema_version)),
        (text("channel_id"), bytes(channel.channel_id.to_vec())),
        (text("action"), text(action)),
        (text("sequence"), u64_value(sequence)),
        (text("state_commitment"), bytes(state_commitment.to_vec())),
        (text("logical_time"), u64_value(logical_time)),
        (text("accepted"), u64_value(if accepted { 1 } else { 0 })),
    ]);
    let receipt_id = sha256(&encode_value(&body)?);
    Ok(ReceiptV0 {
        protocol_version: channel.protocol_version,
        schema_version: channel.schema_version,
        receipt_id,
        channel_id: channel.channel_id,
        action: action.into(),
        participants: [channel.party_a.clone(), channel.party_b.clone()],
        sequence,
        state_commitment,
        previous_state_commitment: previous,
        logical_time,
        finality_snapshot: channel.finality.clone(),
        accepted,
        signature_a: sig_a.to_vec(),
        signature_b: sig_b.to_vec(),
    })
}

/// Open channel after dual-signed open material + capability checks.
///
/// Validation order: signatures → membership/status → capability → transition rules.
pub fn open_channel(
    store: &mut ChannelStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    material: &ChannelOpenMaterialV0,
    sig_a: &SignedMessage,
    sig_b: &SignedMessage,
    grant_a: &CapabilityGrant,
    grant_b: &CapabilityGrant,
    now: u64,
) -> Result<ChannelV0> {
    if material.party_a == material.party_b {
        return Err(Error::ParticipantMismatch);
    }
    let total = material.total_deposit();
    if total != material.opening_balances[0] + material.opening_balances[1] {
        return Err(Error::BalanceConservation);
    }

    let body = material.encode()?;
    let (pk_a, pk_b) = party_keys(registry, &material.party_a, &material.party_b)?;
    verify_open_pair(
        &body,
        sig_a,
        sig_b,
        &pk_a,
        &pk_b,
        MSG_CHANNEL_OPEN,
        material.protocol_version,
        material.schema_version,
    )?;

    require_active_participant(
        registry,
        &material.party_a,
        &material.party_b,
        &material.party_a,
    )?;
    require_active_participant(
        registry,
        &material.party_a,
        &material.party_b,
        &material.party_b,
    )?;
    require_capability(registry, caps, &material.party_a, grant_a, ACTION_OPEN, now)?;
    require_capability(registry, caps, &material.party_b, grant_b, ACTION_OPEN, now)?;

    let channel_id = material.channel_id()?;
    if store.get(&channel_id).is_some() {
        return Err(Error::ChannelAlreadyExists);
    }

    let channel = ChannelV0 {
        protocol_version: material.protocol_version,
        schema_version: material.schema_version,
        channel_id,
        party_a: material.party_a.clone(),
        party_b: material.party_b.clone(),
        asset: material.asset.clone(),
        opening_balances: material.opening_balances,
        total_deposit: total,
        dispute_window: material.dispute_window,
        created_at: material.created_at,
        status: ChannelStatus::Open,
        sequence: 0,
        current_state_commitment: [0u8; 32],
        finality: FinalityViewV0::for_status(ChannelStatus::Open, false, None),
    };

    store.insert_record(
        channel_id,
        ChannelRecord {
            channel: channel.clone(),
            latest_state: None,
            open_material: material.clone(),
        },
    );
    Ok(channel)
}

/// Activate: dual-signed sequence=0 initial state matching opening balances.
pub fn activate_channel(
    store: &mut ChannelStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    channel_id: &[u8; 32],
    dual: &DualSignedUpdate,
    grant_a: &CapabilityGrant,
    grant_b: &CapabilityGrant,
    now: u64,
) -> Result<(ChannelV0, ReceiptV0)> {
    let record = store
        .channels
        .get(channel_id)
        .ok_or(Error::ChannelNotFound)?
        .clone();
    if record.channel.status != ChannelStatus::Open {
        return Err(Error::InvalidChannelStatus);
    }

    let (pk_a, pk_b) = party_keys(registry, &record.channel.party_a, &record.channel.party_b)?;
    let update = verify_dual_signed(
        dual,
        &pk_a,
        &pk_b,
        MSG_CHANNEL_ACTIVATE,
        record.channel.protocol_version,
        record.channel.schema_version,
    )?;

    require_capability(
        registry,
        caps,
        &record.channel.party_a,
        grant_a,
        ACTION_ACTIVATE,
        now,
    )?;
    require_capability(
        registry,
        caps,
        &record.channel.party_b,
        grant_b,
        ACTION_ACTIVATE,
        now,
    )?;

    if update.channel_id != *channel_id {
        return Err(Error::ParticipantMismatch);
    }
    if update.sequence != 0 || update.new_state.sequence != 0 {
        return Err(Error::SequenceSkip);
    }
    if update.new_state.balances != record.channel.opening_balances {
        return Err(Error::BalanceConservation);
    }
    if update.new_state.asset != record.channel.asset {
        return Err(Error::BalanceConservation);
    }
    if update.new_state.channel_id != *channel_id {
        return Err(Error::ParticipantMismatch);
    }
    if update.new_state.status_hint != ChannelStatus::Active {
        return Err(Error::UnauthorizedTransition);
    }
    // previous commitment for activate is zeroed
    if update.previous_state_commitment != [0u8; 32] {
        return Err(Error::StateCommitmentMismatch);
    }

    let commitment = update.new_state.commitment()?;
    let mut channel = record.channel;
    channel.status = ChannelStatus::Active;
    channel.sequence = 0;
    channel.current_state_commitment = commitment;
    channel.finality = FinalityViewV0::for_status(ChannelStatus::Active, true, None);

    let receipt = make_receipt(
        &channel,
        ACTION_ACTIVATE,
        0,
        commitment,
        Some([0u8; 32]),
        update.logical_time,
        true,
        &dual.sig_a.signature,
        &dual.sig_b.signature,
    )?;

    store.insert_record(
        *channel_id,
        ChannelRecord {
            channel: channel.clone(),
            latest_state: Some(update.new_state),
            open_material: record.open_material,
        },
    );
    Ok((channel, receipt))
}

/// Apply dual-signed update while Active (or allow close-bound updates via status_hint).
pub fn apply_update(
    store: &mut ChannelStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    channel_id: &[u8; 32],
    dual: &DualSignedUpdate,
    grant_a: &CapabilityGrant,
    grant_b: &CapabilityGrant,
    now: u64,
) -> Result<(ChannelV0, ReceiptV0)> {
    let record = store
        .channels
        .get(channel_id)
        .ok_or(Error::ChannelNotFound)?
        .clone();
    if record.channel.status != ChannelStatus::Active {
        return Err(Error::InvalidChannelStatus);
    }

    let (pk_a, pk_b) = party_keys(registry, &record.channel.party_a, &record.channel.party_b)?;
    let update = verify_dual_signed(
        dual,
        &pk_a,
        &pk_b,
        MSG_CHANNEL_UPDATE,
        record.channel.protocol_version,
        record.channel.schema_version,
    )?;

    require_capability(
        registry,
        caps,
        &record.channel.party_a,
        grant_a,
        ACTION_UPDATE,
        now,
    )?;
    require_capability(
        registry,
        caps,
        &record.channel.party_b,
        grant_b,
        ACTION_UPDATE,
        now,
    )?;

    validate_state_transition(&record, &update)?;

    let commitment = update.new_state.commitment()?;
    let prev = record.channel.current_state_commitment;
    let mut channel = record.channel;
    channel.sequence = update.sequence;
    channel.current_state_commitment = commitment;
    channel.finality = FinalityViewV0::for_status(ChannelStatus::Active, true, None);

    let receipt = make_receipt(
        &channel,
        ACTION_UPDATE,
        update.sequence,
        commitment,
        Some(prev),
        update.logical_time,
        true,
        &dual.sig_a.signature,
        &dual.sig_b.signature,
    )?;

    store.insert_record(
        *channel_id,
        ChannelRecord {
            channel: channel.clone(),
            latest_state: Some(update.new_state),
            open_material: record.open_material,
        },
    );
    Ok((channel, receipt))
}

fn validate_state_transition(record: &ChannelRecord, update: &StateUpdateV0) -> Result<()> {
    if update.channel_id != record.channel.channel_id {
        return Err(Error::ParticipantMismatch);
    }
    if update.previous_state_commitment != record.channel.current_state_commitment {
        return Err(Error::StateCommitmentMismatch);
    }
    if update.sequence != update.new_state.sequence {
        return Err(Error::MalformedObject("sequence mismatch"));
    }
    if update.sequence <= record.channel.sequence {
        return Err(Error::SequenceStale);
    }
    if update.sequence != record.channel.sequence + 1 {
        return Err(Error::SequenceSkip);
    }
    let sum = update.new_state.balances[0].saturating_add(update.new_state.balances[1]);
    if sum != record.channel.total_deposit {
        return Err(Error::BalanceConservation);
    }
    if update.new_state.asset != record.channel.asset {
        return Err(Error::BalanceConservation);
    }
    if update.new_state.channel_id != record.channel.channel_id {
        return Err(Error::ParticipantMismatch);
    }
    if !matches!(
        update.new_state.status_hint,
        ChannelStatus::Active | ChannelStatus::Closing
    ) {
        return Err(Error::UnauthorizedTransition);
    }
    Ok(())
}

/// Begin cooperative close: dual-signed close on latest state → Closing.
pub fn begin_close(
    store: &mut ChannelStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    channel_id: &[u8; 32],
    dual: &DualSignedUpdate,
    grant_a: &CapabilityGrant,
    grant_b: &CapabilityGrant,
    now: u64,
) -> Result<ChannelV0> {
    let record = store
        .channels
        .get(channel_id)
        .ok_or(Error::ChannelNotFound)?
        .clone();
    if record.channel.status != ChannelStatus::Active {
        return Err(Error::InvalidClose);
    }

    let (pk_a, pk_b) = party_keys(registry, &record.channel.party_a, &record.channel.party_b)?;
    let update = verify_dual_signed(
        dual,
        &pk_a,
        &pk_b,
        MSG_CHANNEL_CLOSE,
        record.channel.protocol_version,
        record.channel.schema_version,
    )?;

    require_capability(
        registry,
        caps,
        &record.channel.party_a,
        grant_a,
        ACTION_CLOSE,
        now,
    )?;
    require_capability(
        registry,
        caps,
        &record.channel.party_b,
        grant_b,
        ACTION_CLOSE,
        now,
    )?;

    // Close must reference latest accepted balances/sequence; status_hint may move to Closing
    // (which changes the state commitment by design).
    if update.channel_id != *channel_id {
        return Err(Error::ParticipantMismatch);
    }
    if update.previous_state_commitment != record.channel.current_state_commitment {
        return Err(Error::StateCommitmentMismatch);
    }
    if update.sequence != record.channel.sequence
        || update.new_state.sequence != record.channel.sequence
    {
        return Err(Error::InvalidClose);
    }
    let Some(latest) = &record.latest_state else {
        return Err(Error::InvalidClose);
    };
    if update.new_state.balances != latest.balances
        || update.new_state.asset != latest.asset
        || update.new_state.channel_id != latest.channel_id
    {
        return Err(Error::InvalidClose);
    }
    if update.new_state.status_hint != ChannelStatus::Closing {
        return Err(Error::UnauthorizedTransition);
    }

    let mut channel = record.channel;
    channel.status = ChannelStatus::Closing;
    let deadline = now.saturating_add(channel.dispute_window);
    channel.finality = FinalityViewV0::for_status(ChannelStatus::Closing, true, Some(deadline));

    store.insert_record(
        *channel_id,
        ChannelRecord {
            channel: channel.clone(),
            latest_state: record.latest_state,
            open_material: record.open_material,
        },
    );
    Ok(channel)
}

/// Finalize cooperative close from Closing → Finalized.
///
/// Trust boundary: both parties must still hold `channel.close` capability;
/// dispute window must have elapsed when a deadline is set on the channel.
pub fn finalize_close(
    store: &mut ChannelStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    channel_id: &[u8; 32],
    grant_a: &CapabilityGrant,
    grant_b: &CapabilityGrant,
    now: u64,
) -> Result<ChannelV0> {
    let record = store.get(channel_id).ok_or(Error::ChannelNotFound)?.clone();
    if record.channel.status != ChannelStatus::Closing {
        return Err(Error::InvalidClose);
    }

    require_capability(
        registry,
        caps,
        &record.channel.party_a,
        grant_a,
        ACTION_CLOSE,
        now,
    )
    .map_err(|_| Error::UntrustedTerminalOperation)?;
    require_capability(
        registry,
        caps,
        &record.channel.party_b,
        grant_b,
        ACTION_CLOSE,
        now,
    )
    .map_err(|_| Error::UntrustedTerminalOperation)?;

    if let Some(deadline) = record.channel.finality.dispute_deadline {
        if now < deadline {
            return Err(Error::DisputeWindowOpen);
        }
    }

    let mut channel = record.channel;
    channel.status = ChannelStatus::Finalized;
    channel.finality = FinalityViewV0::for_status(ChannelStatus::Finalized, true, None);
    store.insert_record(
        *channel_id,
        ChannelRecord {
            channel: channel.clone(),
            latest_state: record.latest_state,
            open_material: record.open_material,
        },
    );
    Ok(channel)
}

/// Abort Open → Finalized with dual-signed abort (reuse close message type on open material body).
pub fn abort_open(
    store: &mut ChannelStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    channel_id: &[u8; 32],
    sig_a: &SignedMessage,
    sig_b: &SignedMessage,
    grant_a: &CapabilityGrant,
    grant_b: &CapabilityGrant,
    now: u64,
) -> Result<ChannelV0> {
    let record = store
        .channels
        .get(channel_id)
        .ok_or(Error::ChannelNotFound)?
        .clone();
    if record.channel.status != ChannelStatus::Open {
        return Err(Error::InvalidChannelStatus);
    }

    let body = record.open_material.encode()?;
    let (pk_a, pk_b) = party_keys(registry, &record.channel.party_a, &record.channel.party_b)?;
    verify_open_pair(
        &body,
        sig_a,
        sig_b,
        &pk_a,
        &pk_b,
        MSG_CHANNEL_CLOSE,
        record.channel.protocol_version,
        record.channel.schema_version,
    )?;

    require_capability(
        registry,
        caps,
        &record.channel.party_a,
        grant_a,
        ACTION_CLOSE,
        now,
    )?;
    require_capability(
        registry,
        caps,
        &record.channel.party_b,
        grant_b,
        ACTION_CLOSE,
        now,
    )?;

    let mut channel = record.channel;
    channel.status = ChannelStatus::Finalized;
    channel.finality = FinalityViewV0::for_status(ChannelStatus::Finalized, true, None);
    store.insert_record(
        *channel_id,
        ChannelRecord {
            channel: channel.clone(),
            latest_state: None,
            open_material: record.open_material,
        },
    );
    Ok(channel)
}

/// Helper to build a dual-signed update from two signing keys (test/runtime convenience).
pub fn sign_update_dual(
    update: &StateUpdateV0,
    key_a: &ed25519_dalek::SigningKey,
    key_b: &ed25519_dalek::SigningKey,
    message_type: &str,
) -> Result<DualSignedUpdate> {
    use crate::crypto::signing::{sign_body, DOMAIN_TAG};
    let body = update.encode()?;
    let (_d, sa) = sign_body(
        key_a,
        DOMAIN_TAG,
        update.protocol_version,
        update.schema_version,
        message_type,
        &body,
    );
    let (_d, sb) = sign_body(
        key_b,
        DOMAIN_TAG,
        update.protocol_version,
        update.schema_version,
        message_type,
        &body,
    );
    Ok(DualSignedUpdate {
        body: body.clone(),
        sig_a: SignedMessage {
            protocol_version: update.protocol_version,
            schema_version: update.schema_version,
            message_type: message_type.into(),
            body: body.clone(),
            signer_key_id: "operational:0".into(),
            signature: sa.to_bytes().to_vec(),
            domain_tag: DOMAIN_TAG.into(),
        },
        sig_b: SignedMessage {
            protocol_version: update.protocol_version,
            schema_version: update.schema_version,
            message_type: message_type.into(),
            body,
            signer_key_id: "operational:0".into(),
            signature: sb.to_bytes().to_vec(),
            domain_tag: DOMAIN_TAG.into(),
        },
    })
}

pub fn sign_open_dual(
    material: &ChannelOpenMaterialV0,
    key_a: &ed25519_dalek::SigningKey,
    key_b: &ed25519_dalek::SigningKey,
    message_type: &str,
) -> Result<(SignedMessage, SignedMessage)> {
    use crate::crypto::signing::{sign_body, DOMAIN_TAG};
    let body = material.encode()?;
    let (_d, sa) = sign_body(
        key_a,
        DOMAIN_TAG,
        material.protocol_version,
        material.schema_version,
        message_type,
        &body,
    );
    let (_d, sb) = sign_body(
        key_b,
        DOMAIN_TAG,
        material.protocol_version,
        material.schema_version,
        message_type,
        &body,
    );
    let msg = |sig: ed25519_dalek::Signature| SignedMessage {
        protocol_version: material.protocol_version,
        schema_version: material.schema_version,
        message_type: message_type.into(),
        body: body.clone(),
        signer_key_id: "operational:0".into(),
        signature: sig.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    };
    Ok((msg(sa), msg(sb)))
}
