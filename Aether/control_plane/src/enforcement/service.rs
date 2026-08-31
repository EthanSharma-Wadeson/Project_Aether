use crate::auth::middleware::AuthContext;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::protocol::state::ProtocolState;
use crate::treasury::TreasuryAdapter;

use super::audit;
use super::decision::decide;
use super::evaluator;
use super::models::{EnforcementDecision, EnforcementRequest};

/// Read-only enforcement service (E2). Never calls Apply or treasury writes.
pub struct EnforcementService;

impl EnforcementService {
    pub async fn evaluate(
        protocol: &ProtocolState,
        treasury: &TreasuryAdapter,
        db: &Db,
        ctx: &AuthContext,
        req: EnforcementRequest,
        request_id: String,
    ) -> Result<EnforcementDecision> {
        let org = treasury.organisation_id().to_string();
        let snap = evaluator::load_snapshot(protocol, treasury, db.pool(), &req)
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        let decision = decide(&req, &snap, &org, Some(request_id));

        // Invariant flags must remain false.
        debug_assert!(!decision.apply_invoked);
        debug_assert!(!decision.treasury_mutated);
        debug_assert!(!decision.proto0_mutated);

        audit::record_decision(db, &ctx.operator_id, &decision).await?;
        Ok(decision)
    }
}
