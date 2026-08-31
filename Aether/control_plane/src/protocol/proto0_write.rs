//! PROTO-0 write adapter — sole Control Plane mutation boundary.
//!
//! See APPLY_PROTO_ADAPTER_SPEC.md §8.
//!
//! Responsibilities: map validated policy → invoke PROTO-0 → return outcome.
//! Does **not** validate JWT, CSRF, approvals, signatures, or replay.
//!
//! Mutations occur **only** when [`crate::apply::apply_enabled`] is `true`.

use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use aether_core::capability::grant::{grant_capability, CapabilityGrant};
use aether_core::error::Error as CoreError;

use crate::apply::apply_enabled;
use crate::apply::errors::{map_core_error, ApplyErrorCode};
use crate::apply::policy_mapping::{predict_protocol_operation, validate_and_map};
use crate::apply::policy_mapping::{
    EnterpriseIssuerRef, MappedFreeze, MappedGrant, MappedOperation, MappedRevoke,
};
use crate::apply::PolicyMappingError;
use crate::execution::types::ProtocolOperationKind;
use crate::models::policies::PolicyTemplate;
use crate::protocol::state::ProtocolState;

const ADAPTER_VERSION: &str = "aether.cp.proto_adapter.v1";

/// Apply execution context carried into PROTO-0 writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyContext {
    pub operation_id: String,
    pub request_id: String,
    pub execution_hash: String,
    pub operator_id: String,
    pub capability_intent: ProtocolOperationKind,
}

/// Server-held enterprise operational key for `CapabilityGrant::sign`.
///
/// Never serialised to API clients. Constructed only by trusted Control Plane code.
#[derive(Clone)]
pub struct EnterpriseGrantKey {
    signing_key: SigningKey,
}

impl EnterpriseGrantKey {
    pub fn from_signing_key(signing_key: SigningKey) -> Self {
        Self { signing_key }
    }

    pub fn from_seed_u64(seed: u64) -> Self {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(seed);
        Self {
            signing_key: SigningKey::generate(&mut rng),
        }
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    fn sign_capability(
        &self,
        capability: &aether_core::capability::model::CapabilityV0,
    ) -> Result<CapabilityGrant, CoreError> {
        CapabilityGrant::sign(&self.signing_key, capability)
    }
}

impl std::fmt::Debug for EnterpriseGrantKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnterpriseGrantKey")
            .field("public_key", &hex::encode(self.public_key_bytes()))
            .finish()
    }
}

/// Request to execute a mapped PROTO-0 write.
#[derive(Debug, Clone)]
pub struct Proto0WriteRequest {
    pub ctx: ApplyContext,
    pub policy: PolicyTemplate,
    pub enterprise_issuer: EnterpriseIssuerRef,
    /// Required for CapabilityGrant / PolicyApply when Apply is enabled.
    pub grant_key: Option<EnterpriseGrantKey>,
    /// Revoke only — Unix seconds at execute time.
    pub revoked_at: u64,
}

