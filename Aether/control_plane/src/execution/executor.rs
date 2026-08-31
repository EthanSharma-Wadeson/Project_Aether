//! Dry-run executor — full governance pipeline without protocol mutation.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::apply::attestation::{AttestationStore, CreateAttestationRequest};
use crate::auth::middleware::AuthContext;
use crate::auth::roles::require_operator;
use crate::db::audit;
use crate::execution::errors::ExecutionError;
use crate::execution::hash::execution_hash_for_policy;
use crate::execution::planner::ExecutionPlanner;
use crate::execution::simulate::{
    predict_protocol_operation, protocol_observation_fingerprint, simulate_policy_apply,
};
use crate::execution::types::{
    DryRunReport, ExecutionMode, ProtocolOperationKind, SimulationOutcome, ValidationStep,
};
use crate::models::policies::{PolicyStatus, PolicyTemplate};
use crate::protocol::proto0;
use crate::protocol::state::ProtocolState;
use crate::signer::SigningGateway;
use sqlx::SqlitePool;

pub struct ExecutionExecutor<'a> {
    pool: &'a SqlitePool,
    protocol: &'a ProtocolState,
    signing: &'a SigningGateway,
}

impl<'a> ExecutionExecutor<'a> {
    pub fn new(
        pool: &'a SqlitePool,
        protocol: &'a ProtocolState,
        signing: &'a SigningGateway,
    ) -> Self {
        Self {
            pool,
            protocol,
            signing,
        }
    }

    /// Exercise the complete governance pipeline in DryRun mode only.
    ///
    /// Never calls PROTO-0 mutation APIs and never modifies `ProtocolState`.
    pub async fn dry_run(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        policy: &PolicyTemplate,
    ) -> Result<DryRunReport, ExecutionError> {
        require_operator(ctx).map_err(|e| ExecutionError::Forbidden(e.to_string()))?;

        let dry_run_id = Uuid::new_v4().to_string();
        // Count fingerprint: non-mutation check only (not Apply binding).
        let before = protocol_observation_fingerprint(self.protocol);

        audit::append(
            self.pool,
            Some(&ctx.operator_id),
            "DRY_RUN_REQUESTED",
            Some(&policy.id),
            Some(json!({
                "request_id": request_id,
                "dry_run_id": dry_run_id,
                "policy_version": policy.version,
                "role": ctx.role.as_str(),
            })),
        )
        .await
        .map_err(|e| ExecutionError::Failed(e.to_string()))?;

        let result = self
            .dry_run_inner(ctx, request_id, &dry_run_id, policy)
            .await;

        let after = protocol_observation_fingerprint(self.protocol);
        if before != after {
            let _ = audit::append(
                self.pool,
                Some(&ctx.operator_id),
                "DRY_RUN_FAILED",
                Some(&policy.id),
                Some(json!({
                    "request_id": request_id,
                    "dry_run_id": dry_run_id,
                    "reason": "protocol fingerprint changed during dry-run",
                    "before": before,
                    "after": after,
                })),
            )
            .await;
            return Err(ExecutionError::Failed(
                "protocol state changed during dry-run (invariant violation)".into(),
            ));
        }

        match &result {
            Ok(report) => {
                let audit_event = audit::append(
                    self.pool,
                    Some(&ctx.operator_id),
                    "DRY_RUN_COMPLETED",
                    Some(&policy.id),
                    Some(json!({
                        "request_id": request_id,
                        "dry_run_id": report.dry_run_id,
                        "operation_id": report.operation_id,
                        "policy_version": policy.version,
                        "execution_hash": report.execution_hash,
                        "signer_identity": report.signer_identity,
                        "dry_run_result": {
                            "executable": report.executable,
                            "predicted_protocol_operation": report.predicted_protocol_operation.as_str(),
                            "blocking_errors": report.blocking_errors,
                            "warnings": report.warnings,
                        },
                    })),
                )
                .await
                .map_err(|e| ExecutionError::Failed(e.to_string()))?;

                if report.executable {
                    self.persist_attestation(ctx, policy, report, &audit_event.id)
                        .await?;
                }
            }
            Err(err) => {
                let _ = audit::append(
                    self.pool,
                    Some(&ctx.operator_id),
                    "DRY_RUN_FAILED",
                    Some(&policy.id),
                    Some(json!({
                        "request_id": request_id,
                        "dry_run_id": dry_run_id,
                        "signer_identity": self.signing.signer_identity(),
                        "dry_run_result": { "error": err.to_string() },
                    })),
                )
                .await;
            }
        }

        result
    }

