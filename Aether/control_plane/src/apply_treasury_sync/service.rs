use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::protocol::state::ProtocolState;
use crate::treasury::TreasuryAdapter;

use super::audit;
use super::compare::compare_snapshot;
use super::models::{SyncObservationReport, SYNC_LEDGER_NOTICE};
use super::queries::{load_snapshot, load_snapshot_for_agent};

/// Read-only sync observation service.
///
/// Hard rules (compile-time discipline + runtime flags on reports):
/// - no PROTO-0 write / Apply calls
/// - no treasury mutations
/// - no automatic repair
pub struct SyncObservationService;

impl SyncObservationService {
    pub async fn observe_org(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        actor_id: &str,
    ) -> Result<SyncObservationReport> {
        let snap = load_snapshot(protocol, treasury)
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        Ok(Self::finish(db, actor_id, snap).await?)
    }

    pub async fn observe_agent(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        actor_id: &str,
        agent_id: &str,
    ) -> Result<SyncObservationReport> {
        let snap = load_snapshot_for_agent(protocol, treasury, agent_id)
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        Ok(Self::finish(db, actor_id, snap).await?)
    }

    async fn finish(
        db: &Db,
        actor_id: &str,
        snap: super::queries::SyncSnapshot,
    ) -> Result<SyncObservationReport> {
        let caps_n = snap.capabilities.len();
        let allocs_n = snap.allocations.len();
        let (status, findings) = compare_snapshot(&snap);
        let report = SyncObservationReport {
            status,
            status_code: status.as_str(),
            organisation_id: snap.organisation_id,
            capabilities_scanned: caps_n,
            allocations_scanned: allocs_n,
            findings,
            request_id: Uuid::new_v4().to_string(),
            observed_at: chrono::Utc::now().to_rfc3339(),
            observation_only: true,
            enforcement: false,
            auto_repair: false,
            ledger_notice: SYNC_LEDGER_NOTICE,
        };
        audit::record_observation(db, actor_id, &report).await?;
        Ok(report)
    }
}
