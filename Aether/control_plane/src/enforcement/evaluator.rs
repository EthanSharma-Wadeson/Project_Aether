//! Load read-only authority snapshots for enforcement.

use std::collections::HashMap;

use crate::db::policies as policy_db;
use crate::protocol::proto0;
use crate::protocol::state::ProtocolState;
use crate::treasury::TreasuryAdapter;
use sqlx::SqlitePool;

use super::errors::EnforcementError;
use super::models::{
    AgentAuthoritySnapshot, AllocationSnap, CapabilitySnap, EnforcementRequest, PolicySnap,
};

pub async fn load_snapshot(
    protocol: &ProtocolState,
    treasury: &TreasuryAdapter,
    pool: &SqlitePool,
    req: &EnforcementRequest,
) -> Result<AgentAuthoritySnapshot, EnforcementError> {
    let org = treasury.organisation_id().to_string();

    let agent_status = match proto0::get_agent(protocol, &req.agent_id) {
        Ok(a) => a.status,
        Err(_) => "not_found".into(),
    };

    let capabilities = proto0::capabilities_for_agent(protocol, &req.agent_id)
        .into_iter()
        .map(|c| CapabilitySnap {
            capability_id: c.capability_id,
            actions: c.actions,
            max_spend: c.max_spend,
            asset: c.asset,
            revoked: c.revoked,
        })
        .collect();

    let rows = treasury
        .engine()
        .list_allocations_for_organisation(&org)
        .await
        .map_err(|e| EnforcementError::Treasury(e.to_string()))?;

    let allocations: Vec<AllocationSnap> = rows
        .into_iter()
        .filter(|a| a.organisation_id == org && a.agent_id == req.agent_id)
        .map(|a| AllocationSnap {
            allocation_id: a.allocation_id,
            organisation_id: a.organisation_id,
            treasury_id: a.treasury_id,
            agent_id: a.agent_id,
            asset_id: a.asset_id.0,
            ceiling_minor: a.ceiling_minor,
            remaining_minor: a.remaining_minor,
            status: a.status.as_str().to_string(),
            expires_at: a.expires_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    let treasuries = treasury
        .engine()
        .list_treasuries(&org)
        .await
        .map_err(|e| EnforcementError::Treasury(e.to_string()))?;
    let mut treasury_status_by_id = HashMap::new();
    for t in &treasuries {
        treasury_status_by_id.insert(t.treasury_id.clone(), t.status.as_str().to_string());
    }

    let mut known_assets: Vec<String> = sqlx::query_as::<_, (String,)>(
        "SELECT asset_id FROM asset_types WHERE enabled = 1 ORDER BY asset_id",
    )
    .fetch_all(treasury.engine().db().pool())
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id,)| id)
    .collect();

    for a in &allocations {
        if !known_assets.iter().any(|x| x == &a.asset_id) {
            known_assets.push(a.asset_id.clone());
        }
    }

    let policy = if let Some(ref pid) = req.policy_id {
        match policy_db::get_policy(pool, pid)
            .await
            .map_err(|e| EnforcementError::Policy(e.to_string()))?
        {
            Some(p) => Some(policy_snap_from(&p)),
            None => None,
        }
    } else {
        let policies = policy_db::list_policies(pool, true, None)
            .await
            .unwrap_or_default();
        policies
            .into_iter()
            .find(|p| p.policy_type == "spend_block")
            .map(|p| policy_snap_from(&p))
    };

    Ok(AgentAuthoritySnapshot {
        agent_id: req.agent_id.clone(),
        agent_status,
        capabilities,
        allocations,
        treasury_status_by_id,
        known_assets,
        policy,
    })
}

fn policy_snap_from(p: &crate::models::policies::PolicyTemplate) -> PolicySnap {
    let blocked = p
        .policy_data
        .get("blocked_actions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    PolicySnap {
        policy_id: p.id.clone(),
        status: p.status.as_str().to_string(),
        policy_type: p.policy_type.clone(),
        blocked_actions: blocked,
    }
}
