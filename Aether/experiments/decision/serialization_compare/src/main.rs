//! DEC-003 decision experiment: compare wire encoding candidates.
//!
//! Candidates:
//!   A. Canonical JSON (sorted keys, minimal separators, UTF-8)
//!   B. CBOR via ciborium + serde (struct field declaration order)
//!   C. serde_json default (non-canonical baseline — must fail determinism)

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct StubWireMessage {
    protocol_version: u32,
    schema_version: u32,
    operational_public_key: Vec<u8>,
    permission_root: Vec<u8>,
    nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct StubWireMessageAltOrder {
    nonce: u64,
    permission_root: Vec<u8>,
    operational_public_key: Vec<u8>,
    schema_version: u32,
    protocol_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct StubWireMessageStrict {
    protocol_version: u32,
    schema_version: u32,
    operational_public_key: Vec<u8>,
    permission_root: Vec<u8>,
    nonce: u64,
}

#[derive(Debug, Default)]
struct CriterionResult {
    canonical_json: bool,
    cbor_serde: bool,
    serde_json_default: bool,
    notes: String,
}

fn sample_message() -> StubWireMessage {
    StubWireMessage {
        protocol_version: 1,
        schema_version: 1,
        operational_public_key: vec![0xAB; 32],
        permission_root: vec![0xCD; 32],
        nonce: 42,
    }
}

/// RFC 8785-style canonical JSON: lexicographically sorted object keys, no whitespace.
fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, String> {
    let canonical = canonicalize_value(value)?;
    serde_json::to_vec(&canonical).map_err(|e| e.to_string())
}

fn canonicalize_value(value: &Value) -> Result<Value, String> {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), canonicalize_value(v)?);
            }
            let mut out = Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Ok(Value::Object(out))
        }
        Value::Array(arr) => Ok(Value::Array(
            arr.iter()
                .map(canonicalize_value)
                .collect::<Result<_, _>>()?,
        )),
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                Ok(Value::Number(u.into()))
            } else if let Some(i) = n.as_i64() {
                Ok(Value::Number(i.into()))
            } else {
                Err(format!("non-integer number in canonical JSON test: {n}"))
            }
        }
        Value::String(s) => Ok(Value::String(s.clone())),
        Value::Bool(b) => Ok(Value::Bool(*b)),
        Value::Null => Ok(Value::Null),
    }
}

fn encode_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

fn decode_cbor<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, String> {
    ciborium::from_reader(bytes).map_err(|e| e.to_string())
}

fn test_deterministic_encoding(msg: &StubWireMessage) -> CriterionResult {
    let mut r = CriterionResult::default();
    let json_val = serde_json::to_value(msg).unwrap();

    let cj1 = canonical_json_bytes(&json_val).unwrap();
    let cj2 = canonical_json_bytes(&json_val).unwrap();
    r.canonical_json = cj1 == cj2;

    let cb1 = encode_cbor(msg).unwrap();
    let cb2 = encode_cbor(msg).unwrap();
    r.cbor_serde = cb1 == cb2;

    let sj1 = serde_json::to_vec(msg).unwrap();
    let sj2 = serde_json::to_vec(msg).unwrap();
    r.serde_json_default = sj1 == sj2;

    r.notes = format!(
        "cj_len={} cb_len={} sj_len={}",
        cj1.len(),
        cb1.len(),
        sj1.len()
    );
    r
}

fn test_field_order_independence() -> CriterionResult {
    let mut r = CriterionResult::default();
    let a = sample_message();
    let b = StubWireMessageAltOrder {
        protocol_version: a.protocol_version,
        schema_version: a.schema_version,
        operational_public_key: a.operational_public_key.clone(),
        permission_root: a.permission_root.clone(),
        nonce: a.nonce,
    };

    let cj_a = canonical_json_bytes(&serde_json::to_value(&a).unwrap()).unwrap();
    let cj_b = canonical_json_bytes(&serde_json::to_value(&b).unwrap()).unwrap();
    r.canonical_json = cj_a == cj_b;

    // CBOR via serde uses struct declaration order — different structs => different bytes.
    let cb_a = encode_cbor(&a).unwrap();
    let cb_b = encode_cbor(&b).unwrap();
    r.cbor_serde = cb_a == cb_b;

    r.notes = if r.canonical_json && !r.cbor_serde {
        "canonical JSON order-independent; CBOR serde order-dependent unless canonical map encoding specified".into()
    } else {
        "see byte comparison".into()
    };
    r
}

fn test_roundtrip(msg: &StubWireMessage) -> CriterionResult {
    let mut r = CriterionResult::default();
    let json_val = serde_json::to_value(msg).unwrap();
    let cj = canonical_json_bytes(&json_val).unwrap();
    let back: StubWireMessage = serde_json::from_slice(&cj).unwrap();
    r.canonical_json = &back == msg;

    let cb = encode_cbor(msg).unwrap();
    let back_cb: StubWireMessage = decode_cbor(&cb).unwrap();
    r.cbor_serde = back_cb == *msg;

    r.notes = "roundtrip equality".into();
    r
}