    /// Persist immutable attestation for an executable dry-run (Apply evidence only).
    async fn persist_attestation(
        &self,
        ctx: &AuthContext,
        policy: &PolicyTemplate,
        report: &DryRunReport,
        audit_reference: &str,
    ) -> Result<(), ExecutionError> {
        let store = AttestationStore::new(self.pool.clone());
        let predicted_changes = json!({
            "operation_id": report.operation_id,
            "policy_id": report.policy_id,
            "policy_version": report.policy_version,
            "policy_hash": policy.hash,
            "predicted_protocol_operation": report.predicted_protocol_operation.as_str(),
            "operation_intent": report.predicted_protocol_operation.as_str(),
            "target_agent": policy.target_agent_id,
            "capability_changes": {
                "actions": policy.policy_data.get("actions").cloned().unwrap_or(Value::Null),
                "constraints": policy.policy_data.get("constraints").cloned().unwrap_or(Value::Null),
                "limits": policy.policy_data.get("limits").cloned().unwrap_or(Value::Null),
                "capability_id": policy.policy_data.get("capability_id").cloned().unwrap_or(Value::Null),
            },
            "blocking_errors": report.blocking_errors,
            "warnings": report.warnings,
        });

        store
            .create_attestation(CreateAttestationRequest {
                dry_run_id: report.dry_run_id.clone(),
                operation_intent: report.predicted_protocol_operation.as_str().to_string(),
                policy_id: report.policy_id.clone(),
                policy_version: report.policy_version,
                execution_hash: report.execution_hash.clone(),
                simulation: report.simulation.clone(),
                protocol_operation_kind: report.predicted_protocol_operation,
                predicted_changes,
                created_by: ctx.operator_id.clone(),
                audit_reference: Some(audit_reference.to_string()),
                ttl: None,
            })
            .await
            .map_err(|e| ExecutionError::Failed(format!("attestation persist failed: {e}")))?;

        Ok(())
    }

