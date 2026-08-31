//! Apply signature preparation — builds and stores signed Apply operations.

use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::errors::SignatureError;
use super::model::{
    ApplyPayloadV1, ApplySignBodyV1, PrepareSignatureRequest, SignatureStatus, SignedOperationV1,
    APPLY_PAYLOAD_SCHEMA, APPLY_SIGN_BODY_SCHEMA, DEFAULT_SIGNATURE_TTL, PURPOSE_APPLY,
    SIGNATURE_ALGORITHM,
};
use super::store::SignatureStore;
use crate::apply::approval::{ApprovalBinding, ApprovalError, ApprovalStore};
use crate::apply::canonical_json::{canonical_json_bytes, canonicalize_json};
use crate::db::audit;
use crate::db::operators::OperatorRole;
use crate::signer::SigningGateway;

/// Prepare a cryptographically bound Apply signature (no protocol mutation).
pub async fn prepare_signature(
    store: &SignatureStore,
    approvals: &ApprovalStore,
    signing: &SigningGateway,
    request: PrepareSignatureRequest,
) -> Result<SignedOperationV1, SignatureError> {
    reject_invalid_purpose(&request.purpose)?;
    reject_viewer_role(&request.signer_role)?;

    // Validate approval binding (active, not expired/consumed).
    let approval = approvals
        .validate_approval(&ApprovalBinding {
            approval_id: request.approval_id.clone(),
            dry_run_id: request.dry_run_id.clone(),
            execution_hash: request.execution_hash.clone(),
            policy_version: request.policy_version,
        })
        .await
        .map_err(map_approval_error)?;

    if approval.policy_id != request.policy_id {
        return Err(SignatureError::PolicyIdMismatch);
    }
    if approval.operation_intent != request.operation_intent {
        return Err(SignatureError::IntentMismatch);
    }
    if approval.dry_run_id != request.dry_run_id {
        return Err(SignatureError::DryRunMismatch);
    }
    if approval.execution_hash != request.execution_hash {
        return Err(SignatureError::ExecutionHashMismatch);
    }

    // Signature cannot outlive approval.
    let now = Utc::now();
    let ttl = request.ttl.unwrap_or(DEFAULT_SIGNATURE_TTL);
    let mut expires_at = now + ttl;
    if expires_at > approval.expires_at {
        expires_at = approval.expires_at;
    }
    if now >= expires_at {
        return Err(SignatureError::ExpiredApproval {
            approval_id: request.approval_id.clone(),
        });
    }

    let operation_id = request
        .operation_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let payload = ApplyPayloadV1 {
        schema: APPLY_PAYLOAD_SCHEMA.into(),
        purpose: PURPOSE_APPLY.into(),
        operation_id: operation_id.clone(),
        request_id: request.request_id.clone(),
        apply_approval_id: request.approval_id.clone(),
        dry_run_id: request.dry_run_id.clone(),
        policy_id: request.policy_id.clone(),
        policy_version: request.policy_version,
        execution_hash: request.execution_hash.clone(),
        capability_intent: request.operation_intent.clone(),
        target_agent: request.target_agent.clone(),
        confirm: true,
        issued_at: now.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
    };

    let payload_hash = hash_apply_payload(&payload)?;
    let signer_identity = signing.signer_identity().to_string();

    let sign_body = ApplySignBodyV1 {
        schema: APPLY_SIGN_BODY_SCHEMA.into(),
        purpose: PURPOSE_APPLY.into(),
        operation_id: operation_id.clone(),
        request_id: request.request_id.clone(),
        payload_hash: payload_hash.clone(),
        signer_identity: signer_identity.clone(),
        signed_at: now.to_rfc3339(),
    };
    let sign_bytes = sign_body_bytes(&sign_body)?;
    let signature = signing
        .sign_message(&sign_bytes)
        .map_err(|e| SignatureError::SignFailed(e.to_string()))?;

    let record = SignedOperationV1 {
        operation_id: operation_id.clone(),
        approval_id: request.approval_id.clone(),
        dry_run_id: request.dry_run_id.clone(),
        execution_hash: request.execution_hash.clone(),
        policy_id: request.policy_id.clone(),
        policy_version: request.policy_version,
        operation_intent: request.operation_intent.clone(),
        signer_id: request.signer_id.clone(),
        signer_role: request.signer_role.clone(),
        signer_identity: signer_identity.clone(),
        signed_at: sign_body.signed_at.clone(),
        created_at: now,
        expires_at,
        signature_algorithm: SIGNATURE_ALGORITHM.into(),
        signature,
        purpose: PURPOSE_APPLY.into(),
        payload_hash,
        request_id: request.request_id.clone(),
        status: SignatureStatus::Prepared,
    };

    store.insert(&record).await?;

    let _ = audit::append(
        store.pool(),
        Some(&request.signer_id),
        "APPLY_SIGNATURE_PREPARED",
        Some(&request.policy_id),
        Some(json!({
            "request_id": request.request_id,
            "operation_id": operation_id,
            "approval_id": request.approval_id,
            "dry_run_id": request.dry_run_id,
            "execution_hash": request.execution_hash,
            "signer": request.signer_id,
            "signer_identity": signer_identity,
            "purpose": PURPOSE_APPLY,
        })),
    )
    .await;

    Ok(record)
}

