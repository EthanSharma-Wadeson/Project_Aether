//! DEC-005 AgentId derivation.

use crate::cbor::{bytes, encode_value, map, text, u32_value};
use crate::crypto::sha256;
use crate::types::AgentId;

/// Derive AgentId from stable material only (schema_version + operational_public_key).
pub fn derive_agent_id(schema_version: u32, operational_public_key: &[u8]) -> AgentId {
    let id_material = map(vec![
        (text("schema_version"), u32_value(schema_version)),
        (
            text("operational_public_key"),
            bytes(operational_public_key.to_vec()),
        ),
    ]);
    let cbor = encode_value(&id_material).expect("id material encodes");
    let mut preimage = Vec::new();
    preimage.extend_from_slice(b"aether:v0:agent-id:");
    preimage.extend_from_slice(&cbor);
    let digest = sha256(&preimage);
    format!("aether:{}", hex::encode(digest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivation_is_deterministic() {
        let pk = [7u8; 32];
        assert_eq!(derive_agent_id(1, &pk), derive_agent_id(1, &pk));
    }

    #[test]
    fn derivation_changes_with_key() {
        assert_ne!(derive_agent_id(1, &[1u8; 32]), derive_agent_id(1, &[2u8; 32]));
    }
}
