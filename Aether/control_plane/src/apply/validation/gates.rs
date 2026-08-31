//! Individual Apply validation gates (G1–G14). Read-only; no PROTO-0.

use chrono::Utc;

use super::errors::{ValidationError, ValidationErrorCode};
use super::model::{ApplyValidationRequest, GateId, GateResult, ValidationResult};
use crate::apply::apply_enabled;
use crate::apply::approval::{ApplyApproval, ApprovalError, ApprovalStatus, ApprovalStore};
use crate::apply::attestation::{
    AttestationBinding, AttestationError, AttestationStatus, AttestationStore, DryRunAttestation,
};
use crate::apply::signature::model::{
    ApplySignBodyV1, SignatureStatus, SignedOperationV1, APPLY_SIGN_BODY_SCHEMA, PURPOSE_APPLY,
};
use crate::apply::signature::prepare::sign_body_bytes;
use crate::apply::signature::SignatureStore;
use crate::auth::roles::require_operator;
use crate::db::policies as policy_db;
use crate::execution::hash::execution_hash_for_policy;
use crate::execution::types::ProtocolOperationKind;
use crate::models::policies::{PolicyStatus, PolicyTemplate};
use crate::signer::SigningGateway;
use sqlx::SqlitePool;

/// Shared stores for the gate chain (no replay / no PROTO-0).
pub struct ValidationContext<'a> {
    pub pool: &'a SqlitePool,
    pub approvals: &'a ApprovalStore,
    pub attestations: &'a AttestationStore,
    pub signatures: &'a SignatureStore,
    pub signing: &'a SigningGateway,
}

pub(crate) fn empty_result(req: &ApplyValidationRequest) -> ValidationResult {
    ValidationResult {
        approved: false,
        operation_id: req.operation_id.clone(),
        approval_id: req.approval_id.clone(),
        dry_run_id: req.dry_run_id.clone(),
        execution_hash: req.execution_hash.clone(),
        validated_at: Utc::now(),
        gate_results: Vec::new(),
        decision_code: None,
    }
}

pub(crate) fn fail(
    partial: &mut ValidationResult,
    gate: GateId,
    code: ValidationErrorCode,
    message: impl Into<String>,
) -> ValidationError {
    let gr = GateResult::failed(gate, code.as_str());
    partial.gate_results.push(gr);
    partial.approved = false;
    partial.validated_at = Utc::now();
    ValidationError::new(code, gate, message, partial.clone())
}

pub(crate) fn pass(partial: &mut ValidationResult, gate: GateId) {
    partial.gate_results.push(GateResult::passed(gate));
}

/// G1 — Request authentication established.
pub(crate) fn gate_authentication(
    req: &ApplyValidationRequest,
    partial: &mut ValidationResult,
) -> Result<(), ValidationError> {
    if !req.authenticated {
        return Err(fail(
            partial,
            GateId::G1,
            ValidationErrorCode::Unauthenticated,
            "request is not authenticated",
        ));
    }
    pass(partial, GateId::G1);
    Ok(())
}

/// G2 — JWT validation.
pub(crate) fn gate_jwt(
    req: &ApplyValidationRequest,
    partial: &mut ValidationResult,
) -> Result<(), ValidationError> {
    if !req.jwt_valid || req.actor.operator_id.is_empty() {
        return Err(fail(
            partial,
            GateId::G2,
            ValidationErrorCode::JwtInvalid,
            "JWT is missing or invalid",
        ));
    }
    pass(partial, GateId::G2);
    Ok(())
}

/// G3 — RBAC (operator or admin).
pub(crate) fn gate_rbac(
    req: &ApplyValidationRequest,
    partial: &mut ValidationResult,
) -> Result<(), ValidationError> {
    if require_operator(&req.actor).is_err() {
        return Err(fail(
            partial,
            GateId::G3,
            ValidationErrorCode::RbacDenied,
            format!("role {} cannot apply", req.actor.role.as_str()),
        ));
    }
    pass(partial, GateId::G3);
    Ok(())
}

/// G4 — CSRF validation.
pub(crate) fn gate_csrf(
    req: &ApplyValidationRequest,
    partial: &mut ValidationResult,
) -> Result<(), ValidationError> {
    if !req.csrf_valid {
        return Err(fail(
            partial,
            GateId::G4,
            ValidationErrorCode::CsrfInvalid,
            "CSRF validation failed",
        ));
    }
    pass(partial, GateId::G4);
    Ok(())
}

