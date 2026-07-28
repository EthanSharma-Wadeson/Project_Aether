//! ChannelStateV0 — simulated shared state snapshot.

use ciborium::value::Value;

use crate::cbor::{as_bytes, as_text, as_u64, bytes, encode_value, map, map_get, text, u64_value};
use crate::channel::model::ChannelStatus;
use crate::crypto::sha256;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelStateV0 {
    pub channel_id: [u8; 32],
    pub sequence: u64,
    pub balances: [u64; 2],
    pub asset: String,
    pub status_hint: ChannelStatus,
    pub created_at: u64,
    pub metadata_commitment: Option<Vec<u8>>,
}

impl ChannelStateV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("channel_id"), bytes(self.channel_id.to_vec())),
            (text("sequence"), u64_value(self.sequence)),
            (
                text("balances"),
                Value::Array(vec![
                    u64_value(self.balances[0]),
                    u64_value(self.balances[1]),
                ]),
            ),
            (text("asset"), text(self.asset.clone())),
            (text("status_hint"), text(self.status_hint.as_str())),
            (text("created_at"), u64_value(self.created_at)),
            (
                text("metadata_commitment"),
                match &self.metadata_commitment {
                    Some(b) => bytes(b.clone()),
                    None => Value::Null,
                },
            ),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn commitment(&self) -> Result<[u8; 32]> {
        Ok(sha256(&self.encode()?))
    }

    pub fn from_cbor_value(value: &Value) -> Result<Self> {
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("channel state must be map"));
        };
        let channel_id: [u8; 32] = as_bytes(map_get(entries, "channel_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("channel_id length"))?;
        let balances = match map_get(entries, "balances")? {
            Value::Array(items) if items.len() == 2 => [as_u64(&items[0])?, as_u64(&items[1])?],
            _ => return Err(Error::MalformedObject("balances")),
        };
        let metadata_commitment = match map_get(entries, "metadata_commitment")? {
            Value::Null => None,
            Value::Bytes(b) => Some(b.clone()),
            _ => return Err(Error::MalformedObject("metadata_commitment")),
        };
        Ok(Self {
            channel_id,
            sequence: as_u64(map_get(entries, "sequence")?)?,
            balances,
            asset: as_text(map_get(entries, "asset")?)?.to_string(),
            status_hint: ChannelStatus::parse(as_text(map_get(entries, "status_hint")?)?)?,
            created_at: as_u64(map_get(entries, "created_at")?)?,
            metadata_commitment,
        })
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        Self::from_cbor_value(&value)
    }
}
