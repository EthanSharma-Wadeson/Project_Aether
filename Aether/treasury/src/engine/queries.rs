//! Read-only query helpers for observation (Phase 19). No mutations.

use crate::error::Result;
use crate::models::*;
use crate::types::*;

use super::store::{get_allocation, get_balance, get_reservation, get_treasury, list_assets, parse_dt};
use super::TreasuryEngine;

/// Security-oriented counters for a single treasury (and org-scoped siblings when needed).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TreasurySecurityMetrics {
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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetBalance {
    pub asset_id: String,
    pub available_minor: i64,
    pub reserved_minor: i64,
    pub escrow_reserved_minor: i64,
}

impl TreasuryEngine {
    pub async fn get_treasury(&self, treasury_id: &str) -> Result<Treasury> {
        get_treasury(self.db.pool(), treasury_id).await
    }

    pub async fn get_allocation(&self, allocation_id: &str) -> Result<Allocation> {
        get_allocation(self.db.pool(), allocation_id).await
    }

    pub async fn get_reservation(&self, reservation_id: &str) -> Result<Reservation> {
        get_reservation(self.db.pool(), reservation_id).await
    }

    pub async fn list_treasuries(&self, organisation_id: &str) -> Result<Vec<Treasury>> {
        let rows: Vec<(
            String,
            String,
            Option<String>,
            String,
            String,
            String,
            String,
            Option<String>,
        )> = sqlx::query_as(
            r#"
            SELECT treasury_id, organisation_id, parent_treasury_id, kind, name, status, created_at, closed_at
            FROM treasuries
            WHERE organisation_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(organisation_id)
        .fetch_all(self.db.pool())
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(Treasury {
                    treasury_id: r.0,
                    organisation_id: r.1,
                    parent_treasury_id: r.2,
                    kind: TreasuryKind::parse(&r.3)?,
                    name: r.4,
                    status: TreasuryStatus::parse(&r.5)?,
                    created_at: parse_dt(&r.6),
                    closed_at: r.7.as_deref().map(parse_dt),
                })
            })
            .collect()
    }

    pub async fn balances_for_treasury(
        &self,
        organisation_id: &str,
        treasury_id: &str,
    ) -> Result<Vec<AssetBalance>> {
        let assets = list_assets(self.db.pool()).await?;
        let mut out = Vec::new();
        for asset in assets {
            let available = get_balance(
                self.db.pool(),
                organisation_id,
                &available_account(treasury_id),
                asset.asset_id.as_str(),
            )
            .await?;
            let reserved = get_balance(
                self.db.pool(),
                organisation_id,
                &reserved_account(treasury_id),
                asset.asset_id.as_str(),
            )
            .await?;
            let escrow = get_balance(
                self.db.pool(),
                organisation_id,
                &escrow_reserved_account(treasury_id),
                asset.asset_id.as_str(),
            )
            .await?;
            if available == 0 && reserved == 0 && escrow == 0 {
                continue;
            }
            out.push(AssetBalance {
                asset_id: asset.asset_id.0,
                available_minor: available,
                reserved_minor: reserved,
                escrow_reserved_minor: escrow,
            });
        }
        Ok(out)
    }

    pub async fn list_allocations_for_treasury(&self, treasury_id: &str) -> Result<Vec<Allocation>> {
        let ids: Vec<(String,)> =
            sqlx::query_as("SELECT allocation_id FROM allocations WHERE treasury_id = ? ORDER BY created_at DESC")
                .bind(treasury_id)
                .fetch_all(self.db.pool())
                .await?;
        let mut out = Vec::new();
        for (id,) in ids {
            out.push(get_allocation(self.db.pool(), &id).await?);
        }
        Ok(out)
    }

    /// Org-scoped allocation list for observation / drift reporting (Phase 24).
    pub async fn list_allocations_for_organisation(
        &self,
        organisation_id: &str,
    ) -> Result<Vec<Allocation>> {
        let ids: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT allocation_id FROM allocations
            WHERE organisation_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(organisation_id)
        .fetch_all(self.db.pool())
        .await?;
        let mut out = Vec::new();
        for (id,) in ids {
            out.push(get_allocation(self.db.pool(), &id).await?);
        }
        Ok(out)
    }

    pub async fn list_reservations_for_treasury(
        &self,
        treasury_id: &str,
    ) -> Result<Vec<Reservation>> {
        let ids: Vec<(String,)> = sqlx::query_as(
            "SELECT reservation_id FROM reservations WHERE treasury_id = ? ORDER BY created_at DESC",
        )
        .bind(treasury_id)
        .fetch_all(self.db.pool())
        .await?;
        let mut out = Vec::new();
        for (id,) in ids {
            out.push(get_reservation(self.db.pool(), &id).await?);
        }
        Ok(out)
    }

    pub async fn list_journal_for_treasury(
        &self,
        organisation_id: &str,
        treasury_id: &str,
    ) -> Result<Vec<JournalEntry>> {
        let all = self.list_journal_entries(organisation_id).await?;
        Ok(all
            .into_iter()
            .filter(|e| e.treasury_id.as_deref() == Some(treasury_id))
            .collect())
    }

    pub async fn list_settlement_posts_for_treasury(
        &self,
        organisation_id: &str,
        treasury_id: &str,
    ) -> Result<Vec<JournalEntry>> {
        let entries = self
            .list_journal_for_treasury(organisation_id, treasury_id)
            .await?;
        Ok(entries
            .into_iter()
            .filter(|e| e.event_type == JournalEventType::SettlementPost)
            .collect())
    }

    pub async fn security_metrics_for_treasury(
        &self,
        organisation_id: &str,
        treasury_id: &str,
    ) -> Result<TreasurySecurityMetrics> {
        let t = get_treasury(self.db.pool(), treasury_id).await?;
        if t.organisation_id != organisation_id {
            return Err(crate::TreasuryError::TreasuryNotFound(treasury_id.into()));
        }

        let frozen_or_expired: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM allocations
            WHERE treasury_id = ? AND status IN ('frozen', 'expired')
            "#,
        )
        .bind(treasury_id)
        .fetch_one(self.db.pool())
        .await?;

        let active_reservations: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reservations WHERE treasury_id = ? AND status = 'active'",
        )
        .bind(treasury_id)
        .fetch_one(self.db.pool())
        .await?;

        let consumed_reservations: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reservations WHERE treasury_id = ? AND status = 'consumed'",
        )
        .bind(treasury_id)
        .fetch_one(self.db.pool())
        .await?;

        let released_reservations: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reservations WHERE treasury_id = ? AND status = 'released'",
        )
        .bind(treasury_id)
        .fetch_one(self.db.pool())
        .await?;

        // Aged = active with expires_at in the past OR created > 24h ago (simple heuristic)
        let aged_active_reservations: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM reservations
            WHERE treasury_id = ? AND status = 'active'
              AND (
                (expires_at IS NOT NULL AND expires_at < datetime('now'))
                OR created_at < datetime('now', '-1 day')
              )
            "#,
        )
        .bind(treasury_id)
        .fetch_one(self.db.pool())
        .await?;

        let settlement_posts: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM journal_entries
            WHERE organisation_id = ? AND treasury_id = ? AND event_type = 'settlement_post'
            "#,
        )
        .bind(organisation_id)
        .bind(treasury_id)
        .fetch_one(self.db.pool())
        .await?;

        let adjustments: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM journal_entries
            WHERE organisation_id = ? AND treasury_id = ? AND event_type = 'adjustment'
            "#,
        )
        .bind(organisation_id)
        .bind(treasury_id)
        .fetch_one(self.db.pool())
        .await?;

        let chargebacks: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM journal_entries
            WHERE organisation_id = ? AND treasury_id = ? AND event_type = 'chargeback'
            "#,
        )
        .bind(organisation_id)
        .bind(treasury_id)
        .fetch_one(self.db.pool())
        .await?;

        Ok(TreasurySecurityMetrics {
            treasury_id: treasury_id.into(),
            organisation_id: organisation_id.into(),
            treasury_status: t.status.as_str().into(),
            frozen_treasury: t.status == TreasuryStatus::Frozen,
            frozen_or_expired_allocations: frozen_or_expired,
            active_reservations,
            consumed_reservations,
            released_reservations,
            aged_active_reservations,
            settlement_posts,
            adjustments,
            chargebacks,
            as_of: now().to_rfc3339(),
        })
    }

    /// Verify account_balances match journal for one treasury available account + asset.
    pub async fn verify_balance_matches_journal(
        &self,
        organisation_id: &str,
        treasury_id: &str,
        asset_id: &str,
    ) -> Result<bool> {
        let account = available_account(treasury_id);
        let projected = get_balance(self.db.pool(), organisation_id, &account, asset_id).await?;
        let from_journal: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(debit_minor - credit_minor), 0)
            FROM journal_entries
            WHERE organisation_id = ? AND account_code = ? AND asset_id = ?
            "#,
        )
        .bind(organisation_id)
        .bind(&account)
        .bind(asset_id)
        .fetch_one(self.db.pool())
        .await?;
        Ok(projected == from_journal)
    }
}