/// G5 — Apply enabled check.
///
/// Phase 6: `apply_enabled()` is false. The gate **passes** so the
/// authorisation chain can be validated; execution remains blocked elsewhere.
pub(crate) fn gate_apply_enabled(partial: &mut ValidationResult) -> Result<(), ValidationError> {
    let _enabled = apply_enabled();
    // Documented: validation may succeed; execution stays blocked while false.
    pass(partial, GateId::G5);
    Ok(())
}

/// G6 — Approval exists.
pub(crate) async fn gate_approval_exists(
    ctx: &ValidationContext<'_>,
    req: &ApplyValidationRequest,
    partial: &mut ValidationResult,
) -> Result<ApplyApproval, ValidationError> {
    match ctx.approvals.get_approval(&req.approval_id).await {
        Ok(Some(a)) => {
            pass(partial, GateId::G6);
            Ok(a)
        }
        Ok(None) => Err(fail(
            partial,
            GateId::G6,
            ValidationErrorCode::ApprovalMissing,
            format!("approval {} not found", req.approval_id),
        )),
        Err(e) => Err(fail(
            partial,
            GateId::G6,
            ValidationErrorCode::Database,
            e.to_string(),
        )),
    }
}

/// G7 — Approval active (not expired / consumed / cancelled).
pub(crate) async fn gate_approval_active(
    ctx: &ValidationContext<'_>,
    req: &ApplyValidationRequest,
    approval: &ApplyApproval,
    partial: &mut ValidationResult,
) -> Result<ApplyApproval, ValidationError> {
    // Re-load via validate to apply lazy expiry.
    use crate::apply::approval::ApprovalBinding;
    match ctx
        .approvals
        .validate_approval(&ApprovalBinding {
            approval_id: req.approval_id.clone(),
            dry_run_id: req.dry_run_id.clone(),
            execution_hash: req.execution_hash.clone(),
            policy_version: approval.policy_version,
        })
        .await
    {
        Ok(a) => {
            pass(partial, GateId::G7);
            Ok(a)
        }
        Err(ApprovalError::Expired { .. }) => Err(fail(
            partial,
            GateId::G7,
            ValidationErrorCode::ApprovalExpired,
            "approval expired",
        )),
        Err(ApprovalError::Consumed { .. }) => Err(fail(
            partial,
            GateId::G7,
            ValidationErrorCode::ApprovalConsumed,
            "approval already consumed",
        )),
        Err(ApprovalError::NotFound { .. }) => Err(fail(
            partial,
            GateId::G7,
            ValidationErrorCode::ApprovalMissing,
            "approval not found",
        )),
        Err(ApprovalError::Cancelled { .. })
        | Err(ApprovalError::ExecutionHashMismatch)
        | Err(ApprovalError::PolicyVersionMismatch { .. })
        | Err(ApprovalError::IntentMismatch)
        | Err(ApprovalError::PolicyIdMismatch) => Err(fail(
            partial,
            GateId::G7,
            ValidationErrorCode::ApprovalInactive,
            "approval is not active for this binding",
        )),
        Err(e) => {
            let code = match approval.status {
                ApprovalStatus::Expired => ValidationErrorCode::ApprovalExpired,
                ApprovalStatus::Consumed => ValidationErrorCode::ApprovalConsumed,
                _ => ValidationErrorCode::ApprovalInactive,
            };
            Err(fail(partial, GateId::G7, code, e.to_string()))
        }
    }
}

/// G8 — Dry-run attestation valid (executable, not expired, hash match).
pub(crate) async fn gate_attestation_valid(
    ctx: &ValidationContext<'_>,
    req: &ApplyValidationRequest,
    approval: &ApplyApproval,
    partial: &mut ValidationResult,
) -> Result<DryRunAttestation, ValidationError> {
    match ctx
        .attestations
        .get_valid_attestation(&AttestationBinding {
            dry_run_id: req.dry_run_id.clone(),
            execution_hash: req.execution_hash.clone(),
            policy_version: approval.policy_version,
            operation_intent: approval.operation_intent.clone(),
        })
        .await
    {
        Ok(a) if a.status == AttestationStatus::Executable => {
            pass(partial, GateId::G8);
            Ok(a)
        }
        Ok(_) => Err(fail(
            partial,
            GateId::G8,
            ValidationErrorCode::AttestationInvalid,
            "attestation is not executable",
        )),
        Err(AttestationError::Expired { .. }) => Err(fail(
            partial,
            GateId::G8,
            ValidationErrorCode::AttestationExpired,
            "attestation expired",
        )),
        Err(AttestationError::NotFound { .. }) => Err(fail(
            partial,
            GateId::G8,
            ValidationErrorCode::AttestationNotFound,
            "attestation not found",
        )),
        Err(AttestationError::ExecutionHashMismatch)
        | Err(AttestationError::PolicyVersionMismatch { .. })
        | Err(AttestationError::IntentMismatch) => Err(fail(
            partial,
            GateId::G8,
            ValidationErrorCode::AttestationInvalid,
            "attestation binding mismatch",
        )),
        Err(e) => Err(fail(
            partial,
            GateId::G8,
            ValidationErrorCode::AttestationInvalid,
            e.to_string(),
        )),
    }
}

