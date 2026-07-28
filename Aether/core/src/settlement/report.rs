//! Settlement reports and evidence packages.

#![allow(clippy::too_many_arguments)]

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, bytes, encode_value, map, map_get, text, u32_value,
    u64_value,
};
use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::settlement::model::{
    SettlementStatus, SETTLEMENT_PROTOCOL_VERSION, SETTLEMENT_SCHEMA_VERSION,
};

/// Independently verifiable settlement report (adapter evidence + Aether binding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementReportV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub settlement_id: [u8; 32],
    pub external_reference: String,
    pub status: SettlementStatus,
    pub amount: u64,
    pub commitments: Vec<[u8; 32]>,
    pub adapter_identity: String,
    pub proof_token: Option<[u8; 32]>,
    pub correlation_id: [u8; 32],
}

impl SettlementReportV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("settlement_id"), bytes(self.settlement_id.to_vec())),
            (
                text("external_reference"),
                text(self.external_reference.clone()),
            ),
            (text("status"), text(self.status.as_str())),
            (text("amount"), u64_value(self.amount)),
            (
                text("commitments"),
                Value::Array(self.commitments.iter().map(|c| bytes(c.to_vec())).collect()),
            ),
            (
                text("adapter_identity"),
                text(self.adapter_identity.clone()),
            ),
            (
                text("proof_token"),
                match &self.proof_token {
                    Some(t) => bytes(t.to_vec()),
                    None => Value::Null,
                },
            ),
            (text("correlation_id"), bytes(self.correlation_id.to_vec())),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("settlement report map"));
        };
        fn fixed32(entries: &[(Value, Value)], key: &str) -> Result<[u8; 32]> {
            let b = as_bytes(map_get(entries, key)?)?;
            if b.len() != 32 {
                return Err(Error::MalformedObject("bytes32 length"));
            }
            let mut out = [0u8; 32];
            out.copy_from_slice(b);
            Ok(out)
        }
        let commitments = match map_get(&entries, "commitments")? {
            Value::Array(arr) => {
                let mut out = Vec::with_capacity(arr.len());
                for v in arr {
                    let b = as_bytes(v)?;
                    if b.len() != 32 {
                        return Err(Error::MalformedObject("commitment length"));
                    }
                    let mut c = [0u8; 32];
                    c.copy_from_slice(b);
                    out.push(c);
                }
                out
            }
            _ => return Err(Error::MalformedObject("commitments")),
        };
        let proof_token = match map_get(&entries, "proof_token")? {
            Value::Null => None,
            Value::Bytes(b) => {
                if b.len() != 32 {
                    return Err(Error::MalformedObject("proof_token length"));
                }
                let mut t = [0u8; 32];
                t.copy_from_slice(b);
                Some(t)
            }
            _ => return Err(Error::MalformedObject("proof_token")),
        };
        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            settlement_id: fixed32(&entries, "settlement_id")?,
            external_reference: as_text(map_get(&entries, "external_reference")?)?.into(),
            status: SettlementStatus::parse(as_text(map_get(&entries, "status")?)?)?,
            amount: as_u64(map_get(&entries, "amount")?)?,
            commitments,
            adapter_identity: as_text(map_get(&entries, "adapter_identity")?)?.into(),
            proof_token,
            correlation_id: fixed32(&entries, "correlation_id")?,
        })
    }

    pub fn commitment(&self) -> Result<[u8; 32]> {
        Ok(sha256(&self.encode()?))
    }
}

/// Evidence package used to compute `evidence_commitment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementEvidencePackageV0 {
    pub settlement_binding_id: [u8; 32],
    pub escrow_terminal_commitment: [u8; 32],
    pub capability_grant_ids: Vec<[u8; 32]>,
    pub adapter_response_bytes: Vec<u8>,
    pub logical_time: u64,
}

impl SettlementEvidencePackageV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (
                text("settlement_binding_id"),
                bytes(self.settlement_binding_id.to_vec()),
            ),
            (
                text("escrow_terminal_commitment"),
                bytes(self.escrow_terminal_commitment.to_vec()),
            ),
            (
                text("capability_grant_ids"),
                Value::Array(
                    self.capability_grant_ids
                        .iter()
                        .map(|c| bytes(c.to_vec()))
                        .collect(),
                ),
            ),
            (
                text("adapter_response_bytes"),
                bytes(self.adapter_response_bytes.clone()),
            ),
            (text("logical_time"), u64_value(self.logical_time)),
        ])
    }

    pub fn commitment(&self) -> Result<[u8; 32]> {
        Ok(sha256(&encode_value(&self.to_cbor_value())?))
    }
}

pub fn empty_evidence_commitment(
    binding_id: [u8; 32],
    escrow_commitment: [u8; 32],
    now: u64,
) -> Result<[u8; 32]> {
    SettlementEvidencePackageV0 {
        settlement_binding_id: binding_id,
        escrow_terminal_commitment: escrow_commitment,
        capability_grant_ids: vec![],
        adapter_response_bytes: vec![],
        logical_time: now,
    }
    .commitment()
}

pub fn report_from_adapter(
    settlement_id: [u8; 32],
    correlation_id: [u8; 32],
    adapter_identity: &str,
    external_reference: &str,
    status: SettlementStatus,
    amount: u64,
    proof_token: Option<[u8; 32]>,
    extra_commitments: Vec<[u8; 32]>,
) -> SettlementReportV0 {
    SettlementReportV0 {
        protocol_version: SETTLEMENT_PROTOCOL_VERSION,
        schema_version: SETTLEMENT_SCHEMA_VERSION,
        settlement_id,
        external_reference: external_reference.into(),
        status,
        amount,
        commitments: extra_commitments,
        adapter_identity: adapter_identity.into(),
        proof_token,
        correlation_id,
    }
}
