//! PermissionRootV0 and RootAuthorityCapabilityV0.

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, bytes, encode_value, map, map_get, text, u32_value,
    u64_value,
};
use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::types::{ActionSelector, AgentId, RateLimit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraints {
    pub max_spend: Option<u64>,
    pub asset: Option<String>,
    pub counterparties: Option<Vec<AgentId>>,
    pub rate_limit: Option<RateLimit>,
    pub valid_after: Option<u64>,
    pub valid_before: Option<u64>,
}

impl Constraints {
    pub fn unrestricted() -> Self {
        Self {
            max_spend: None,
            asset: None,
            counterparties: None,
            rate_limit: None,
            valid_after: None,
            valid_before: None,
        }
    }

    pub fn to_cbor_value(&self) -> Value {
        let counterparties = match &self.counterparties {
            Some(list) => Value::Array(list.iter().map(|id| text(id.clone())).collect()),
            None => Value::Null,
        };
        let rate_limit = match &self.rate_limit {
            Some(r) => map(vec![
                (text("max_ops"), u64_value(r.max_ops)),
                (text("window_seconds"), u64_value(r.window_seconds)),
            ]),
            None => Value::Null,
        };
        map(vec![
            (
                text("max_spend"),
                self.max_spend.map(u64_value).unwrap_or(Value::Null),
            ),
            (
                text("asset"),
                self.asset
                    .as_ref()
                    .map(|a| text(a.clone()))
                    .unwrap_or(Value::Null),
            ),
            (text("counterparties"), counterparties),
            (text("rate_limit"), rate_limit),
            (
                text("valid_after"),
                self.valid_after.map(u64_value).unwrap_or(Value::Null),
            ),
            (
                text("valid_before"),
                self.valid_before.map(u64_value).unwrap_or(Value::Null),
            ),
        ])
    }

    pub fn from_cbor_value(value: &Value) -> Result<Self> {
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("constraints must be map"));
        };
        let max_spend = optional_u64(map_get(entries, "max_spend")?)?;
        let asset = optional_text(map_get(entries, "asset")?)?;
        let counterparties = match map_get(entries, "counterparties")? {
            Value::Null => None,
            Value::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    out.push(as_text(item)?.to_string());
                }
                Some(out)
            }
            _ => return Err(Error::MalformedObject("counterparties")),
        };
        let rate_limit = match map_get(entries, "rate_limit")? {
            Value::Null => None,
            Value::Map(rl) => Some(RateLimit {
                max_ops: as_u64(map_get(rl, "max_ops")?)?,
                window_seconds: as_u64(map_get(rl, "window_seconds")?)?,
            }),
            _ => return Err(Error::MalformedObject("rate_limit")),
        };
        Ok(Self {
            max_spend,
            asset,
            counterparties,
            rate_limit,
            valid_after: optional_u64(map_get(entries, "valid_after")?)?,
            valid_before: optional_u64(map_get(entries, "valid_before")?)?,
        })
    }
}

fn optional_u64(value: &Value) -> Result<Option<u64>> {
    match value {
        Value::Null => Ok(None),
        other => Ok(Some(as_u64(other)?)),
    }
}

fn optional_text(value: &Value) -> Result<Option<String>> {
    match value {
        Value::Null => Ok(None),
        other => Ok(Some(as_text(other)?.to_string())),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootAuthorityCapabilityV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub issuer: AgentId,
    pub subject: AgentId,
    pub actions: Vec<ActionSelector>,
    pub constraints: Constraints,
    pub max_delegation_depth: u32,
}

impl RootAuthorityCapabilityV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("issuer"), text(self.issuer.clone())),
            (text("subject"), text(self.subject.clone())),
            (
                text("actions"),
                Value::Array(self.actions.iter().map(|a| text(a.clone())).collect()),
            ),
            (text("constraints"), self.constraints.to_cbor_value()),
            (
                text("max_delegation_depth"),
                u32_value(self.max_delegation_depth),
            ),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn capability_id(&self) -> [u8; 32] {
        let bytes = self.encode().expect("root authority encodes");
        sha256(&bytes)
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("root authority must be map"));
        };
        let actions = match map_get(&entries, "actions")? {
            Value::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    out.push(as_text(item)?.to_string());
                }
                out
            }
            _ => return Err(Error::MalformedObject("actions")),
        };
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            issuer: as_text(map_get(&entries, "issuer")?)?.to_string(),
            subject: as_text(map_get(&entries, "subject")?)?.to_string(),
            actions,
            constraints: Constraints::from_cbor_value(map_get(&entries, "constraints")?)?,
            max_delegation_depth: as_u32(map_get(&entries, "max_delegation_depth")?)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRootV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub agent_id: AgentId,
    pub root_version: u64,
    pub root_authority_capability_id: [u8; 32],
}

impl PermissionRootV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("agent_id"), text(self.agent_id.clone())),
            (text("root_version"), u64_value(self.root_version)),
            (
                text("root_authority_capability_id"),
                bytes(self.root_authority_capability_id.to_vec()),
            ),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn commitment(&self) -> Result<[u8; 32]> {
        Ok(sha256(&self.encode()?))
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("permission root must be map"));
        };
        let id_bytes = as_bytes(map_get(&entries, "root_authority_capability_id")?)?;
        let root_authority_capability_id: [u8; 32] = id_bytes
            .try_into()
            .map_err(|_| Error::MalformedObject("root_authority_capability_id length"))?;
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            agent_id: as_text(map_get(&entries, "agent_id")?)?.to_string(),
            root_version: as_u64(map_get(&entries, "root_version")?)?,
            root_authority_capability_id,
        })
    }

    pub fn verify_commitment(&self, expected: &[u8; 32]) -> Result<()> {
        if &self.commitment()? != expected {
            return Err(Error::PermissionRootMismatch);
        }
        Ok(())
    }

    pub fn verify_against_authority(&self, authority: &RootAuthorityCapabilityV0) -> Result<()> {
        if self.root_authority_capability_id != authority.capability_id() {
            return Err(Error::InvalidRootAuthority);
        }
        if self.agent_id != authority.issuer || self.agent_id != authority.subject {
            return Err(Error::AgentIdMismatch);
        }
        Ok(())
    }
}
