//! PROTO-4 settlement constants and status enums.

use crate::error::{Error, Result};

pub const SETTLEMENT_PROTOCOL_VERSION: u32 = 1;
pub const SETTLEMENT_SCHEMA_VERSION: u32 = 1;

pub const PROVIDER_ENTERPRISE_LEDGER_V0: &str = "enterprise.ledger.v0";

pub const ACTION_BIND: &str = "settlement.bind";
pub const ACTION_SETTLE: &str = "settlement.settle";
pub const ACTION_QUERY: &str = "settlement.query";
pub const ACTION_CANCEL: &str = "settlement.cancel";

pub const MSG_ACCOUNT_BIND: &str = "settlement.account_bind";
pub const MSG_SETTLE_REQUEST: &str = "settlement.request";
pub const MSG_SETTLE_SUBMIT: &str = "settlement.submit";
pub const MSG_SETTLE_QUERY: &str = "settlement.query";
pub const MSG_SETTLE_CANCEL: &str = "settlement.cancel";
pub const MSG_SETTLE_FINALIZE: &str = "settlement.finalize";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementStatus {
    Requested,
    Submitted,
    Accepted,
    Confirmed,
    Finalized,
    Failed,
    Cancelled,
    DisputedExternal,
}

impl SettlementStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Submitted => "submitted",
            Self::Accepted => "accepted",
            Self::Confirmed => "confirmed",
            Self::Finalized => "finalized",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::DisputedExternal => "disputed_external",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "requested" => Ok(Self::Requested),
            "submitted" => Ok(Self::Submitted),
            "accepted" => Ok(Self::Accepted),
            "confirmed" => Ok(Self::Confirmed),
            "finalized" => Ok(Self::Finalized),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "disputed_external" => Ok(Self::DisputedExternal),
            _ => Err(Error::MalformedObject("settlement status")),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Finalized | Self::Failed | Self::Cancelled | Self::DisputedExternal
        )
    }

    pub fn is_active(self) -> bool {
        !self.is_terminal()
    }

    pub fn allows_cancel(self) -> bool {
        matches!(self, Self::Requested | Self::Submitted | Self::Accepted)
    }

    pub fn may_set_hard_settlement(self) -> bool {
        matches!(self, Self::Confirmed | Self::Finalized)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EconomicOutcome {
    ReleaseToProvider,
    RefundToPayer,
    FeeOnly,
}

impl EconomicOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReleaseToProvider => "release_to_provider",
            Self::RefundToPayer => "refund_to_payer",
            Self::FeeOnly => "fee_only",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "release_to_provider" => Ok(Self::ReleaseToProvider),
            "refund_to_payer" => Ok(Self::RefundToPayer),
            "fee_only" => Ok(Self::FeeOnly),
            _ => Err(Error::MalformedObject("economic outcome")),
        }
    }
}
