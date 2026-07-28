//! Network signature and identity verification.

use crate::crypto::verify::{verify_signed_message, SignedMessage};
use crate::error::{Error, Result};
use crate::identity::registry::IdentityRegistry;
use crate::network::envelope::{MessageEnvelopeV0, MSG_NET_ENVELOPE};
use crate::network::hello::ProtocolHelloV0;
use crate::types::AgentStatus;

pub fn require_active_identity(registry: &IdentityRegistry, agent_id: &str) -> Result<()> {
    let entry = registry.get(agent_id).ok_or(Error::UnknownNetworkAgent)?;
    if entry.status != AgentStatus::Active {
        return Err(Error::IdentityNotActive);
    }
    Ok(())
}

pub fn verify_hello_signature(
    signed: &SignedMessage,
    registry: &IdentityRegistry,
    message_type: &str,
) -> Result<ProtocolHelloV0> {
    let hello = ProtocolHelloV0::decode(&signed.body)?;
    if hello.encode()? != signed.body {
        return Err(Error::MalformedObject("non-canonical hello"));
    }
    let entry = registry
        .get(&hello.agent_id)
        .ok_or(Error::UnknownNetworkAgent)?;
    if entry.status != AgentStatus::Active {
        return Err(Error::IdentityNotActive);
    }
    verify_signed_message(
        signed,
        &entry.identity.operational_public_key,
        message_type,
        hello.protocol_version,
        hello.schema_version,
    )?;
    if signed.signer_key_id != hello.agent_id {
        return Err(Error::SessionIdentityMismatch);
    }
    Ok(hello)
}

pub fn verify_envelope_signature(
    signed: &SignedMessage,
    registry: &IdentityRegistry,
) -> Result<MessageEnvelopeV0> {
    let envelope = MessageEnvelopeV0::decode(&signed.body)?;
    let entry = registry
        .get(&envelope.sender_agent_id)
        .ok_or(Error::UnknownNetworkAgent)?;
    if entry.status != AgentStatus::Active {
        return Err(Error::IdentityNotActive);
    }
    verify_signed_message(
        signed,
        &entry.identity.operational_public_key,
        MSG_NET_ENVELOPE,
        envelope.protocol_version,
        envelope.schema_version,
    )?;
    if signed.signer_key_id != envelope.sender_agent_id {
        return Err(Error::SessionIdentityMismatch);
    }
    Ok(envelope)
}
