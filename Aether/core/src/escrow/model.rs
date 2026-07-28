//! Escrow message types and signed intent models.

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, encode_value, map, map_get, text, u32_value, u64_value,
};
use crate::crypto::verify::SignedMessage;
use crate::error::{Error, Result};
use crate::types::AgentId;

pub const MSG_ESCROW_CREATE: &str = "escrow.create";
pub const MSG_ESCROW_FUND: &str = "escrow.fund";
pub const MSG_ESCROW_SUBMIT_RECEIPT: &str = "escrow.submit_receipt";
pub const MSG_ESCROW_RELEASE: &str = "escrow.release";
pub const MSG_ESCROW_REFUND: &str = "escrow.refund";
pub const MSG_ESCROW_DISPUTE: &str = "escrow.dispute";
pub const MSG_ESCROW_RESOLVE: &str = "escrow.resolve";
pub const MSG_ESCROW_CANCEL: &str = "escrow.cancel";

pub const ACTION_CREATE: &str = "escrow.create";
pub const ACTION_FUND: &str = "escrow.fund";
pub const ACTION_SUBMIT_RECEIPT: &str = "escrow.submit_receipt";
pub const ACTION_RELEASE: &str = "escrow.release";
pub const ACTION_REFUND: &str = "escrow.refund";
pub const ACTION_DISPUTE: &str = "escrow.dispute";
pub const ACTION_RESOLVE: &str = "escrow.resolve";
pub const ACTION_CANCEL: &str = "escrow.cancel";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualSignedTerms {
    pub body: Vec<u8>,
    pub sig_payer: SignedMessage,
    pub sig_provider: SignedMessage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualSignedCancel {
    pub body: Vec<u8>,
    pub sig_payer: SignedMessage,
    pub sig_provider: SignedMessage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowFundingV0 {
    pub escrow_id: [u8; 32],
    pub payer: AgentId,
    pub amount: u64,
    pub fee_reservation: u64,
    pub logical_time: u64,
}

impl EscrowFundingV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (
                text("escrow_id"),
                crate::cbor::bytes(self.escrow_id.to_vec()),
            ),
            (text("payer"), text(self.payer.clone())),
            (text("amount"), u64_value(self.amount)),
            (text("fee_reservation"), u64_value(self.fee_reservation)),
            (text("logical_time"), u64_value(self.logical_time)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("funding must be map"));
        };
        let b = as_bytes(map_get(&entries, "escrow_id")?)?;
        Ok(Self {
            escrow_id: b
                .try_into()
                .map_err(|_| Error::MalformedObject("escrow_id"))?,
            payer: as_text(map_get(&entries, "payer")?)?.into(),
            amount: as_u64(map_get(&entries, "amount")?)?,
            fee_reservation: as_u64(map_get(&entries, "fee_reservation")?)?,
            logical_time: as_u64(map_get(&entries, "logical_time")?)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowReleaseV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub escrow_id: [u8; 32],
    pub receipt_id: [u8; 32],
    pub actor: AgentId,
    pub logical_time: u64,
}

impl EscrowReleaseV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (
                text("escrow_id"),
                crate::cbor::bytes(self.escrow_id.to_vec()),
            ),
            (
                text("receipt_id"),
                crate::cbor::bytes(self.receipt_id.to_vec()),
            ),
            (text("actor"), text(self.actor.clone())),
            (text("logical_time"), u64_value(self.logical_time)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("release must be map"));
        };
        let escrow_id: [u8; 32] = as_bytes(map_get(&entries, "escrow_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("escrow_id"))?;
        let receipt_id: [u8; 32] = as_bytes(map_get(&entries, "receipt_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("receipt_id"))?;
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            escrow_id,
            receipt_id,
            actor: as_text(map_get(&entries, "actor")?)?.into(),
            logical_time: as_u64(map_get(&entries, "logical_time")?)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowRefundV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub escrow_id: [u8; 32],
    pub actor: AgentId,
    pub reason: String,
    pub logical_time: u64,
}

impl EscrowRefundV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (
                text("escrow_id"),
                crate::cbor::bytes(self.escrow_id.to_vec()),
            ),
            (text("actor"), text(self.actor.clone())),
            (text("reason"), text(self.reason.clone())),
            (text("logical_time"), u64_value(self.logical_time)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("refund must be map"));
        };
        let escrow_id: [u8; 32] = as_bytes(map_get(&entries, "escrow_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("escrow_id"))?;
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            escrow_id,
            actor: as_text(map_get(&entries, "actor")?)?.into(),
            reason: as_text(map_get(&entries, "reason")?)?.into(),
            logical_time: as_u64(map_get(&entries, "logical_time")?)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowCancelV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub escrow_id: [u8; 32],
    pub logical_time: u64,
}

impl EscrowCancelV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (
                text("escrow_id"),
                crate::cbor::bytes(self.escrow_id.to_vec()),
            ),
            (text("logical_time"), u64_value(self.logical_time)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("cancel must be map"));
        };
        let escrow_id: [u8; 32] = as_bytes(map_get(&entries, "escrow_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("escrow_id"))?;
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            escrow_id,
            logical_time: as_u64(map_get(&entries, "logical_time")?)?,
        })
    }
}
