//! SecureSessionV0 — authenticated session state machine (local simulation).

use std::collections::{HashMap, HashSet};

use crate::cbor::{encode_value, map, text, u32_value, u64_value};
use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::network::model::SessionStatus;
use crate::types::AgentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecureSessionV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub session_id: [u8; 32],
    pub local_agent_id: AgentId,
    pub peer_agent_id: AgentId,
    pub status: SessionStatus,
    pub local_hello_nonce: u64,
    pub peer_hello_nonce: Option<u64>,
    pub created_at: u64,
    pub established_at: Option<u64>,
    pub expires_at: u64,
    pub negotiated_features: Vec<String>,
}

impl SecureSessionV0 {
    /// Deterministic session id from both parties' hellos.
    ///
    /// Agents are ordered lexicographically so either side derives the same id.
    pub fn derive_session_id(
        agent_a: &str,
        agent_b: &str,
        nonce_a: u64,
        nonce_b: u64,
        protocol_version: u32,
    ) -> Result<[u8; 32]> {
        let (low, high, nonce_low, nonce_high) = if agent_a <= agent_b {
            (agent_a, agent_b, nonce_a, nonce_b)
        } else {
            (agent_b, agent_a, nonce_b, nonce_a)
        };
        let body = map(vec![
            (text("agent_low"), text(low)),
            (text("agent_high"), text(high)),
            (text("nonce_low"), u64_value(nonce_low)),
            (text("nonce_high"), u64_value(nonce_high)),
            (text("protocol_version"), u32_value(protocol_version)),
        ]);
        Ok(sha256(&encode_value(&body)?))
    }

    pub fn is_participant(&self, agent_id: &str) -> bool {
        agent_id == self.local_agent_id || agent_id == self.peer_agent_id
    }

    pub fn check_not_expired(&self, now: u64) -> Result<()> {
        if self.status == SessionStatus::Closed {
            return Err(Error::InvalidSessionStatus);
        }
        if now > self.expires_at {
            return Err(Error::SessionExpired);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub session: SecureSessionV0,
    pub seen_message_ids: HashSet<[u8; 32]>,
    pub seen_nonces: HashSet<u64>,
}

#[derive(Debug, Default)]
pub struct SessionStore {
    pub(crate) sessions: HashMap<[u8; 32], SessionRecord>,
    /// Pending initiator sessions keyed by (local, peer, local_nonce).
    pub(crate) pending: HashMap<(AgentId, AgentId, u64), SessionRecord>,
    /// Global hello-nonce replay set (simulation-wide).
    pub(crate) global_nonces: HashSet<u64>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, session_id: &[u8; 32]) -> Option<&SessionRecord> {
        self.sessions.get(session_id)
    }

    pub fn get_pending(&self, local: &str, peer: &str, local_nonce: u64) -> Option<&SessionRecord> {
        self.pending.get(&(local.into(), peer.into(), local_nonce))
    }

    pub(crate) fn insert_record(&mut self, session_id: [u8; 32], record: SessionRecord) {
        self.sessions.insert(session_id, record);
    }

    pub(crate) fn insert_pending(
        &mut self,
        local: &str,
        peer: &str,
        local_nonce: u64,
        record: SessionRecord,
    ) {
        self.pending
            .insert((local.into(), peer.into(), local_nonce), record);
    }

    pub(crate) fn take_pending(
        &mut self,
        local: &str,
        peer: &str,
        local_nonce: u64,
    ) -> Option<SessionRecord> {
        self.pending
            .remove(&(local.into(), peer.into(), local_nonce))
    }

    pub fn nonce_seen(&self, nonce: u64) -> bool {
        self.global_nonces.contains(&nonce)
    }

    pub(crate) fn mark_global_nonce(&mut self, nonce: u64) {
        self.global_nonces.insert(nonce);
    }
}

/// Create a local session shell in `Created` status (pre-hello).
pub fn create_session(
    local_agent_id: &str,
    peer_agent_id: &str,
    protocol_version: u32,
    schema_version: u32,
    now: u64,
    ttl: u64,
) -> Result<SecureSessionV0> {
    if local_agent_id == peer_agent_id {
        return Err(Error::SessionIdentityMismatch);
    }
    Ok(SecureSessionV0 {
        protocol_version,
        schema_version,
        session_id: [0u8; 32],
        local_agent_id: local_agent_id.into(),
        peer_agent_id: peer_agent_id.into(),
        status: SessionStatus::Created,
        local_hello_nonce: 0,
        peer_hello_nonce: None,
        created_at: now,
        established_at: None,
        expires_at: now.checked_add(ttl).ok_or(Error::SessionExpired)?,
        negotiated_features: Vec::new(),
    })
}

/// Close an established (or any non-terminal) session.
pub fn close_session(
    store: &mut SessionStore,
    session_id: &[u8; 32],
    actor_id: &str,
    now: u64,
) -> Result<SecureSessionV0> {
    let mut record = store.get(session_id).ok_or(Error::SessionNotFound)?.clone();
    if record.session.status.is_terminal() {
        return Err(Error::InvalidSessionStatus);
    }
    if !record.session.is_participant(actor_id) {
        return Err(Error::SessionIdentityMismatch);
    }
    let _ = now;
    record.session.status = SessionStatus::Closed;
    let session = record.session.clone();
    store.insert_record(*session_id, record);
    Ok(session)
}
