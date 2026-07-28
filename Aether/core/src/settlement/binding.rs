//! Settlement account binding and escrow settlement binding models.

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, bytes, encode_value, map, map_get, optional_bytes, text,
    u32_value, u64_value,
};
use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::settlement::model::{
    EconomicOutcome, SettlementStatus, SETTLEMENT_PROTOCOL_VERSION, SETTLEMENT_SCHEMA_VERSION,
};
use crate::types::AgentId;

/// AgentId ↔ external account under a named settlement provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementAccountBindingV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub binding_id: [u8; 32],
    pub agent_id: AgentId,
    pub settlement_provider: String,
    pub external_account_ref: String,
    pub asset: String,
    pub scope: String,
    pub binding_version: u64,
    pub commitments: Vec<[u8; 32]>,
    pub valid_after: u64,
    pub valid_before: u64,
    pub created_at: u64,
}

impl SettlementAccountBindingV0 {
    pub fn validate(&self) -> Result<()> {
        if self.external_account_ref.is_empty() {
            return Err(Error::MalformedObject("empty external_account_ref"));
        }
        if self.settlement_provider.is_empty() {
            return Err(Error::MalformedObject("empty settlement_provider"));
        }
        if self.agent_id.is_empty() {
            return Err(Error::MalformedObject("empty agent_id"));
        }
        if self.valid_after > self.valid_before {
            return Err(Error::MalformedObject("invalid validity window"));
        }
        Ok(())
    }

    pub fn compute_binding_id(&self) -> Result<[u8; 32]> {
        Ok(sha256(&encode_value(&self.to_cbor_value_without_id())?))
    }

    pub fn with_computed_id(mut self) -> Result<Self> {
        self.validate()?;
        self.binding_id = self.compute_binding_id()?;
        Ok(self)
    }

    pub fn verify_id(&self) -> Result<()> {
        self.validate()?;
        if self.binding_id != self.compute_binding_id()? {
            return Err(Error::MalformedObject("account binding_id mismatch"));
        }
        Ok(())
    }

    fn to_cbor_value_without_id(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("agent_id"), text(self.agent_id.clone())),
            (
                text("settlement_provider"),
                text(self.settlement_provider.clone()),
            ),
            (
                text("external_account_ref"),
                text(self.external_account_ref.clone()),
            ),
            (text("asset"), text(self.asset.clone())),
            (text("scope"), text(self.scope.clone())),
            (text("binding_version"), u64_value(self.binding_version)),
            (
                text("commitments"),
                Value::Array(self.commitments.iter().map(|c| bytes(c.to_vec())).collect()),
            ),
            (text("valid_after"), u64_value(self.valid_after)),
            (text("valid_before"), u64_value(self.valid_before)),
            (text("created_at"), u64_value(self.created_at)),
        ])
    }

    pub fn to_cbor_value(&self) -> Value {
        let mut entries = match self.to_cbor_value_without_id() {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        entries.insert(0, (text("binding_id"), bytes(self.binding_id.to_vec())));
        Value::Map(entries)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("account binding map"));
        };
        let binding_id = as_bytes(map_get(&entries, "binding_id")?)?;
        let mut id = [0u8; 32];
        if binding_id.len() != 32 {
            return Err(Error::MalformedObject("binding_id length"));
        }
        id.copy_from_slice(binding_id);

        let commitments_val = map_get(&entries, "commitments")?;
        let commitments = match commitments_val {
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
            _ => return Err(Error::MalformedObject("commitments array")),
        };

        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            binding_id: id,
            agent_id: as_text(map_get(&entries, "agent_id")?)?.into(),
            settlement_provider: as_text(map_get(&entries, "settlement_provider")?)?.into(),
            external_account_ref: as_text(map_get(&entries, "external_account_ref")?)?.into(),
            asset: as_text(map_get(&entries, "asset")?)?.into(),
            scope: as_text(map_get(&entries, "scope")?)?.into(),
            binding_version: as_u64(map_get(&entries, "binding_version")?)?,
            commitments,
            valid_after: as_u64(map_get(&entries, "valid_after")?)?,
            valid_before: as_u64(map_get(&entries, "valid_before")?)?,
            created_at: as_u64(map_get(&entries, "created_at")?)?,
        })
    }
}

