use std::collections::HashMap;

use aether_treasury::{Amount, AssetId, TreasuryEngine, TreasuryKind};

use crate::auth::middleware::AuthContext;
use crate::config::Config;
use crate::db::operators::OperatorRole;

use super::errors::TreasuryCpError;
use super::models::*;
use super::queries::*;

/// Read-only Control Plane adapter over `aether-treasury`.
#[derive(Clone)]
pub struct TreasuryAdapter {
    engine: TreasuryEngine,
    organisation_id: String,
}

impl TreasuryAdapter {
    pub async fn connect(config: &Config) -> Result<Self, TreasuryCpError> {
        let path = if config.treasury_db_path.is_empty() {
            derive_treasury_path(&config.db_path)
        } else {
            config.treasury_db_path.clone()
        };
        let db = aether_treasury::db::TreasuryDb::connect(&path)
            .await
            .map_err(|e| TreasuryCpError::Engine(e.to_string()))?;
        let engine = TreasuryEngine::new(db);
        let adapter = Self {
            engine,
            organisation_id: config.treasury_organisation_id.clone(),
        };
        if config.treasury_bootstrap_demo {
            adapter.bootstrap_demo_if_empty().await?;
        }
        Ok(adapter)
    }

    pub fn organisation_id(&self) -> &str {
        &self.organisation_id
    }

    pub fn engine(&self) -> &TreasuryEngine {
        &self.engine
    }

    /// Bound organisation for the authenticated caller (single-tenant CP binding).
    pub fn caller_organisation(&self, _ctx: &AuthContext) -> &str {
        &self.organisation_id
    }

    async fn bootstrap_demo_if_empty(&self) -> Result<(), TreasuryCpError> {
        let existing = self
            .engine
            .list_treasuries(&self.organisation_id)
            .await
            .map_err(TreasuryCpError::from)?;
        if !existing.is_empty() {
            return Ok(());
        }

        let _ = self
            .engine
            .register_asset(AssetId::new("GBP"), 2, "fiat")
            .await;
        let _ = self
            .engine
            .register_asset(AssetId::new("USD"), 2, "fiat")
            .await;

        let org = self
            .engine
            .create_treasury(
                &self.organisation_id,
                None,
                TreasuryKind::Organisation,
                "Organisation Treasury",
            )
            .await
            .map_err(TreasuryCpError::from)?;
        let dept = self
            .engine
            .create_treasury(
                &self.organisation_id,
                Some(org.treasury_id.clone()),
                TreasuryKind::Department,
                "Engineering",
            )
            .await
            .map_err(TreasuryCpError::from)?;
        let _project = self
            .engine
            .create_treasury(
                &self.organisation_id,
                Some(dept.treasury_id.clone()),
                TreasuryKind::Project,
                "Agent Fleet Alpha",
            )
            .await
            .map_err(TreasuryCpError::from)?;

        self.engine
            .fund(
                &org.treasury_id,
                AssetId::new("GBP"),
                Amount(100_000),
                "bootstrap-fund",
                "bootstrap-fund-gbp",
            )
            .await
            .map_err(TreasuryCpError::from)?;
        self.engine
            .fund(
                &dept.treasury_id,
                AssetId::new("GBP"),
                Amount(25_000),
                "bootstrap-dept",
                "bootstrap-fund-dept",
            )
            .await
            .map_err(TreasuryCpError::from)?;

        let (alloc, _) = self
            .engine
            .create_allocation(
                &dept.treasury_id,
                "demo-agent-1",
                AssetId::new("GBP"),
                Amount(10_000),
                Amount(0),
                None,
                "bootstrap-alloc",
                "bootstrap-alloc-1",
            )
            .await
            .map_err(TreasuryCpError::from)?;

        let (res, _) = self
            .engine
            .reserve(
                &dept.treasury_id,
                &alloc.allocation_id,
                AssetId::new("GBP"),
                Amount(1_500),
                "bootstrap-res",
                "bootstrap-res-1",
                None,
            )
            .await
            .map_err(TreasuryCpError::from)?;

        let (esc, _) = self
            .engine
            .escrow_reserve(
                &dept.treasury_id,
                Some(&alloc.allocation_id),
                AssetId::new("GBP"),
                Amount(500),
                "demo-escrow-1",
                "bootstrap-esc",
                "bootstrap-esc-1",
                None,
            )
            .await
            .map_err(TreasuryCpError::from)?;

        let _ = self
            .engine
            .settlement_post(&esc.reservation_id, "bootstrap-settle", "bootstrap-settle-1")
            .await
            .map_err(TreasuryCpError::from)?;

        let _ = res;
        Ok(())
    }

    async fn require_treasury(
        &self,
        org: &str,
        treasury_id: &str,
    ) -> Result<aether_treasury::Treasury, TreasuryCpError> {
        let t = self.engine.get_treasury(treasury_id).await?;
        if t.organisation_id != org {
            // Tenancy: do not leak existence across orgs
            return Err(TreasuryCpError::NotFound);
        }
        Ok(t)
    }

