//! ProtocolHelloV0 — agent introduction and compatibility negotiation.

use ciborium::value::Value;
use ed25519_dalek::SigningKey;

use crate::cbor::{
    as_text, as_u32, as_u64, encode_value, map, map_get, text, u32_value, u64_value,
};
use crate::crypto::signing::{sign_body, DOMAIN_TAG};
use crate::crypto::verify::SignedMessage;
use crate::error::{Error, Result};
use crate::identity::registry::IdentityRegistry;
use crate::network::model::{
    NetworkFeature, SessionStatus, MIN_PROTOCOL_VERSION, NETWORK_PROTOCOL_VERSION,
    NETWORK_SCHEMA_VERSION,
};
use crate::network::session::{SecureSessionV0, SessionRecord, SessionStore};
use crate::network::verify::{require_active_identity, verify_hello_signature};
use crate::types::AgentId;

pub const MSG_NET_HELLO: &str = "net.hello";
pub const MSG_NET_HELLO_ACCEPT: &str = "net.hello.accept";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolHelloV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub agent_id: AgentId,
    pub supported_features: Vec<String>,
    pub timestamp: u64,
    pub nonce: u64,
}

impl ProtocolHelloV0 {
    pub fn default_features() -> Vec<String> {
        vec![
            NetworkFeature::EnvelopeV0.as_str().into(),
            NetworkFeature::SessionV0.as_str().into(),
        ]
    }

    pub fn to_cbor_value(&self) -> Value {
        let features: Vec<Value> = self
            .supported_features
            .iter()
            .map(|f| text(f.clone()))
            .collect();
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("agent_id"), text(self.agent_id.clone())),
            (text("supported_features"), Value::Array(features)),
            (text("timestamp"), u64_value(self.timestamp)),
            (text("nonce"), u64_value(self.nonce)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("hello must be map"));
        };
        let features = match map_get(&entries, "supported_features")? {
            Value::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(as_text(item)?.to_string());
                }
                out
            }
            _ => return Err(Error::MalformedObject("supported_features")),
        };
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            agent_id: as_text(map_get(&entries, "agent_id")?)?.into(),
            supported_features: features,
            timestamp: as_u64(map_get(&entries, "timestamp")?)?,
            nonce: as_u64(map_get(&entries, "nonce")?)?,
        })
    }

    pub fn validate_version(&self) -> Result<()> {
        if self.protocol_version < MIN_PROTOCOL_VERSION
            || self.protocol_version > NETWORK_PROTOCOL_VERSION
        {
            return Err(Error::UnsupportedProtocolVersion);
        }
        if self.schema_version != NETWORK_SCHEMA_VERSION {
            return Err(Error::UnsupportedProtocolVersion);
        }
        Ok(())
    }
}

pub fn create_hello(
    agent_id: &str,
    timestamp: u64,
    nonce: u64,
    features: Option<Vec<String>>,
) -> ProtocolHelloV0 {
    ProtocolHelloV0 {
        protocol_version: NETWORK_PROTOCOL_VERSION,
        schema_version: NETWORK_SCHEMA_VERSION,
        agent_id: agent_id.into(),
        supported_features: features.unwrap_or_else(ProtocolHelloV0::default_features),
        timestamp,
        nonce,
    }
}

pub fn sign_hello(
    hello: &ProtocolHelloV0,
    signing_key: &SigningKey,
    message_type: &str,
) -> Result<SignedMessage> {
    let body = hello.encode()?;
    if hello.encode()? != body {
        return Err(Error::MalformedObject("non-canonical hello"));
    }
    let (_, signature) = sign_body(
        signing_key,
        DOMAIN_TAG,
        hello.protocol_version,
        hello.schema_version,
        message_type,
        &body,
    );
    Ok(SignedMessage {
        protocol_version: hello.protocol_version,
        schema_version: hello.schema_version,
        message_type: message_type.into(),
        body,
        signer_key_id: hello.agent_id.clone(),
        signature: signature.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    })
}

/// Initiator: Created → HelloSent. Stores pending session keyed by local nonce.
pub fn initiate_hello(
    store: &mut SessionStore,
    registry: &IdentityRegistry,
    local_agent_id: &str,
    peer_agent_id: &str,
    signed_hello: &SignedMessage,
    now: u64,
    ttl: u64,
) -> Result<SecureSessionV0> {
    // 1. Signature
    let hello = verify_hello_signature(signed_hello, registry, MSG_NET_HELLO)?;
    // 2. Identity
    if hello.agent_id != local_agent_id {
        return Err(Error::SessionIdentityMismatch);
    }
    require_active_identity(registry, local_agent_id)?;
    require_active_identity(registry, peer_agent_id)?;
    // 3–4. Semantics
    hello.validate_version()?;
    if store.nonce_seen(hello.nonce) {
        return Err(Error::MessageReplay);
    }
    if store
        .get_pending(local_agent_id, peer_agent_id, hello.nonce)
        .is_some()
    {
        return Err(Error::SessionAlreadyExists);
    }

    let session = SecureSessionV0 {
        protocol_version: hello.protocol_version,
        schema_version: hello.schema_version,
        session_id: [0u8; 32],
        local_agent_id: local_agent_id.into(),
        peer_agent_id: peer_agent_id.into(),
        status: SessionStatus::HelloSent,
        local_hello_nonce: hello.nonce,
        peer_hello_nonce: None,
        created_at: now,
        established_at: None,
        expires_at: now.checked_add(ttl).ok_or(Error::SessionExpired)?,
        negotiated_features: hello.supported_features.clone(),
    };

    let mut seen_nonces = std::collections::HashSet::new();
    seen_nonces.insert(hello.nonce);

    store.insert_pending(
        local_agent_id,
        peer_agent_id,
        hello.nonce,
        SessionRecord {
            session: session.clone(),
            seen_message_ids: Default::default(),
            seen_nonces,
        },
    );
    store.mark_global_nonce(hello.nonce);
    Ok(session)
}