/// G9 — execution_hash matches across dry-run, approval, and signature.
pub(crate) async fn gate_execution_hash_match(
    ctx: &ValidationContext<'_>,
    req: &ApplyValidationRequest,
    approval: &ApplyApproval,
    attestation: &DryRunAttestation,
    partial: &mut ValidationResult,
) -> Result<(), ValidationError> {
    let signed = match ctx.signatures.get(&req.operation_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            // Signature existence is G10; for G9 we still need the hash if present.
            // If missing, fail G9 only when we cannot form the triple — defer to G10
            // by checking request hash against approval + attestation first.
            if attestation.execution_hash != approval.execution_hash
                || attestation.execution_hash != req.execution_hash
                || approval.execution_hash != req.execution_hash
            {
                return Err(fail(
                    partial,
                    GateId::G9,
                    ValidationErrorCode::ExecutionHashMismatch,
                    "execution_hash mismatch between dry-run and approval",
                ));
            }
            // Signature missing — still pass G9 on the available bindings; G10 fails.
            pass(partial, GateId::G9);
            return Ok(());
        }
        Err(e) => {
            return Err(fail(
                partial,
                GateId::G9,
                ValidationErrorCode::Database,
                e.to_string(),
            ));
        }
    };

    let dry = &attestation.execution_hash;
    let appr = &approval.execution_hash;
    let sig = &signed.execution_hash;
    let req_h = &req.execution_hash;

    if dry != appr || dry != sig || appr != sig || dry != req_h {
        return Err(fail(
            partial,
            GateId::G9,
            ValidationErrorCode::ExecutionHashMismatch,
            "execution_hash mismatch across dry-run, approval, and signature",
        ));
    }
    pass(partial, GateId::G9);
    Ok(())
}

/// G10 — Signed operation exists.
pub(crate) async fn gate_signed_operation_exists(
    ctx: &ValidationContext<'_>,
    req: &ApplyValidationRequest,
    partial: &mut ValidationResult,
) -> Result<SignedOperationV1, ValidationError> {
    match ctx.signatures.get(&req.operation_id).await {
        Ok(Some(s)) => {
            pass(partial, GateId::G10);
            Ok(s)
        }
        Ok(None) => Err(fail(
            partial,
            GateId::G10,
            ValidationErrorCode::SignedOperationMissing,
            format!("signed operation {} not found", req.operation_id),
        )),
        Err(e) => Err(fail(
            partial,
            GateId::G10,
            ValidationErrorCode::Database,
            e.to_string(),
        )),
    }
}

/// G11 — Signature valid (Ed25519, purpose=apply, signer authorised).
pub(crate) fn gate_signature_valid(
    ctx: &ValidationContext<'_>,
    signed: &SignedOperationV1,
    partial: &mut ValidationResult,
) -> Result<(), ValidationError> {
    if signed.purpose != PURPOSE_APPLY {
        return Err(fail(
            partial,
            GateId::G11,
            ValidationErrorCode::SignaturePurposeInvalid,
            format!("signature purpose '{}' is not apply", signed.purpose),
        ));
    }

    let role = signed.signer_role.as_str();
    if role != "operator" && role != "admin" {
        return Err(fail(
            partial,
            GateId::G11,
            ValidationErrorCode::SignatureSignerUnauthorised,
            format!("signer role '{role}' cannot authorise Apply"),
        ));
    }

    if signed.signer_identity != ctx.signing.signer_identity() {
        return Err(fail(
            partial,
            GateId::G11,
            ValidationErrorCode::SignatureInvalid,
            "signer identity mismatch",
        ));
    }

    let sign_body = ApplySignBodyV1 {
        schema: APPLY_SIGN_BODY_SCHEMA.into(),
        purpose: PURPOSE_APPLY.into(),
        operation_id: signed.operation_id.clone(),
        request_id: signed.request_id.clone(),
        payload_hash: signed.payload_hash.clone(),
        signer_identity: signed.signer_identity.clone(),
        signed_at: signed.signed_at.clone(),
    };

    let bytes = match sign_body_bytes(&sign_body) {
        Ok(b) => b,
        Err(e) => {
            return Err(fail(
                partial,
                GateId::G11,
                ValidationErrorCode::SignatureInvalid,
                e.to_string(),
            ));
        }
    };

    if ctx
        .signing
        .verify_message(&bytes, &signed.signature)
        .is_err()
    {
        return Err(fail(
            partial,
            GateId::G11,
            ValidationErrorCode::SignatureInvalid,
            "Ed25519 signature verification failed",
        ));
    }

    pass(partial, GateId::G11);
    Ok(())
}

