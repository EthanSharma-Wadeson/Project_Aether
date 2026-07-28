//! EscrowTermsV0 — dual-signed bilateral agreement.

use ciborium::value::Value;

use crate::cbor::{
    as_text, as_u32, as_u64, encode_value, map, map_get, optional_bytes, text, u32_value, u64_value,
};
use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::types::AgentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowTermsV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub payer: AgentId,
    pub provider: AgentId,
    pub asset: String,
    pub principal_amount: u64,
    pub max_protocol_fee: u64,
    pub claim_type: String,
    pub required_result_code: String,
    pub accept_output_commitment: bool,
    pub expected_output_commitment: Option<Vec<u8>>,
    pub fund_before: u64,
    pub receipt_before: u64,
    pub dispute_window: u64,
    pub terms_version: u64,
}

impl EscrowTermsV0 {
    pub fn validate(&self) -> Result<()> {
        if self.payer == self.provider {
            return Err(Error::InvalidEscrowTerms);
        }
        if self.principal_amount == 0 {
            return Err(Error::InvalidEscrowTerms);
        }
        if self.max_protocol_fee > self.principal_amount {
            return Err(Error::InvalidEscrowTerms);
        }
        if self.fund_before >= self.receipt_before {
            return Err(Error::InvalidEscrowTerms);
        }
        if self.accept_output_commitment && self.expected_output_commitment.is_none() {
            return Err(Error::InvalidEscrowTerms);
        }
        Ok(())
    }

    pub fn escrow_id(&self) -> Result<[u8; 32]> {
        self.validate()?;
        let body = self.encode()?;
        Ok(sha256(&body))
    }

    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("payer"), text(self.payer.clone())),
            (text("provider"), text(self.provider.clone())),
            (text("asset"), text(self.asset.clone())),
            (text("principal_amount"), u64_value(self.principal_amount)),
            (text("max_protocol_fee"), u64_value(self.max_protocol_fee)),
            (text("claim_type"), text(self.claim_type.clone())),
            (
                text("required_result_code"),
                text(self.required_result_code.clone()),
            ),
            (
                text("accept_output_commitment"),
                u64_value(if self.accept_output_commitment { 1 } else { 0 }),
            ),
            (
                text("expected_output_commitment"),
                optional_bytes(&self.expected_output_commitment),
            ),
            (text("fund_before"), u64_value(self.fund_before)),
            (text("receipt_before"), u64_value(self.receipt_before)),
            (text("dispute_window"), u64_value(self.dispute_window)),
            (text("terms_version"), u64_value(self.terms_version)),
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
            return Err(Error::MalformedObject("terms must be map"));
        };
        let accept = as_u64(map_get(entries, "accept_output_commitment")?)? != 0;
        Ok(Self {
            protocol_version: as_u32(map_get(entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(entries, "schema_version")?)?,
            payer: as_text(map_get(entries, "payer")?)?.into(),
            provider: as_text(map_get(entries, "provider")?)?.into(),
            asset: as_text(map_get(entries, "asset")?)?.into(),
            principal_amount: as_u64(map_get(entries, "principal_amount")?)?,
            max_protocol_fee: as_u64(map_get(entries, "max_protocol_fee")?)?,
            claim_type: as_text(map_get(entries, "claim_type")?)?.into(),
            required_result_code: as_text(map_get(entries, "required_result_code")?)?.into(),
            accept_output_commitment: accept,
            expected_output_commitment: crate::cbor::as_optional_bytes(map_get(
                entries,
                "expected_output_commitment",
            )?)?,
            fund_before: as_u64(map_get(entries, "fund_before")?)?,
            receipt_before: as_u64(map_get(entries, "receipt_before")?)?,
            dispute_window: as_u64(map_get(entries, "dispute_window")?)?,
            terms_version: as_u64(map_get(entries, "terms_version")?)?,
        })
    }
}
