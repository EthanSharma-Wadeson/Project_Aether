//! Minimal dispute resolution — highest valid dual-signed state wins.
//!
//! Dispute evidence must form a valid commitment chain from the applied baseline.
//! Terminal resolution requires PROTO-0 `channel.dispute` capability.

#![allow(clippy::too_many_arguments)]

use crate::capability::grant::CapabilityGrant;
use crate::capability::model::CapabilityStore;
use crate::channel::chain::{
    collect_chained_states, select_highest_state, verify_direct_extension,
};
use crate::channel::model::{
    ChannelStatus, ChannelV0, DualSignedUpdate, FinalityViewV0, ACTION_DISPUTE, MSG_CHANNEL_UPDATE,
};
use crate::channel::state::ChannelStateV0;
use crate::channel::transition::{ChannelRecord, ChannelStore};
use crate::channel::verify::{require_capability, verify_dual_signed};
use crate::error::{Error, Result};
use crate::identity::registry::IdentityRegistry;

#[derive(Debug, Clone)]
pub struct DisputeEvidenceV0 {
    pub channel_id: [u8; 32],
    pub claimed_update: DualSignedUpdate,
    pub logical_time_raised: u64,
}

fn validate_update_semantics(
    update: &crate::channel::model::StateUpdateV0,
    record: &ChannelRecord,
) -> Result<()> {
    if update.channel_id != record.channel.channel_id {
        return Err(Error::InvalidDisputeEvidence);
    }
    let sum = update.new_state.balances[0].saturating_add(update.new_state.balances[1]);
    if sum != record.channel.total_deposit || update.new_state.asset != record.channel.asset {
        return Err(Error::InvalidDisputeEvidence);
    }
    if update.sequence != update.new_state.sequence {
        return Err(Error::InvalidDisputeEvidence);
    }
    Ok(())
}

/// Raise dispute from Active or Closing with admissible dual-signed evidence.
///
/// Evidence must be a direct commitment-chain extension of the applied state.
/// Validation order: signatures → membership → capability → chain/semantics.
pub fn raise_dispute(
    store: &mut ChannelStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    evidence: &DisputeEvidenceV0,
    raiser_id: &str,
    grant: &CapabilityGrant,
    now: u64,
) -> Result<ChannelV0> {
    let record = store
        .get(&evidence.channel_id)
        .ok_or(Error::ChannelNotFound)?
        .clone();

    if !matches!(
        record.channel.status,
        ChannelStatus::Active | ChannelStatus::Closing
    ) {
        return Err(Error::UnauthorizedTransition);
    }

    if raiser_id != record.channel.party_a && raiser_id != record.channel.party_b {
        return Err(Error::ParticipantMismatch);
    }

    let a = registry
        .get(&record.channel.party_a)
        .ok_or(Error::IdentityNotFound)?;
    let b = registry
        .get(&record.channel.party_b)
        .ok_or(Error::IdentityNotFound)?;

    let update = verify_dual_signed(
        &evidence.claimed_update,
        &a.identity.operational_public_key,
        &b.identity.operational_public_key,
        MSG_CHANNEL_UPDATE,
        record.channel.protocol_version,
        record.channel.schema_version,
    )
    .map_err(|_| Error::InvalidDisputeEvidence)?;

    require_capability(registry, caps, raiser_id, grant, ACTION_DISPUTE, now)?;

    if update.channel_id != evidence.channel_id {
        return Err(Error::InvalidDisputeEvidence);
    }
    validate_update_semantics(&update, &record)?;
    verify_direct_extension(&record, &update)?;

    let _ = evidence.logical_time_raised;

    let mut channel = record.channel;
    channel.status = ChannelStatus::Disputed;
    let deadline = now.saturating_add(channel.dispute_window);
    channel.finality = FinalityViewV0::for_status(ChannelStatus::Disputed, false, Some(deadline));

    let commitment = update.new_state.commitment()?;
    let sequence = update.sequence;
    let latest = Some(update.new_state.clone());

    channel.sequence = sequence;
    channel.current_state_commitment = commitment;

    store.insert_record(
        evidence.channel_id,
        ChannelRecord {
            channel: channel.clone(),
            latest_state: latest,
            open_material: record.open_material,
        },
    );
    Ok(channel)
}

/// Resolve dispute to the highest valid chained dual-signed candidate.
///
/// Requires an authorised participant with `channel.dispute` capability.
/// Candidates with broken commitment chains or skipped sequences are ignored.
pub fn resolve_dispute(
    store: &mut ChannelStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    channel_id: &[u8; 32],
    candidates: &[DualSignedUpdate],
    resolver_id: &str,
    grant: &CapabilityGrant,
    now: u64,
) -> Result<(ChannelV0, ChannelStateV0)> {
    let record = store.get(channel_id).ok_or(Error::ChannelNotFound)?.clone();
    if record.channel.status != ChannelStatus::Disputed {
        return Err(Error::InvalidChannelStatus);
    }

    if resolver_id != record.channel.party_a && resolver_id != record.channel.party_b {
        return Err(Error::UntrustedTerminalOperation);
    }
    require_capability(registry, caps, resolver_id, grant, ACTION_DISPUTE, now)
        .map_err(|_| Error::UntrustedTerminalOperation)?;

    let a = registry
        .get(&record.channel.party_a)
        .ok_or(Error::IdentityNotFound)?;
    let b = registry
        .get(&record.channel.party_b)
        .ok_or(Error::IdentityNotFound)?;

    let mut verified_updates = Vec::new();
    for dual in candidates {
        let Ok(update) = verify_dual_signed(
            dual,
            &a.identity.operational_public_key,
            &b.identity.operational_public_key,
            MSG_CHANNEL_UPDATE,
            record.channel.protocol_version,
            record.channel.schema_version,
        ) else {
            continue;
        };
        if update.channel_id != *channel_id {
            continue;
        }
        if validate_update_semantics(&update, &record).is_err() {
            continue;
        }
        verified_updates.push(update);
    }

    let known = collect_chained_states(&record, &verified_updates)?;
    let state = select_highest_state(&known, record.latest_state.as_ref())
        .ok_or(Error::InvalidDisputeEvidence)?;

    let commitment = state.commitment()?;
    let mut channel = record.channel;
    channel.status = ChannelStatus::Finalized;
    channel.sequence = state.sequence;
    channel.current_state_commitment = commitment;
    channel.finality = FinalityViewV0::for_status(ChannelStatus::Finalized, true, None);

    store.insert_record(
        *channel_id,
        ChannelRecord {
            channel: channel.clone(),
            latest_state: Some(state.clone()),
            open_material: record.open_material,
        },
    );
    let _ = now;
    Ok((channel, state))
}