/// G12 — Signature TTL valid.
pub(crate) fn gate_signature_ttl(
    signed: &SignedOperationV1,
    partial: &mut ValidationResult,
) -> Result<(), ValidationError> {
    if signed.status == SignatureStatus::Expired || signed.expires_at <= Utc::now() {
        return Err(fail(
            partial,
            GateId::G12,
            ValidationErrorCode::SignatureExpired,
            "signature expired",
        ));
    }
    pass(partial, GateId::G12);
    Ok(())
}

/// G13 — Policy version unchanged (not archived / modified / hash-changed).
pub(crate) async fn gate_policy_version_unchanged(
    ctx: &ValidationContext<'_>,
    signed: &SignedOperationV1,
    attestation: &DryRunAttestation,
    partial: &mut ValidationResult,
) -> Result<PolicyTemplate, ValidationError> {
    let policy = match policy_db::get_policy(ctx.pool, &signed.policy_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Err(fail(
                partial,
                GateId::G13,
                ValidationErrorCode::PolicyNotFound,
                format!("policy {} not found", signed.policy_id),
            ));
        }
        Err(e) => {
            return Err(fail(
                partial,
                GateId::G13,
                ValidationErrorCode::Database,
                e.to_string(),
            ));
        }
    };

    if policy.status == PolicyStatus::Archived {
        return Err(fail(
            partial,
            GateId::G13,
            ValidationErrorCode::PolicyArchived,
            "policy is archived",
        ));
    }

    if policy.status != PolicyStatus::Approved {
        return Err(fail(
            partial,
            GateId::G13,
            ValidationErrorCode::PolicyNotApproved,
            format!("policy status is {}", policy.status.as_str()),
        ));
    }

    if policy.version != signed.policy_version {
        return Err(fail(
            partial,
            GateId::G13,
            ValidationErrorCode::PolicyVersionMismatch,
            format!(
                "policy version changed: expected {}, got {}",
                signed.policy_version, policy.version
            ),
        ));
    }

    // Detect content / hash drift via recomputed execution hash.
    let kind = parse_intent_kind(&signed.operation_intent);
    let current_hash = execution_hash_for_policy(&policy, kind);
    if current_hash != signed.execution_hash || current_hash != attestation.execution_hash {
        // Prefer distinguishing hash change vs version when version matched.
        if policy.hash.is_empty() {
            return Err(fail(
                partial,
                GateId::G13,
                ValidationErrorCode::StaleExecutionHash,
                "recomputed execution_hash does not match binding",
            ));
        }
        return Err(fail(
            partial,
            GateId::G13,
            ValidationErrorCode::PolicyHashChanged,
            "policy content hash changed relative to dry-run binding",
        ));
    }

    pass(partial, GateId::G13);
    Ok(policy)
}

/// G14 — Re-simulation required (Phase 6: no execution).
pub(crate) fn gate_resimulation_required(
    partial: &mut ValidationResult,
) -> Result<(), ValidationError> {
    // Checkpoint passes: authorisation chain is complete; execution must
    // still re-simulate in a future phase.
    pass(partial, GateId::G14);
    partial.decision_code = Some(ValidationErrorCode::RequiresResimulation.as_str().into());
    Ok(())
}

fn parse_intent_kind(intent: &str) -> ProtocolOperationKind {
    match intent {
        "CapabilityRevoke" => ProtocolOperationKind::CapabilityRevoke,
        "FreezeIdentity" => ProtocolOperationKind::FreezeIdentity,
        _ => ProtocolOperationKind::CapabilityGrant,
    }
}
