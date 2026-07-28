//! `authorise_action` — local deterministic authority check.

use crate::capability::model::{CapabilityStore, CapabilityV0, MSG_CAPABILITY_GRANT};
use crate::capability::grant::{validate_capability_semantics, CapabilityGrant};
use crate::crypto::verify::{verify_signed_message, SignedMessage};
use crate::error::Error;
use crate::identity::registry::IdentityRegistry;
use crate::types::{ActionRequest, AgentStatus, Authorisation, RejectReason};

/// Verification pipeline (normative):
/// 1. Authenticate SignedMessage under the claimed issuer key
/// 2. Check acting identity status
/// 3. Validate permission_root / root_version binding
/// 4. Evaluate CapabilityV0 semantics
/// 5. Only then allow simulated economic authorisation
///
/// Structural decode of the body is allowed only to locate the claimed issuer key.
/// No authorisation decision is made until signature verification succeeds.
pub fn authorise_action(
    registry: &IdentityRegistry,
    store: &CapabilityStore,
    agent_id: &str,
    grant: Option<&CapabilityGrant>,
    request: &ActionRequest,
    now: u64,
) -> Authorisation {
    match authorise_action_inner(registry, store, agent_id, grant, request, now) {
        Ok(()) => Authorisation::Authorised,
        Err(reason) => Authorisation::Rejected(reason),
    }
}

fn authorise_action_inner(
    registry: &IdentityRegistry,
    store: &CapabilityStore,
    agent_id: &str,
    grant: Option<&CapabilityGrant>,
    request: &ActionRequest,
    now: u64,
) -> Result<(), RejectReason> {
    let entry = registry.get(agent_id).ok_or(RejectReason::AgentIdMismatch)?;

    match entry.status {
        AgentStatus::Frozen => return Err(RejectReason::IdentityFrozen),
        AgentStatus::Revoked => return Err(RejectReason::IdentityRevoked),
        AgentStatus::Active => {}
    }

    // P0-A11: identity alone is insufficient
    let grant = grant.ok_or(RejectReason::IdentityOnlyBypass)?;

    // Structural parse to discover claimed issuer (untrusted until verified).
    let capability =
        CapabilityV0::decode(&grant.message.body).map_err(|_| RejectReason::MalformedObject)?;

    let issuer_entry = registry
        .get(&capability.issuer)
        .ok_or(RejectReason::AgentIdMismatch)?;

    // Step 1: authenticate envelope under issuer operational key.
    verify_signed_message(
        &grant.message,
        &issuer_entry.identity.operational_public_key,
        MSG_CAPABILITY_GRANT,
        capability.protocol_version,
        capability.schema_version,
    )
    .map_err(map_verify_err)?;

    // Canonical re-encode check after authentication.
    if capability
        .encode()
        .map_err(|_| RejectReason::MalformedObject)?
        != grant.message.body
    {
        return Err(RejectReason::MalformedObject);
    }

    if !subject_is_agent(&capability, &entry.agent_id) {
        return Err(RejectReason::AgentIdMismatch);
    }

    let capability_id = capability
        .capability_id()
        .map_err(|_| RejectReason::MalformedObject)?;

    if store.is_revoked(&capability_id) {
        return Err(RejectReason::CapabilityRevoked);
    }
    if let Some(parent_id) = &capability.parent_capability_id {
        if store.is_revoked(parent_id) {
            return Err(RejectReason::CapabilityRevoked);
        }
    }

    if let Some(record) = store.get(&capability_id) {
        if record.granted_under_root_version != issuer_entry.permission_root_material.root_version {
            return Err(RejectReason::StaleRootVersion);
        }
    } else if capability.delegation_depth > 0 {
        let parent_id = capability
            .parent_capability_id
            .ok_or(RejectReason::ParentMissing)?;
        if store.get(&parent_id).is_none() {
            return Err(RejectReason::ParentMissing);
        }
    }

    // Step 3: permission root binding for issuer
    issuer_entry
        .permission_root_material
        .verify_commitment(&issuer_entry.identity.permission_root)
        .map_err(|_| RejectReason::PermissionRootMismatch)?;
    issuer_entry
        .permission_root_material
        .verify_against_authority(&issuer_entry.root_authority)
        .map_err(|e| match e {
            Error::AgentIdMismatch => RejectReason::AgentIdMismatch,
            Error::InvalidRootAuthority => RejectReason::InvalidRootAuthority,
            _ => RejectReason::PermissionRootMismatch,
        })?;

    // Step 4: capability semantics
    validate_capability_semantics(&capability, &issuer_entry.root_authority, store).map_err(
        |e| match e {
            Error::ParentMissing => RejectReason::ParentMissing,
            Error::CapabilityRevoked => RejectReason::CapabilityRevoked,
            Error::Escalation(_) => RejectReason::Escalation,
            Error::ExcessiveDelegationDepth => RejectReason::ExcessiveDelegationDepth,
            Error::InvalidDelegationDepth => RejectReason::InvalidDelegationDepth,
            Error::MalformedObject(_) => RejectReason::MalformedObject,
            _ => RejectReason::Escalation,
        },
    )?;

    if let Some(valid_after) = capability.constraints.valid_after {
        if now < valid_after {
            return Err(RejectReason::CapabilityNotYetValid);
        }
    }
    if let Some(valid_before) = capability.constraints.valid_before {
        if now >= valid_before {
            return Err(RejectReason::CapabilityExpired);
        }
    }

    if !capability.actions.iter().any(|a| a == &request.action) {
        return Err(RejectReason::ActionNotPermitted);
    }

    if let Some(spend) = request.spend {
        if let Some(max) = capability.constraints.max_spend {
            if spend > max {
                return Err(RejectReason::SpendExceeded);
            }
        }
    }

    if let Some(req_asset) = &request.asset {
        if let Some(allowed) = &capability.constraints.asset {
            if allowed != req_asset {
                return Err(RejectReason::AssetNotPermitted);
            }
        }
    }

    if let Some(cp) = &request.counterparty {
        if let Some(allowed) = &capability.constraints.counterparties {
            if !allowed.iter().any(|a| a == cp) {
                return Err(RejectReason::CounterpartyNotPermitted);
            }
        }
    }

    Ok(())
}

fn subject_is_agent(capability: &CapabilityV0, agent_id: &str) -> bool {
    matches!(&capability.subject, crate::types::SubjectRef::AgentId(id) if id == agent_id)
}

fn map_verify_err(err: Error) -> RejectReason {
    match err {
        Error::InvalidSignature | Error::InvalidPublicKey => RejectReason::InvalidSignature,
        Error::SigningContextMismatch => RejectReason::SigningContextMismatch,
        Error::MalformedCbor | Error::MalformedObject(_) => RejectReason::MalformedObject,
        _ => RejectReason::InvalidSignature,
    }
}

/// P0-A12 helper: secondary signature alone cannot authorise.
pub fn authorise_with_secondary_signature_only(
    _registry: &IdentityRegistry,
    _agent_id: &str,
    _secondary: &SignedMessage,
    _request: &ActionRequest,
) -> Authorisation {
    Authorisation::Rejected(RejectReason::MissingCapability)
}