fn test_unknown_fields() -> CriterionResult {
    let mut r = CriterionResult::default();
    let mut obj = serde_json::to_value(sample_message()).unwrap();
    obj.as_object_mut().unwrap().insert(
        "unexpected_field".into(),
        Value::String("x".into()),
    );

    let cj = canonical_json_bytes(&obj).unwrap();
    let strict: Result<StubWireMessageStrict, _> = serde_json::from_slice(&cj);
    r.canonical_json = strict.is_err();

    let mut map = serde_json::Map::new();
    map.insert("protocol_version".into(), 1.into());
    map.insert("schema_version".into(), 1.into());
    map.insert("operational_public_key".into(), Value::Array(vec![]));
    map.insert("permission_root".into(), Value::Array(vec![]));
    map.insert("nonce".into(), 42.into());
    map.insert("unexpected_field".into(), "x".into());
    let cb_bytes = encode_cbor(&map).unwrap();
    let strict_cb: Result<StubWireMessageStrict, _> = decode_cbor(&cb_bytes);
    r.cbor_serde = strict_cb.is_err();

    r.notes = "deny_unknown_fields rejects extra keys when enforced at decode".into();
    r
}

fn test_integer_edges() -> CriterionResult {
    let mut r = CriterionResult::default();
    let mut msg = sample_message();
    msg.nonce = u64::MAX;
    msg.protocol_version = u32::MAX;

    let cj = canonical_json_bytes(&serde_json::to_value(&msg).unwrap()).unwrap();
    let back: StubWireMessage = serde_json::from_slice(&cj).unwrap();
    r.canonical_json = back.nonce == u64::MAX && back.protocol_version == u32::MAX;

    let cb = encode_cbor(&msg).unwrap();
    let back_cb: StubWireMessage = decode_cbor(&cb).unwrap();
    r.cbor_serde = back_cb.nonce == u64::MAX;

    r.notes = format!("u64::MAX preserved cj={} cb={}", r.canonical_json, r.cbor_serde);
    r
}

fn test_binary_fields(msg: &StubWireMessage) -> CriterionResult {
    let mut r = CriterionResult::default();
    let cj = canonical_json_bytes(&serde_json::to_value(msg).unwrap()).unwrap();
    // Canonical JSON encodes byte arrays as base64 in serde_json by default.
    let text = String::from_utf8_lossy(&cj);
    r.canonical_json = text.contains("qrs") || text.contains("base64") || text.contains("yrLD"); // AB* CD* base64

    let cb = encode_cbor(msg).unwrap();
    r.cbor_serde = cb.windows(32).any(|w| w == &msg.operational_public_key[..]);

    r.notes = format!(
        "JSON uses base64 text for bytes (human-readable, larger); CBOR uses byte strings (compact). cj_has_b64={} cb_has_raw={}",
        r.canonical_json, r.cbor_serde
    );
    r
}

fn main() {
    let msg = sample_message();
    let results = vec![
        ("deterministic_encoding", test_deterministic_encoding(&msg)),
        ("field_order_independence", test_field_order_independence()),
        ("encode_decode_stability", test_roundtrip(&msg)),
        ("unknown_field_behaviour", test_unknown_fields()),
        ("integer_edge_cases", test_integer_edges()),
        ("binary_key_signature_handling", test_binary_fields(&msg)),
    ];

    println!("DEC-003 experiment: serialization_compare");
    println!("{:-<72}", "");
    for (name, r) in &results {
        println!(
            "{name}: canonical_json={} cbor_serde={} | {}",
            r.canonical_json, r.cbor_serde, r.notes
        );
    }

    // Human auditability (qualitative)
    let cj = canonical_json_bytes(&serde_json::to_value(&msg).unwrap()).unwrap();
    let cb = encode_cbor(&msg).unwrap();
    println!("{:-<72}", "");
    println!("human_auditability:");
    println!("  canonical_json_sample: {}", String::from_utf8_lossy(&cj));
    println!("  cbor_hex_sample: {}", hex::encode(&cb[..cb.len().min(48)]));
    println!("  cbor_total_bytes: {}", cb.len());
    println!("  canonical_json_total_bytes: {}", cj.len());

    // Cross-language feasibility (documentary scores)
    println!("{:-<72}", "");
    println!("cross_language_feasibility (qualitative):");
    println!("  canonical_json: HIGH — JCS/RFC 8785 ecosystem; many parsers");
    println!("  cbor_serde: MEDIUM-HIGH — RFC 8949 widely implemented; serde field order needs spec lock");
    println!("  serde_json_default: LOW — not suitable for canonical wire");

    let cj_hash = hex::encode(Sha256::digest(&cj));
    let cb_hash = hex::encode(Sha256::digest(&cb));
    println!("{:-<72}", "");
    println!("fixture_hashes:");
    println!("  canonical_json_sha256: {cj_hash}");
    println!("  cbor_serde_sha256: {cb_hash}");

    println!("RESULT: completed — see RESULTS.md for interpretation");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_is_deterministic() {
        let r = test_deterministic_encoding(&sample_message());
        assert!(r.canonical_json);
    }

    #[test]
    fn canonical_json_field_order_independent() {
        let r = test_field_order_independence();
        assert!(r.canonical_json);
    }

    #[test]
    fn cbor_field_order_dependent_with_serde() {
        let r = test_field_order_independence();
        assert!(!r.cbor_serde, "expected CBOR serde to be order-dependent without canonical map rules");
    }
}