    pub async fn overview(&self, ctx: &AuthContext) -> Result<TreasuryOverview, TreasuryCpError> {
        let org = self.caller_organisation(ctx).to_string();
        let treasuries = self.engine.list_treasuries(&org).await?;
        let mut items = Vec::new();
        let mut totals: HashMap<String, BalanceDto> = HashMap::new();

        for t in treasuries {
            let balances = self
                .engine
                .balances_for_treasury(&org, &t.treasury_id)
                .await?;
            for b in &balances {
                let entry = totals.entry(b.asset_id.clone()).or_insert(BalanceDto {
                    asset_id: b.asset_id.clone(),
                    available_minor: 0,
                    reserved_minor: 0,
                    escrow_reserved_minor: 0,
                });
                entry.available_minor += b.available_minor;
                entry.reserved_minor += b.reserved_minor;
                entry.escrow_reserved_minor += b.escrow_reserved_minor;
            }
            items.push(map_treasury(t, balances));
        }

        Ok(TreasuryOverview {
            organisation_id: org,
            treasuries: items,
            totals: totals.into_values().collect(),
            observation_only: true,
            note: "Observation only — no funding, custody, or mutation via Control Plane.",
        })
    }

    pub async fn get_treasury(
        &self,
        ctx: &AuthContext,
        treasury_id: &str,
    ) -> Result<TreasuryDetail, TreasuryCpError> {
        let org = self.caller_organisation(ctx).to_string();
        let t = self.require_treasury(&org, treasury_id).await?;
        let balances = self
            .engine
            .balances_for_treasury(&org, treasury_id)
            .await?;
        Ok(TreasuryDetail {
            treasury: map_treasury(t, balances),
            note: "Treasury node detail (read-only). Balances are treasury accounting truth."
                .into(),
        })
    }

    pub async fn allocations(
        &self,
        ctx: &AuthContext,
        treasury_id: &str,
    ) -> Result<Vec<AllocationDto>, TreasuryCpError> {
        let org = self.caller_organisation(ctx).to_string();
        let _ = self.require_treasury(&org, treasury_id).await?;
        let rows = self
            .engine
            .list_allocations_for_treasury(treasury_id)
            .await?;
        Ok(rows.into_iter().map(map_allocation).collect())
    }

    pub async fn reservations(
        &self,
        ctx: &AuthContext,
        treasury_id: &str,
    ) -> Result<Vec<ReservationDto>, TreasuryCpError> {
        let org = self.caller_organisation(ctx).to_string();
        let _ = self.require_treasury(&org, treasury_id).await?;
        let rows = self
            .engine
            .list_reservations_for_treasury(treasury_id)
            .await?;
        Ok(rows.into_iter().map(map_reservation).collect())
    }

    pub async fn journal(
        &self,
        ctx: &AuthContext,
        treasury_id: &str,
    ) -> Result<Vec<JournalEntryDto>, TreasuryCpError> {
        let org = self.caller_organisation(ctx).to_string();
        let _ = self.require_treasury(&org, treasury_id).await?;
        let rows = self
            .engine
            .list_journal_for_treasury(&org, treasury_id)
            .await?;
        let mut mapped: Vec<_> = rows.into_iter().map(map_journal_entry).collect();
        if ctx.role == OperatorRole::Operator {
            mapped.retain(|e| journal_event_allowed_for_operator(&e.event_type));
        }
        Ok(mapped)
    }

    pub async fn settlements(
        &self,
        ctx: &AuthContext,
        treasury_id: &str,
    ) -> Result<Vec<SettlementPostDto>, TreasuryCpError> {
        let org = self.caller_organisation(ctx).to_string();
        let _ = self.require_treasury(&org, treasury_id).await?;
        let rows = self
            .engine
            .list_settlement_posts_for_treasury(&org, treasury_id)
            .await?;
        Ok(rows.into_iter().filter_map(map_settlement_post).collect())
    }

    pub async fn security(
        &self,
        ctx: &AuthContext,
        treasury_id: &str,
    ) -> Result<SecurityViewDto, TreasuryCpError> {
        if ctx.role != OperatorRole::Admin {
            return Err(TreasuryCpError::Forbidden(
                "admin role required for treasury security view".into(),
            ));
        }
        let org = self.caller_organisation(ctx).to_string();
        let _ = self.require_treasury(&org, treasury_id).await?;
        let m = self
            .engine
            .security_metrics_for_treasury(&org, treasury_id)
            .await?;
        Ok(map_security(m))
    }
}

fn derive_treasury_path(cp_db_path: &str) -> String {
    if cp_db_path.contains("://") {
        return format!("{cp_db_path}-treasury");
    }
    if let Some(stripped) = cp_db_path.strip_suffix(".db") {
        format!("{stripped}.treasury.db")
    } else {
        format!("{cp_db_path}.treasury.db")
    }
}
