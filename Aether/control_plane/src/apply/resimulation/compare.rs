//! Comparison of attested dry-run evidence vs fresh re-simulation snapshot.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::model::{ComparisonSnapshot, ResimulationDecisionCode};
use crate::apply::attestation::DryRunAttestation;
use crate::apply::canonical_json::canonicalize_json;
use crate::apply::signature::SignedOperationV1;
use crate::execution::types::{ProtocolOperationKind, SimulationOutcome};
use crate::models::policies::PolicyTemplate;

/// Build the attested snapshot from durable dry-run evidence + signed binding.
pub fn attested_snapshot(
    attestation: &DryRunAttestation,
    signed: &SignedOperationV1,
    policy: &PolicyTemplate,
) -> ComparisonSnapshot {
    let policy_hash = attestation
        .predicted_changes
        .get("policy_hash")
        .and_then(|v| v.as_str())
        .unwrap_or(policy.hash.as_str())
        .to_string();

    let target = attestation
        .predicted_changes
        .get("target_agent")
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_str().map(str::to_string)
            }
        })
        .or_else(|| policy.target_agent_id.clone());

    ComparisonSnapshot {
        policy_id: attestation.policy_id.clone(),
        policy_version: attestation.policy_version,
        policy_hash,
        operation_intent: attestation.operation_intent.clone(),
        protocol_operation_kind: attestation.protocol_operation_kind.as_str().to_string(),
        target_agent: target,
        execution_hash: attestation.execution_hash.clone(),
        simulation_executable: attestation
            .simulation_result
            .get("executable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        predicted_changes: normalize_predicted(&attestation.predicted_changes, signed),
    }
}

/// Build a fresh snapshot from the current policy + new simulation.
pub fn fresh_snapshot(
    policy: &PolicyTemplate,
    signed: &SignedOperationV1,
    kind: ProtocolOperationKind,
    execution_hash: &str,
    simulation: &SimulationOutcome,
) -> ComparisonSnapshot {
    let predicted = build_predicted_changes(policy, signed, kind, simulation);
    ComparisonSnapshot {
        policy_id: policy.id.clone(),
        policy_version: policy.version,
        policy_hash: policy.hash.clone(),
        operation_intent: kind.as_str().to_string(),
        protocol_operation_kind: kind.as_str().to_string(),
        target_agent: policy.target_agent_id.clone(),
        execution_hash: execution_hash.to_string(),
        simulation_executable: simulation.executable,
        predicted_changes: predicted,
    }
}

/// Normalize predicted-change documents for stable comparison.
pub fn build_predicted_changes(
    policy: &PolicyTemplate,
    signed: &SignedOperationV1,
    kind: ProtocolOperationKind,
    simulation: &SimulationOutcome,
) -> Value {
    json!({
        "operation_id": signed.operation_id,
        "policy_id": policy.id,
        "policy_version": policy.version,
        "policy_hash": policy.hash,
        "predicted_protocol_operation": kind.as_str(),
        "operation_intent": kind.as_str(),
        "target_agent": policy.target_agent_id,
        "executable": simulation.executable,
        "simulation_reason": simulation.reason,
        "capability_changes": {
            "actions": policy.policy_data.get("actions").cloned().unwrap_or(Value::Null),
            "constraints": policy.policy_data.get("constraints").cloned().unwrap_or(Value::Null),
            "limits": policy.policy_data.get("limits").cloned().unwrap_or(Value::Null),
            "capability_id": policy.policy_data.get("capability_id").cloned().unwrap_or(Value::Null),
        },
        "blocking_errors": Value::Array(Vec::new()),
        "warnings": Value::Array(Vec::new()),
    })
}

fn normalize_predicted(raw: &Value, signed: &SignedOperationV1) -> Value {
    let mut out = raw.clone();
    if let Some(obj) = out.as_object_mut() {
        obj.entry("operation_id".to_string())
            .or_insert_with(|| json!(signed.operation_id));
        // Ensure capability_changes exists for key-wise compare.
        if !obj.contains_key("capability_changes") {
            obj.insert(
                "capability_changes".into(),
                json!({
                    "actions": raw.get("actions").cloned().unwrap_or(Value::Null),
                    "constraints": raw.get("constraints").cloned().unwrap_or(Value::Null),
                    "limits": raw.get("limits").cloned().unwrap_or(Value::Null),
                    "capability_id": raw.get("capability_id").cloned().unwrap_or(Value::Null),
                }),
            );
        }
    }
    out
}

/// Compare attested vs fresh snapshots. First mismatch wins.
pub fn compare_snapshots(
    attested: &ComparisonSnapshot,
    fresh: &ComparisonSnapshot,
) -> ResimulationDecisionCode {
    // Policy identity
    if attested.policy_id != fresh.policy_id
        || attested.policy_version != fresh.policy_version
        || attested.policy_hash != fresh.policy_hash
    {
        return ResimulationDecisionCode::PolicyChanged;
    }

    // Intent identity
    if attested.operation_intent != fresh.operation_intent
        || attested.protocol_operation_kind != fresh.protocol_operation_kind
        || attested.target_agent != fresh.target_agent
    {
        return ResimulationDecisionCode::StateChanged;
    }

    // Execution hash
    if attested.execution_hash != fresh.execution_hash {
        return ResimulationDecisionCode::HashMismatch;
    }

    // Predicted changes / simulation outcome
    if !predicted_changes_match(&attested.predicted_changes, &fresh.predicted_changes)
        || attested.simulation_executable != fresh.simulation_executable
        || !fresh.simulation_executable
    {
        return ResimulationDecisionCode::StateChanged;
    }

    ResimulationDecisionCode::Approved
}

fn predicted_changes_match(attested: &Value, fresh: &Value) -> bool {
    let keys = [
        "predicted_protocol_operation",
        "operation_intent",
        "target_agent",
        "policy_id",
        "policy_version",
        "capability_changes",
    ];
    for key in keys {
        let a = attested.get(key).unwrap_or(&Value::Null);
        let b = fresh.get(key).unwrap_or(&Value::Null);
        if key == "capability_changes" {
            if !capability_slice_match(a, b) {
                return false;
            }
            continue;
        }
        // Null on attested side means not recorded at dry-run time.
        if a.is_null() {
            continue;
        }
        if a != b {
            return false;
        }
    }
    true
}

fn capability_slice_match(a: &Value, b: &Value) -> bool {
    for key in ["actions", "constraints", "limits", "capability_id"] {
        let av = a.get(key).unwrap_or(&Value::Null);
        let bv = b.get(key).unwrap_or(&Value::Null);
        // Null on attested side means "not recorded" — do not treat as mismatch.
        if av.is_null() {
            continue;
        }
        if av != bv {
            return false;
        }
    }
    true
}

/// Deterministic hash of a comparison snapshot (canonical JSON → SHA-256 hex).
pub fn comparison_hash(snapshot: &ComparisonSnapshot) -> Result<String, String> {
    let value = serde_json::to_value(snapshot).map_err(|e| e.to_string())?;
    let canonical = canonicalize_json(&value).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&canonical).map_err(|e| e.to_string())?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