    async fn dry_run_inner(
        &self,
        ctx: &AuthContext,
        request_id: &str,
        dry_run_id: &str,
        policy: &PolicyTemplate,
    ) -> Result<DryRunReport, ExecutionError> {
        let mut validation_results = Vec::new();
        let mut warnings = Vec::new();
        let mut blocking_errors = Vec::new();
        let predicted = predict_protocol_operation(policy);
        let execution_hash = execution_hash_for_policy(policy, predicted);

        // --- Policy approved ---
        let approved = policy.status == PolicyStatus::Approved;
        push_step(
            &mut validation_results,
            "policy_approved",
            approved,
            if approved {
                "policy status is approved".into()
            } else {
                format!("policy status is {}", policy.status.as_str())
            },
        );
        if !approved {
            blocking_errors.push(format!(
                "Approved policy required (have {})",
                policy.status.as_str()
            ));
            return Ok(finish_report(
                false,
                validation_results,
                warnings,
                blocking_errors,
                policy,
                dry_run_id,
                None,
                None,
                predicted,
                "op-none",
                &execution_hash,
                SimulationOutcome {
                    executable: false,
                    reason: Some("Approved policy required".into()),
                },
            ));
        }

        // --- Operator authorised ---
        push_step(
            &mut validation_results,
            "operator_authorised",
            true,
            format!("role {} authorised for dry-run", ctx.role.as_str()),
        );

        // --- Plan + signer ---
        let plan = ExecutionPlanner::plan(policy, ctx, request_id)?;
        let signer_identity = self.signing.signer_identity().to_string();
        push_step(
            &mut validation_results,
            "signer_available",
            true,
            format!("signer identity {signer_identity}"),
        );

        let signed = match self
            .signing
            .sign_operation(&plan.governance_operation)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                push_step(
                    &mut validation_results,
                    "signature_valid",
                    false,
                    e.to_string(),
                );
                blocking_errors.push(format!("Invalid signer / signing failed: {e}"));
                return Ok(finish_report(
                    false,
                    validation_results,
                    warnings,
                    blocking_errors,
                    policy,
                    dry_run_id,
                    Some(signer_identity),
                    None,
                    plan.protocol_operation,
                    &plan.operation_id,
                    &execution_hash,
                    SimulationOutcome {
                        executable: false,
                        reason: Some("signing failed".into()),
                    },
                ));
            }
        };

        let verify_ok = self
            .signing
            .verify_signature(&signed.operation, &signed.signature)
            .is_ok();
        push_step(
            &mut validation_results,
            "signature_valid",
            verify_ok,
            if verify_ok {
                "signature verified".into()
            } else {
                "signature verification failed".into()
            },
        );
        if !verify_ok {
            blocking_errors.push("Signature valid check failed".into());
        }

        // --- Protocol adapter (read-only) ---
        push_step(
            &mut validation_results,
            "protocol_adapter_available",
            true,
            "read-only PROTO-0 observation adapters available".into(),
        );

        // --- Target exists ---
        match &policy.target_agent_id {
            None => {
                warnings.push("target_agent_id not set — grant simulation may block".into());
                push_step(
                    &mut validation_results,
                    "target_exists",
                    false,
                    "no target_agent_id on policy".into(),
                );
            }
            Some(id) => match proto0::get_agent(self.protocol, id) {
                Ok(a) => push_step(
                    &mut validation_results,
                    "target_exists",
                    true,
                    format!("agent {} status={}", a.agent_id, a.status),
                ),
                Err(_) => {
                    push_step(
                        &mut validation_results,
                        "target_exists",
                        false,
                        format!("agent {id} not found"),
                    );
                    blocking_errors.push(format!("Target exists check failed: {id}"));
                }
            },
        }

        // --- Protocol simulation (read-only) ---
        let simulation = simulate_policy_apply(self.protocol, policy);
        push_step(
            &mut validation_results,
            "protocol_simulation",
            simulation.executable,
            simulation
                .reason
                .clone()
                .unwrap_or_else(|| "simulation would accept".into()),
        );
        if !simulation.executable {
            if let Some(reason) = &simulation.reason {
                blocking_errors.push(reason.clone());
            }
        }

        // --- Execution hash binding present ---
        push_step(
            &mut validation_results,
            "execution_hash_bound",
            !execution_hash.is_empty(),
            format!("execution_hash={execution_hash}"),
        );

        let executable = blocking_errors.is_empty();

        Ok(finish_report(
            executable,
            validation_results,
            warnings,
            blocking_errors,
            policy,
            dry_run_id,
            Some(signer_identity),
            Some(signed.signature),
            plan.protocol_operation,
            &plan.operation_id,
            &execution_hash,
            simulation,
        ))
    }
}

fn push_step(out: &mut Vec<ValidationStep>, name: &str, passed: bool, detail: String) {
    out.push(ValidationStep {
        name: name.into(),
        passed,
        detail,
    });
}

#[allow(clippy::too_many_arguments)]
fn finish_report(
    executable: bool,
    validation_results: Vec<ValidationStep>,
    warnings: Vec<String>,
    blocking_errors: Vec<String>,
    policy: &PolicyTemplate,
    dry_run_id: &str,
    signer_identity: Option<String>,
    signature_hex: Option<String>,
    predicted: ProtocolOperationKind,
    operation_id: &str,
    execution_hash: &str,
    simulation: SimulationOutcome,
) -> DryRunReport {
    DryRunReport {
        executable,
        validation_results,
        predicted_protocol_operation: predicted,
        warnings,
        blocking_errors,
        execution_mode: ExecutionMode::DryRun,
        dry_run_id: dry_run_id.into(),
        operation_id: operation_id.into(),
        policy_id: policy.id.clone(),
        policy_version: policy.version,
        execution_hash: execution_hash.into(),
        signer_identity,
        signature_hex,
        simulation,
        note: "Dry-run only — protocol state was not modified. Apply is not enabled. execution_hash is content-addressed Apply binding; dry-run signatures must not be reused for Apply.".into(),
    }
}
