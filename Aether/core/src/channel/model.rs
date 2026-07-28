//! Channel data models (PROTO-1).

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, bytes, encode_value, map, map_get, text, u32_value,
    u64_value,
};
use crate::crypto::sha256;
use crate::crypto::verify::SignedMessage;
use crate::error::{Error, Result};
use crate::types::AgentId;

pub const MSG_CHANNEL_OPEN: &str = "channel.open";
pub const MSG_CHANNEL_ACTIVATE: &str = "channel.activate";
pub const MSG_CHANNEL_UPDATE: &str = "channel.update";
pub const MSG_CHANNEL_CLOSE: &str = "channel.close";
pub const MSG_CHANNEL_DISPUTE: &str = "channel.dispute";

pub const ACTION_OPEN: &str = "channel.open";
pub const ACTION_ACTIVATE: &str = "channel.activate";
pub const ACTION_UPDATE: &str = "channel.update";
pub const ACTION_CLOSE: &str = "channel.close";
pub const ACTION_DISPUTE: &str = "channel.dispute";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelStatus {
    Open,
    Active,
    Closing,
    Disputed,
    Finalized,
}

impl ChannelStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Active => "active",
            Self::Closing => "closing",
            Self::Disputed => "disputed",
            Self::Finalized => "finalized",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "open" => Ok(Self::Open),
            "active" => Ok(Self::Active),
            "closing" => Ok(Self::Closing),
            "disputed" => Ok(Self::Disputed),
            "finalized" => Ok(Self::Finalized),
            _ => Err(Error::MalformedObject("channel status")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalityViewV0 {
    pub soft_local_agreement: bool,
    pub dispute_window_open: bool,
    pub dispute_deadline: Option<u64>,
    pub hard_settlement_placeholder: bool,
    pub finalized: bool,
}

impl FinalityViewV0 {
    pub fn for_status(status: ChannelStatus, soft: bool, dispute_deadline: Option<u64>) -> Self {
        Self {
            soft_local_agreement: soft,
            dispute_window_open: matches!(status, ChannelStatus::Disputed | ChannelStatus::Closing)
                && dispute_deadline.is_some(),
            dispute_deadline,
            hard_settlement_placeholder: false,
            finalized: status == ChannelStatus::Finalized,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelOpenMaterialV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub party_a: AgentId,
    pub party_b: AgentId,
    pub asset: String,
    pub opening_balances: [u64; 2],
    pub dispute_window: u64,
    pub created_at: u64,
}

impl ChannelOpenMaterialV0 {
    pub fn total_deposit(&self) -> u64 {
        self.opening_balances[0].saturating_add(self.opening_balances[1])
    }

    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("party_a"), text(self.party_a.clone())),
            (text("party_b"), text(self.party_b.clone())),
            (text("asset"), text(self.asset.clone())),
            (
                text("opening_balances"),
                Value::Array(vec![
                    u64_value(self.opening_balances[0]),
                    u64_value(self.opening_balances[1]),
                ]),
            ),
            (text("dispute_window"), u64_value(self.dispute_window)),
            (text("created_at"), u64_value(self.created_at)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn channel_id(&self) -> Result<[u8; 32]> {
        Ok(sha256(&self.encode()?))
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("open material must be map"));
        };
        let balances = match map_get(&entries, "opening_balances")? {
            Value::Array(items) if items.len() == 2 => [as_u64(&items[0])?, as_u64(&items[1])?],
            _ => return Err(Error::MalformedObject("opening_balances")),
        };
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            party_a: as_text(map_get(&entries, "party_a")?)?.to_string(),
            party_b: as_text(map_get(&entries, "party_b")?)?.to_string(),
            asset: as_text(map_get(&entries, "asset")?)?.to_string(),
            opening_balances: balances,
            dispute_window: as_u64(map_get(&entries, "dispute_window")?)?,
            created_at: as_u64(map_get(&entries, "created_at")?)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub channel_id: [u8; 32],
    pub party_a: AgentId,
    pub party_b: AgentId,
    pub asset: String,
    pub opening_balances: [u64; 2],
    pub total_deposit: u64,
    pub dispute_window: u64,
    pub created_at: u64,
    pub status: ChannelStatus,
    pub sequence: u64,
    pub current_state_commitment: [u8; 32],
    pub finality: FinalityViewV0,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateUpdateV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub channel_id: [u8; 32],
    pub previous_state_commitment: [u8; 32],
    pub new_state: crate::channel::state::ChannelStateV0,
    pub sequence: u64,
    pub logical_time: u64,
}

impl StateUpdateV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("channel_id"), bytes(self.channel_id.to_vec())),
            (
                text("previous_state_commitment"),
                bytes(self.previous_state_commitment.to_vec()),
            ),
            (text("new_state"), self.new_state.to_cbor_value()),
            (text("sequence"), u64_value(self.sequence)),
            (text("logical_time"), u64_value(self.logical_time)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("state update must be map"));
        };
        let channel_id: [u8; 32] = as_bytes(map_get(&entries, "channel_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("channel_id length"))?;
        let previous_state_commitment: [u8; 32] =
            as_bytes(map_get(&entries, "previous_state_commitment")?)?
                .try_into()
                .map_err(|_| Error::MalformedObject("previous_state_commitment length"))?;
        let new_state = crate::channel::state::ChannelStateV0::from_cbor_value(map_get(
            &entries,
            "new_state",
        )?)?;
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            channel_id,
            previous_state_commitment,
            new_state,
            sequence: as_u64(map_get(&entries, "sequence")?)?,
            logical_time: as_u64(map_get(&entries, "logical_time")?)?,
        })
    }
}

/// Dual-signed update: both envelopes must wrap identical body bytes.
#[derive(Debug, Clone)]
pub struct DualSignedUpdate {
    pub body: Vec<u8>,
    pub sig_a: SignedMessage,
    pub sig_b: SignedMessage,
}

impl DualSignedUpdate {
    pub fn from_update(
        update: &StateUpdateV0,
        sig_a: SignedMessage,
        sig_b: SignedMessage,
    ) -> Result<Self> {
        let body = update.encode()?;
        Ok(Self { body, sig_a, sig_b })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub receipt_id: [u8; 32],
    pub channel_id: [u8; 32],
    pub action: String,
    pub participants: [AgentId; 2],
    pub sequence: u64,
    pub state_commitment: [u8; 32],
    pub previous_state_commitment: Option<[u8; 32]>,
    pub logical_time: u64,
    pub finality_snapshot: FinalityViewV0,
    pub accepted: bool,
    pub signature_a: Vec<u8>,
    pub signature_b: Vec<u8>,
}

impl ReceiptV0 {
    pub fn compute_id(without_id: &Value) -> Result<[u8; 32]> {
        Ok(sha256(&encode_value(without_id)?))
    }
}
