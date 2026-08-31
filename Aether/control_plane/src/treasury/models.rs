use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TreasuryListItem {
    pub treasury_id: String,
    pub organisation_id: String,
    pub parent_treasury_id: Option<String>,
    pub kind: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub balances: Vec<BalanceDto>,
}

#[derive(Debug, Serialize)]
pub struct BalanceDto {
    pub asset_id: String,
    pub available_minor: i64,
    pub reserved_minor: i64,
    pub escrow_reserved_minor: i64,
}

#[derive(Debug, Serialize)]
pub struct TreasuryDetail {
    pub treasury: TreasuryListItem,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AllocationDto {
    pub allocation_id: String,
    pub organisation_id: String,
    pub treasury_id: String,
    pub agent_id: String,
    pub asset_id: String,
    pub ceiling_minor: i64,
    pub remaining_minor: i64,
    pub status: String,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ReservationDto {
    pub reservation_id: String,
    pub organisation_id: String,
    pub treasury_id: String,
    pub allocation_id: Option<String>,
    pub asset_id: String,
    pub amount_minor: i64,
    pub kind: String,
    pub status: String,
    pub escrow_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub read_only: bool,
}

#[derive(Debug, Serialize)]
pub struct JournalEntryDto {
    pub entry_id: String,
    pub batch_id: String,
    pub event_type: String,
    pub treasury_id: Option<String>,
    pub allocation_id: Option<String>,
    pub reservation_id: Option<String>,
    pub asset_id: String,
    pub account_code: String,
    pub debit_minor: i64,
    pub credit_minor: i64,
    pub request_id: String,
    pub created_at: String,
    pub immutable: bool,
}

#[derive(Debug, Serialize)]
pub struct SettlementPostDto {
    pub entry_id: String,
    pub batch_id: String,
    pub treasury_id: Option<String>,
    pub reservation_id: Option<String>,
    pub asset_id: String,
    pub amount_minor: i64,
    pub created_at: String,
    pub request_id: String,
    /// Distinguishes treasury accounting posts from PROTO-4 settlement evidence.
    pub accounting_truth: &'static str,
    pub note: &'static str,
}

#[derive(Debug, Serialize)]
pub struct SecurityViewDto {
    pub treasury_id: String,
    pub organisation_id: String,
    pub treasury_status: String,
    pub frozen_treasury: bool,
    pub frozen_or_expired_allocations: i64,
    pub active_reservations: i64,
    pub consumed_reservations: i64,
    pub released_reservations: i64,
    pub aged_active_reservations: i64,
    pub settlement_posts: i64,
    pub adjustments: i64,
    pub chargebacks: i64,
    pub as_of: String,
    pub related_audit_hint: &'static str,
}

#[derive(Debug, Serialize)]
pub struct TreasuryOverview {
    pub organisation_id: String,
    pub treasuries: Vec<TreasuryListItem>,
    pub totals: Vec<BalanceDto>,
    pub observation_only: bool,
    pub note: &'static str,
}
