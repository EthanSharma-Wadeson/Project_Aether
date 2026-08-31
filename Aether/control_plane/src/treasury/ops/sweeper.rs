use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::config::Config;
use crate::db::audit;
use crate::treasury::write::audit::append_mutation_audit;
use crate::treasury::write::models::TreasuryOperation;
use crate::treasury::TreasuryAdapter;

use aether_treasury::ORPHAN_RESERVATION_AGE_SECS;

#[derive(Debug, Clone, Serialize, Default)]
pub struct SweepReport {
    pub scanned: usize,
    pub expired_marked: usize,
    pub released: usize,
    pub failed: usize,
    pub reservation_ids: Vec<String>,
    pub errors: Vec<String>,
}

/// Run one sweeper pass: expire due/orphan actives and release with journal + audit.
pub async fn run_sweeper_once(
    adapter: &TreasuryAdapter,
    cp_pool: &sqlx::SqlitePool,
    config: &Config,
) -> SweepReport {
    let org = adapter.organisation_id();
    let orphan = if config.treasury_reservation_orphan_age_secs > 0 {
        config.treasury_reservation_orphan_age_secs
    } else {
        ORPHAN_RESERVATION_AGE_SECS
    };

    let mut report = SweepReport::default();
    let candidates = match adapter.engine().list_sweep_candidates(org, orphan).await {
        Ok(c) => c,
        Err(e) => {
            report.errors.push(e.to_string());
            return report;
        }
    };
    report.scanned = candidates.len();

    for r in candidates {
        let request_id = format!("sweeper-{}", Uuid::new_v4());
        let idem = format!(
            "sweep:{}:{}",
            r.reservation_id,
            r.expires_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| r.created_at.to_rfc3339())
        );
        match adapter
            .engine()
            .expire_and_release(&r.reservation_id, &request_id, &idem)
            .await
        {
            Ok((res, batch, marked_expired)) => {
                if marked_expired {
                    report.expired_marked += 1;
                    let _ = append_mutation_audit(
                        cp_pool,
                        org,
                        None,
                        &request_id,
                        "treasury-sweeper",
                        TreasuryOperation::Release,
                        Some(&res.treasury_id),
                        Some("expired"),
                        Some(&batch.batch_id),
                        "reservation_expired",
                        json!({ "reservation_id": res.reservation_id, "actor": "treasury-sweeper" }),
                    )
                    .await;
                    let _ = audit::append(
                        cp_pool,
                        None,
                        "treasury.mutation.reservation_expired",
                        Some(&res.reservation_id),
                        Some(json!({ "request_id": request_id, "source": "sweeper" })),
                    )
                    .await;
                }
                report.released += 1;
                report.reservation_ids.push(res.reservation_id.clone());
                let _ = append_mutation_audit(
                    cp_pool,
                    org,
                    None,
                    &request_id,
                    "treasury-sweeper",
                    TreasuryOperation::Release,
                    Some(&res.treasury_id),
                    Some("released"),
                    Some(&batch.batch_id),
                    "reservation_released",
                    json!({ "reservation_id": res.reservation_id, "journal_batch_id": batch.batch_id }),
                )
                .await;
                let _ = audit::append(
                    cp_pool,
                    None,
                    "treasury.mutation.reservation_released",
                    Some(&res.reservation_id),
                    Some(json!({
                        "request_id": request_id,
                        "journal_batch_id": batch.batch_id,
                        "source": "sweeper"
                    })),
                )
                .await;
            }
            Err(e) => {
                report.failed += 1;
                report.errors.push(format!("{}: {e}", r.reservation_id));
                tracing::warn!(
                    reservation_id = %r.reservation_id,
                    error = %e,
                    "treasury sweeper failed to release reservation"
                );
            }
        }
    }

    if report.scanned > 0 {
        tracing::info!(
            scanned = report.scanned,
            released = report.released,
            failed = report.failed,
            "treasury reservation sweeper pass complete"
        );
    }
    let _ = Utc::now();
    report
}

/// Background loop — no silent recovery without audit (each release audited above).
pub fn spawn_sweeper_loop(
    adapter: TreasuryAdapter,
    cp_pool: sqlx::SqlitePool,
    config: Config,
) {
    if !config.treasury_sweeper_enabled || config.treasury_sweeper_interval_secs == 0 {
        tracing::info!("treasury sweeper loop disabled");
        return;
    }
    let interval = std::time::Duration::from_secs(config.treasury_sweeper_interval_secs.max(5));
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            let _ = run_sweeper_once(&adapter, &cp_pool, &config).await;
        }
    });
}
