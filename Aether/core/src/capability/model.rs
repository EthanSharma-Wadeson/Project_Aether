//! CapabilityV0 — unsigned semantic authority object.

use std::collections::HashMap;

use ciborium::value::Value;

use crate::cbor::{as_bytes, as_text, as_u32, bytes, encode_value, map, map_get, text, u32_value};
use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::permission::root::Constraints;
use crate::types::{ActionSelector, AgentId, SubjectRef};

pub const MSG_CAPABILITY_GRANT: &str = "capability.grant";
pub const MSG_CAPABILITY_REVOKE: &str = "capability.revoke";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub issuer: AgentId,
    pub subject: SubjectRef,
    pub actions: Vec<ActionSelector>,
    pub constraints: Constraints,
    pub delegation_depth: u32,
    pub parent_capability_id: Option<[u8; 32]>,
}

impl CapabilityV0 {
    pub fn to_cbor_value(&self) -> Value {
        let subject = match &self.subject {
            SubjectRef::AgentId(id) => map(vec![(text("agent_id"), text(id.clone()))]),
            SubjectRef::PublicKey(pk) => map(vec![(text("public_key"), bytes(pk.clone()))]),
        };
        let parent = match &self.parent_capability_id {
            Some(id) => bytes(id.to_vec()),
            None => Value::Null,
        };
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("issuer"), text(self.issuer.clone())),
            (text("subject"), subject),
            (
                text("actions"),
                Value::Array(self.actions.iter().map(|a| text(a.clone())).collect()),
            ),
            (text("constraints"), self.constraints.to_cbor_value()),
            (text("delegation_depth"), u32_value(self.delegation_depth)),
            (text("parent_capability_id"), parent),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn capability_id(&self) -> Result<[u8; 32]> {
        Ok(sha256(&self.encode()?))
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("capability must be map"));
        };
        let subject_val = map_get(&entries, "subject")?;
        let Value::Map(subject_map) = subject_val else {
            return Err(Error::MalformedObject("subject must be map"));
        };
        let subject = if let Ok(id) = map_get(subject_map, "agent_id") {
            SubjectRef::AgentId(as_text(id)?.to_string())
        } else if let Ok(pk) = map_get(subject_map, "public_key") {
            SubjectRef::PublicKey(as_bytes(pk)?.to_vec())
        } else {
            return Err(Error::MalformedObject("subject form"));
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
        let parent_capability_id = match map_get(&entries, "parent_capability_id")? {
            Value::Null => None,
            Value::Bytes(b) => {
                let id: [u8; 32] = b
                    .as_slice()
                    .try_into()
                    .map_err(|_| Error::MalformedObject("parent_capability_id length"))?;
                Some(id)
            }
            _ => return Err(Error::MalformedObject("parent_capability_id")),
        };
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            issuer: as_text(map_get(&entries, "issuer")?)?.to_string(),
            subject,
            actions,
            constraints: Constraints::from_cbor_value(map_get(&entries, "constraints")?)?,
            delegation_depth: as_u32(map_get(&entries, "delegation_depth")?)?,
            parent_capability_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityRecord {
    pub body: CapabilityV0,
    pub capability_id: [u8; 32],
    pub granted_under_root_version: u64,
}

#[derive(Debug, Default)]
pub struct CapabilityStore {
    caps: HashMap<[u8; 32], CapabilityRecord>,
    revoked: HashMap<[u8; 32], u64>,
}

impl CapabilityStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, record: CapabilityRecord) {
        self.caps.insert(record.capability_id, record);
    }

    pub fn get(&self, id: &[u8; 32]) -> Option<&CapabilityRecord> {
        self.caps.get(id)
    }

    pub fn revoke(&mut self, capability_id: [u8; 32], revoked_at: u64) {
        self.revoked.insert(capability_id, revoked_at);
    }

    pub fn is_revoked(&self, capability_id: &[u8; 32]) -> bool {
        self.revoked.contains_key(capability_id)
    }
}
