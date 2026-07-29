//! PROTO-3 data models: ReputationEventV0, EvidenceRefV0, and supporting enums.

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, bytes, encode_value, map, map_get, text, u32_value,
    u64_value,
};
use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::types::AgentId;

pub const REPUTATION_PROTOCOL_VERSION: u32 = 1;
pub const REPUTATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    EscrowFunded,
    EscrowReceiptAccepted,
    EscrowReleased,
    EscrowRefunded,
    EscrowExpired,
    EscrowDisputeRaised,
    EscrowDisputeResolved,
    SettlementRequested,
    SettlementFinalized,
    SettlementFailed,
    ChannelClosedCooperative,
    ChannelDisputeLost,
    IdentityRevoked,
    IntegrityReplayRejected,
}

impl EventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EscrowFunded => "aether.reputation.escrow.funded.v0",
            Self::EscrowReceiptAccepted => "aether.reputation.escrow.receipt_accepted.v0",
            Self::EscrowReleased => "aether.reputation.escrow.released.v0",
            Self::EscrowRefunded => "aether.reputation.escrow.refunded.v0",
            Self::EscrowExpired => "aether.reputation.escrow.expired.v0",
            Self::EscrowDisputeRaised => "aether.reputation.escrow.dispute_raised.v0",
            Self::EscrowDisputeResolved => "aether.reputation.escrow.dispute_resolved.v0",
            Self::SettlementRequested => "aether.reputation.settlement.requested.v0",
            Self::SettlementFinalized => "aether.reputation.settlement.finalized.v0",
            Self::SettlementFailed => "aether.reputation.settlement.failed.v0",
            Self::ChannelClosedCooperative => "aether.reputation.channel.closed_cooperative.v0",
            Self::ChannelDisputeLost => "aether.reputation.channel.dispute_lost.v0",
            Self::IdentityRevoked => "aether.reputation.identity.revoked.v0",
            Self::IntegrityReplayRejected => "aether.reputation.integrity.replay_rejected.v0",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "aether.reputation.escrow.funded.v0" => Ok(Self::EscrowFunded),
            "aether.reputation.escrow.receipt_accepted.v0" => Ok(Self::EscrowReceiptAccepted),
            "aether.reputation.escrow.released.v0" => Ok(Self::EscrowReleased),
            "aether.reputation.escrow.refunded.v0" => Ok(Self::EscrowRefunded),
            "aether.reputation.escrow.expired.v0" => Ok(Self::EscrowExpired),
            "aether.reputation.escrow.dispute_raised.v0" => Ok(Self::EscrowDisputeRaised),
            "aether.reputation.escrow.dispute_resolved.v0" => Ok(Self::EscrowDisputeResolved),
            "aether.reputation.settlement.requested.v0" => Ok(Self::SettlementRequested),
            "aether.reputation.settlement.finalized.v0" => Ok(Self::SettlementFinalized),
            "aether.reputation.settlement.failed.v0" => Ok(Self::SettlementFailed),
            "aether.reputation.channel.closed_cooperative.v0" => Ok(Self::ChannelClosedCooperative),
            "aether.reputation.channel.dispute_lost.v0" => Ok(Self::ChannelDisputeLost),
            "aether.reputation.identity.revoked.v0" => Ok(Self::IdentityRevoked),
            "aether.reputation.integrity.replay_rejected.v0" => Ok(Self::IntegrityReplayRejected),
            _ => Err(Error::MalformedObject("unknown reputation event type")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribution {
    Credit,
    Debit,
    Neutral,
    Fault,
}

impl Attribution {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Credit => "credit",
            Self::Debit => "debit",
            Self::Neutral => "neutral",
            Self::Fault => "fault",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "credit" => Ok(Self::Credit),
            "debit" => Ok(Self::Debit),
            "neutral" => Ok(Self::Neutral),
            "fault" => Ok(Self::Fault),
            _ => Err(Error::MalformedObject("unknown attribution")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSourceProto {
    Proto0,
    Proto1,
    Proto2,
    Proto4,
    ProtoNet0,
}

impl EvidenceSourceProto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proto0 => "PROTO-0",
            Self::Proto1 => "PROTO-1",
            Self::Proto2 => "PROTO-2",
            Self::Proto4 => "PROTO-4",
            Self::ProtoNet0 => "PROTO-NET-0",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "PROTO-0" => Ok(Self::Proto0),
            "PROTO-1" => Ok(Self::Proto1),
            "PROTO-2" => Ok(Self::Proto2),
            "PROTO-4" => Ok(Self::Proto4),
            "PROTO-NET-0" => Ok(Self::ProtoNet0),
            _ => Err(Error::MalformedObject("unknown source proto")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceRefType {
    EscrowTerminal,
    SettlementBinding,
    Receipt,
    ChannelClose,
    IdentityStatus,
}

impl EvidenceRefType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EscrowTerminal => "escrow_terminal",
            Self::SettlementBinding => "settlement_binding",
            Self::Receipt => "receipt",
            Self::ChannelClose => "channel_close",
            Self::IdentityStatus => "identity_status",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "escrow_terminal" => Ok(Self::EscrowTerminal),
            "settlement_binding" => Ok(Self::SettlementBinding),
            "receipt" => Ok(Self::Receipt),
            "channel_close" => Ok(Self::ChannelClose),
            "identity_status" => Ok(Self::IdentityStatus),
            _ => Err(Error::MalformedObject("unknown evidence ref type")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRefV0 {
    pub ref_type: EvidenceRefType,
    pub commitment: [u8; 32],
    pub source_proto: EvidenceSourceProto,
    pub locator: Option<[u8; 32]>,
}

impl EvidenceRefV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("ref_type"), text(self.ref_type.as_str())),
            (text("commitment"), bytes(self.commitment.to_vec())),
            (text("source_proto"), text(self.source_proto.as_str())),
            (
                text("locator"),
                match self.locator {
                    Some(l) => bytes(l.to_vec()),
                    None => Value::Null,
                },
            ),
        ])
    }

    pub fn from_cbor_value(value: &Value) -> Result<Self> {
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("evidence ref must be map"));
        };
        let commitment_bytes = as_bytes(map_get(entries, "commitment")?)?;
        let commitment: [u8; 32] = commitment_bytes
            .try_into()
            .map_err(|_| Error::MalformedObject("commitment length"))?;
        let locator = match map_get(entries, "locator")? {
            Value::Null => None,
            v => {
                let b = as_bytes(v)?;
                let arr: [u8; 32] = b
                    .try_into()
                    .map_err(|_| Error::MalformedObject("locator length"))?;
                Some(arr)
            }
        };
        Ok(Self {
            ref_type: EvidenceRefType::parse(as_text(map_get(entries, "ref_type")?)?)?,
            commitment,
            source_proto: EvidenceSourceProto::parse(as_text(map_get(entries, "source_proto")?)?)?,
            locator,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReputationEventV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub event_id: [u8; 32],
    pub event_type: EventType,
    pub subject_agent_id: AgentId,
    pub counterparty_agent_id: Option<AgentId>,
    pub logical_time: u64,
    pub evidence_refs: Vec<EvidenceRefV0>,
    pub attribution: Attribution,
    pub weight_hint: Option<u64>,
}

impl ReputationEventV0 {
    /// Canonical body without event_id — used to compute the deterministic event_id.
    pub fn body_cbor(&self) -> Value {
        let evidence_arr: Vec<Value> = self
            .evidence_refs
            .iter()
            .map(|e| e.to_cbor_value())
            .collect();
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("event_type"), text(self.event_type.as_str())),
            (
                text("subject_agent_id"),
                text(self.subject_agent_id.clone()),
            ),
            (
                text("counterparty_agent_id"),
                match &self.counterparty_agent_id {
                    Some(id) => text(id.clone()),
                    None => Value::Null,
                },
            ),
            (text("logical_time"), u64_value(self.logical_time)),
            (text("evidence_refs"), Value::Array(evidence_arr)),
            (text("attribution"), text(self.attribution.as_str())),
            (
                text("weight_hint"),
                match self.weight_hint {
                    Some(w) => u64_value(w),
                    None => Value::Null,
                },
            ),
        ])
    }

    pub fn compute_event_id(&self) -> Result<[u8; 32]> {
        Ok(sha256(&encode_value(&self.body_cbor())?))
    }

    pub fn with_computed_id(mut self) -> Result<Self> {
        self.event_id = self.compute_event_id()?;
        Ok(self)
    }

    pub fn verify_id(&self) -> Result<()> {
        let expected = self.compute_event_id()?;
        if self.event_id != expected {
            return Err(Error::MalformedObject("event_id mismatch"));
        }
        Ok(())
    }

    pub fn to_cbor_value(&self) -> Value {
        let mut entries = match self.body_cbor() {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        entries.insert(2, (text("event_id"), bytes(self.event_id.to_vec())));
        Value::Map(entries)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("reputation event must be map"));
        };

        let event_id_bytes = as_bytes(map_get(&entries, "event_id")?)?;
        let event_id: [u8; 32] = event_id_bytes
            .try_into()
            .map_err(|_| Error::MalformedObject("event_id length"))?;

        let counterparty = match map_get(&entries, "counterparty_agent_id")? {
            Value::Null => None,
            v => Some(as_text(v)?.to_string()),
        };

        let evidence_refs = match map_get(&entries, "evidence_refs")? {
            Value::Array(arr) => arr
                .iter()
                .map(EvidenceRefV0::from_cbor_value)
                .collect::<Result<Vec<_>>>()?,
            _ => return Err(Error::MalformedObject("evidence_refs array")),
        };

        let weight_hint = match map_get(&entries, "weight_hint")? {
            Value::Null => None,
            v => Some(as_u64(v)?),
        };

        Ok(Self {
            protocol_version: as_u32(map_get(&entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(&entries, "schema_version")?)?,
            event_id,
            event_type: EventType::parse(as_text(map_get(&entries, "event_type")?)?)?,
            subject_agent_id: as_text(map_get(&entries, "subject_agent_id")?)?.to_string(),
            counterparty_agent_id: counterparty,
            logical_time: as_u64(map_get(&entries, "logical_time")?)?,
            evidence_refs,
            attribution: Attribution::parse(as_text(map_get(&entries, "attribution")?)?)?,
            weight_hint,
        })
    }
}
