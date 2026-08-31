use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Opaque asset identifier (GBP, USD, EUR, ENTERPRISE_LEDGER, USDC, …).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub String);

impl AssetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Integer minor units (scale defined on asset_types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Amount(pub i64);

impl Amount {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn minor(v: i64) -> crate::Result<Self> {
        if v < 0 {
            return Err(crate::TreasuryError::NegativeAmount);
        }
        Ok(Self(v))
    }

    pub fn checked_add(self, other: Amount) -> crate::Result<Amount> {
        self.0
            .checked_add(other.0)
            .filter(|&v| v >= 0)
            .map(Amount)
            .ok_or(crate::TreasuryError::NegativeAmount)
    }

    pub fn checked_sub(self, other: Amount) -> crate::Result<Amount> {
        self.0
            .checked_sub(other.0)
            .filter(|&v| v >= 0)
            .map(Amount)
            .ok_or(crate::TreasuryError::InsufficientFunds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreasuryKind {
    Organisation,
    Department,
    Project,
    Reserve,
    EscrowFunding,
}

impl TreasuryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Organisation => "organisation",
            Self::Department => "department",
            Self::Project => "project",
            Self::Reserve => "reserve",
            Self::EscrowFunding => "escrow_funding",
        }
    }

    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "organisation" => Ok(Self::Organisation),
            "department" => Ok(Self::Department),
            "project" => Ok(Self::Project),
            "reserve" => Ok(Self::Reserve),
            "escrow_funding" => Ok(Self::EscrowFunding),
            _ => Err(crate::TreasuryError::Validation(format!(
                "unknown treasury kind: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreasuryStatus {
    Active,
    Frozen,
    Closed,
}

impl TreasuryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Frozen => "frozen",
            Self::Closed => "closed",
        }
    }

    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "active" => Ok(Self::Active),
            "frozen" => Ok(Self::Frozen),
            "closed" => Ok(Self::Closed),
            _ => Err(crate::TreasuryError::Validation(format!(
                "unknown treasury status: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocationStatus {
    Active,
    Frozen,
    Expired,
    Closed,
}

impl AllocationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Frozen => "frozen",
            Self::Expired => "expired",
            Self::Closed => "closed",
        }
    }

    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "active" => Ok(Self::Active),
            "frozen" => Ok(Self::Frozen),
            "expired" => Ok(Self::Expired),
            "closed" => Ok(Self::Closed),
            _ => Err(crate::TreasuryError::Validation(format!(
                "unknown allocation status: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationKind {
    Budget,
    Escrow,
}

impl ReservationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Budget => "budget",
            Self::Escrow => "escrow",
        }
    }

    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "budget" => Ok(Self::Budget),
            "escrow" => Ok(Self::Escrow),
            _ => Err(crate::TreasuryError::Validation(format!(
                "unknown reservation kind: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationStatus {
    Active,
    Released,
    Consumed,
    Expired,
    /// Hold requires operator attention (orphan / failed recovery); funds still reserved until release.
    Stuck,
}

impl ReservationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Released => "released",
            Self::Consumed => "consumed",
            Self::Expired => "expired",
            Self::Stuck => "stuck",
        }
    }

    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "active" => Ok(Self::Active),
            "released" => Ok(Self::Released),
            "consumed" => Ok(Self::Consumed),
            "expired" => Ok(Self::Expired),
            "stuck" => Ok(Self::Stuck),
            _ => Err(crate::TreasuryError::Validation(format!(
                "unknown reservation status: {s}"
            ))),
        }
    }

    /// Statuses that still hold reserved funds and can be released.
    pub fn is_releasable(self) -> bool {
        matches!(self, Self::Active | Self::Expired | Self::Stuck)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalEventType {
    Funding,
    Allocation,
    Reservation,
    Release,
    EscrowReservation,
    SettlementPost,
    Refund,
    Chargeback,
    Adjustment,
}

impl JournalEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Funding => "funding",
            Self::Allocation => "allocation",
            Self::Reservation => "reservation",
            Self::Release => "release",
            Self::EscrowReservation => "escrow_reservation",
            Self::SettlementPost => "settlement_post",
            Self::Refund => "refund",
            Self::Chargeback => "chargeback",
            Self::Adjustment => "adjustment",
        }
    }
}

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// Account code helpers (asset-side control accounts).
pub fn available_account(treasury_id: &str) -> String {
    format!("treasury_available:{treasury_id}")
}

pub fn reserved_account(treasury_id: &str) -> String {
    format!("treasury_reserved:{treasury_id}")
}

pub fn escrow_reserved_account(treasury_id: &str) -> String {
    format!("escrow_reserved:{treasury_id}")
}

pub fn funding_source_account() -> String {
    "funding_source:external".into()
}

pub fn expense_account() -> String {
    "expense:agent_spend".into()
}

pub fn suspense_account() -> String {
    "suspense:clearing".into()
}

pub fn allocation_control_account(allocation_id: &str) -> String {
    format!("allocation_control:{allocation_id}")
}
