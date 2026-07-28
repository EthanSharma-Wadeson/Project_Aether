//! DEC-001 / DEC-004A decision experiment: Rust + ed25519-dalek sign/verify stub.
//! Signs a fixed payload (not yet canonical wire bytes — that is DEC-004B / DEC-003).

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

fn main() {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key: VerifyingKey = signing_key.verifying_key();

    // Stub payload resembling a pre-canonical message body (fixed bytes for primitive test).
    let payload = b"aether:v0:stub:identity-register:schema=1";
    let digest = Sha256::digest(payload);

    let signature = signing_key.sign(&digest);
    let verify_ok = verifying_key.verify(&digest, &signature).is_ok();

    println!("DEC-001/DEC-004A experiment: rust_sign_verify");
    println!("  toolchain: {}", option_env!("RUSTC_VERSION").unwrap_or("unknown"));
    println!("  crate: ed25519-dalek 2.x");
    println!("  payload_len: {}", payload.len());
    println!("  digest_hex: {}", hex::encode(digest));
    println!("  pubkey_hex: {}", hex::encode(verifying_key.as_bytes()));
    println!("  sig_hex: {}", hex::encode(signature.to_bytes()));
    println!("  verify_ok: {}", verify_ok);

    assert!(verify_ok, "signature verification failed");
    println!("RESULT: PASS");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_roundtrip() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let digest = Sha256::digest(b"test-payload");
        let sig = signing_key.sign(&digest);
        assert!(verifying_key.verify(&digest, &sig).is_ok());
    }

    #[test]
    fn wrong_payload_fails() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let sig = signing_key.sign(&Sha256::digest(b"a"));
        assert!(verifying_key.verify(&Sha256::digest(b"b"), &sig).is_err());
    }
}
