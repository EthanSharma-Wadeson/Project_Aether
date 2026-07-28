//! Schema-locked CBOR helpers using explicit field order (not serde map order).

use ciborium::value::{Integer, Value};

use crate::error::{Error, Result};

pub fn encode_value(value: &Value) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes).map_err(|_| Error::MalformedCbor)?;
    Ok(bytes)
}

pub fn decode_value(bytes: &[u8]) -> Result<Value> {
    ciborium::from_reader(bytes).map_err(|_| Error::MalformedCbor)
}

pub fn u32_value(n: u32) -> Value {
    Value::Integer(Integer::from(n))
}

pub fn u64_value(n: u64) -> Value {
    Value::Integer(Integer::from(n))
}

pub fn text(s: impl Into<String>) -> Value {
    Value::Text(s.into())
}

pub fn bytes(b: impl Into<Vec<u8>>) -> Value {
    Value::Bytes(b.into())
}

pub fn map(entries: Vec<(Value, Value)>) -> Value {
    Value::Map(entries)
}

pub fn optional_bytes(opt: &Option<Vec<u8>>) -> Value {
    match opt {
        Some(b) => Value::Bytes(b.clone()),
        None => Value::Null,
    }
}

pub fn as_u64(value: &Value) -> Result<u64> {
    match value {
        Value::Integer(i) => i64::try_from(*i)
            .map_err(|_| Error::MalformedObject("integer out of range"))
            .and_then(|n| u64::try_from(n).map_err(|_| Error::MalformedObject("negative integer"))),
        _ => Err(Error::MalformedObject("expected integer")),
    }
}

pub fn as_u32(value: &Value) -> Result<u32> {
    let n = as_u64(value)?;
    u32::try_from(n).map_err(|_| Error::MalformedObject("u32 overflow"))
}

pub fn as_text(value: &Value) -> Result<&str> {
    match value {
        Value::Text(s) => Ok(s.as_str()),
        _ => Err(Error::MalformedObject("expected text")),
    }
}

pub fn as_bytes(value: &Value) -> Result<&[u8]> {
    match value {
        Value::Bytes(b) => Ok(b.as_slice()),
        _ => Err(Error::MalformedObject("expected bytes")),
    }
}

pub fn as_optional_bytes(value: &Value) -> Result<Option<Vec<u8>>> {
    match value {
        Value::Null => Ok(None),
        Value::Bytes(b) => Ok(Some(b.clone())),
        _ => Err(Error::MalformedObject("expected bytes or null")),
    }
}

pub fn map_get<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a Value> {
    map.iter()
        .find(|(k, _)| matches!(k, Value::Text(t) if t == key))
        .map(|(_, v)| v)
        .ok_or(Error::MalformedObject("missing field"))
}
