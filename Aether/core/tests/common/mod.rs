//! Shared helpers for PROTO-0 acceptance tests.

#![allow(dead_code)]

use aether_core::capability::model::{CapabilityStore, CapabilityV0};
use aether_core::capability::grant::{grant_capability, CapabilityGrant};
use aether_core::identity::agent_identity::IdentityBundle;
use aether_core::identity::registry::IdentityRegistry;
use aether_core::permission::root::{Constraints, RootAuthorityCapabilityV0};
use aether_core::types::{ActionRequest, ActionSelector, RateLimit, SubjectRef};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

pub const PROTOCOL_VERSION: u32 = 1;
pub const SCHEMA_VERSION: u32 = 1;

pub fn root_authority_template(actions: Vec<ActionSelector>, max_spend: Option<u64>) -> RootAuthorityCapabilityV0 {
    RootAuthorityCapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: String::new(),
        subject: String::new(),
        actions,
        constraints: Constraints {
            max_spend,
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 10,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(10_000),
        },
        max_delegation_depth: 2,
    }
}

pub fn fresh_agent(actions: Vec<ActionSelector>, max_spend: Option<u64>) -> (IdentityBundle, IdentityRegistry) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let root = root_authority_template(actions, max_spend);
    let bundle = IdentityBundle::create(
        signing_key,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
        root,
        1,
        None,
    )
    .expect("create identity");
    let mut registry = IdentityRegistry::new();
    registry
        .register_bundle(&bundle, 0)
        .expect("register identity");
    (bundle, registry)
}

pub fn direct_capability(
    bundle: &IdentityBundle,
    actions: Vec<ActionSelector>,
    max_spend: Option<u64>,
) -> CapabilityV0 {
    CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: bundle.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(bundle.identity.derived_agent_id()),
        actions,
        constraints: Constraints {
            max_spend,
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 5,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(5_000),
        },
        delegation_depth: 0,
        parent_capability_id: None,
    }
}

pub fn grant_and_store(
    bundle: &IdentityBundle,
    registry: &IdentityRegistry,
    store: &mut CapabilityStore,
    capability: &CapabilityV0,
) -> CapabilityGrant {
    let grant = CapabilityGrant::sign(&bundle.signing_key, capability).expect("sign grant");
    grant_capability(
        &grant,
        registry,
        store,
        bundle.identity.operational_public_key.as_slice(),
    )
    .expect("store grant");
    grant
}

pub fn action(action: &str, spend: Option<u64>) -> ActionRequest {
    ActionRequest {
        action: action.into(),
        spend,
        asset: Some("AETHER_TEST".into()),
        counterparty: None,
    }
}