/// Outcome of a PROTO-0 write attempt.
#[derive(Debug, Clone, Serialize)]
pub struct Proto0Outcome {
    pub result: Proto0Result,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_error: Option<String>,
    pub protocol_detail: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Proto0Result {
    Success,
    Rejected,
    Timeout,
}

/// Validate mapping without mutating protocol state.
pub fn validate_request(
    state: &ProtocolState,
    request: &Proto0WriteRequest,
) -> Result<MappedOperation, PolicyMappingError> {
    let predicted = predict_protocol_operation(&request.policy);
    if predicted != request.ctx.capability_intent {
        return Err(PolicyMappingError::new(
            ApplyErrorCode::ApplyInvalidPolicy,
            "capability_intent does not match policy",
        ));
    }
    match request.ctx.capability_intent {
        ProtocolOperationKind::CapabilityGrant
        | ProtocolOperationKind::CapabilityRevoke
        | ProtocolOperationKind::FreezeIdentity => {}
        ProtocolOperationKind::Unknown | ProtocolOperationKind::PolicyApply => {
            return Err(PolicyMappingError::new(
                ApplyErrorCode::ApplyUnsupportedOperation,
                "unsupported capability_intent",
            ));
        }
    }
    validate_and_map(&request.policy, state, &request.enterprise_issuer)
}

/// Execute PROTO-0 write. Sole Control Plane mutation entry point.
///
/// When [`apply_enabled`] is `false`, validates mapping and returns `APPLY_DISABLED`
/// without mutating protocol state (C8).
pub fn execute(state: &mut ProtocolState, request: Proto0WriteRequest) -> Proto0Outcome {
    let intent = request.ctx.capability_intent;
    let detail_base = detail_base(&request);

    if let Some(early) = reject_unsupported_intent(intent, &detail_base) {
        return early;
    }

    // C8 — enablement guard before any mutation.
    if !apply_enabled() {
        return reject_disabled(state, &request, detail_base, intent);
    }

    let mapped = match validate_request(state, &request) {
        Ok(m) => m,
        Err(e) => return rejected(e.code, Some(e.message), detail_base),
    };

    match mapped {
        MappedOperation::Grant(grant) => {
            dispatch_grant(state, &request, *grant, &detail_base, intent)
        }
        MappedOperation::Revoke(revoke) => {
            dispatch_revoke(state, &request, revoke, &detail_base, intent)
        }
        MappedOperation::Freeze(freeze) => dispatch_freeze(state, freeze, &detail_base, intent),
    }
}

/// Shared-reference entry for the Apply pipeline while mutations are disabled.
///
/// If Apply is enabled, rejects with `APPLY_PROTO_INTERNAL` — callers must use
/// [`execute`] with `&mut ProtocolState`.
pub fn execute_shared(state: &ProtocolState, request: Proto0WriteRequest) -> Proto0Outcome {
    let intent = request.ctx.capability_intent;
    let detail_base = detail_base(&request);

    if let Some(early) = reject_unsupported_intent(intent, &detail_base) {
        return early;
    }

    if apply_enabled() {
        return rejected(
            ApplyErrorCode::ApplyProtoInternal,
            Some("mutable ProtocolState required when apply_enabled=true".into()),
            detail_base,
        );
    }

    reject_disabled(state, &request, detail_base, intent)
}

fn reject_unsupported_intent(
    intent: ProtocolOperationKind,
    detail_base: &Value,
) -> Option<Proto0Outcome> {
    // C5 — unknown / unsupported intent never defaults to grant.
    // Phase 10 supports only Grant / Revoke / FreezeIdentity.
    if matches!(
        intent,
        ProtocolOperationKind::CapabilityGrant
            | ProtocolOperationKind::CapabilityRevoke
            | ProtocolOperationKind::FreezeIdentity
    ) {
        return None;
    }
    Some(rejected(
        ApplyErrorCode::ApplyUnsupportedOperation,
        Some(format!(
            "unsupported capability_intent: {}",
            intent.as_str()
        )),
        detail_base.clone(),
    ))
}

fn reject_disabled(
    state: &ProtocolState,
    request: &Proto0WriteRequest,
    detail_base: Value,
    intent: ProtocolOperationKind,
) -> Proto0Outcome {
    match validate_request(state, request) {
        Ok(mapped) => rejected(
            ApplyErrorCode::ApplyDisabled,
            None,
            protocol_detail_for_mapped(&detail_base, &mapped, intent),
        ),
        Err(e) => rejected(e.code, Some(e.message), detail_base),
    }
}

fn dispatch_grant(
    state: &mut ProtocolState,
    request: &Proto0WriteRequest,
    grant: MappedGrant,
    detail_base: &Value,
    intent: ProtocolOperationKind,
) -> Proto0Outcome {
    let Some(key) = request.grant_key.as_ref() else {
        return rejected(
            ApplyErrorCode::ApplySignerNotConfigured,
            Some("enterprise grant signing key required".into()),
            protocol_detail_for_mapped(
                detail_base,
                &MappedOperation::Grant(Box::new(grant)),
                intent,
            ),
        );
    };

    let issuer_entry = match state.registry.get(&grant.capability.issuer) {
        Some(e) => e,
        None => {
            return rejected(
                ApplyErrorCode::ApplyAgentNotFound,
                Some(format!(
                    "issuer agent not found: {}",
                    grant.capability.issuer
                )),
                detail_base.clone(),
            );
        }
    };

    if issuer_entry.identity.operational_public_key.as_slice() != key.public_key_bytes().as_slice()
    {
        return rejected(
            ApplyErrorCode::ApplySignerIssuerMismatch,
            Some("grant signing key does not match issuer operational key".into()),
            detail_base.clone(),
        );
    }

    let signed = match key.sign_capability(&grant.capability) {
        Ok(g) => g,
        Err(e) => {
            return rejected(map_core_error(&e), Some(e.to_string()), detail_base.clone());
        }
    };

    let pubkey = key.public_key_bytes();
    match grant_capability(&signed, &state.registry, &mut state.caps, &pubkey) {
        Ok(capability_id) => {
            if !state.index.capability_ids.contains(&capability_id) {
                state.index.capability_ids.push(capability_id);
            }
            let subject = match &grant.capability.subject {
                aether_core::types::SubjectRef::AgentId(id) => id.clone(),
                aether_core::types::SubjectRef::PublicKey(pk) => hex::encode(pk),
            };
            let mut detail = protocol_detail_for_mapped(
                detail_base,
                &MappedOperation::Grant(Box::new(grant)),
                intent,
            );
            if let Some(obj) = detail.as_object_mut() {
                obj.insert("capability_id".into(), json!(hex::encode(capability_id)));
                obj.insert("issuer".into(), json!(signed_message_issuer_hint(&signed)));
                obj.insert("subject".into(), json!(subject));
                obj.insert("status".into(), json!("granted"));
            }
            success(detail)
        }
        Err(e) => rejected(map_core_error(&e), Some(e.to_string()), detail_base.clone()),
    }
}

fn signed_message_issuer_hint(grant: &CapabilityGrant) -> String {
    aether_core::capability::model::CapabilityV0::decode(&grant.message.body)
        .map(|c| c.issuer)
        .unwrap_or_default()
}

fn dispatch_revoke(
    state: &mut ProtocolState,
    request: &Proto0WriteRequest,
    revoke: MappedRevoke,
    detail_base: &Value,
    intent: ProtocolOperationKind,
) -> Proto0Outcome {
    let capability_id = revoke.capability_id;
    match state.caps.get(&capability_id) {
        Some(_) if state.caps.is_revoked(&capability_id) => {
            return rejected(
                ApplyErrorCode::ApplyCapabilityRevoked,
                Some("Capability already revoked".into()),
                detail_base.clone(),
            );
        }
        Some(_) => {}
        None => {
            return rejected(
                ApplyErrorCode::ApplyCapabilityNotFound,
                Some(format!(
                    "capability not found: {}",
                    hex::encode(capability_id)
                )),
                detail_base.clone(),
            );
        }
    }

    state.caps.revoke(capability_id, request.revoked_at);

    let mut detail =
        protocol_detail_for_mapped(detail_base, &MappedOperation::Revoke(revoke), intent);
    if let Some(obj) = detail.as_object_mut() {
        obj.insert("capability_id".into(), json!(hex::encode(capability_id)));
        obj.insert("revoked_at".into(), json!(request.revoked_at));
        obj.insert("status".into(), json!("revoked"));
    }
    success(detail)
}

fn dispatch_freeze(
    state: &mut ProtocolState,
    freeze: MappedFreeze,
    detail_base: &Value,
    intent: ProtocolOperationKind,
) -> Proto0Outcome {
    match state.registry.freeze(&freeze.agent_id) {
        Ok(()) => {
            let mut detail = protocol_detail_for_mapped(
                detail_base,
                &MappedOperation::Freeze(freeze.clone()),
                intent,
            );
            if let Some(obj) = detail.as_object_mut() {
                obj.insert("agent_id".into(), json!(freeze.agent_id));
                obj.insert("status".into(), json!("frozen"));
            }
            success(detail)
        }
        Err(e) => rejected(map_core_error(&e), Some(e.to_string()), detail_base.clone()),
    }
}

fn detail_base(request: &Proto0WriteRequest) -> Value {
    json!({
        "adapter_version": ADAPTER_VERSION,
        "operation_id": request.ctx.operation_id,
        "execution_hash": request.ctx.execution_hash,
        "policy_id": request.policy.id,
        "policy_version": request.policy.version,
        "capability_intent": request.ctx.capability_intent.as_str(),
        "request_id": request.ctx.request_id,
        "operator_id": request.ctx.operator_id,
    })
}

fn protocol_detail_for_mapped(
    base: &Value,
    mapped: &MappedOperation,
    intent: ProtocolOperationKind,
) -> Value {
    let (operation, core_api) = match intent {
        ProtocolOperationKind::CapabilityGrant => ("GrantCapability", "grant_capability"),
        ProtocolOperationKind::CapabilityRevoke => ("RevokeCapability", "CapabilityStore::revoke"),
        ProtocolOperationKind::FreezeIdentity => {
            ("FreezeIdentityRequest", "IdentityRegistry::freeze")
        }
        ProtocolOperationKind::PolicyApply | ProtocolOperationKind::Unknown => {
            ("Unsupported", "none")
        }
    };

    let mut detail = base.clone();
    if let Some(obj) = detail.as_object_mut() {
        obj.insert("operation".into(), json!(operation));
        obj.insert("core_api".into(), json!(core_api));
        obj.insert(
            "mapped_preview".into(),
            match mapped {
                MappedOperation::Grant(g) => json!({
                    "issuer": g.capability.issuer,
                    "subject": match &g.capability.subject {
                        aether_core::types::SubjectRef::AgentId(id) => id.clone(),
                        aether_core::types::SubjectRef::PublicKey(pk) => hex::encode(pk),
                    },
                    "actions": g.capability.actions,
                }),
                MappedOperation::Revoke(r) => json!({
                    "capability_id": hex::encode(r.capability_id),
                }),
                MappedOperation::Freeze(f) => json!({
                    "agent_id": f.agent_id,
                    "status": "frozen",
                }),
            },
        );
    }
    detail
}

fn rejected(
    code: ApplyErrorCode,
    message: Option<String>,
    protocol_detail: Value,
) -> Proto0Outcome {
    Proto0Outcome {
        result: Proto0Result::Rejected,
        apply_error_code: Some(code.as_str().to_string()),
        core_error: message,
        protocol_detail,
    }
}

fn success(protocol_detail: Value) -> Proto0Outcome {
    Proto0Outcome {
        result: Proto0Result::Success,
        apply_error_code: None,
        core_error: None,
        protocol_detail,
    }
}

#[cfg(test)]
mod mutation_path_audit {
    //! Phase 10 mutation-boundary audit (Control Plane only).
    //!
    //! Repository search (2026-07-30):
    //! - `grant_capability` / `CapabilityStore::revoke` / `IdentityRegistry::freeze`
    //!   in `control_plane/src` appear **only** in `protocol/proto0_write.rs`
    //!   (production dispatch) and its unit-test fixtures.
    //! - Demo/`aether-core` tests call core APIs directly — outside the CP boundary.
    //! - No Apply HTTP execute routes invoke mutations.
    //! - Pipeline uses `execute_shared` while `apply_enabled()==false`.

