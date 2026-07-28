//! CapabilityGrant = SignedMessage<CapabilityV0>

use ed25519_dalek::SigningKey;

use crate::capability::model::{CapabilityRecord, CapabilityStore, CapabilityV0, MSG_CAPABILITY_GRANT};
use crate::capability::delegation::{is_subset_actions, narrows_constraints};
use crate::crypto::signing::{sign_body, DOMAIN_TAG};
use crate::crypto::verify::{verify_signed_message, SignedMessage};
use crate::error::{Error, Result};
use crate::identity::registry::IdentityRegistry;
use crate::permission::root::RootAuthorityCapabilityV0;

#[derive(Debug, Clone)]
pub struct CapabilityGrant {
    pub message: SignedMessage,
}

impl CapabilityGrant {
    pub fn sign(signing_key: &SigningKey, capability: &CapabilityV0) -> Result<Self> {
        let body = capability.encode()?;
        let (_digest, signature) = sign_body(
            signing_key,
            DOMAIN_TAG,
            capability.protocol_version,
            capability.schema_version,
            MSG_CAPABILITY_GRANT,
            &body,
        );
        Ok(Self {
            message: SignedMessage {
                protocol_version: capability.protocol_version,
                schema_version: capability.schema_version,
                message_type: MSG_CAPABILITY_GRANT.into(),
                body,
                signer_key_id: "operational:0".into(),
                signature: signature.to_bytes().to_vec(),
                domain_tag: DOMAIN_TAG.into(),
            },
        })
    }
}

/// Verify envelope first, then decode and check semantics against registry/root/parent.
pub fn verify_capability_grant(
    grant: &CapabilityGrant,
    registry: &IdentityRegistry,
    store: &CapabilityStore,
    expected_issuer_public_key: &[u8],
) -> Result<CapabilityV0> {
    // 1–2: reconstruct body bytes already in message; verify signature + context
    verify_signed_message(
        &grant.message,
        expected_issuer_public_key,
        MSG_CAPABILITY_GRANT,
        grant.message.protocol_version,
        grant.message.schema_version,
    )?;

    // 3: decode and validate semantics
    let capability = CapabilityV0::decode(&grant.message.body)?;
    if capability.encode()? != grant.message.body {
        return Err(Error::MalformedObject("non-canonical capability body"));
    }
    if capability.protocol_version != grant.message.protocol_version
        || capability.schema_version != grant.message.schema_version
    {
        return Err(Error::SigningContextMismatch);
    }

    let issuer_entry = registry
        .get(&capability.issuer)
        .ok_or(Error::IdentityNotFound)?;
    if issuer_entry.identity.operational_public_key != expected_issuer_public_key {
        return Err(Error::InvalidSignature);
    }

    validate_capability_semantics(&capability, &issuer_entry.root_authority, store)?;
    Ok(capability)
}

pub fn validate_capability_semantics(
    capability: &CapabilityV0,
    root_authority: &RootAuthorityCapabilityV0,
    store: &CapabilityStore,
) -> Result<()> {
    if capability.delegation_depth == 0 {
        if capability.parent_capability_id.is_some() {
            return Err(Error::MalformedObject("direct grant must omit parent"));
        }
        if !is_subset_actions(&capability.actions, &root_authority.actions) {
            return Err(Error::Escalation("actions vs root"));
        }
        narrows_constraints(&capability.constraints, &root_authority.constraints)?;
        if capability.delegation_depth > root_authority.max_delegation_depth {
            return Err(Error::ExcessiveDelegationDepth);
        }
        return Ok(());
    }

    let parent_id = capability
        .parent_capability_id
        .ok_or(Error::ParentMissing)?;
    let parent = store.get(&parent_id).ok_or(Error::ParentMissing)?;
    if store.is_revoked(&parent_id) {
        return Err(Error::CapabilityRevoked);
    }
    if capability.delegation_depth != parent.body.delegation_depth + 1 {
        return Err(Error::InvalidDelegationDepth);
    }
    if capability.delegation_depth > root_authority.max_delegation_depth {
        return Err(Error::ExcessiveDelegationDepth);
    }
    if !is_subset_actions(&capability.actions, &parent.body.actions) {
        return Err(Error::Escalation("actions vs parent"));
    }
    narrows_constraints(&capability.constraints, &parent.body.constraints)?;
    Ok(())
}

/// Verify grant and insert into store bound to the issuer's active root version.
pub fn grant_capability(
    grant: &CapabilityGrant,
    registry: &IdentityRegistry,
    store: &mut CapabilityStore,
    expected_issuer_public_key: &[u8],
) -> Result<[u8; 32]> {
    let capability = verify_capability_grant(grant, registry, store, expected_issuer_public_key)?;
    let issuer_entry = registry
        .get(&capability.issuer)
        .ok_or(Error::IdentityNotFound)?;
    let capability_id = capability.capability_id()?;
    store.insert(CapabilityRecord {
        body: capability,
        capability_id,
        granted_under_root_version: issuer_entry.permission_root_material.root_version,
    });
    Ok(capability_id)
}
