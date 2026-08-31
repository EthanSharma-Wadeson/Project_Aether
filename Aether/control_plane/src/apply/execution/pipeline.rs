//! Apply execution pipeline — guarded orchestration that stops before PROTO-0 mutation.

use serde_json::json;

use super::errors::{ExecutionPipelineError, ExecutionPipelineErrorCode};
use super::model::{
    ApplyExecutionContext, ApplyExecutionPhase, ExecuteApplyRequest, ExecuteApplyResult,
};
use super::steps::{
    new_context, step_begin_execution, step_finalise_rejected, step_proto0_boundary, step_reserve,
    step_resimulate, step_validate, ExecutionPipelineContext,
};
use crate::apply::enablement::guard_pipeline_entry;
use crate::apply::errors::ApplyErrorCode;
use crate::db::audit;
use crate::execution::simulate::protocol_observation_fingerprint;

/// Run the Apply execution pipeline (Phase 11: blocked at PROTO-0 while disabled).
///
/// Normative Phase 11 order (C1 — safer than historical reserve→resim docs):
/// validation → re-simulation → atomic reserve+consume → begin execution →
/// freshness recheck → `proto0_write` (disabled) → finalise rejected → audit.
///
/// Failures before reserve do **not** create replay rows or consume approvals.
pub async fn execute_apply(
    pipe: &ExecutionPipelineContext<'_>,
    request: ExecuteApplyRequest,
) -> Result<ExecuteApplyResult, ExecutionPipelineError> {
    // C8 / C9 — early enablement + confirm guards.
    guard_pipeline_entry(&request)?;

    let mut exec = new_context(&request);
    let fingerprint_before = protocol_observation_fingerprint(pipe.protocol);

    audit_event(pipe, &exec, "APPLY_EXECUTION_STARTED", "started", None).await;

    // 1–2. Validation
    if let Err(err) = step_validate(pipe, &request, &mut exec).await {
        audit_event(
            pipe,
            &exec,
            "APPLY_EXECUTION_REJECTED",
            err.code.as_str(),
            None,
        )
        .await;
        return Err(err);
    }
    audit_event(pipe, &exec, "APPLY_VALIDATION_PASSED", "validated", None).await;

    // 3. Re-simulation (before replay — C1)
    if let Err(err) = step_resimulate(pipe, &mut exec).await {
        audit_event(
            pipe,
            &exec,
            "APPLY_EXECUTION_REJECTED",
            err.code.as_str(),
            None,
        )
        .await;
        return Err(err);
    }
    audit_event(
        pipe,
        &exec,
        "APPLY_RESIMULATION_APPROVED",
        "resimulation_approved",
        None,
    )
    .await;

    // 4. Atomic replay reserve + approval consume (C2)
    if let Err(err) = step_reserve(pipe, &mut exec).await {
        audit_event(
            pipe,
            &exec,
            "APPLY_EXECUTION_REJECTED",
            err.code.as_str(),
            None,
        )
        .await;
        return Err(err);
    }
    audit_event(pipe, &exec, "APPLY_REPLAY_RESERVED", "reserved", None).await;

    // 5. Begin executing
    if let Err(err) = step_begin_execution(pipe, &mut exec).await {
        let _ = pipe
            .replay
            .mark_aborted(
                &exec.operation_id,
                Some(err.to_string()),
                Some(exec.audit_correlation_id.clone()),
            )
            .await;
        let _ = super::steps::advance(&mut exec, ApplyExecutionPhase::Aborted);
        audit_event(
            pipe,
            &exec,
            "APPLY_EXECUTION_REJECTED",
            err.code.as_str(),
            None,
        )
        .await;
        return Err(err);
    }

    // 6. PROTO-0 adapter boundary (disabled)
    let boundary = match step_proto0_boundary(pipe, &exec).await {
        Ok(outcome) => outcome,
        Err(err) => {
            let _ = pipe
                .replay
                .mark_rejected(
                    &exec.operation_id,
                    Some(err.to_string()),
                    Some(exec.audit_correlation_id.clone()),
                )
                .await;
            let _ = super::steps::advance(&mut exec, ApplyExecutionPhase::Rejected);
            audit_event(
                pipe,
                &exec,
                "APPLY_EXECUTION_BLOCKED",
                err.code.as_str(),
                None,
            )
            .await;
            audit_event(
                pipe,
                &exec,
                "APPLY_EXECUTION_REJECTED",
                err.code.as_str(),
                None,
            )
            .await;
            return Err(err);
        }
    };

    let disabled = ApplyErrorCode::ApplyExecutionDisabled.as_str();
    audit_event(pipe, &exec, "APPLY_PROTO_BOUNDARY", disabled, None).await;
    audit_event(pipe, &exec, "APPLY_EXECUTION_BLOCKED", disabled, None).await;

    // 7. Finalise as rejected (known disabled outcome — not stuck)
    if let Err(err) = step_finalise_rejected(pipe, &mut exec, disabled).await {
        audit_event(
            pipe,
            &exec,
            "APPLY_EXECUTION_REJECTED",
            err.code.as_str(),
            None,
        )
        .await;
        return Err(err);
    }

    let fingerprint_after = protocol_observation_fingerprint(pipe.protocol);
    let protocol_unchanged = fingerprint_before == fingerprint_after;

    audit_event(pipe, &exec, "APPLY_EXECUTION_FINALISED", disabled, None).await;

    if !protocol_unchanged {
        return Err(ExecutionPipelineError::new(
            ExecutionPipelineErrorCode::Failed,
            "protocol observation fingerprint changed (INV: no mutation)",
            exec.phase,
            Some(exec),
        ));
    }

    Ok(ExecuteApplyResult {
        context: exec,
        outcome_code: boundary.pipeline_code.as_str().into(),
        protocol_unchanged: true,
        replay_reserved: true,
    })
}

