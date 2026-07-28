//! Escrow status and snapshot models.

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_text, as_u32, as_u64, bytes, encode_value, map, map_get, optional_bytes, text,
    u32_value, u64_value,
};
use crate::error::{Error, Result};
use crate::escrow::terms::EscrowTermsV0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscrowStatus {
    Proposed,
    Funded,
    ReceiptAccepted,
    Released,
    Refunded,
    Disputed,
    ResolvedReleased,
    ResolvedRefunded,
    Cancelled,
    Expired,
}

impl EscrowStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Funded => "funded",
            Self::ReceiptAccepted => "receipt_accepted",
            Self::Released => "released",
            Self::Refunded => "refunded",
            Self::Disputed => "disputed",
            Self::ResolvedReleased => "resolved_released",
            Self::ResolvedRefunded => "resolved_refunded",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "proposed" => Ok(Self::Proposed),
            "funded" => Ok(Self::Funded),
            "receipt_accepted" => Ok(Self::ReceiptAccepted),
            "released" => Ok(Self::Released),
            "refunded" => Ok(Self::Refunded),
            "disputed" => Ok(Self::Disputed),
            "resolved_released" => Ok(Self::ResolvedReleased),
            "resolved_refunded" => Ok(Self::ResolvedRefunded),
            "cancelled" => Ok(Self::Cancelled),
            "expired" => Ok(Self::Expired),
            _ => Err(Error::MalformedObject("escrow status")),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Released
                | Self::Refunded
                | Self::ResolvedReleased
                | Self::ResolvedRefunded
                | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscrowOutcome {
    None,
    Released,
    Refunded,
}

