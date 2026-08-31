use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetType {
    pub asset_id: AssetId,
    pub scale: i64,
    pub class: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treasury {
    pub treasury_id: String,
    pub organisation_id: String,
    pub parent_treasury_id: Option<String>,
    pub kind: TreasuryKind,
    pub name: String,
    pub status: TreasuryStatus,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allocation {
    pub allocation_id: String,
    pub organisation_id: String,
    pub treasury_id: String,
    pub agent_id: String,
    pub asset_id: AssetId,
    pub ceiling_minor: i64,
    pub remaining_minor: i64,
    pub status: AllocationStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reservation {
    pub reservation_id: String,
    pub organisation_id: String,
    pub treasury_id: String,
    pub allocation_id: Option<String>,
    pub asset_id: AssetId,
    pub amount_minor: i64,
    pub kind: ReservationKind,
    pub status: ReservationStatus,
    pub escrow_id: Option<String>,
    pub idempotency_key: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalBatch {
    pub batch_id: String,
    pub organisation_id: String,
    pub event_type: JournalEventType,
    pub request_id: String,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub entry_id: String,
    pub batch_id: String,
    pub organisation_id: String,
    pub event_type: JournalEventType,
    pub treasury_id: Option<String>,
    pub allocation_id: Option<String>,
    pub reservation_id: Option<String>,
    pub asset_id: AssetId,
    pub account_code: String,
    pub debit_minor: i64,
    pub credit_minor: i64,
    pub request_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct JournalLine {
    pub account_code: String,
    pub asset_id: AssetId,
    pub debit_minor: i64,
    pub credit_minor: i64,
    pub treasury_id: Option<String>,
    pub allocation_id: Option<String>,
    pub reservation_id: Option<String>,
}

impl JournalLine {
    pub fn debit(
        account: impl Into<String>,
        asset: AssetId,
        amount: Amount,
        treasury_id: Option<String>,
    ) -> crate::Result<Self> {
        if amount.0 < 0 {
            return Err(crate::TreasuryError::NegativeAmount);
        }
        Ok(Self {
            account_code: account.into(),
            asset_id: asset,
            debit_minor: amount.0,
            credit_minor: 0,
            treasury_id,
            allocation_id: None,
            reservation_id: None,
        })
    }

    pub fn credit(
        account: impl Into<String>,
        asset: AssetId,
        amount: Amount,
        treasury_id: Option<String>,
    ) -> crate::Result<Self> {
        if amount.0 < 0 {
            return Err(crate::TreasuryError::NegativeAmount);
        }
        Ok(Self {
            account_code: account.into(),
            asset_id: asset,
            debit_minor: 0,
            credit_minor: amount.0,
            treasury_id,
            allocation_id: None,
            reservation_id: None,
        })
    }
}
