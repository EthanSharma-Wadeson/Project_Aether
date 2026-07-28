//! DEC-004B signing preimage construction and Ed25519 sign.

use ed25519_dalek::{Signature, Signer, SigningKey};

use crate::crypto::sha256;

pub const DOMAIN_TAG: &str = "aether:v0:sign:v1";

/// Build the domain-separated signing preimage (DEC-004B).
pub fn signing_preimage(
    domain_tag: &str,
    protocol_version: u32,
    schema_version: u32,
    message_type: &str,
    canonical_cbor_body: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(domain_tag.as_bytes());
    out.push(0x00);
    out.extend_from_slice(&protocol_version.to_be_bytes());
    out.push(0x00);
    out.extend_from_slice(&schema_version.to_be_bytes());
    out.push(0x00);
    let mt = message_type.as_bytes();
    out.extend_from_slice(&(mt.len() as u16).to_be_bytes());
    out.extend_from_slice(mt);
    out.push(0x00);
    out.extend_from_slice(&(canonical_cbor_body.len() as u32).to_be_bytes());
    out.extend_from_slice(canonical_cbor_body);
    out
}

pub fn sign_digest(signing_key: &SigningKey, digest: &[u8; 32]) -> Signature {
    signing_key.sign(digest)
}

pub fn sign_body(
    signing_key: &SigningKey,
    domain_tag: &str,
    protocol_version: u32,
    schema_version: u32,
    message_type: &str,
    canonical_cbor_body: &[u8],
) -> ([u8; 32], Signature) {
    let preimage = signing_preimage(
        domain_tag,
        protocol_version,
        schema_version,
        message_type,
        canonical_cbor_body,
    );
    let digest = sha256(&preimage);
    let signature = sign_digest(signing_key, &digest);
    (digest, signature)
}