impl EscrowOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Released => "released",
            Self::Refunded => "refunded",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "none" => Ok(Self::None),
            "released" => Ok(Self::Released),
            "refunded" => Ok(Self::Refunded),
            _ => Err(Error::MalformedObject("escrow outcome")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicFinalityViewV0 {
    pub soft_local_agreement: bool,
    pub dispute_window_open: bool,
    pub dispute_deadline: Option<u64>,
    pub hard_settlement_placeholder: bool,
    pub finalized: bool,
    pub outcome: EscrowOutcome,
}

impl EconomicFinalityViewV0 {
    pub fn for_status(status: EscrowStatus, soft: bool, dispute_deadline: Option<u64>) -> Self {
        let outcome = match status {
            EscrowStatus::Released | EscrowStatus::ResolvedReleased => EscrowOutcome::Released,
            EscrowStatus::Refunded | EscrowStatus::ResolvedRefunded => EscrowOutcome::Refunded,
            _ => EscrowOutcome::None,
        };
        Self {
            soft_local_agreement: soft,
            dispute_window_open: matches!(
                status,
                EscrowStatus::ReceiptAccepted | EscrowStatus::Disputed
            ) && dispute_deadline.is_some(),
            dispute_deadline,
            hard_settlement_placeholder: false,
            finalized: status.is_terminal(),
            outcome,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub escrow_id: [u8; 32],
    pub terms: EscrowTermsV0,
    pub status: EscrowStatus,
    pub funded_amount: u64,
    pub fee_reserved: u64,
    pub fee_consumed: u64,
    pub fee_refunded: u64,
    pub receipt_id: Option<[u8; 32]>,
    pub created_at: u64,
    pub funded_at: Option<u64>,
    pub receipt_submitted_at: Option<u64>,
    pub finalized_at: Option<u64>,
    pub finality: EconomicFinalityViewV0,
}

impl EscrowV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("escrow_id"), bytes(self.escrow_id.to_vec())),
            (text("terms"), self.terms.to_cbor_value()),
            (text("status"), text(self.status.as_str())),
            (text("funded_amount"), u64_value(self.funded_amount)),
            (text("fee_reserved"), u64_value(self.fee_reserved)),
            (text("fee_consumed"), u64_value(self.fee_consumed)),
            (text("fee_refunded"), u64_value(self.fee_refunded)),
            (
                text("receipt_id"),
                optional_bytes(&self.receipt_id.map(|id| id.to_vec())),
            ),
            (text("created_at"), u64_value(self.created_at)),
            (
                text("funded_at"),
                self.funded_at.map_or(Value::Null, u64_value),
            ),
            (
                text("receipt_submitted_at"),
                self.receipt_submitted_at.map_or(Value::Null, u64_value),
            ),
            (
                text("finalized_at"),
                self.finalized_at.map_or(Value::Null, u64_value),
            ),
            (text("finality"), self.finality.to_cbor_value()),
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
            return Err(Error::MalformedObject("escrow must be map"));
        };
        let terms_val = map_get(entries, "terms")?;
        let receipt_raw = map_get(entries, "receipt_id")?;
        let receipt_id = match receipt_raw {
            Value::Null => None,
            _ => {
                let b = as_bytes(receipt_raw)?;
                let arr: [u8; 32] = b
                    .try_into()
                    .map_err(|_| Error::MalformedObject("receipt_id"))?;
                Some(arr)
            }
        };
        let funded_at = match map_get(entries, "funded_at")? {
            Value::Null => None,
            v => Some(as_u64(v)?),
        };
        let receipt_submitted_at = match map_get(entries, "receipt_submitted_at")? {
            Value::Null => None,
            v => Some(as_u64(v)?),
        };
        let finalized_at = match map_get(entries, "finalized_at")? {
            Value::Null => None,
            v => Some(as_u64(v)?),
        };
        Ok(Self {
            protocol_version: as_u32(map_get(entries, "protocol_version")?)?,
            schema_version: as_u32(map_get(entries, "schema_version")?)?,
            escrow_id: {
                let b = as_bytes(map_get(entries, "escrow_id")?)?;
                b.try_into()
                    .map_err(|_| Error::MalformedObject("escrow_id"))?
            },
            terms: EscrowTermsV0::from_cbor_value(terms_val)?,
            status: EscrowStatus::parse(as_text(map_get(entries, "status")?)?)?,
            funded_amount: as_u64(map_get(entries, "funded_amount")?)?,
            fee_reserved: as_u64(map_get(entries, "fee_reserved")?)?,
            fee_consumed: as_u64(map_get(entries, "fee_consumed")?)?,
            fee_refunded: as_u64(map_get(entries, "fee_refunded")?)?,
            receipt_id,
            created_at: as_u64(map_get(entries, "created_at")?)?,
            funded_at,
            receipt_submitted_at,
            finalized_at,
            finality: EconomicFinalityViewV0::from_cbor_value(map_get(entries, "finality")?)?,
        })
    }

    pub fn check_conservation(&self) -> Result<()> {
        let left = self
            .funded_amount
            .checked_add(self.fee_reserved)
            .ok_or(Error::EscrowValueConservation)?;
        let principal_out = if self.status.is_terminal() {
            match self.status {
                EscrowStatus::Released | EscrowStatus::ResolvedReleased => self.funded_amount,
                EscrowStatus::Refunded | EscrowStatus::ResolvedRefunded => self.funded_amount,
                EscrowStatus::Cancelled => 0,
                _ => 0,
            }
        } else {
            self.funded_amount
        };
        let right = principal_out
            .checked_add(self.fee_consumed)
            .and_then(|v| v.checked_add(self.fee_refunded))
            .ok_or(Error::EscrowValueConservation)?;
        if self.status.is_terminal() && self.funded_amount > 0 && left != right {
            return Err(Error::EscrowValueConservation);
        }
        Ok(())
    }
}

impl EconomicFinalityViewV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (
                text("soft_local_agreement"),
                u64_value(if self.soft_local_agreement { 1 } else { 0 }),
            ),
            (
                text("dispute_window_open"),
                u64_value(if self.dispute_window_open { 1 } else { 0 }),
            ),
            (
                text("dispute_deadline"),
                self.dispute_deadline.map_or(Value::Null, u64_value),
            ),
            (
                text("hard_settlement_placeholder"),
                u64_value(if self.hard_settlement_placeholder {
                    1
                } else {
                    0
                }),
            ),
            (
                text("finalized"),
                u64_value(if self.finalized { 1 } else { 0 }),
            ),
            (text("outcome"), text(self.outcome.as_str())),
        ])
    }

    pub fn from_cbor_value(value: &Value) -> Result<Self> {
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("finality must be map"));
        };
        let dispute_deadline = match map_get(entries, "dispute_deadline")? {
            Value::Null => None,
            v => Some(as_u64(v)?),
        };
        Ok(Self {
            soft_local_agreement: as_u64(map_get(entries, "soft_local_agreement")?)? != 0,
            dispute_window_open: as_u64(map_get(entries, "dispute_window_open")?)? != 0,
            dispute_deadline,
            hard_settlement_placeholder: as_u64(map_get(entries, "hard_settlement_placeholder")?)?
                != 0,
            finalized: as_u64(map_get(entries, "finalized")?)? != 0,
            outcome: EscrowOutcome::parse(as_text(map_get(entries, "outcome")?)?)?,
        })
    }
}
