use serde_json::json;

use crate::db::Db;
use crate::error::Result;
use crate::db::audit;

use super::models::SyncObservationReport;

pub async fn record_observation(
    db: &Db,
    actor_id: &str,
    report: &SyncObservationReport,
) -> Result<()> {
    audit::append(
        db.pool(),
        Some(actor_id),
        "observation.apply_treasury_sync",
        Some(&report.request_id),
        Some(json!({
            "status": report.status_code,
            "organisation_id": report.organisation_id,
            "findings_count": report.findings.len(),
            "capabilities_scanned": report.capabilities_scanned,
            "allocations_scanned": report.allocations_scanned,
            "observation_only": true,
            "enforcement": false,
            "auto_repair": false,
            "finding_kinds": report
                .findings
                .iter()
                .map(|f| f.kind.as_str())
                .collect::<Vec<_>>(),
        })),
    )
    .await?;
    Ok(())
}
