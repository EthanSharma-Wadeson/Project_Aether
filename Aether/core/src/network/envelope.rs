//! MessageEnvelopeV0 — universal authenticated transport container.

use ciborium::value::Value;
use ed25519_dalek::SigningKey;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, bytes, encode_value, map, map_get, text, u32_value,
    u64_value,
};
use crate::crypto::sha256;
use crate::crypto::signing::{sign_body, DOMAIN_TAG};
use crate::crypto::verify::SignedMessage;
use crate::error::{Error, Result};
use crate::identity::registry::IdentityRegistry;
use crate::network::model::{SessionStatus, ENVELOPE_VERSION, NETWORK_SCHEMA_VERSION};
use crate::network::session::SessionStore;
use crate::network::verify::{require_active_identity, verify_envelope_signature};
use crate::types::AgentId;

pub const MSG_NET_ENVELOPE: &str = "net.envelope";

/// Transport envelope. Signature proves sender key control over these fields only.
///
/// Does **not** prove payload correctness, business validity, or economic authorisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEnvelopeV0 {
    pub envelope_version: u32,
    pub protocol_version: u32,
    pub schema_version: u32,
    pub session_id: [u8; 32],
    pub sender_agent_id: AgentId,
    pub receiver_agent_id: AgentId,
    pub message_type: String,
    pub message_id: [u8; 32],
    pub timestamp: u64,
    pub payload_commitment: [u8; 32],
}

