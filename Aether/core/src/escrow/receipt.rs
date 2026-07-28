//! Settlement receipt models (distinct from PROTO-1 ReceiptV0).

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, bytes, encode_value, map, map_get, optional_bytes, text,
    u32_value, u64_value,
};
use crate::crypto::sha256;
use crate::crypto::verify::SignedMessage;
use crate::error::{Error, Result};
use crate::types::AgentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementReceiptV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub receipt_id: [u8; 32],
    pub escrow_id: [u8; 32],
    pub terms_version: u64,
    pub payer: AgentId,
    pub provider: AgentId,
    pub claim_type: String,
    pub result_code: String,
    pub output_commitment: Option<Vec<u8>>,
    pub claimed_amount: u64,
    pub logical_time: u64,
    pub receipt_nonce: u64,
}

impl SettlementReceiptV0 {
    pub fn body_for_id(&self) -> Result<Value> {
        Ok(map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("escrow_id"), bytes(self.escrow_id.to_vec())),
            (text("terms_version"), u64_value(self.terms_version)),
            (text("payer"), text(self.payer.clone())),
            (text("provider"), text(self.provider.clone())),
            (text("claim_type"), text(self.claim_type.clone())),
            (text("result_code"), text(self.result_code.clone())),
            (
                text("output_commitment"),
                optional_bytes(&self.output_commitment),
            ),
            (text("claimed_amount"), u64_value(self.claimed_amount)),
            (text("logical_time"), u64_value(self.logical_time)),
            (text("receipt_nonce"), u64_value(self.receipt_nonce)),
        ]))
    }

    pub fn compute_receipt_id(&self) -> Result<[u8; 32]> {
        let body = encode_value(&self.body_for_id()?)?;
        Ok(sha256(&body))
    }

    pub fn with_computed_id(mut self) -> Result<Self> {
        self.receipt_id = self.compute_receipt_id()?;
        Ok(self)
    }

    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("receipt_id"), bytes(self.receipt_id.to_vec())),
            (text("escrow_id"), bytes(self.escrow_id.to_vec())),
            (text("terms_version"), u64_value(self.terms_version)),
            (text("payer"), text(self.payer.clone())),
            (text("provider"), text(self.provider.clone())),
            (text("claim_type"), text(self.claim_type.clone())),
            (text("result_code"), text(self.result_code.clone())),
            (
                text("output_commitment"),
                optional_bytes(&self.output_commitment),
            ),
            (text("claimed_amount"), u64_value(self.claimed_amount)),
            (text("logical_time"), u64_value(self.logical_time)),
            (text("receipt_nonce"), u64_value(self.receipt_nonce)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes)?;
        Self::from_cbor_value(&value)
    }

    pub fn from_cbor_value(value: &Value) -> Result<Self> {
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("receipt must be map"));
        };
        let receipt_id: [u8; 32] = as_bytes(map_get(entries, "receipt_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("receipt_id"))?;
        let escrow_id: [u8; 32] = as_bytes(map_get(entries, "escrow_id")?)?
            .try_into()
            .map_err(|_| Error::MalformedObject("escrow_id"))?;
        Ok(Self {
            protocol_version: as_u32(map_get(entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(entries, "schema_version")?)?,
            receipt_id,
            escrow_id,
            terms_version: as_u64(map_get(entries, "terms_version")?)?,
            payer: as_text(map_get(entries, "payer")?)?.into(),
            provider: as_text(map_get(entries, "provider")?)?.into(),
            claim_type: as_text(map_get(entries, "claim_type")?)?.into(),
            result_code: as_text(map_get(entries, "result_code")?)?.into(),
            output_commitment: crate::cbor::as_optional_bytes(map_get(
                entries,
                "output_commitment",
            )?)?,
            claimed_amount: as_u64(map_get(entries, "claimed_amount")?)?,
            logical_time: as_u64(map_get(entries, "logical_time")?)?,
            receipt_nonce: as_u64(map_get(entries, "receipt_nonce")?)?,
        })
    }

    pub fn verify_id(&self) -> Result<()> {
        if self.receipt_id != self.compute_receipt_id()? {
            return Err(Error::InvalidReceipt);
        }
        if self.encode()? != encode_value(&self.to_cbor_value())? {
            return Err(Error::MalformedObject("non-canonical receipt"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementEvidenceV0 {
    pub escrow_id: [u8; 32],
    pub receipt: SignedMessage,
    pub logical_time_presented: u64,
}
