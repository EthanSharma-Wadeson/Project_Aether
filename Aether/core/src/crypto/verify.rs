//! Ed25519 verification for signed protocol messages.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::crypto::sha256;
use crate::crypto::signing::{signing_preimage, DOMAIN_TAG};
use crate::error::{Error, Result};
use crate::types::RejectReason;

/// Logical signed envelope (DEC-004B). Signature is on the envelope, not inside body fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedMessage {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub message_type: String,
    pub body: Vec<u8>,
    pub signer_key_id: String,
    pub signature: Vec<u8>,
    /// Domain tag used when this message was signed (must match on verify).
    pub domain_tag: String,
}

pub fn verifying_key_from_bytes(bytes: &[u8]) -> Result<VerifyingKey> {
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| Error::InvalidPublicKey)?;
    VerifyingKey::from_bytes(&arr).map_err(|_| Error::InvalidPublicKey)
}

pub fn verify_digest(public_key: &[u8], digest: &[u8; 32], signature: &[u8]) -> Result<()> {
    let vk = verifying_key_from_bytes(public_key)?;
    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| Error::InvalidSignature)?;
    let sig = Signature::from_bytes(&sig_bytes);
    vk.verify(digest, &sig).map_err(|_| Error::InvalidSignature)
}

pub fn verify_signed_message(
    message: &SignedMessage,
    public_key: &[u8],
    expected_message_type: &str,
    expected_protocol_version: u32,
    expected_schema_version: u32,
) -> Result<()> {
    if message.domain_tag != DOMAIN_TAG {
        return Err(Error::SigningContextMismatch);
    }
    if message.message_type != expected_message_type {
        return Err(Error::SigningContextMismatch);
    }
    if message.protocol_version != expected_protocol_version
        || message.schema_version != expected_schema_version
    {
        return Err(Error::SigningContextMismatch);
    }

    let preimage = signing_preimage(
        &message.domain_tag,
        message.protocol_version,
        message.schema_version,
        &message.message_type,
        &message.body,
    );
    let digest = sha256(&preimage);
    verify_digest(public_key, &digest, &message.signature)
}

pub fn reject_from_verify_error(err: Error) -> RejectReason {
    match err {
        Error::InvalidSignature | Error::InvalidPublicKey => RejectReason::InvalidSignature,
        Error::SigningContextMismatch => RejectReason::SigningContextMismatch,
        Error::MalformedCbor | Error::MalformedObject(_) => RejectReason::MalformedObject,
        _ => RejectReason::InvalidSignature, // fail closed
    }
}
