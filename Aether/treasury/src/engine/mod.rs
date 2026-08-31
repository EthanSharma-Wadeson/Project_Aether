mod journal;
mod store;
mod queries;

pub use queries::{AssetBalance, TreasurySecurityMetrics};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::TreasuryDb;
use crate::error::{Result, TreasuryError};
use crate::models::*;
use crate::types::*;

use journal::{post_batch, post_batch_tx, BatchSpec};
use store::*;

/// Internal Treasury accounting engine (no rails, no HTTP).
#[derive(Clone)]
pub struct TreasuryEngine {
    db: TreasuryDb,
}

impl TreasuryEngine {
    pub fn new(db: TreasuryDb) -> Self {
        Self { db }
    }

    pub async fn in_memory() -> Result<Self> {
        Ok(Self::new(TreasuryDb::connect_in_memory().await?))
    }

    pub fn db(&self) -> &TreasuryDb {
        &self.db
    }

    // ── Assets ───────────────────────────────────────────────────────────

    pub async fn register_asset(
        &self,
        asset_id: AssetId,
        scale: i64,
        class: impl Into<String>,
    ) -> Result<AssetType> {
        if scale < 0 {
            return Err(TreasuryError::Validation("scale must be >= 0".into()));
        }
        let created_at = now();
        sqlx::query(
            r#"
            INSERT INTO asset_types (asset_id, scale, class, enabled, created_at)
            VALUES (?, ?, ?, 1, ?)
            "#,
        )
        .bind(asset_id.as_str())
        .bind(scale)
        .bind(class.into())
        .bind(created_at.to_rfc3339())
        .execute(self.db.pool())
        .await?;
        get_asset(self.db.pool(), asset_id.as_str()).await
    }

    // ── Hierarchy ────────────────────────────────────────────────────────

    pub async fn create_treasury(
        &self,
        organisation_id: impl Into<String>,
        parent_treasury_id: Option<String>,
        kind: TreasuryKind,
        name: impl Into<String>,
    ) -> Result<Treasury> {
        let organisation_id = organisation_id.into();
        if let Some(ref parent_id) = parent_treasury_id {
            let parent = get_treasury(self.db.pool(), parent_id).await?;
            if parent.organisation_id != organisation_id {
                return Err(TreasuryError::InvalidHierarchy(
                    "parent organisation mismatch".into(),
                ));
            }
            ensure_treasury_mutable(&parent)?;
            if kind == TreasuryKind::Organisation {
                return Err(TreasuryError::InvalidHierarchy(
                    "organisation treasury cannot have a parent".into(),
                ));
            }
        } else if kind != TreasuryKind::Organisation {
            return Err(TreasuryError::InvalidHierarchy(
                "non-organisation treasury requires parent".into(),
            ));
        }

        let treasury_id = Uuid::new_v4().to_string();
        let created_at = now();
        sqlx::query(
            r#"
            INSERT INTO treasuries
              (treasury_id, organisation_id, parent_treasury_id, kind, name, status, created_at, closed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NULL)
            "#,
        )
        .bind(&treasury_id)
        .bind(&organisation_id)
        .bind(&parent_treasury_id)
        .bind(kind.as_str())
        .bind(name.into())
        .bind(TreasuryStatus::Active.as_str())
        .bind(created_at.to_rfc3339())
        .execute(self.db.pool())
        .await?;
        get_treasury(self.db.pool(), &treasury_id).await
    }

    pub async fn freeze_treasury(&self, treasury_id: &str) -> Result<Treasury> {
        let t = get_treasury(self.db.pool(), treasury_id).await?;
        if t.status == TreasuryStatus::Closed {
            return Err(TreasuryError::TreasuryClosed(treasury_id.into()));
        }
        sqlx::query("UPDATE treasuries SET status = ? WHERE treasury_id = ?")
            .bind(TreasuryStatus::Frozen.as_str())
            .bind(treasury_id)
            .execute(self.db.pool())
            .await?;
        get_treasury(self.db.pool(), treasury_id).await
    }

    /// Restore a frozen treasury to active (governance-gated in Control Plane).
    pub async fn unfreeze_treasury(&self, treasury_id: &str) -> Result<Treasury> {
        let t = get_treasury(self.db.pool(), treasury_id).await?;
        if t.status == TreasuryStatus::Closed {
            return Err(TreasuryError::TreasuryClosed(treasury_id.into()));
        }
        if t.status != TreasuryStatus::Frozen {
            return Err(TreasuryError::Validation(format!(
                "treasury is not frozen: {}",
                treasury_id
            )));
        }
        sqlx::query("UPDATE treasuries SET status = ? WHERE treasury_id = ?")
            .bind(TreasuryStatus::Active.as_str())
            .bind(treasury_id)
            .execute(self.db.pool())
            .await?;
        get_treasury(self.db.pool(), treasury_id).await
    }