/// Responder: verifies peer hello, signs accept, creates Established session.
pub fn accept_hello(
    store: &mut SessionStore,
    registry: &IdentityRegistry,
    local_agent_id: &str,
    peer_hello: &SignedMessage,
    local_accept: &SignedMessage,
    now: u64,
    ttl: u64,
) -> Result<SecureSessionV0> {
    // 1. Signatures
    let inbound = verify_hello_signature(peer_hello, registry, MSG_NET_HELLO)?;
    let accept = verify_hello_signature(local_accept, registry, MSG_NET_HELLO_ACCEPT)?;
    // 2. Identity
    if accept.agent_id != local_agent_id {
        return Err(Error::SessionIdentityMismatch);
    }
    if inbound.agent_id == local_agent_id {
        return Err(Error::SessionIdentityMismatch);
    }
    require_active_identity(registry, local_agent_id)?;
    require_active_identity(registry, &inbound.agent_id)?;
    // 3–4. Semantics
    inbound.validate_version()?;
    accept.validate_version()?;
    if store.nonce_seen(inbound.nonce) || store.nonce_seen(accept.nonce) {
        return Err(Error::MessageReplay);
    }

    let negotiated = intersect_features(&inbound.supported_features, &accept.supported_features);
    if negotiated.is_empty() {
        return Err(Error::UnsupportedProtocolVersion);
    }

    let session_id = SecureSessionV0::derive_session_id(
        &inbound.agent_id,
        local_agent_id,
        inbound.nonce,
        accept.nonce,
        inbound.protocol_version,
    )?;
    if store.get(&session_id).is_some() {
        return Err(Error::SessionAlreadyExists);
    }

    let session = SecureSessionV0 {
        protocol_version: inbound.protocol_version,
        schema_version: inbound.schema_version,
        session_id,
        local_agent_id: local_agent_id.into(),
        peer_agent_id: inbound.agent_id.clone(),
        status: SessionStatus::Established,
        local_hello_nonce: accept.nonce,
        peer_hello_nonce: Some(inbound.nonce),
        created_at: now,
        established_at: Some(now),
        expires_at: now.checked_add(ttl).ok_or(Error::SessionExpired)?,
        negotiated_features: negotiated,
    };

    let mut seen_nonces = std::collections::HashSet::new();
    seen_nonces.insert(inbound.nonce);
    seen_nonces.insert(accept.nonce);

    store.insert_record(
        session_id,
        SessionRecord {
            session: session.clone(),
            seen_message_ids: Default::default(),
            seen_nonces,
        },
    );
    store.mark_global_nonce(inbound.nonce);
    store.mark_global_nonce(accept.nonce);
    Ok(session)
}

/// Initiator: HelloSent → Established after verifying peer accept.
pub fn complete_hello(
    store: &mut SessionStore,
    registry: &IdentityRegistry,
    local_agent_id: &str,
    peer_agent_id: &str,
    local_hello_nonce: u64,
    peer_accept: &SignedMessage,
    now: u64,
) -> Result<SecureSessionV0> {
    let pending = store
        .take_pending(local_agent_id, peer_agent_id, local_hello_nonce)
        .ok_or(Error::SessionNotFound)?;

    if pending.session.status != SessionStatus::HelloSent {
        return Err(Error::InvalidSessionStatus);
    }
    if now > pending.session.expires_at {
        return Err(Error::SessionExpired);
    }

    // 1. Signature
    let accept = verify_hello_signature(peer_accept, registry, MSG_NET_HELLO_ACCEPT)?;
    // 2. Identity
    if accept.agent_id != peer_agent_id {
        return Err(Error::SessionIdentityMismatch);
    }
    require_active_identity(registry, &accept.agent_id)?;
    // 3–4. Semantics
    accept.validate_version()?;
    if pending.seen_nonces.contains(&accept.nonce) {
        return Err(Error::MessageReplay);
    }

    let negotiated = intersect_features(
        &pending.session.negotiated_features,
        &accept.supported_features,
    );
    if negotiated.is_empty() {
        return Err(Error::UnsupportedProtocolVersion);
    }

    let session_id = SecureSessionV0::derive_session_id(
        local_agent_id,
        peer_agent_id,
        local_hello_nonce,
        accept.nonce,
        pending.session.protocol_version,
    )?;
    if store.get(&session_id).is_some() {
        return Err(Error::SessionAlreadyExists);
    }

    let mut session = pending.session;
    session.session_id = session_id;
    session.status = SessionStatus::Established;
    session.peer_hello_nonce = Some(accept.nonce);
    session.established_at = Some(now);
    session.negotiated_features = negotiated;

    let mut seen_nonces = pending.seen_nonces;
    seen_nonces.insert(accept.nonce);

    store.insert_record(
        session_id,
        SessionRecord {
            session: session.clone(),
            seen_message_ids: pending.seen_message_ids,
            seen_nonces,
        },
    );
    store.mark_global_nonce(accept.nonce);
    Ok(session)
}

fn intersect_features(a: &[String], b: &[String]) -> Vec<String> {
    a.iter()
        .filter(|f| b.iter().any(|g| g == *f))
        .cloned()
        .collect()
}
