//! In-memory identity registry for PROTO-0.

use std::collections::HashMap;

use ed25519_dalek::SigningKey;

use crate::crypto::signing::{sign_body, DOMAIN_TAG};
use crate::crypto::verify::{verify_signed_message, SignedMessage};
use crate::error::{Error, Result};
use crate::identity::agent_identity::{
    verify_identity_register, AgentIdentityV0, IdentityBundle, MSG_IDENTITY_UPDATE_ROOT,
};
use crate::permission::root::{PermissionRootV0, RootAuthorityCapabilityV0};
use crate::types::{AgentId, AgentStatus};

#[derive(Debug, Clone)]
pub struct AgentRegistryEntryV0 {
    pub agent_id: AgentId,
    pub identity: AgentIdentityV0,
    pub permission_root_material: PermissionRootV0,
    pub root_authority: RootAuthorityCapabilityV0,
    pub status: AgentStatus,
    pub registered_at: u64,
}

#[derive(Debug, Default)]
pub struct IdentityRegistry {
    entries: HashMap<AgentId, AgentRegistryEntryV0>,
}

impl IdentityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, agent_id: &str) -> Option<&AgentRegistryEntryV0> {
        self.entries.get(agent_id)
    }

    pub fn get_mut(&mut self, agent_id: &str) -> Option<&mut AgentRegistryEntryV0> {
        self.entries.get_mut(agent_id)
    }

    /// Register from a verified identity register message + root material.
    pub fn register(
        &mut self,
        message: &SignedMessage,
        permission_root: PermissionRootV0,
        root_authority: RootAuthorityCapabilityV0,
        registered_at: u64,
    ) -> Result<AgentId> {
        let identity = verify_identity_register(message)?;
        let agent_id = identity.derived_agent_id();

        if self.entries.contains_key(&agent_id) {
            return Err(Error::IdentityAlreadyRegistered);
        }

        permission_root.verify_commitment(&identity.permission_root)?;
        if permission_root.agent_id != agent_id {
            return Err(Error::AgentIdMismatch);
        }
        permission_root.verify_against_authority(&root_authority)?;

        self.entries.insert(
            agent_id.clone(),
            AgentRegistryEntryV0 {
                agent_id: agent_id.clone(),
                identity,
                permission_root_material: permission_root,
                root_authority,
                status: AgentStatus::Active,
                registered_at,
            },
        );
        Ok(agent_id)
    }

    pub fn register_bundle(&mut self, bundle: &IdentityBundle, registered_at: u64) -> Result<AgentId> {
        let message = bundle.sign_register()?;
        self.register(
            &message,
            bundle.permission_root.clone(),
            bundle.root_authority.clone(),
            registered_at,
        )
    }

    pub fn freeze(&mut self, agent_id: &str) -> Result<()> {
        let entry = self.entries.get_mut(agent_id).ok_or(Error::IdentityNotFound)?;
        entry.status = AgentStatus::Frozen;
        Ok(())
    }

    pub fn revoke_identity(&mut self, agent_id: &str) -> Result<()> {
        let entry = self.entries.get_mut(agent_id).ok_or(Error::IdentityNotFound)?;
        entry.status = AgentStatus::Revoked;
        Ok(())
    }

    /// Authorised permission-root update. Requires monotonic root_version.
    pub fn update_permission_root(
        &mut self,
        agent_id: &str,
        signing_key: &SigningKey,
        new_root: PermissionRootV0,
        new_authority: RootAuthorityCapabilityV0,
    ) -> Result<()> {
        let entry = self.entries.get(agent_id).ok_or(Error::IdentityNotFound)?;
        if entry.status != AgentStatus::Active {
            return Err(Error::IdentityNotActive);
        }
        if new_root.agent_id != entry.agent_id {
            return Err(Error::AgentIdMismatch);
        }
        if new_root.root_version <= entry.permission_root_material.root_version {
            return Err(Error::NonMonotonicRootVersion);
        }
        new_root.verify_against_authority(&new_authority)?;
        let commitment = new_root.commitment()?;

        let mut updated_identity = entry.identity.clone();
        updated_identity.permission_root = commitment;

        let body = updated_identity.encode()?;
        let (_digest, signature) = sign_body(
            signing_key,
            DOMAIN_TAG,
            updated_identity.protocol_version,
            updated_identity.schema_version,
            MSG_IDENTITY_UPDATE_ROOT,
            &body,
        );
        let message = SignedMessage {
            protocol_version: updated_identity.protocol_version,
            schema_version: updated_identity.schema_version,
            message_type: MSG_IDENTITY_UPDATE_ROOT.into(),
            body,
            signer_key_id: "operational:0".into(),
            signature: signature.to_bytes().to_vec(),
            domain_tag: DOMAIN_TAG.into(),
        };
        verify_signed_message(
            &message,
            &updated_identity.operational_public_key,
            MSG_IDENTITY_UPDATE_ROOT,
            updated_identity.protocol_version,
            updated_identity.schema_version,
        )?;

        let entry = self.entries.get_mut(agent_id).ok_or(Error::IdentityNotFound)?;
        entry.identity = updated_identity;
        entry.permission_root_material = new_root;
        entry.root_authority = new_authority;
        Ok(())
    }
}
