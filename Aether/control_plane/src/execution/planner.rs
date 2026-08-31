//! Execution planner — Approved Policy → GovernanceOperation → ExecutionPlan.

use chrono::Utc;
use serde_json::json;

use crate::auth::middleware::AuthContext;
use crate::execution::errors::ExecutionError;
use crate::execution::simulate::predict_protocol_operation;
use crate::execution::types::{ExecutionMode, ExecutionPlan};
use crate::models::policies::{PolicyStatus, PolicyTemplate};
use crate::signer::operation::GovernanceOperation;

const VALIDATION_STEPS: &[&str] = &[
    "policy_approved",
    "operator_authorised",
    "signer_available",
    "signature_valid",
    "protocol_adapter_available",
    "target_exists",
    "protocol_simulation",
];

pub struct ExecutionPlanner;

impl ExecutionPlanner {
    /// Build a dry-run execution plan. Does not mutate protocol or policy state.
    pub fn plan(
        policy: &PolicyTemplate,
        ctx: &AuthContext,
        request_id: &str,
    ) -> Result<ExecutionPlan, ExecutionError> {
        if policy.status != PolicyStatus::Approved {
            return Err(ExecutionError::BadRequest(format!(
                "dry-run requires approved policy (have {})",
                policy.status.as_str()
            )));
        }

        let protocol_operation = predict_protocol_operation(policy);
        let action = match protocol_operation {
            crate::execution::types::ProtocolOperationKind::CapabilityGrant => {
                "protocol.capability.grant.dry_run"
            }
            crate::execution::types::ProtocolOperationKind::CapabilityRevoke => {
                "protocol.capability.revoke.dry_run"
            }
            crate::execution::types::ProtocolOperationKind::FreezeIdentity => {
                "protocol.identity.freeze.dry_run"
            }
            crate::execution::types::ProtocolOperationKind::PolicyApply => {
                "protocol.policy.apply.dry_run"
            }
            crate::execution::types::ProtocolOperationKind::Unknown => "protocol.unknown.dry_run",
        };

        let payload = serde_json::to_vec(&json!({
            "policy_id": policy.id,
            "policy_version": policy.version,
            "policy_hash": policy.hash,
            "policy_type": policy.policy_type,
            "policy_data": policy.policy_data,
            "execution_mode": ExecutionMode::DryRun.as_str(),
        }))
        .map_err(|e| ExecutionError::Failed(e.to_string()))?;

        let mut gov =
            GovernanceOperation::new(request_id, &ctx.operator_id, ctx.role, action, &payload)
                .with_policy(&policy.id, policy.version);

        if let Some(target) = &policy.target_agent_id {
            gov = gov.with_target(target);
        }

        Ok(ExecutionPlan {
            operation_id: gov.operation_id.clone(),
            policy_id: policy.id.clone(),
            policy_version: policy.version,
            target: policy.target_agent_id.clone(),
            action: action.into(),
            signer_required: true,
            protocol_operation,
            validation_steps: VALIDATION_STEPS.iter().map(|s| (*s).to_string()).collect(),
            execution_mode: ExecutionMode::DryRun,
            governance_operation: gov,
            planned_at: Utc::now(),
        })
    }
}
