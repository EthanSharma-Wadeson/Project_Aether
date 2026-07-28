//! Cryptographic primitives for PROTO-0 (DEC-004A / DEC-004B).

pub mod signing;
pub mod verify;

pub use signing::{sign_digest, signing_preimage, DOMAIN_TAG};
pub use verify::{verify_digest, verify_signed_message};

use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}