fn audit_payload(exec: &ApplyExecutionContext, result: &str) -> serde_json::Value {
    json!({
        "request_id": exec.request_id,
        "operation_id": exec.operation_id,
        "policy_id": exec.policy_id,
        "policy_version": exec.policy_version,
        "execution_hash": exec.execution_hash,
        "dry_run_id": exec.dry_run_id,
        "approval_id": exec.approval_id,
        "actor": exec.actor_id,
        "audit_correlation_id": exec.audit_correlation_id,
        "phase": exec.phase.as_str(),
        "result": result,
        "operation_intent": exec.operation_intent,
    })
}

async fn audit_event(
    pipe: &ExecutionPipelineContext<'_>,
    exec: &ApplyExecutionContext,
    action: &str,
    result: &str,
    _extra: Option<serde_json::Value>,
) {
    let _ = audit::append(
        pipe.pool,
        Some(&exec.actor_id),
        action,
        Some(&exec.operation_id),
        Some(audit_payload(exec, result)),
    )
    .await;
}

/// Reconstruct Apply audit trail by `request_id` (C7).
pub async fn reconstruct_audit_by_request_id(
    pool: &sqlx::SqlitePool,
    request_id: &str,
) -> Result<Vec<crate::models::audit::AuditEvent>, crate::error::Error> {
    let like = format!("%\"request_id\":\"{request_id}\"%");
    let rows = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            String,
        ),
    >(
        r#"
        SELECT id, operator_id, action, target, metadata, created_at
        FROM audit_log
        WHERE metadata LIKE ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(&like)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, operator_id, action, target, metadata, created_at)| {
            crate::models::audit::AuditEvent {
                id,
                operator_id,
                action,
                target,
                metadata: metadata.and_then(|m| serde_json::from_str(&m).ok()),
                created_at: created_at.parse().unwrap_or_else(|_| chrono::Utc::now()),
            }
        })
        .collect())
}
