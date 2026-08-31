//! Read-only mapping helpers.

use aether_treasury::{
    Allocation, AssetBalance, JournalEntry, JournalEventType, Reservation, Treasury,
    TreasurySecurityMetrics,
};

use super::models::*;

pub fn map_balance(b: AssetBalance) -> BalanceDto {
    BalanceDto {
        asset_id: b.asset_id,
        available_minor: b.available_minor,
        reserved_minor: b.reserved_minor,
        escrow_reserved_minor: b.escrow_reserved_minor,
    }
}

pub fn map_treasury(t: Treasury, balances: Vec<AssetBalance>) -> TreasuryListItem {
    TreasuryListItem {
        treasury_id: t.treasury_id,
        organisation_id: t.organisation_id,
        parent_treasury_id: t.parent_treasury_id,
        kind: t.kind.as_str().into(),
        name: t.name,
        status: t.status.as_str().into(),
        created_at: t.created_at.to_rfc3339(),
        balances: balances.into_iter().map(map_balance).collect(),
    }
}

pub fn map_allocation(a: Allocation) -> AllocationDto {
    AllocationDto {
        allocation_id: a.allocation_id,
        organisation_id: a.organisation_id,
        treasury_id: a.treasury_id,
        agent_id: a.agent_id,
        asset_id: a.asset_id.0,
        ceiling_minor: a.ceiling_minor,
        remaining_minor: a.remaining_minor,
        status: a.status.as_str().into(),
        expires_at: a.expires_at.map(|t| t.to_rfc3339()),
        created_at: a.created_at.to_rfc3339(),
    }
}

pub fn map_reservation(r: Reservation) -> ReservationDto {
    ReservationDto {
        reservation_id: r.reservation_id,
        organisation_id: r.organisation_id,
        treasury_id: r.treasury_id,
        allocation_id: r.allocation_id,
        asset_id: r.asset_id.0,
        amount_minor: r.amount_minor,
        kind: r.kind.as_str().into(),
        status: r.status.as_str().into(),
        escrow_id: r.escrow_id,
        created_at: r.created_at.to_rfc3339(),
        updated_at: r.updated_at.to_rfc3339(),
        read_only: true,
    }
}

pub fn map_journal_entry(e: JournalEntry) -> JournalEntryDto {
    JournalEntryDto {
        entry_id: e.entry_id,
        batch_id: e.batch_id,
        event_type: e.event_type.as_str().into(),
        treasury_id: e.treasury_id,
        allocation_id: e.allocation_id,
        reservation_id: e.reservation_id,
        asset_id: e.asset_id.0,
        account_code: e.account_code,
        debit_minor: e.debit_minor,
        credit_minor: e.credit_minor,
        request_id: e.request_id,
        created_at: e.created_at.to_rfc3339(),
        immutable: true,
    }
}

pub fn map_settlement_post(e: JournalEntry) -> Option<SettlementPostDto> {
    if e.event_type != JournalEventType::SettlementPost {
        return None;
    }
    // One row per batch: expense debit line only
    if e.debit_minor == 0 || !e.account_code.starts_with("expense:") {
        return None;
    }
    Some(SettlementPostDto {
        entry_id: e.entry_id,
        batch_id: e.batch_id,
        treasury_id: e.treasury_id,
        reservation_id: e.reservation_id,
        asset_id: e.asset_id.0,
        amount_minor: e.debit_minor,
        created_at: e.created_at.to_rfc3339(),
        request_id: e.request_id,
        accounting_truth: "treasury_journal",
        note: "Treasury settlement_post is accounting truth for org books; PROTO-4 soft/hard finality is separate protocol evidence.",
    })
}

pub fn map_security(m: TreasurySecurityMetrics) -> SecurityViewDto {
    SecurityViewDto {
        treasury_id: m.treasury_id,
        organisation_id: m.organisation_id,
        treasury_status: m.treasury_status,
        frozen_treasury: m.frozen_treasury,
        frozen_or_expired_allocations: m.frozen_or_expired_allocations,
        active_reservations: m.active_reservations,
        consumed_reservations: m.consumed_reservations,
        released_reservations: m.released_reservations,
        aged_active_reservations: m.aged_active_reservations,
        settlement_posts: m.settlement_posts,
        adjustments: m.adjustments,
        chargebacks: m.chargebacks,
        as_of: m.as_of,
        related_audit_hint: "Correlate with GET /api/audit using request_id / treasury targets",
    }
}

/// Operator journal filter (Phase 18 RBAC): exclude adjustment/chargeback unless admin.
pub fn journal_event_allowed_for_operator(event_type: &str) -> bool {
    !matches!(event_type, "adjustment" | "chargeback")
}
