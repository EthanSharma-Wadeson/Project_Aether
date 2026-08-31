//! Deterministic JSON canonicalization for `execution_hash` and policy content hashes.
//!
//! See APPLY_PROTO_ADAPTER_SPEC.md §6.

use serde_json::{Map, Number, Value};

use crate::apply::errors::ApplyErrorCode;
use crate::apply::PolicyMappingError;

/// Recursively canonicalize a JSON value for cross-platform hashing.
///
/// - Object keys: sorted lexicographically by UTF-8 bytes
/// - Arrays: element order preserved
/// - Numbers: integers only (u64 / i64); floats rejected
pub fn canonicalize_json(value: &Value) -> Result<Value, PolicyMappingError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Bool(b) => Ok(Value::Bool(*b)),
        Value::Number(n) => canonicalize_number(n),
        Value::String(s) => Ok(Value::String(s.clone())),
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(canonicalize_json(item)?);
            }
            Ok(Value::Array(out))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            let mut out = Map::new();
            for key in keys {
                let v = canonicalize_json(&map[key])?;
                out.insert(key.clone(), v);
            }
            Ok(Value::Object(out))
        }
    }
}

fn canonicalize_number(n: &Number) -> Result<Value, PolicyMappingError> {
    if let Some(i) = n.as_i64() {
        return Ok(Value::Number(Number::from(i)));
    }
    if let Some(u) = n.as_u64() {
        return Ok(Value::Number(Number::from(u)));
    }
    Err(PolicyMappingError::new(
        ApplyErrorCode::ApplyInvalidPolicy,
        "policy_data must not contain floating-point numbers",
    ))
}

/// Serialize canonical JSON to compact UTF-8 bytes (no whitespace).
pub fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, PolicyMappingError> {
    let canonical = canonicalize_json(value)?;
    serde_json::to_vec(&canonical).map_err(|e| {
        PolicyMappingError::new(
            ApplyErrorCode::ApplyInvalidPolicy,
            format!("canonical json encode failed: {e}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_object_keys_lexicographically() {
        let input = json!({"zeta": 1, "alpha": "β", "capability_id": "00"});
        let out = canonicalize_json(&input).unwrap();
        let obj = out.as_object().unwrap();
        let keys: Vec<_> = obj.keys().collect();
        assert_eq!(keys, vec!["alpha", "capability_id", "zeta"]);
    }

    #[test]
    fn preserves_array_order() {
        let input = json!({"actions": ["transfer", "read"]});
        let out = canonicalize_json(&input).unwrap();
        assert_eq!(out["actions"], json!(["transfer", "read"]));
    }

    #[test]
    fn nested_objects_sorted() {
        let input = json!({"outer": {"z": 1, "a": 2}});
        let out = canonicalize_json(&input).unwrap();
        let inner = out["outer"].as_object().unwrap();
        let keys: Vec<_> = inner.keys().collect();
        assert_eq!(keys, vec!["a", "z"]);
    }

    #[test]
    fn rejects_floats() {
        let err = canonicalize_json(&json!({"x": 1.5})).unwrap_err();
        assert_eq!(err.code, ApplyErrorCode::ApplyInvalidPolicy);
    }

    #[test]
    fn null_preserved() {
        let out = canonicalize_json(&json!({"target": null})).unwrap();
        assert!(out["target"].is_null());
    }
}
