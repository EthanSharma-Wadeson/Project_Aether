//! AgentIdentityV0 and related registration material.

use ciborium::value::Value;

use crate::cbor::{
    as_bytes, as_optional_bytes, as_u32, bytes, encode_value, map, map_get, optional_bytes, text,
    u32_value,
};
use crate::crypto::sha256;
use crate::crypto::signing::{sign_body, DOMAIN_TAG};
use crate::crypto::verify::{verify_signed_message, SignedMessage};
use crate::error::{Error, Result};
use crate::identity::agent_id::derive_agent_id;
use crate::permission::root::{PermissionRootV0, RootAuthorityCapabilityV0};
use crate::types::AgentId;
use ed25519_dalek::SigningKey;

pub const MSG_IDENTITY_REGISTER: &str = "identity.register";
pub const MSG_IDENTITY_UPDATE_ROOT: &str = "identity.update_root";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentityV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub operational_public_key: Vec<u8>,
    pub permission_root: [u8; 32],
    pub metadata_commitment: Option<Vec<u8>>,
}

impl AgentIdentityV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (
                text("operational_public_key"),
                bytes(self.operational_public_key.clone()),
            ),
            (
                text("permission_root"),
                bytes(self.permission_root.to_vec()),
            ),
            (
                text("metadata_commitment"),
                optional_bytes(&self.metadata_commitment),
            ),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn decode(bytes_in: &[u8]) -> Result<Self> {
        let value = crate::cbor::decode_value(bytes_in)?;
        let Value::Map(entries) = value else {
            return Err(Error::MalformedObject("identity must be map"));
        };
        let protocol_version = as_u32(map_get(&entries, "protocol_version")?)?;
        let schema_version = as_u32(map_get(&entries, "schema_version")?)?;
        let operational_public_key = as_bytes(map_get(&entries, "operational_public_key")?)?.to_vec();
        if operational_public_key.len() != 32 {
            return Err(Error::MalformedObject("operational_public_key length"));
        }
        let permission_root_bytes = as_bytes(map_get(&entries, "permission_root")?)?;
        let permission_root: [u8; 32] = permission_root_bytes
            .try_into()
            .map_err(|_| Error::MalformedObject("permission_root length"))?;
        let metadata_commitment = as_optional_bytes(map_get(&entries, "metadata_commitment")?)?;
        Ok(Self {
            protocol_version,
            schema_version,
            operational_public_key,
            permission_root,
            metadata_commitment,
        })
    }

    pub fn derived_agent_id(&self) -> AgentId {
        derive_agent_id(self.schema_version, &self.operational_public_key)
    }
}

/// Local helper bundling key material used to construct a registerable identity.
#[derive(Debug, Clone)]
pub struct IdentityBundle {
    pub signing_key: SigningKey,
    pub identity: AgentIdentityV0,
    pub permission_root: PermissionRootV0,
    pub root_authority: RootAuthorityCapabilityV0,
}

impl IdentityBundle {
    pub fn create(
        signing_key: SigningKey,
        protocol_version: u32,
        schema_version: u32,
        root_authority: RootAuthorityCapabilityV0,
        root_version: u64,
        metadata_commitment: Option<Vec<u8>>,
    ) -> Result<Self> {
        let agent_id = derive_agent_id(schema_version, signing_key.verifying_key().as_bytes());
        let mut root_authority = root_authority;
        root_authority.issuer = agent_id.clone();
        root_authority.subject = agent_id.clone();
        root_authority.protocol_version = protocol_version;
        root_authority.schema_version = schema_version;

        let permission_root = PermissionRootV0 {
            protocol_version,
            schema_version,
            agent_id,
            root_version,
            root_authority_capability_id: root_authority.capability_id(),
        };
        let commitment = permission_root.commitment()?;

        let identity = AgentIdentityV0 {
            protocol_version,
            schema_version,
            operational_public_key: signing_key.verifying_key().as_bytes().to_vec(),
            permission_root: commitment,
            metadata_commitment,
        };

        Ok(Self {
            signing_key,
            identity,
            permission_root,
            root_authority,
        })
    }

    pub fn sign_register(&self) -> Result<SignedMessage> {
        let body = self.identity.encode()?;
        let (_digest, signature) = sign_body(
            &self.signing_key,
            DOMAIN_TAG,
            self.identity.protocol_version,
            self.identity.schema_version,
            MSG_IDENTITY_REGISTER,
            &body,
        );
        Ok(SignedMessage {
            protocol_version: self.identity.protocol_version,
            schema_version: self.identity.schema_version,
            message_type: MSG_IDENTITY_REGISTER.into(),
            body,
            signer_key_id: "operational:0".into(),
            signature: signature.to_bytes().to_vec(),
            domain_tag: DOMAIN_TAG.into(),
        })
    }
}

pub fn verify_identity_register(message: &SignedMessage) -> Result<AgentIdentityV0> {
    let identity = AgentIdentityV0::decode(&message.body)?;
    verify_signed_message(
        message,
        &identity.operational_public_key,
        MSG_IDENTITY_REGISTER,
        identity.protocol_version,
        identity.schema_version,
    )?;
    // Re-encode check: body must be canonical for this object.
    if identity.encode()? != message.body {
        return Err(Error::MalformedObject("non-canonical identity body"));
    }
    Ok(identity)
}

pub fn hash_bytes(data: &[u8]) -> [u8; 32] {
    sha256(data)
}