pub fn reject_invalid_purpose(purpose: &str) -> Result<(), SignatureError> {
    let p = purpose.trim().to_lowercase();
    if p == PURPOSE_APPLY {
        return Ok(());
    }
    Err(SignatureError::InvalidPurpose {
        purpose: purpose.to_string(),
    })
}

pub fn reject_viewer_role(role: &str) -> Result<(), SignatureError> {
    if role == OperatorRole::Viewer.as_str() {
        return Err(SignatureError::InvalidSignerRole {
            role: role.to_string(),
        });
    }
    if role != OperatorRole::Operator.as_str() && role != OperatorRole::Admin.as_str() {
        return Err(SignatureError::InvalidSignerRole {
            role: role.to_string(),
        });
    }
    Ok(())
}

/// Deterministic payload hash using canonical JSON.
pub fn hash_apply_payload(payload: &ApplyPayloadV1) -> Result<String, SignatureError> {
    let value = payload_to_ordered_value(payload)?;
    let bytes =
        canonical_json_bytes(&value).map_err(|e| SignatureError::Canonicalize(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

pub fn sign_body_bytes(body: &ApplySignBodyV1) -> Result<Vec<u8>, SignatureError> {
    // Fixed field order per APPLY_SIGN_BODY_SCHEMA (normative).
    let mut map = Map::new();
    map.insert("schema".into(), Value::String(body.schema.clone()));
    map.insert("purpose".into(), Value::String(body.purpose.clone()));
    map.insert(
        "operation_id".into(),
        Value::String(body.operation_id.clone()),
    );
    map.insert("request_id".into(), Value::String(body.request_id.clone()));
    map.insert(
        "payload_hash".into(),
        Value::String(body.payload_hash.clone()),
    );
    map.insert(
        "signer_identity".into(),
        Value::String(body.signer_identity.clone()),
    );
    map.insert("signed_at".into(), Value::String(body.signed_at.clone()));
    serde_json::to_vec(&Value::Object(map)).map_err(|e| SignatureError::Canonicalize(e.to_string()))
}

fn payload_to_ordered_value(payload: &ApplyPayloadV1) -> Result<Value, SignatureError> {
    // Build with fixed key order matching APPLY_PAYLOAD_SCHEMA.
    let mut map = Map::new();
    map.insert("schema".into(), Value::String(payload.schema.clone()));
    map.insert("purpose".into(), Value::String(payload.purpose.clone()));
    map.insert(
        "operation_id".into(),
        Value::String(payload.operation_id.clone()),
    );
    map.insert(
        "request_id".into(),
        Value::String(payload.request_id.clone()),
    );
    map.insert(
        "apply_approval_id".into(),
        Value::String(payload.apply_approval_id.clone()),
    );
    map.insert(
        "dry_run_id".into(),
        Value::String(payload.dry_run_id.clone()),
    );
    map.insert("policy_id".into(), Value::String(payload.policy_id.clone()));
    map.insert(
        "policy_version".into(),
        Value::Number(payload.policy_version.into()),
    );
    map.insert(
        "execution_hash".into(),
        Value::String(payload.execution_hash.clone()),
    );
    map.insert(
        "capability_intent".into(),
        Value::String(payload.capability_intent.clone()),
    );
    map.insert(
        "target_agent".into(),
        payload
            .target_agent
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .unwrap_or(Value::Null),
    );
    map.insert("confirm".into(), Value::Bool(payload.confirm));
    map.insert("issued_at".into(), Value::String(payload.issued_at.clone()));
    map.insert(
        "expires_at".into(),
        Value::String(payload.expires_at.clone()),
    );
    // Nested objects (none currently) still go through canonicalize for conformance.
    canonicalize_json(&Value::Object(map)).map_err(|e| SignatureError::Canonicalize(e.to_string()))
}

fn map_approval_error(err: ApprovalError) -> SignatureError {
    use crate::apply::approval::ApprovalErrorCode;
    match err.code() {
        ApprovalErrorCode::NotFound => SignatureError::MissingApproval {
            approval_id: match &err {
                ApprovalError::NotFound { approval_id } => approval_id.clone(),
                _ => String::new(),
            },
        },
        ApprovalErrorCode::Expired => SignatureError::ExpiredApproval {
            approval_id: match &err {
                ApprovalError::Expired { approval_id } => approval_id.clone(),
                _ => String::new(),
            },
        },
        ApprovalErrorCode::Consumed => SignatureError::ConsumedApproval {
            approval_id: match &err {
                ApprovalError::Consumed { approval_id } => approval_id.clone(),
                _ => String::new(),
            },
        },
        ApprovalErrorCode::Cancelled => SignatureError::CancelledApproval {
            approval_id: match &err {
                ApprovalError::Cancelled { approval_id } => approval_id.clone(),
                _ => String::new(),
            },
        },
        ApprovalErrorCode::ExecutionHashMismatch => SignatureError::ExecutionHashMismatch,
        ApprovalErrorCode::PolicyVersionMismatch => match err {
            ApprovalError::PolicyVersionMismatch { expected, actual } => {
                SignatureError::PolicyVersionMismatch { expected, actual }
            }
            _ => SignatureError::ApprovalBindingFailed(err.to_string()),
        },
        _ => SignatureError::ApprovalBindingFailed(err.to_string()),
    }
}