/// Escrow outcome ↔ external settlement attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementBindingV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub binding_id: [u8; 32],
    pub escrow_id: [u8; 32],
    pub terms_version: u64,
    pub economic_outcome: EconomicOutcome,
    pub principal_amount: u64,
    pub fee_amount: u64,
    pub asset: String,
    pub payer_agent_id: AgentId,
    pub provider_agent_id: AgentId,
    pub settlement_provider: String,
    pub payer_account_binding_id: [u8; 32],
    pub provider_account_binding_id: Option<[u8; 32]>,
    pub destination_binding_id: [u8; 32],
    pub external_settlement_ref: Option<String>,
    pub settlement_status: SettlementStatus,
    pub aether_escrow_status: String,
    pub requested_at: u64,
    pub submitted_at: Option<u64>,
    pub accepted_at: Option<u64>,
    pub confirmed_at: Option<u64>,
    pub finalized_at: Option<u64>,
    pub evidence_commitment: [u8; 32],
    pub adapter_receipt_commitment: Option<[u8; 32]>,
    pub correlation_id: [u8; 32],
}

impl SettlementBindingV0 {
    /// Immutable intent fields used for binding_id / correlation_id.
    pub fn intent_cbor(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("escrow_id"), bytes(self.escrow_id.to_vec())),
            (text("terms_version"), u64_value(self.terms_version)),
            (
                text("economic_outcome"),
                text(self.economic_outcome.as_str()),
            ),
            (text("principal_amount"), u64_value(self.principal_amount)),
            (text("fee_amount"), u64_value(self.fee_amount)),
            (text("asset"), text(self.asset.clone())),
            (text("payer_agent_id"), text(self.payer_agent_id.clone())),
            (
                text("provider_agent_id"),
                text(self.provider_agent_id.clone()),
            ),
            (
                text("settlement_provider"),
                text(self.settlement_provider.clone()),
            ),
            (
                text("payer_account_binding_id"),
                bytes(self.payer_account_binding_id.to_vec()),
            ),
            (
                text("provider_account_binding_id"),
                optional_bytes(&self.provider_account_binding_id.map(|b| b.to_vec())),
            ),
            (
                text("destination_binding_id"),
                bytes(self.destination_binding_id.to_vec()),
            ),
            (
                text("aether_escrow_status"),
                text(self.aether_escrow_status.clone()),
            ),
        ])
    }

    pub fn compute_binding_id(&self) -> Result<[u8; 32]> {
        Ok(sha256(&encode_value(&self.intent_cbor())?))
    }

    pub fn compute_correlation_id(&self) -> Result<[u8; 32]> {
        // correlation_id is deterministic over the same intent inputs.
        Ok(sha256(&encode_value(&self.intent_cbor())?))
    }

    pub fn with_computed_ids(mut self) -> Result<Self> {
        self.binding_id = self.compute_binding_id()?;
        self.correlation_id = self.compute_correlation_id()?;
        Ok(self)
    }

    pub fn settlement_id(&self) -> [u8; 32] {
        self.binding_id
    }

    pub fn to_cbor_value(&self) -> Value {
        let mut entries = match self.intent_cbor() {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        entries.insert(0, (text("binding_id"), bytes(self.binding_id.to_vec())));
        entries.push((
            text("external_settlement_ref"),
            match &self.external_settlement_ref {
                Some(s) => text(s.clone()),
                None => Value::Null,
            },
        ));
        entries.push((
            text("settlement_status"),
            text(self.settlement_status.as_str()),
        ));
        entries.push((text("requested_at"), u64_value(self.requested_at)));
        entries.push((text("submitted_at"), optional_u64(self.submitted_at)));
        entries.push((text("accepted_at"), optional_u64(self.accepted_at)));
        entries.push((text("confirmed_at"), optional_u64(self.confirmed_at)));
        entries.push((text("finalized_at"), optional_u64(self.finalized_at)));
        entries.push((
            text("evidence_commitment"),
            bytes(self.evidence_commitment.to_vec()),
        ));
        entries.push((
            text("adapter_receipt_commitment"),
            optional_bytes(&self.adapter_receipt_commitment.map(|b| b.to_vec())),
        ));
        entries.push((text("correlation_id"), bytes(self.correlation_id.to_vec())));
        Value::Map(entries)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("settlement binding map"));
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

        let provider_account_binding_id = match map_get(&entries, "provider_account_binding_id")? {
            Value::Null => None,
            Value::Bytes(b) => {
                if b.len() != 32 {
                    return Err(Error::MalformedObject("provider binding length"));
                }
                let mut id = [0u8; 32];
                id.copy_from_slice(b);
                Some(id)
            }
            _ => return Err(Error::MalformedObject("provider_account_binding_id")),
        };

        let external_settlement_ref = match map_get(&entries, "external_settlement_ref")? {
            Value::Null => None,
            Value::Text(s) => Some(s.clone()),
            _ => return Err(Error::MalformedObject("external_settlement_ref")),
        };

        let adapter_receipt_commitment = match map_get(&entries, "adapter_receipt_commitment")? {
            Value::Null => None,
            Value::Bytes(b) => {
                if b.len() != 32 {
                    return Err(Error::MalformedObject("adapter receipt length"));
                }
                let mut id = [0u8; 32];
                id.copy_from_slice(b);
                Some(id)
            }
            _ => return Err(Error::MalformedObject("adapter_receipt_commitment")),
        };

        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            binding_id: fixed32(&entries, "binding_id")?,
            escrow_id: fixed32(&entries, "escrow_id")?,
            terms_version: as_u64(map_get(&entries, "terms_version")?)?,
            economic_outcome: EconomicOutcome::parse(as_text(map_get(
                &entries,
                "economic_outcome",
            )?)?)?,
            principal_amount: as_u64(map_get(&entries, "principal_amount")?)?,
            fee_amount: as_u64(map_get(&entries, "fee_amount")?)?,
            asset: as_text(map_get(&entries, "asset")?)?.into(),
            payer_agent_id: as_text(map_get(&entries, "payer_agent_id")?)?.into(),
            provider_agent_id: as_text(map_get(&entries, "provider_agent_id")?)?.into(),
            settlement_provider: as_text(map_get(&entries, "settlement_provider")?)?.into(),
            payer_account_binding_id: fixed32(&entries, "payer_account_binding_id")?,
            provider_account_binding_id,
            destination_binding_id: fixed32(&entries, "destination_binding_id")?,
            external_settlement_ref,
            settlement_status: SettlementStatus::parse(as_text(map_get(
                &entries,
                "settlement_status",
            )?)?)?,
            aether_escrow_status: as_text(map_get(&entries, "aether_escrow_status")?)?.into(),
            requested_at: as_u64(map_get(&entries, "requested_at")?)?,
            submitted_at: optional_u64_decode(map_get(&entries, "submitted_at")?)?,
            accepted_at: optional_u64_decode(map_get(&entries, "accepted_at")?)?,
            confirmed_at: optional_u64_decode(map_get(&entries, "confirmed_at")?)?,
            finalized_at: optional_u64_decode(map_get(&entries, "finalized_at")?)?,
            evidence_commitment: fixed32(&entries, "evidence_commitment")?,
            adapter_receipt_commitment,
            correlation_id: fixed32(&entries, "correlation_id")?,
        })
    }

    pub fn amount(&self) -> u64 {
        self.principal_amount
    }
}