    pub async fn close_treasury(&self, treasury_id: &str) -> Result<Treasury> {
        let t = get_treasury(self.db.pool(), treasury_id).await?;
        if t.status == TreasuryStatus::Closed {
            return Err(TreasuryError::TreasuryClosed(treasury_id.into()));
        }
        // Allow close only when all registered assets have zero available+reserved+escrow
        let assets = list_assets(self.db.pool()).await?;
        for asset in assets {
            let avail = self
                .available_balance(&t.organisation_id, treasury_id, &asset.asset_id)
                .await?;
            let reserved = self
                .reserved_balance(&t.organisation_id, treasury_id, &asset.asset_id)
                .await?;
            let escrow = self
                .escrow_reserved_balance(&t.organisation_id, treasury_id, &asset.asset_id)
                .await?;
            if avail.0 != 0 || reserved.0 != 0 || escrow.0 != 0 {
                return Err(TreasuryError::Validation(format!(
                    "cannot close treasury with non-zero balances for {}",
                    asset.asset_id
                )));
            }
        }
        let closed_at = now();
        sqlx::query("UPDATE treasuries SET status = ?, closed_at = ? WHERE treasury_id = ?")
            .bind(TreasuryStatus::Closed.as_str())
            .bind(closed_at.to_rfc3339())
            .bind(treasury_id)
            .execute(self.db.pool())
            .await?;
        get_treasury(self.db.pool(), treasury_id).await
    }

    // ── Balances ─────────────────────────────────────────────────────────

    pub async fn available_balance(
        &self,
        organisation_id: &str,
        treasury_id: &str,
        asset_id: &AssetId,
    ) -> Result<Amount> {
        let bal = get_balance(
            self.db.pool(),
            organisation_id,
            &available_account(treasury_id),
            asset_id.as_str(),
        )
        .await?;
        Ok(Amount(bal))
    }

    pub async fn reserved_balance(
        &self,
        organisation_id: &str,
        treasury_id: &str,
        asset_id: &AssetId,
    ) -> Result<Amount> {
        let bal = get_balance(
            self.db.pool(),
            organisation_id,
            &reserved_account(treasury_id),
            asset_id.as_str(),
        )
        .await?;
        Ok(Amount(bal))
    }

    pub async fn escrow_reserved_balance(
        &self,
        organisation_id: &str,
        treasury_id: &str,
        asset_id: &AssetId,
    ) -> Result<Amount> {
        let bal = get_balance(
            self.db.pool(),
            organisation_id,
            &escrow_reserved_account(treasury_id),
            asset_id.as_str(),
        )
        .await?;
        Ok(Amount(bal))
    }

    // ── Funding ──────────────────────────────────────────────────────────

