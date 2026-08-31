//! Apply signature verification — validates purpose, bindings, expiry, and crypto.

use serde_json::json;

use super::errors::SignatureError;
use super::model::{
    ApplySignBodyV1, SignatureStatus, SignedOperationV1, APPLY_SIGN_BODY_SCHEMA, PURPOSE_APPLY,
};
use super::prepare::{reject_invalid_purpose, sign_body_bytes};
use super::store::SignatureStore;
use crate::apply::approval::{ApprovalBinding, ApprovalError, ApprovalStore};
use crate::db::audit;
use crate::signer::SigningGateway;

/// Verify a prepared Apply signature and mark it `valid` on success.
pub async fn verify_signature(
    store: &SignatureStore,
    approvals: &ApprovalStore,
    signing: &SigningGateway,
    operation_id: &str,
) -> Result<SignedOperationV1, SignatureError> {
    let record = store
        .get(operation_id)
        .await?
        .ok_or_else(|| SignatureError::NotFound {
            operation_id: operation_id.to_string(),
        })?;

    match verify_inner(store, approvals, signing, &record).await {
        Ok(valid) => {
            let _ = audit::append(
                store.pool(),
                Some(&record.signer_id),
                "APPLY_SIGNATURE_VERIFIED",
                Some(&record.policy_id),
                Some(json!({
                    "request_id": record.request_id,
                    "operation_id": record.operation_id,
                    "approval_id": record.approval_id,
                    "dry_run_id": record.dry_run_id,
                    "execution_hash": record.execution_hash,
                    "signer": record.signer_id,
                })),
            )
            .await;
            Ok(valid)
        }
        Err(err) => {
            let _ = audit::append(
                store.pool(),
                Some(&record.signer_id),
                "APPLY_SIGNATURE_REJECTED",
                Some(&record.policy_id),
                Some(json!({
                    "request_id": record.request_id,
                    "operation_id": record.operation_id,
                    "approval_id": record.approval_id,
                    "dry_run_id": record.dry_run_id,
                    "execution_hash": record.execution_hash,
                    "signer": record.signer_id,
                    "failure_code": err.code().as_str(),
                    "reason": err.to_string(),
                })),
            )
            .await;
            Err(err)
        }
    }
}

async fn verify_inner(
    store: &SignatureStore,
    approvals: &ApprovalStore,
    signing: &SigningGateway,
    record: &SignedOperationV1,
) -> Result<SignedOperationV1, SignatureError> {
    reject_invalid_purpose(&record.purpose)?;

    if record.status == SignatureStatus::Expired {
        return Err(SignatureError::ExpiredSignature {
            operation_id: record.operation_id.clone(),
        });
    }

    // Re-check approval still active and bindings match (INV-S2, INV-S3).
    let approval = approvals
        .validate_approval(&ApprovalBinding {
            approval_id: record.approval_id.clone(),
            dry_run_id: record.dry_run_id.clone(),
            execution_hash: record.execution_hash.clone(),
            policy_version: record.policy_version,
        })
        .await
        .map_err(map_approval_error)?;

    if approval.policy_id != record.policy_id {
        return Err(SignatureError::PolicyIdMismatch);
    }
    if approval.operation_intent != record.operation_intent {
        return Err(SignatureError::IntentMismatch);
    }

    // Crypto verify: signer_identity must match current configured signer (key rotation).
    if record.signer_identity != signing.signer_identity() {
        return Err(SignatureError::SignerMismatch);
    }

    let sign_body = ApplySignBodyV1 {
        schema: APPLY_SIGN_BODY_SCHEMA.into(),
        purpose: PURPOSE_APPLY.into(),
        operation_id: record.operation_id.clone(),
        request_id: record.request_id.clone(),
        payload_hash: record.payload_hash.clone(),
        signer_identity: record.signer_identity.clone(),
        signed_at: record.signed_at.clone(),
    };

    let bytes = sign_body_bytes(&sign_body)?;
    signing
        .verify_message(&bytes, &record.signature)
        .map_err(|e| SignatureError::InvalidSignature {
            reason: e.to_string(),
        })?;

    match record.status {
        SignatureStatus::Prepared => store.mark_valid(&record.operation_id).await,
        SignatureStatus::Valid => Ok(record.clone()),
        SignatureStatus::Expired => Err(SignatureError::ExpiredSignature {
            operation_id: record.operation_id.clone(),
        }),
    }
}

/// Fetch a signed operation (lazy expiry). Does not mutate protocol state.
pub async fn get_signed_operation(
    store: &SignatureStore,
    operation_id: &str,
) -> Result<Option<SignedOperationV1>, SignatureError> {
    store.get(operation_id).await
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
        _ => SignatureError::ApprovalBindingFailed(err.to_string()),
    }
}
