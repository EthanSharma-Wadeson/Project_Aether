//! Commitment-chain validation for dispute evidence (PROTO-1 remediation).

use std::collections::BTreeMap;

use crate::channel::model::ChannelStatus;
use crate::channel::model::StateUpdateV0;
use crate::channel::state::ChannelStateV0;
use crate::channel::transition::ChannelRecord;
use crate::error::{Error, Result};

/// Whether `update` links to a parent state already in `known` (or genesis rules at seq 0).
pub fn chains_from_known(
    update: &StateUpdateV0,
    record: &ChannelRecord,
    known: &BTreeMap<u64, ChannelStateV0>,
) -> Result<bool> {
    let seq = update.sequence;
    if seq == 0 {
        let ok = update.previous_state_commitment == [0u8; 32]
            && update.new_state.balances == record.channel.opening_balances
            && update.new_state.asset == record.channel.asset
            && update.new_state.channel_id == record.channel.channel_id
            && update.new_state.sequence == 0;
        return Ok(ok);
    }
    let prev_seq = seq - 1;
    let Some(parent) = known.get(&prev_seq) else {
        return Ok(false);
    };
    let parent_commitment = parent.commitment()?;
    Ok(update.previous_state_commitment == parent_commitment)
}

/// Collect all updates that form a valid commitment chain from the applied baseline.
pub fn collect_chained_states(
    record: &ChannelRecord,
    updates: &[StateUpdateV0],
) -> Result<BTreeMap<u64, ChannelStateV0>> {
    let mut known: BTreeMap<u64, ChannelStateV0> = BTreeMap::new();
    if let Some(state) = &record.latest_state {
        known.insert(state.sequence, state.clone());
    }

    let mut pending: Vec<StateUpdateV0> = updates.to_vec();
    loop {
        let mut progress = false;
        let mut remaining = Vec::new();
        for update in pending {
            if known.contains_key(&update.sequence) {
                continue;
            }
            if chains_from_known(&update, record, &known)? {
                known.insert(update.sequence, update.new_state.clone());
                progress = true;
            } else {
                remaining.push(update);
            }
        }
        if !progress {
            break;
        }
        pending = remaining;
    }
    Ok(known)
}

/// Direct single-hop extension from the channel's currently applied state (raise path).
pub fn verify_direct_extension(record: &ChannelRecord, update: &StateUpdateV0) -> Result<()> {
    if update.sequence != record.channel.sequence + 1 {
        return Err(Error::SequenceSkip);
    }
    if update.previous_state_commitment != record.channel.current_state_commitment {
        return Err(Error::StateCommitmentMismatch);
    }
    if update.sequence != update.new_state.sequence {
        return Err(Error::MalformedObject("sequence mismatch"));
    }
    if !matches!(
        update.new_state.status_hint,
        ChannelStatus::Active | ChannelStatus::Closing
    ) {
        return Err(Error::UnauthorizedTransition);
    }
    Ok(())
}

/// Highest chained sequence; deterministic tie-break at equal sequence.
pub fn select_highest_state(
    known: &BTreeMap<u64, ChannelStateV0>,
    applied: Option<&ChannelStateV0>,
) -> Option<ChannelStateV0> {
    let max_seq = *known.keys().max()?;
    let mut candidates: Vec<&ChannelStateV0> =
        known.values().filter(|s| s.sequence == max_seq).collect();
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() == 1 {
        return Some(candidates[0].clone());
    }
    if let Some(applied) = applied {
        if applied.sequence == max_seq {
            if let Ok(c) = applied.commitment() {
                for s in &candidates {
                    if s.commitment().ok() == Some(c) {
                        return Some((*s).clone());
                    }
                }
            }
        }
    }
    candidates.sort_by_key(|s| s.commitment().unwrap_or([0u8; 32]));
    candidates.last().map(|s| (*s).clone())
}
