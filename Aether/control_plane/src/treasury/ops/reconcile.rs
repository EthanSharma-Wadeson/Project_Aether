use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::middleware::AuthContext;
use crate::config::Config;
use crate::db::audit;
use crate::treasury::write::approval::MutationStore;
use crate::treasury::write::audit::append_mutation_audit;
use crate::treasury::write::errors::TreasuryWriteError;
use crate::treasury::write::models::{MutationStatus, TreasuryOperation};
use crate::treasury::TreasuryAdapter;

use super::sweeper::{run_sweeper_once, SweepReport};

#[derive(Clone)]
pub struct ReconcileService {
    pool: sqlx::SqlitePool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ReconcileReport {
    pub stuck_executing_mutations: Vec<String>,
    pub mutations_marked_failed: usize,
    pub expired_reservations: Vec<String>,
    pub stuck_reservations: Vec<String>,
    pub sweep: Option<SweepReport>,
    pub ledger_notice: &'static str,
}

impl ReconcileService {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Detect stuck `executing` mutations and optionally expire due reservations (no silent balance edits).
    pub async fn scan(
        &self,
        adapter: &TreasuryAdapter,
        config: &Config,
    ) -> Result<ReconcileReport, TreasuryWriteError> {
        let org = adapter.organisation_id();
        let timeout = Duration::seconds(config.treasury_mutation_executing_timeout_secs.max(60));
        let cutoff = Utc::now() - timeout;

        let rows = sqlx::query(
            r#"
            SELECT mutation_id, created_at FROM treasury_mutation_requests
            WHERE organisation_id = ? AND status = 'executing'
            "#,
        )
        .bind(org)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        let mut stuck_executing = Vec::new();
        for row in rows {
            let id: String = row.get("mutation_id");
            let created: String = row.get("created_at");
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&created) {
                if dt.with_timezone(&Utc) <= cutoff {
                    stuck_executing.push(id);
                }
            } else {
                stuck_executing.push(id);
            }
        }

        let counts = adapter
            .engine()
            .ops_reservation_counts(org)
            .await
            .map_err(TreasuryWriteError::from)?;
        let expired = if counts.expired > 0 || counts.due_active > 0 {
            adapter
                .engine()
                .list_sweep_candidates(org, config.treasury_reservation_orphan_age_secs)
                .await
                .map_err(TreasuryWriteError::from)?
                .into_iter()
                .map(|r| r.reservation_id)
                .collect()
        } else {
            Vec::new()
        };

        let stuck_reservations: Vec<String> = sqlx::query_scalar(
            "SELECT reservation_id FROM reservations WHERE organisation_id = ? AND status = 'stuck'",
        )
        .bind(org)
        .fetch_all(adapter.engine().db().pool())
        .await
        .map_err(|e| TreasuryWriteError::Unavailable(e.to_string()))?;

        Ok(ReconcileReport {
            stuck_executing_mutations: stuck_executing,
            mutations_marked_failed: 0,
            expired_reservations: expired,
            stuck_reservations,
            sweep: None,
            ledger_notice: crate::treasury::write::models::LEDGER_NOTICE,
        })
    }

    /// Mark timed-out executing mutations failed (audited; no journal inventing).
    pub async fn reconcile_stuck_mutations(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        config: &Config,
        request_id: &str,
    ) -> Result<ReconcileReport, TreasuryWriteError> {
        let mut report = self.scan(adapter, config).await?;
        let store = MutationStore::new(self.pool.clone());
        let org = adapter.organisation_id();

        for mid in report.stuck_executing_mutations.clone() {
            let finished = store
                .finish_execute(
                    org,
                    &mid,
                    None,
                    "reconciled_stuck_executing",
                    false,
                )
                .await?;
            report.mutations_marked_failed += 1;
            append_mutation_audit(
                &self.pool,
                org,
                Some(&mid),
                request_id,
                &ctx.operator_id,
                finished.operation,
                finished.target.as_deref(),
                Some("failed"),
                None,
                "reservation_reconciled",
                json!({
                    "kind": "stuck_executing_mutation",
                    "prior_status": MutationStatus::Executing.as_str()
                }),
            )
            .await?;
            let _ = audit::append(
                &self.pool,
                Some(&ctx.operator_id),
                "treasury.mutation.reservation_reconciled",
                Some(&mid),
                Some(json!({
                    "request_id": request_id,
                    "kind": "stuck_executing_mutation",
                    "outcome": "failed"
                })),
            )
            .await;
        }
        Ok(report)
    }

    /// Run sweeper + optional mutation reconcile (admin, audited).
    pub async fn run_full(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        config: &Config,
        request_id: &str,
        run_sweep: bool,
        fix_mutations: bool,
    ) -> Result<ReconcileReport, TreasuryWriteError> {
        let mut report = if fix_mutations {
            self.reconcile_stuck_mutations(adapter, ctx, config, request_id)
                .await?
        } else {
            self.scan(adapter, config).await?
        };

        if run_sweep {
            let sweep = run_sweeper_once(adapter, &self.pool, config).await;
            let _ = audit::append(
                &self.pool,
                Some(&ctx.operator_id),
                "treasury.mutation.reservation_reconciled",
                None,
                Some(json!({
                    "request_id": request_id,
                    "kind": "sweep",
                    "released": sweep.released,
                    "failed": sweep.failed
                })),
            )
            .await;
            report.sweep = Some(sweep);
        }
        Ok(report)
    }

    /// Force-release a stuck/expired reservation (admin; journal via engine; audited).
    pub async fn force_release_reservation(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        reservation_id: &str,
        reason: &str,
        request_id: &str,
    ) -> Result<serde_json::Value, TreasuryWriteError> {
        if reason.trim().is_empty() {
            return Err(TreasuryWriteError::BadRequest(
                "reason required for force release".into(),
            ));
        }
        let org = adapter.organisation_id();
        let r = adapter
            .engine()
            .get_reservation(reservation_id)
            .await
            .map_err(TreasuryWriteError::from)?;
        if r.organisation_id != org {
            return Err(TreasuryWriteError::NotFound(
                "reservation not found".into(),
            ));
        }
        let idem = format!("reconcile-force:{reservation_id}:{}", Uuid::new_v4());
        let (res, batch) = adapter
            .engine()
            .release_reservation(reservation_id, request_id, &idem)
            .await
            .map_err(TreasuryWriteError::from)?;
        append_mutation_audit(
            &self.pool,
            org,
            None,
            request_id,
            &ctx.operator_id,
            TreasuryOperation::Release,
            Some(&res.treasury_id),
            Some("reconciled"),
            Some(&batch.batch_id),
            "reservation_reconciled",
            json!({
                "reservation_id": reservation_id,
                "reason": reason,
                "journal_batch_id": batch.batch_id
            }),
        )
        .await?;
        let _ = audit::append(
            &self.pool,
            Some(&ctx.operator_id),
            "treasury.mutation.reservation_reconciled",
            Some(reservation_id),
            Some(json!({
                "request_id": request_id,
                "reason": reason,
                "journal_batch_id": batch.batch_id
            })),
        )
        .await;
        Ok(json!({
            "reservation_id": res.reservation_id,
            "status": res.status.as_str(),
            "journal_batch_id": batch.batch_id,
            "outcome": "released",
            "ledger_notice": crate::treasury::write::models::LEDGER_NOTICE,
        }))
    }
}