    /// Record internal funding (no external execution). Credits available.
    pub async fn fund(
        &self,
        treasury_id: &str,
        asset_id: AssetId,
        amount: Amount,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<JournalBatch> {
        let amount = Amount::minor(amount.0)?;
        let treasury = get_treasury(self.db.pool(), treasury_id).await?;
        ensure_treasury_mutable(&treasury)?;
        ensure_asset_enabled(self.db.pool(), asset_id.as_str()).await?;

        let mut lines = vec![
            JournalLine::debit(
                available_account(treasury_id),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
            JournalLine::credit(
                funding_source_account(),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
        ];
        // funding_source is credit-normal (liability/equity-like); allow negative
        // asset-side check only on treasury_available.
        post_batch(
            self.db.pool(),
            BatchSpec {
                organisation_id: treasury.organisation_id,
                event_type: JournalEventType::Funding,
                request_id: request_id.into(),
                idempotency_key: idempotency_key.into(),
                lines: &mut lines,
                enforce_non_negative_accounts: &[available_account(treasury_id)],
            },
        )
        .await
    }

    // ── Allocations ──────────────────────────────────────────────────────

    /// Create an agent spending allocation ceiling (does not move title to agent).
    /// Optionally moves `initial` from available → reserved and sets remaining.
    pub async fn create_allocation(
        &self,
        treasury_id: &str,
        agent_id: impl Into<String>,
        asset_id: AssetId,
        ceiling: Amount,
        initial: Amount,
        expires_at: Option<DateTime<Utc>>,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<(Allocation, Option<JournalBatch>)> {
        let ceiling = Amount::minor(ceiling.0)?;
        let initial = Amount::minor(initial.0)?;
        if initial.0 > ceiling.0 {
            return Err(TreasuryError::Validation(
                "initial allocation exceeds ceiling".into(),
            ));
        }
        let treasury = get_treasury(self.db.pool(), treasury_id).await?;
        ensure_treasury_mutable(&treasury)?;
        ensure_asset_enabled(self.db.pool(), asset_id.as_str()).await?;

        let allocation_id = Uuid::new_v4().to_string();
        let created_at = now();
        let agent_id = agent_id.into();

        let mut tx = self.db.pool().begin().await?;
        // Serialize funding checks
        sqlx::query("SELECT 1 FROM treasuries WHERE treasury_id = ?")
            .bind(treasury_id)
            .fetch_optional(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            INSERT INTO allocations
              (allocation_id, organisation_id, treasury_id, agent_id, asset_id,
               ceiling_minor, remaining_minor, status, expires_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&allocation_id)
        .bind(&treasury.organisation_id)
        .bind(treasury_id)
        .bind(&agent_id)
        .bind(asset_id.as_str())
        .bind(ceiling.0)
        .bind(ceiling.0) // remaining starts at full authority ceiling
        .bind(AllocationStatus::Active.as_str())
        .bind(expires_at.map(|t| t.to_rfc3339()))
        .bind(created_at.to_rfc3339())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        let batch = if initial.0 > 0 {
            let mut lines = vec![
                JournalLine::debit(
                    reserved_account(treasury_id),
                    asset_id.clone(),
                    initial,
                    Some(treasury_id.into()),
                )?,
                JournalLine::credit(
                    available_account(treasury_id),
                    asset_id.clone(),
                    initial,
                    Some(treasury_id.into()),
                )?,
            ];
            for line in &mut lines {
                line.allocation_id = Some(allocation_id.clone());
            }
            Some(
                post_batch(
                    self.db.pool(),
                    BatchSpec {
                        organisation_id: treasury.organisation_id.clone(),
                        event_type: JournalEventType::Allocation,
                        request_id: request_id.into(),
                        idempotency_key: idempotency_key.into(),
                        lines: &mut lines,
                        enforce_non_negative_accounts: &[
                            available_account(treasury_id),
                            reserved_account(treasury_id),
                        ],
                    },
                )
                .await?,
            )
        } else {
            None
        };

        let allocation = get_allocation(self.db.pool(), &allocation_id).await?;
        Ok((allocation, batch))
    }

    pub async fn expire_allocation_if_needed(&self, allocation_id: &str) -> Result<Allocation> {
        let mut a = get_allocation(self.db.pool(), allocation_id).await?;
        if a.status == AllocationStatus::Active {
            if let Some(exp) = a.expires_at {
                if exp <= now() {
                    sqlx::query("UPDATE allocations SET status = ? WHERE allocation_id = ?")
                        .bind(AllocationStatus::Expired.as_str())
                        .bind(allocation_id)
                        .execute(self.db.pool())
                        .await?;
                    a = get_allocation(self.db.pool(), allocation_id).await?;
                }
            }
        }
        Ok(a)
    }

    pub async fn close_allocation(&self, allocation_id: &str) -> Result<Allocation> {
        let a = get_allocation(self.db.pool(), allocation_id).await?;
        sqlx::query("UPDATE allocations SET status = ?, remaining_minor = 0 WHERE allocation_id = ?")
            .bind(AllocationStatus::Closed.as_str())
            .bind(allocation_id)
            .execute(self.db.pool())
            .await?;
        let _ = a;
        get_allocation(self.db.pool(), allocation_id).await
    }

    /// Raise allocation ceiling (and remaining). Optionally move `fund_reserved`
    /// from available → reserved (ledger acknowledgment only).
    pub async fn increase_allocation(
        &self,
        allocation_id: &str,
        new_ceiling: Amount,
        fund_reserved: Amount,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<(Allocation, Option<JournalBatch>)> {
        let new_ceiling = Amount::minor(new_ceiling.0)?;
        let fund_reserved = Amount::minor(fund_reserved.0)?;
        let a = self.expire_allocation_if_needed(allocation_id).await?;
        ensure_allocation_reservable(&a, &a.asset_id)?;
        let treasury = get_treasury(self.db.pool(), &a.treasury_id).await?;
        ensure_treasury_mutable(&treasury)?;
        if new_ceiling.0 <= a.ceiling_minor {
            return Err(TreasuryError::Validation(
                "new_ceiling must exceed current ceiling".into(),
            ));
        }
        let delta = new_ceiling.0 - a.ceiling_minor;
        let new_remaining = a.remaining_minor + delta;

        let mut tx = self.db.pool().begin().await?;
        sqlx::query(
            "UPDATE allocations SET ceiling_minor = ?, remaining_minor = ? WHERE allocation_id = ?",
        )
        .bind(new_ceiling.0)
        .bind(new_remaining)
        .bind(allocation_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        let batch = if fund_reserved.0 > 0 {
            let mut lines = vec![
                JournalLine::debit(
                    reserved_account(&a.treasury_id),
                    a.asset_id.clone(),
                    fund_reserved,
                    Some(a.treasury_id.clone()),
                )?,
                JournalLine::credit(
                    available_account(&a.treasury_id),
                    a.asset_id.clone(),
                    fund_reserved,
                    Some(a.treasury_id.clone()),
                )?,
            ];
            for line in &mut lines {
                line.allocation_id = Some(allocation_id.into());
            }
            Some(
                post_batch(
                    self.db.pool(),
                    BatchSpec {
                        organisation_id: a.organisation_id.clone(),
                        event_type: JournalEventType::Allocation,
                        request_id: request_id.into(),
                        idempotency_key: idempotency_key.into(),
                        lines: &mut lines,
                        enforce_non_negative_accounts: &[
                            available_account(&a.treasury_id),
                            reserved_account(&a.treasury_id),
                        ],
                    },
                )
                .await?,
            )
        } else {
            let _ = (request_id, idempotency_key);
            None
        };

        Ok((get_allocation(self.db.pool(), allocation_id).await?, batch))
    }

    /// Lower allocation ceiling without dropping below already-committed authority.
    pub async fn decrease_allocation(
        &self,
        allocation_id: &str,
        new_ceiling: Amount,
    ) -> Result<Allocation> {
        let new_ceiling = Amount::minor(new_ceiling.0)?;
        let a = self.expire_allocation_if_needed(allocation_id).await?;
        ensure_allocation_reservable(&a, &a.asset_id)?;
        let treasury = get_treasury(self.db.pool(), &a.treasury_id).await?;
        ensure_treasury_mutable(&treasury)?;
        if new_ceiling.0 >= a.ceiling_minor {
            return Err(TreasuryError::Validation(
                "new_ceiling must be less than current ceiling".into(),
            ));
        }
        let used = a.ceiling_minor - a.remaining_minor;
        if new_ceiling.0 < used {
            return Err(TreasuryError::Validation(
                "new_ceiling cannot be below committed reservation authority".into(),
            ));
        }
        let new_remaining = new_ceiling.0 - used;
        sqlx::query(
            "UPDATE allocations SET ceiling_minor = ?, remaining_minor = ? WHERE allocation_id = ?",
        )
        .bind(new_ceiling.0)
        .bind(new_remaining)
        .bind(allocation_id)
        .execute(self.db.pool())
        .await?;
        get_allocation(self.db.pool(), allocation_id).await
    }

    // ── Reservations ─────────────────────────────────────────────────────

    /// Reserve funds from treasury available against an allocation (budget hold).
    pub async fn reserve(
        &self,
        treasury_id: &str,
        allocation_id: &str,
        asset_id: AssetId,
        amount: Amount,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(Reservation, JournalBatch)> {
        self.reserve_inner(
            treasury_id,
            Some(allocation_id),
            asset_id,
            amount,
            ReservationKind::Budget,
            None,
            request_id.into(),
            idempotency_key.into(),
            expires_at,
            JournalEventType::Reservation,
            reserved_account(treasury_id),
        )
        .await
    }

    /// Escrow reservation — holds funds tagged to an escrow id.
    pub async fn escrow_reserve(
        &self,
        treasury_id: &str,
        allocation_id: Option<&str>,
        asset_id: AssetId,
        amount: Amount,
        escrow_id: impl Into<String>,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(Reservation, JournalBatch)> {
        self.reserve_inner(
            treasury_id,
            allocation_id,
            asset_id,
            amount,
            ReservationKind::Escrow,
            Some(escrow_id.into()),
            request_id.into(),
            idempotency_key.into(),
            expires_at,
            JournalEventType::EscrowReservation,
            escrow_reserved_account(treasury_id),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn reserve_inner(
        &self,
        treasury_id: &str,
        allocation_id: Option<&str>,
        asset_id: AssetId,
        amount: Amount,
        kind: ReservationKind,
        escrow_id: Option<String>,
        request_id: String,
        idempotency_key: String,
        expires_at: Option<DateTime<Utc>>,
        event_type: JournalEventType,
        dest_reserved_account: String,
    ) -> Result<(Reservation, JournalBatch)> {
        let amount = Amount::minor(amount.0)?;
        if amount.0 == 0 {
            return Err(TreasuryError::Validation(
                "reservation amount must be > 0".into(),
            ));
        }

        let expires_at = Some(normalize_reservation_expiry(expires_at)?);

        // Idempotent replay
        if let Some(existing) =
            find_reservation_by_idem(self.db.pool(), &idempotency_key).await?
        {
            let batch = find_batch_by_idem(
                self.db.pool(),
                &existing.organisation_id,
                &idempotency_key,
            )
            .await?
            .ok_or_else(|| TreasuryError::Validation("reservation without batch".into()))?;
            return Ok((existing, batch));
        }

        let treasury = get_treasury(self.db.pool(), treasury_id).await?;
        ensure_treasury_mutable(&treasury)?;
        ensure_asset_enabled(self.db.pool(), asset_id.as_str()).await?;

        let mut alloc_id_owned: Option<String> = None;
        if let Some(aid) = allocation_id {
            let a = self.expire_allocation_if_needed(aid).await?;
            ensure_allocation_reservable(&a, &asset_id)?;
            if a.remaining_minor < amount.0 {
                return Err(TreasuryError::InsufficientAllocation);
            }
            alloc_id_owned = Some(a.allocation_id);
        }

        let reservation_id = Uuid::new_v4().to_string();
        let created_at = now();

        let mut lines = vec![
            JournalLine::debit(
                dest_reserved_account,
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
            JournalLine::credit(
                available_account(treasury_id),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
        ];
        for line in &mut lines {
            line.allocation_id = alloc_id_owned.clone();
            line.reservation_id = Some(reservation_id.clone());
        }

        let mut tx = self.db.pool().begin().await?;

        // Lock treasury row for concurrent reservation safety
        sqlx::query("UPDATE treasuries SET name = name WHERE treasury_id = ?")
            .bind(treasury_id)
            .execute(&mut *tx)
            .await?;

        let available = get_balance_tx(
            &mut tx,
            &treasury.organisation_id,
            &available_account(treasury_id),
            asset_id.as_str(),
        )
        .await?;
        if available < amount.0 {
            return Err(TreasuryError::InsufficientFunds);
        }

        if let Some(ref aid) = alloc_id_owned {
            let rem: i64 = sqlx::query_scalar(
                "SELECT remaining_minor FROM allocations WHERE allocation_id = ?",
            )
            .bind(aid)
            .fetch_one(&mut *tx)
            .await?;
            if rem < amount.0 {
                return Err(TreasuryError::InsufficientAllocation);
            }
            sqlx::query(
                "UPDATE allocations SET remaining_minor = remaining_minor - ? WHERE allocation_id = ?",
            )
            .bind(amount.0)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            r#"
            INSERT INTO reservations
              (reservation_id, organisation_id, treasury_id, allocation_id, asset_id,
               amount_minor, kind, status, escrow_id, idempotency_key, expires_at,
               created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&reservation_id)
        .bind(&treasury.organisation_id)
        .bind(treasury_id)
        .bind(&alloc_id_owned)
        .bind(asset_id.as_str())
        .bind(amount.0)
        .bind(kind.as_str())
        .bind(ReservationStatus::Active.as_str())
        .bind(&escrow_id)
        .bind(&idempotency_key)
        .bind(expires_at.map(|t| t.to_rfc3339()))
        .bind(created_at.to_rfc3339())
        .bind(created_at.to_rfc3339())
        .execute(&mut *tx)
        .await?;

        let batch = post_batch_tx(
            &mut tx,
            BatchSpec {
                organisation_id: treasury.organisation_id.clone(),
                event_type,
                request_id,
                idempotency_key: idempotency_key.clone(),
                lines: &mut lines,
                enforce_non_negative_accounts: &[available_account(treasury_id)],
            },
        )
        .await?;

        tx.commit().await?;

        let reservation = get_reservation(self.db.pool(), &reservation_id).await?;
        Ok((reservation, batch))
    }

    /// Release an active, expired, or stuck reservation back to available.
    pub async fn release_reservation(
        &self,
        reservation_id: &str,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<(Reservation, JournalBatch)> {
        let r = get_reservation(self.db.pool(), reservation_id).await?;
        if !r.status.is_releasable() {
            return Err(TreasuryError::ReservationNotActive(reservation_id.into()));
        }
        let treasury = get_treasury(self.db.pool(), &r.treasury_id).await?;
        // Allow release even if frozen (unwinding holds); block if closed
        if treasury.status == TreasuryStatus::Closed {
            return Err(TreasuryError::TreasuryClosed(r.treasury_id.clone()));
        }

        let src_account = match r.kind {
            ReservationKind::Budget => reserved_account(&r.treasury_id),
            ReservationKind::Escrow => escrow_reserved_account(&r.treasury_id),
        };
        let amount = Amount(r.amount_minor);
        let prior_status = r.status;

        let mut tx = self.db.pool().begin().await?;
        let updated = sqlx::query(
            "UPDATE reservations SET status = ?, updated_at = ? WHERE reservation_id = ? AND status = ?",
        )
        .bind(ReservationStatus::Released.as_str())
        .bind(now().to_rfc3339())
        .bind(reservation_id)
        .bind(prior_status.as_str())
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(TreasuryError::ReservationNotActive(reservation_id.into()));
        }

        if let Some(ref aid) = r.allocation_id {
            sqlx::query(
                "UPDATE allocations SET remaining_minor = remaining_minor + ? WHERE allocation_id = ?",
            )
            .bind(r.amount_minor)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        let mut lines = vec![
            JournalLine::debit(
                available_account(&r.treasury_id),
                r.asset_id.clone(),
                amount,
                Some(r.treasury_id.clone()),
            )?,
            JournalLine::credit(
                src_account,
                r.asset_id.clone(),
                amount,
                Some(r.treasury_id.clone()),
            )?,
        ];
        for line in &mut lines {
            line.allocation_id = r.allocation_id.clone();
            line.reservation_id = Some(reservation_id.into());
        }

        let batch = post_batch(
            self.db.pool(),
            BatchSpec {
                organisation_id: r.organisation_id.clone(),
                event_type: JournalEventType::Release,
                request_id: request_id.into(),
                idempotency_key: idempotency_key.into(),
                lines: &mut lines,
                enforce_non_negative_accounts: &[
                    available_account(&r.treasury_id),
                    reserved_account(&r.treasury_id),
                    escrow_reserved_account(&r.treasury_id),
                ],
            },
        )
        .await?;

        Ok((get_reservation(self.db.pool(), reservation_id).await?, batch))
    }

    /// Settlement post: consume escrow/budget reservation into expense (no rail).
    pub async fn settlement_post(
        &self,
        reservation_id: &str,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<(Reservation, JournalBatch)> {
        let r = get_reservation(self.db.pool(), reservation_id).await?;
        if r.status != ReservationStatus::Active {
            return Err(TreasuryError::ReservationNotActive(reservation_id.into()));
        }
        if let Some(exp) = r.expires_at {
            if exp <= now() {
                return Err(TreasuryError::Validation(format!(
                    "reservation expired and cannot be settled: {reservation_id}"
                )));
            }
        }
        let treasury = get_treasury(self.db.pool(), &r.treasury_id).await?;
        if treasury.status == TreasuryStatus::Closed {
            return Err(TreasuryError::TreasuryClosed(r.treasury_id.clone()));
        }

        let src_account = match r.kind {
            ReservationKind::Budget => reserved_account(&r.treasury_id),
            ReservationKind::Escrow => escrow_reserved_account(&r.treasury_id),
        };
        let amount = Amount(r.amount_minor);

        let mut tx = self.db.pool().begin().await?;
        sqlx::query(
            "UPDATE reservations SET status = ?, updated_at = ? WHERE reservation_id = ? AND status = ?",
        )
        .bind(ReservationStatus::Consumed.as_str())
        .bind(now().to_rfc3339())
        .bind(reservation_id)
        .bind(ReservationStatus::Active.as_str())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        let mut lines = vec![
            JournalLine::debit(
                expense_account(),
                r.asset_id.clone(),
                amount,
                Some(r.treasury_id.clone()),
            )?,
            JournalLine::credit(
                src_account,
                r.asset_id.clone(),
                amount,
                Some(r.treasury_id.clone()),
            )?,
        ];
        for line in &mut lines {
            line.allocation_id = r.allocation_id.clone();
            line.reservation_id = Some(reservation_id.into());
        }

        let batch = post_batch(
            self.db.pool(),
            BatchSpec {
                organisation_id: r.organisation_id.clone(),
                event_type: JournalEventType::SettlementPost,
                request_id: request_id.into(),
                idempotency_key: idempotency_key.into(),
                lines: &mut lines,
                enforce_non_negative_accounts: &[
                    reserved_account(&r.treasury_id),
                    escrow_reserved_account(&r.treasury_id),
                ],
            },
        )
        .await?;

        Ok((get_reservation(self.db.pool(), reservation_id).await?, batch))
    }

    /// Refund: reverse a consumed settlement via suspense (compensating entry).
    pub async fn refund(
        &self,
        treasury_id: &str,
        asset_id: AssetId,
        amount: Amount,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<JournalBatch> {
        let amount = Amount::minor(amount.0)?;
        let treasury = get_treasury(self.db.pool(), treasury_id).await?;
        ensure_treasury_mutable(&treasury)?;
        ensure_asset_enabled(self.db.pool(), asset_id.as_str()).await?;

        let mut lines = vec![
            JournalLine::debit(
                available_account(treasury_id),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
            JournalLine::credit(
                expense_account(),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
        ];
        post_batch(
            self.db.pool(),
            BatchSpec {
                organisation_id: treasury.organisation_id,
                event_type: JournalEventType::Refund,
                request_id: request_id.into(),
                idempotency_key: idempotency_key.into(),
                lines: &mut lines,
                enforce_non_negative_accounts: &[],
            },
        )
        .await
    }

    /// Chargeback: park in suspense then restore available (two-step simplified to one compensating pair via suspense).
    pub async fn chargeback(
        &self,
        treasury_id: &str,
        asset_id: AssetId,
        amount: Amount,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<JournalBatch> {
        let amount = Amount::minor(amount.0)?;
        let treasury = get_treasury(self.db.pool(), treasury_id).await?;
        ensure_treasury_mutable(&treasury)?;
        ensure_asset_enabled(self.db.pool(), asset_id.as_str()).await?;

        // expense ↓ via suspense, available ↑
        let mut lines = vec![
            JournalLine::debit(
                suspense_account(),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
            JournalLine::credit(
                expense_account(),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
            JournalLine::debit(
                available_account(treasury_id),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
            JournalLine::credit(
                suspense_account(),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
        ];
        post_batch(
            self.db.pool(),
            BatchSpec {
                organisation_id: treasury.organisation_id,
                event_type: JournalEventType::Chargeback,
                request_id: request_id.into(),
                idempotency_key: idempotency_key.into(),
                lines: &mut lines,
                enforce_non_negative_accounts: &[],
            },
        )
        .await
    }

    /// Manual adjustment (compensating). Requires balanced lines supplied by caller accounts —
    /// foundation helper: move between available and suspense.
    pub async fn adjustment_to_suspense(
        &self,
        treasury_id: &str,
        asset_id: AssetId,
        amount: Amount,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<JournalBatch> {
        let amount = Amount::minor(amount.0)?;
        let treasury = get_treasury(self.db.pool(), treasury_id).await?;
        ensure_treasury_mutable(&treasury)?;
        ensure_asset_enabled(self.db.pool(), asset_id.as_str()).await?;

        let mut lines = vec![
            JournalLine::debit(
                suspense_account(),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
            JournalLine::credit(
                available_account(treasury_id),
                asset_id.clone(),
                amount,
                Some(treasury_id.into()),
            )?,
        ];
        post_batch(
            self.db.pool(),
            BatchSpec {
                organisation_id: treasury.organisation_id,
                event_type: JournalEventType::Adjustment,
                request_id: request_id.into(),
                idempotency_key: idempotency_key.into(),
                lines: &mut lines,
                enforce_non_negative_accounts: &[available_account(treasury_id)],
            },
        )
        .await
    }

    // ── Reads / integrity ────────────────────────────────────────────────

    pub async fn list_journal_entries(&self, organisation_id: &str) -> Result<Vec<JournalEntry>> {
        list_entries(self.db.pool(), organisation_id).await
    }

    pub async fn assert_journal_balanced(&self, batch_id: &str) -> Result<()> {
        let (debits, credits): (i64, i64) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(debit_minor),0), COALESCE(SUM(credit_minor),0)
            FROM journal_entries WHERE batch_id = ?
            "#,
        )
        .bind(batch_id)
        .fetch_one(self.db.pool())
        .await?;
        if debits != credits {
            return Err(TreasuryError::UnbalancedBatch);
        }
        Ok(())
    }

    /// Rebuild balances from journal (replay). Used for immutability/replay tests.
    pub async fn replay_balances(&self, organisation_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM account_balances WHERE organisation_id = ?")
            .bind(organisation_id)
            .execute(self.db.pool())
            .await?;
        let entries = list_entries(self.db.pool(), organisation_id).await?;
        for e in entries {
            apply_balance_delta(
                self.db.pool(),
                organisation_id,
                &e.account_code,
                e.asset_id.as_str(),
                e.debit_minor - e.credit_minor,
                &[], // replay trusts historical journal; skip non-neg during rebuild
            )
            .await?;
        }
        Ok(())
    }

    /// Reject any attempt to mutate journal rows.
    pub async fn try_mutate_journal_entry(&self, entry_id: &str) -> Result<()> {
        let res = sqlx::query("UPDATE journal_entries SET debit_minor = debit_minor WHERE entry_id = ?")
            .bind(entry_id)
            .execute(self.db.pool())
            .await?;
        // Engine policy: surface immutability — application layer forbids updates.
        // We deliberately return ImmutabilityViolation if a row exists (caller must not update).
        if res.rows_affected() > 0 {
            return Err(TreasuryError::ImmutabilityViolation);
        }
        Err(TreasuryError::Validation("entry not found".into()))
    }

    /// List reservations due for sweep (TTL expired or orphaned active rows).
    pub async fn list_sweep_candidates(
        &self,
        organisation_id: &str,
        orphan_age_secs: i64,
    ) -> Result<Vec<Reservation>> {
        let now = now();
        let orphan_cutoff = now - chrono::Duration::seconds(orphan_age_secs);
        let rows: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT reservation_id FROM reservations
            WHERE organisation_id = ?
              AND status = 'active'
              AND (
                (expires_at IS NOT NULL AND expires_at <= ?)
                OR (expires_at IS NULL AND created_at <= ?)
              )
            ORDER BY created_at ASC
            "#,
        )
        .bind(organisation_id)
        .bind(now.to_rfc3339())
        .bind(orphan_cutoff.to_rfc3339())
        .fetch_all(self.db.pool())
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for id in rows {
            out.push(get_reservation(self.db.pool(), &id).await?);
        }
        Ok(out)
    }

    /// Mark an active reservation expired (funds still held until release).
    pub async fn mark_reservation_expired(&self, reservation_id: &str) -> Result<Reservation> {
        let r = get_reservation(self.db.pool(), reservation_id).await?;
        if r.status != ReservationStatus::Active {
            return Err(TreasuryError::ReservationNotActive(reservation_id.into()));
        }
        let due = r
            .expires_at
            .map(|t| t <= now())
            .unwrap_or(true);
        if !due {
            return Err(TreasuryError::Validation(
                "reservation is not past expires_at".into(),
            ));
        }
        sqlx::query(
            "UPDATE reservations SET status = ?, updated_at = ? WHERE reservation_id = ? AND status = ?",
        )
        .bind(ReservationStatus::Expired.as_str())
        .bind(now().to_rfc3339())
        .bind(reservation_id)
        .bind(ReservationStatus::Active.as_str())
        .execute(self.db.pool())
        .await?;
        get_reservation(self.db.pool(), reservation_id).await
    }

    /// Mark reservation stuck (requires audited recovery; funds still held).
    pub async fn mark_reservation_stuck(&self, reservation_id: &str) -> Result<Reservation> {
        let r = get_reservation(self.db.pool(), reservation_id).await?;
        if r.status != ReservationStatus::Active && r.status != ReservationStatus::Expired {
            return Err(TreasuryError::ReservationNotActive(reservation_id.into()));
        }
        sqlx::query(
            "UPDATE reservations SET status = ?, updated_at = ? WHERE reservation_id = ?",
        )
        .bind(ReservationStatus::Stuck.as_str())
        .bind(now().to_rfc3339())
        .bind(reservation_id)
        .execute(self.db.pool())
        .await?;
        get_reservation(self.db.pool(), reservation_id).await
    }

    /// Expire (status) then release with journal — primary sweeper action.
    pub async fn expire_and_release(
        &self,
        reservation_id: &str,
        request_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<(Reservation, JournalBatch, bool)> {
        let r = get_reservation(self.db.pool(), reservation_id).await?;
        let mut marked_expired = false;
        if r.status == ReservationStatus::Active {
            let _ = self.mark_reservation_expired(reservation_id).await?;
            marked_expired = true;
        }
        let request_id = request_id.into();
        let idempotency_key = idempotency_key.into();
        let (reservation, batch) = self
            .release_reservation(reservation_id, request_id, idempotency_key)
            .await?;
        Ok((reservation, batch, marked_expired))
    }

    /// Operational counters for monitoring (Phase 22.5).
    pub async fn ops_reservation_counts(
        &self,
        organisation_id: &str,
    ) -> Result<ReservationOpsCounts> {
        let active: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reservations WHERE organisation_id = ? AND status = 'active'",
        )
        .bind(organisation_id)
        .fetch_one(self.db.pool())
        .await?;
        let expired: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reservations WHERE organisation_id = ? AND status = 'expired'",
        )
        .bind(organisation_id)
        .fetch_one(self.db.pool())
        .await?;
        let stuck: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reservations WHERE organisation_id = ? AND status = 'stuck'",
        )
        .bind(organisation_id)
        .fetch_one(self.db.pool())
        .await?;
        let released: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reservations WHERE organisation_id = ? AND status = 'released'",
        )
        .bind(organisation_id)
        .fetch_one(self.db.pool())
        .await?;
        let consumed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reservations WHERE organisation_id = ? AND status = 'consumed'",
        )
        .bind(organisation_id)
        .fetch_one(self.db.pool())
        .await?;
        let due_active: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM reservations
            WHERE organisation_id = ? AND status = 'active'
              AND expires_at IS NOT NULL AND expires_at <= ?
            "#,
        )
        .bind(organisation_id)
        .bind(now().to_rfc3339())
        .fetch_one(self.db.pool())
        .await?;
        Ok(ReservationOpsCounts {
            active,
            expired,
            stuck,
            released,
            consumed,
            due_active,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReservationOpsCounts {
    pub active: i64,
    pub expired: i64,
    pub stuck: i64,
    pub released: i64,
    pub consumed: i64,
    pub due_active: i64,
}

fn ensure_treasury_mutable(t: &Treasury) -> Result<()> {
    match t.status {
        TreasuryStatus::Active => Ok(()),
        TreasuryStatus::Frozen => Err(TreasuryError::TreasuryFrozen(t.treasury_id.clone())),
        TreasuryStatus::Closed => Err(TreasuryError::TreasuryClosed(t.treasury_id.clone())),
    }
}

fn ensure_allocation_reservable(a: &Allocation, asset_id: &AssetId) -> Result<()> {
    if a.asset_id != *asset_id {
        return Err(TreasuryError::AssetMismatch {
            expected: a.asset_id.0.clone(),
            got: asset_id.0.clone(),
        });
    }
    match a.status {
        AllocationStatus::Active => Ok(()),
        AllocationStatus::Expired => Err(TreasuryError::AllocationExpired(a.allocation_id.clone())),
        _ => Err(TreasuryError::AllocationNotActive(a.allocation_id.clone())),
    }
}

/// Default TTL when caller omits `expires_at` (Phase 22.5 / OH-C1).
pub const DEFAULT_RESERVATION_TTL_SECS: i64 = 900;
/// Maximum reservation lifetime from creation.
pub const MAX_RESERVATION_TTL_SECS: i64 = 86_400;
/// Orphan age when `expires_at` is somehow missing on legacy rows.
pub const ORPHAN_RESERVATION_AGE_SECS: i64 = 86_400;

fn normalize_reservation_expiry(expires_at: Option<DateTime<Utc>>) -> Result<DateTime<Utc>> {
    let now = now();
    let max = now + chrono::Duration::seconds(MAX_RESERVATION_TTL_SECS);
    match expires_at {
        None => Ok(now + chrono::Duration::seconds(DEFAULT_RESERVATION_TTL_SECS)),
        Some(t) if t <= now => Err(TreasuryError::Validation(
            "expires_at must be in the future".into(),
        )),
        Some(t) if t > max => Err(TreasuryError::Validation(format!(
            "expires_at exceeds max TTL of {MAX_RESERVATION_TTL_SECS}s"
        ))),
        Some(t) => Ok(t),
    }
}