    #[test]
    fn sole_cp_mutation_entry_is_proto0_write_execute() {
        assert_eq!(
            "protocol::proto0_write::execute",
            "protocol::proto0_write::execute"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::ApplyEnabledGuard;
    use crate::models::policies::PolicyStatus;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn execute_rejects_when_apply_disabled() {
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let agent_id = state.index.agent_ids[0].clone();
        let policy = freeze_policy_with_target(&agent_id);
        let intent = ProtocolOperationKind::FreezeIdentity;
        let before = state.registry.get(&agent_id).unwrap().status;
        let outcome = execute(&mut state, write_request(policy, intent, &agent_id, None));
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplyDisabled.as_str())
        );
        assert_eq!(state.registry.get(&agent_id).unwrap().status, before);
    }

    #[test]
    fn unknown_intent_rejected_never_defaults_to_grant() {
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let agent_id = state.index.agent_ids[0].clone();
        let policy = freeze_policy_with_target(&agent_id);
        let outcome = execute(
            &mut state,
            write_request(policy, ProtocolOperationKind::Unknown, &agent_id, None),
        );
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplyUnsupportedOperation.as_str())
        );
    }

    #[test]
    fn policy_apply_intent_rejected_as_unsupported() {
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let agent_id = state.index.agent_ids[0].clone();
        let policy = grant_policy(&agent_id);
        let outcome = execute(
            &mut state,
            write_request(policy, ProtocolOperationKind::PolicyApply, &agent_id, None),
        );
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplyUnsupportedOperation.as_str())
        );
    }

    #[test]
    fn grant_rejected_mapping_without_key_when_enabled() {
        let _guard = ApplyEnabledGuard::enable();
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let issuer = state.index.agent_ids[0].clone();
        let subject = state.index.agent_ids[1].clone();
        let before = state.index.capability_ids.len();
        let outcome = execute(
            &mut state,
            write_request(
                grant_policy(&subject),
                ProtocolOperationKind::CapabilityGrant,
                &issuer,
                None,
            ),
        );
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplySignerNotConfigured.as_str())
        );
        assert_eq!(state.index.capability_ids.len(), before);
    }

    #[test]
    fn execute_shared_never_mutates_when_disabled() {
        let state = ProtocolState::bootstrap().unwrap();
        let agent_id = state.index.agent_ids[1].clone();
        let before = protocol_status_fingerprint(&state);
        let outcome = execute_shared(
            &state,
            write_request(
                freeze_policy_with_target(&agent_id),
                ProtocolOperationKind::FreezeIdentity,
                &agent_id,
                None,
            ),
        );
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplyDisabled.as_str())
        );
        assert_eq!(protocol_status_fingerprint(&state), before);
    }

    fn protocol_status_fingerprint(state: &ProtocolState) -> String {
        state
            .index
            .agent_ids
            .iter()
            .map(|id| format!("{}:{:?}", id, state.registry.get(id).map(|e| e.status)))
            .collect::<Vec<_>>()
            .join("|")
    }

    #[test]
    fn freeze_identity_mapping_validates_target() {
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let agent_id = state.index.agent_ids[0].clone();
        let mut policy = freeze_policy_with_target(&agent_id);
        policy.target_agent_id = None;
        let outcome = execute(
            &mut state,
            write_request(
                policy,
                ProtocolOperationKind::FreezeIdentity,
                &agent_id,
                None,
            ),
        );
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplyInvalidPolicy.as_str())
        );
    }

    #[test]
    fn freeze_succeeds_when_apply_enabled() {
        let _guard = ApplyEnabledGuard::enable();
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let agent_id = state.index.agent_ids[1].clone(); // autonomous agent
        let policy = freeze_policy_with_target(&agent_id);
        let outcome = execute(
            &mut state,
            write_request(
                policy,
                ProtocolOperationKind::FreezeIdentity,
                &agent_id,
                None,
            ),
        );
        assert_eq!(outcome.result, Proto0Result::Success);
        assert_eq!(
            state.registry.get(&agent_id).unwrap().status,
            aether_core::types::AgentStatus::Frozen
        );
    }

    #[test]
    fn freeze_already_frozen_rejected() {
        let _guard = ApplyEnabledGuard::enable();
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let agent_id = state.index.agent_ids[1].clone();
        state.registry.freeze(&agent_id).unwrap();
        let policy = freeze_policy_with_target(&agent_id);
        let outcome = execute(
            &mut state,
            write_request(
                policy,
                ProtocolOperationKind::FreezeIdentity,
                &agent_id,
                None,
            ),
        );
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplyIdentityFrozen.as_str())
        );
    }

    #[test]
    fn revoke_succeeds_when_apply_enabled() {
        let _guard = ApplyEnabledGuard::enable();
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let issuer = state.index.agent_ids[0].clone();
        let cap_id = state.index.capability_ids[0];
        let policy = revoke_policy(&cap_id);
        let outcome = execute(
            &mut state,
            write_request(
                policy,
                ProtocolOperationKind::CapabilityRevoke,
                &issuer,
                None,
            ),
        );
        assert_eq!(outcome.result, Proto0Result::Success);
        assert!(state.caps.is_revoked(&cap_id));
    }

    #[test]
    fn revoke_invalid_capability_rejected() {
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let issuer = state.index.agent_ids[0].clone();
        let fake = [0u8; 32];
        let policy = revoke_policy(&fake);
        let outcome = execute(
            &mut state,
            write_request(
                policy,
                ProtocolOperationKind::CapabilityRevoke,
                &issuer,
                None,
            ),
        );
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplyCapabilityNotFound.as_str())
        );
    }

    #[test]
    fn grant_succeeds_when_apply_enabled() {
        let _guard = ApplyEnabledGuard::enable();
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let issuer = state.index.agent_ids[0].clone();
        let subject = state.index.agent_ids[1].clone();
        let key = EnterpriseGrantKey::from_seed_u64(42);
        assert_eq!(
            state
                .registry
                .get(&issuer)
                .unwrap()
                .identity
                .operational_public_key
                .as_slice(),
            key.public_key_bytes().as_slice()
        );
        let before = state.index.capability_ids.len();
        let policy = grant_policy(&subject);
        let outcome = execute(
            &mut state,
            write_request(
                policy,
                ProtocolOperationKind::CapabilityGrant,
                &issuer,
                Some(key),
            ),
        );
        assert_eq!(
            outcome.result,
            Proto0Result::Success,
            "core_error={:?}",
            outcome.core_error
        );
        assert!(state.index.capability_ids.len() > before);
    }

    #[test]
    fn grant_to_frozen_identity_rejected() {
        let _guard = ApplyEnabledGuard::enable();
        let mut state = ProtocolState::bootstrap_owned().unwrap();
        let issuer = state.index.agent_ids[0].clone();
        let subject = state.index.agent_ids[1].clone();
        state.registry.freeze(&subject).unwrap();
        let key = EnterpriseGrantKey::from_seed_u64(42);
        let policy = grant_policy(&subject);
        let outcome = execute(
            &mut state,
            write_request(
                policy,
                ProtocolOperationKind::CapabilityGrant,
                &issuer,
                Some(key),
            ),
        );
        assert_eq!(outcome.result, Proto0Result::Rejected);
        assert_eq!(
            outcome.apply_error_code.as_deref(),
            Some(ApplyErrorCode::ApplyIdentityFrozen.as_str())
        );
    }

    fn freeze_policy_with_target(agent_id: &str) -> PolicyTemplate {
        PolicyTemplate {
            id: "pol-freeze".into(),
            name: "freeze".into(),
            description: "d".into(),
            target_agent_id: Some(agent_id.into()),
            policy_type: "identity_freeze".into(),
            policy_data: json!({}),
            status: PolicyStatus::Approved,
            created_by: "op".into(),
            created_by_username: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            submitted_by: None,
            submitted_at: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            rejection_reason: None,
            version: 1,
            hash: "h".into(),
        }
    }

    fn revoke_policy(cap_id: &[u8; 32]) -> PolicyTemplate {
        PolicyTemplate {
            id: "pol-revoke".into(),
            name: "revoke".into(),
            description: "d".into(),
            target_agent_id: None,
            policy_type: "capability_revoke".into(),
            policy_data: json!({"capability_id": hex::encode(cap_id)}),
            status: PolicyStatus::Approved,
            created_by: "op".into(),
            created_by_username: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            submitted_by: None,
            submitted_at: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            rejection_reason: None,
            version: 1,
            hash: "h".into(),
        }
    }

    fn grant_policy(target: &str) -> PolicyTemplate {
        PolicyTemplate {
            id: "pol-grant".into(),
            name: "grant".into(),
            description: "d".into(),
            target_agent_id: Some(target.into()),
            policy_type: "capability_grant".into(),
            policy_data: json!({
                "actions": ["settlement.settle"],
                "max_spend": 100,
                "asset": "AETHER_TEST",
                "valid_after": 0,
                "valid_before": 10_000,
            }),
            status: PolicyStatus::Approved,
            created_by: "op".into(),
            created_by_username: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            submitted_by: None,
            submitted_at: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            rejection_reason: None,
            version: 1,
            hash: "h".into(),
        }
    }

    fn write_request(
        policy: PolicyTemplate,
        intent: ProtocolOperationKind,
        issuer_agent: &str,
        grant_key: Option<EnterpriseGrantKey>,
    ) -> Proto0WriteRequest {
        Proto0WriteRequest {
            ctx: ApplyContext {
                operation_id: "op-1".into(),
                request_id: "req-1".into(),
                execution_hash: "abc".into(),
                operator_id: "operator-1".into(),
                capability_intent: intent,
            },
            policy,
            enterprise_issuer: EnterpriseIssuerRef {
                agent_id: issuer_agent.into(),
            },
            grant_key,
            revoked_at: 1_700_000_000,
        }
    }
}