fn optional_u64(v: Option<u64>) -> Value {
    match v {
        Some(n) => u64_value(n),
        None => Value::Null,
    }
}

fn optional_u64_decode(v: &Value) -> Result<Option<u64>> {
    match v {
        Value::Null => Ok(None),
        other => Ok(Some(as_u64(other)?)),
    }
}

/// Unsigned settle intent used to build a binding request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementIntentV0 {
    pub escrow_id: [u8; 32],
    pub terms_version: u64,
    pub economic_outcome: EconomicOutcome,
    pub principal_amount: u64,
    pub fee_amount: u64,
    pub asset: String,
    pub settlement_provider: String,
    pub payer_account_binding_id: [u8; 32],
    pub provider_account_binding_id: Option<[u8; 32]>,
    pub aether_escrow_status: String,
}

impl SettlementIntentV0 {
    pub fn destination_binding_id(&self) -> [u8; 32] {
        match self.economic_outcome {
            EconomicOutcome::ReleaseToProvider => self
                .provider_account_binding_id
                .unwrap_or(self.payer_account_binding_id),
            EconomicOutcome::RefundToPayer | EconomicOutcome::FeeOnly => {
                self.payer_account_binding_id
            }
        }
    }

    pub fn to_proto_binding(
        &self,
        payer_agent_id: AgentId,
        provider_agent_id: AgentId,
        now: u64,
        evidence_commitment: [u8; 32],
    ) -> Result<SettlementBindingV0> {
        let binding = SettlementBindingV0 {
            protocol_version: SETTLEMENT_PROTOCOL_VERSION,
            schema_version: SETTLEMENT_SCHEMA_VERSION,
            binding_id: [0u8; 32],
            escrow_id: self.escrow_id,
            terms_version: self.terms_version,
            economic_outcome: self.economic_outcome,
            principal_amount: self.principal_amount,
            fee_amount: self.fee_amount,
            asset: self.asset.clone(),
            payer_agent_id,
            provider_agent_id,
            settlement_provider: self.settlement_provider.clone(),
            payer_account_binding_id: self.payer_account_binding_id,
            provider_account_binding_id: self.provider_account_binding_id,
            destination_binding_id: self.destination_binding_id(),
            external_settlement_ref: None,
            settlement_status: SettlementStatus::Requested,
            aether_escrow_status: self.aether_escrow_status.clone(),
            requested_at: now,
            submitted_at: None,
            accepted_at: None,
            confirmed_at: None,
            finalized_at: None,
            evidence_commitment,
            adapter_receipt_commitment: None,
            correlation_id: [0u8; 32],
        };
        binding.with_computed_ids()
    }
}