impl MessageEnvelopeV0 {
    pub fn body_for_signing(&self) -> Value {
        map(vec![
            (text("envelope_version"), u32_value(self.envelope_version)),
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("session_id"), bytes(self.session_id.to_vec())),
            (text("sender_agent_id"), text(self.sender_agent_id.clone())),
            (
                text("receiver_agent_id"),
                text(self.receiver_agent_id.clone()),
            ),
            (text("message_type"), text(self.message_type.clone())),
            (text("message_id"), bytes(self.message_id.to_vec())),
            (text("timestamp"), u64_value(self.timestamp)),
            (
                text("payload_commitment"),
                bytes(self.payload_commitment.to_vec()),
            ),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.body_for_signing())
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("envelope must be map"));
        };
        let session_id: [u8; 32] = as_bytes(map_get(&entries, "session_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("session_id"))?;
        let message_id: [u8; 32] = as_bytes(map_get(&entries, "message_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("message_id"))?;
        let payload_commitment: [u8; 32] = as_bytes(map_get(&entries, "payload_commitment")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("payload_commitment"))?;
        Ok(Self {
            envelope_version: as_u32(map_get(&entries, "envelope_version")?)?,
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            session_id,
            sender_agent_id: as_text(map_get(&entries, "sender_agent_id")?)?.into(),
            receiver_agent_id: as_text(map_get(&entries, "receiver_agent_id")?)?.into(),
            message_type: as_text(map_get(&entries, "message_type")?)?.into(),
            message_id,
            timestamp: as_u64(map_get(&entries, "timestamp")?)?,
            payload_commitment,
        })
    }

    pub fn commit_payload(payload: &[u8]) -> [u8; 32] {
        sha256(payload)
    }

    pub fn derive_message_id(
        session_id: &[u8; 32],
        sender: &str,
        counter: u64,
        payload_commitment: &[u8; 32],
    ) -> Result<[u8; 32]> {
        let body = map(vec![
            (text("session_id"), bytes(session_id.to_vec())),
            (text("sender"), text(sender)),
            (text("counter"), u64_value(counter)),
            (
                text("payload_commitment"),
                bytes(payload_commitment.to_vec()),
            ),
        ]);
        Ok(sha256(&encode_value(&body)?))
    }
}

pub fn sign_envelope(
    envelope: &MessageEnvelopeV0,
    signing_key: &SigningKey,
) -> Result<SignedMessage> {
    let body = envelope.encode()?;
    let (_, signature) = sign_body(
        signing_key,
        DOMAIN_TAG,
        envelope.protocol_version,
        envelope.schema_version,
        MSG_NET_ENVELOPE,
        &body,
    );
    Ok(SignedMessage {
        protocol_version: envelope.protocol_version,
        schema_version: envelope.schema_version,
        message_type: MSG_NET_ENVELOPE.into(),
        body,
        signer_key_id: envelope.sender_agent_id.clone(),
        signature: signature.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    })
}

/// Build a signed envelope for an established session.
pub fn make_envelope(
    store: &SessionStore,
    session_id: &[u8; 32],
    sender_agent_id: &str,
    message_type: &str,
    payload: &[u8],
    counter: u64,
    timestamp: u64,
) -> Result<MessageEnvelopeV0> {
    let record = store.get(session_id).ok_or(Error::SessionNotFound)?;
    if record.session.status != SessionStatus::Established {
        return Err(Error::InvalidSessionStatus);
    }
    if sender_agent_id != record.session.local_agent_id {
        return Err(Error::SessionIdentityMismatch);
    }
    let payload_commitment = MessageEnvelopeV0::commit_payload(payload);
    let message_id = MessageEnvelopeV0::derive_message_id(
        session_id,
        sender_agent_id,
        counter,
        &payload_commitment,
    )?;
    Ok(MessageEnvelopeV0 {
        envelope_version: ENVELOPE_VERSION,
        protocol_version: record.session.protocol_version,
        schema_version: NETWORK_SCHEMA_VERSION,
        session_id: *session_id,
        sender_agent_id: sender_agent_id.into(),
        receiver_agent_id: record.session.peer_agent_id.clone(),
        message_type: message_type.into(),
        message_id,
        timestamp,
        payload_commitment,
    })
}

/// Deliver envelope into receiver's session store.
///
/// Validation order: signature → identity → session → semantics → mutate.
pub fn deliver_envelope(
    store: &mut SessionStore,
    registry: &IdentityRegistry,
    signed: &SignedMessage,
    expected_receiver: &str,
    payload: &[u8],
    now: u64,
) -> Result<MessageEnvelopeV0> {
    // 1. Signature
    let envelope = verify_envelope_signature(signed, registry)?;
    if envelope.encode()? != signed.body {
        return Err(Error::MalformedObject("non-canonical envelope"));
    }

    // 2. Identity
    require_active_identity(registry, &envelope.sender_agent_id)?;
    require_active_identity(registry, &envelope.receiver_agent_id)?;
    if envelope.receiver_agent_id != expected_receiver {
        return Err(Error::EnvelopeReceiverMismatch);
    }

    // 3. Session state
    let mut record = store
        .get(&envelope.session_id)
        .ok_or(Error::SessionNotFound)?
        .clone();
    record.session.check_not_expired(now)?;
    if !record.session.status.allows_envelope() {
        return Err(Error::InvalidSessionStatus);
    }

    // Session identity binding
    if envelope.sender_agent_id != record.session.peer_agent_id
        || envelope.receiver_agent_id != record.session.local_agent_id
    {
        // Also allow if this store view is the sender's copy (local=sender).
        let sender_ok = envelope.sender_agent_id == record.session.local_agent_id
            && envelope.receiver_agent_id == record.session.peer_agent_id;
        let receiver_ok = envelope.sender_agent_id == record.session.peer_agent_id
            && envelope.receiver_agent_id == record.session.local_agent_id;
        if !sender_ok && !receiver_ok {
            return Err(Error::SessionIdentityMismatch);
        }
    }

    // 4. Message semantics
    if envelope.envelope_version != ENVELOPE_VERSION {
        return Err(Error::UnsupportedProtocolVersion);
    }
    let expected_commitment = MessageEnvelopeV0::commit_payload(payload);
    if envelope.payload_commitment != expected_commitment {
        return Err(Error::PayloadCommitmentMismatch);
    }
    if record.seen_message_ids.contains(&envelope.message_id) {
        return Err(Error::DuplicateMessageId);
    }

    // 5. Mutate
    record.seen_message_ids.insert(envelope.message_id);
    store.insert_record(envelope.session_id, record);
    Ok(envelope)
}

/// Verify envelope and return decoded body without mutating store (inspection helper).
pub fn verify_and_open_envelope(
    store: &SessionStore,
    registry: &IdentityRegistry,
    signed: &SignedMessage,
    expected_receiver: &str,
    payload: &[u8],
    now: u64,
) -> Result<MessageEnvelopeV0> {
    let envelope = verify_envelope_signature(signed, registry)?;
    require_active_identity(registry, &envelope.sender_agent_id)?;
    if envelope.receiver_agent_id != expected_receiver {
        return Err(Error::EnvelopeReceiverMismatch);
    }
    let record = store
        .get(&envelope.session_id)
        .ok_or(Error::SessionNotFound)?;
    record.session.check_not_expired(now)?;
    if !record.session.status.allows_envelope() {
        return Err(Error::InvalidSessionStatus);
    }
    if MessageEnvelopeV0::commit_payload(payload) != envelope.payload_commitment {
        return Err(Error::PayloadCommitmentMismatch);
    }
    if record.seen_message_ids.contains(&envelope.message_id) {
        return Err(Error::DuplicateMessageId);
    }
    Ok(envelope)
}
