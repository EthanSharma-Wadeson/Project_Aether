//! Demo harness — wires PROTO-0/2/4 without modifying protocol code.

use aether_core::capability::grant::{grant_capability, CapabilityGrant};
use aether_core::capability::model::{CapabilityStore, CapabilityV0};
use aether_core::error::Error;
use aether_core::identity::agent_identity::IdentityBundle;
use aether_core::identity::registry::IdentityRegistry;
use aether_core::permission::root::{Constraints, RootAuthorityCapabilityV0};
use aether_core::settlement::{
    bind_account, new_account_binding, sign_account_binding, MockSettlementAdapterV0,
    SettlementAccountBindingV0, SettlementStore, PROVIDER_ENTERPRISE_LEDGER_V0,
};
use aether_core::types::{ActionSelector, RateLimit, SubjectRef};
use aether_core::escrow::fee::BalanceLedger;
use aether_core::escrow::store::EscrowStore;
use ed25519_dalek::SigningKey;
use rand::rngs::StdRng;
use rand::SeedableRng;

pub const PROTOCOL_VERSION: u32 = 1;
pub const SCHEMA_VERSION: u32 = 1;

/// Enterprise treasury, autonomous agent, and service provider.
pub struct EnterpriseWorld {
    pub registry: IdentityRegistry,
    pub caps: CapabilityStore,
    pub escrows: EscrowStore,
    pub ledger: BalanceLedger,
    pub settlements: SettlementStore,
    pub adapter: MockSettlementAdapterV0,
    pub enterprise: IdentityBundle,
    pub agent: IdentityBundle,
    pub provider: IdentityBundle,
    pub enterprise_grant: CapabilityGrant,
    pub provider_grant: CapabilityGrant,
    pub agent_delegate_grant: CapabilityGrant,
    pub enterprise_account: SettlementAccountBindingV0,
    pub provider_account: SettlementAccountBindingV0,
}

pub struct DemoConfig {
    pub seed: u64,
    pub principal: u64,
    pub max_fee: u64,
    pub agent_max_spend: u64,
    pub fund_before: u64,
    pub receipt_before: u64,
    pub dispute_window: u64,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            principal: 1_000,
            max_fee: 10,
            agent_max_spend: 1_000,
            fund_before: 100,
            receipt_before: 500,
            dispute_window: 100,
        }
    }
}

pub fn escrow_actions() -> Vec<ActionSelector> {
    vec![
        "escrow.create".into(),
        "escrow.fund".into(),
        "escrow.submit_receipt".into(),
        "escrow.release".into(),
        "escrow.refund".into(),
        "escrow.dispute".into(),
        "escrow.resolve".into(),
        "escrow.cancel".into(),
    ]
}

pub fn settlement_actions() -> Vec<ActionSelector> {
    vec![
        "settlement.bind".into(),
        "settlement.settle".into(),
        "settlement.query".into(),
        "settlement.cancel".into(),
    ]
}

pub fn enterprise_root_actions() -> Vec<ActionSelector> {
    let mut a = escrow_actions();
    a.extend(settlement_actions());
    a
}

fn root_template(actions: Vec<ActionSelector>, max_spend: Option<u64>) -> RootAuthorityCapabilityV0 {
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
                max_ops: 100,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(10_000),
        },
        max_delegation_depth: 2,
    }
}

fn register_seeded(
    registry: &mut IdentityRegistry,
    actions: Vec<ActionSelector>,
    max_spend: Option<u64>,
    seed: u64,
) -> IdentityBundle {
    let mut rng = StdRng::seed_from_u64(seed);
    let signing_key = SigningKey::generate(&mut rng);
    let root = root_template(actions, max_spend);
    let bundle =
        IdentityBundle::create(signing_key, PROTOCOL_VERSION, SCHEMA_VERSION, root, 1, None)
            .expect("create identity");
    registry
        .register_bundle(&bundle, 0)
        .expect("register identity");
    bundle
}

fn direct_cap(
    issuer: &IdentityBundle,
    subject_id: &str,
    actions: Vec<ActionSelector>,
    max_spend: Option<u64>,
) -> CapabilityV0 {
    CapabilityV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        issuer: issuer.identity.derived_agent_id(),
        subject: SubjectRef::AgentId(subject_id.into()),
        actions,
        constraints: Constraints {
            max_spend,
            asset: Some("AETHER_TEST".into()),
            counterparties: None,
            rate_limit: Some(RateLimit {
                max_ops: 100,
                window_seconds: 60,
            }),
            valid_after: Some(0),
            valid_before: Some(10_000),
        },
        delegation_depth: 0,
        parent_capability_id: None,
    }
}

fn grant(
    bundle: &IdentityBundle,
    registry: &IdentityRegistry,
    store: &mut CapabilityStore,
    cap: &CapabilityV0,
) -> CapabilityGrant {
    let g = CapabilityGrant::sign(&bundle.signing_key, cap).expect("sign grant");
    grant_capability(
        &g,
        registry,
        store,
        bundle.identity.operational_public_key.as_slice(),
    )
    .expect("store grant");
    g
}

/// Bootstrap enterprise, agent, provider, capabilities, and settlement account bindings.
pub fn setup_world(config: &DemoConfig) -> EnterpriseWorld {
    let mut registry = IdentityRegistry::new();
    let enterprise = register_seeded(
        &mut registry,
        enterprise_root_actions(),
        Some(100_000),
        config.seed,
    );
    let agent = register_seeded(
        &mut registry,
        vec!["settlement.settle".into()],
        None,
        config.seed.wrapping_add(1),
    );
    let provider = register_seeded(
        &mut registry,
        {
            let mut a = escrow_actions();
            a.push("settlement.bind".into());
            a
        },
        Some(10_000),
        config.seed.wrapping_add(2),
    );

    let mut caps = CapabilityStore::new();
    let enterprise_cap = direct_cap(
        &enterprise,
        &enterprise.identity.derived_agent_id(),
        enterprise_root_actions(),
        Some(100_000),
    );
    let enterprise_grant = grant(&enterprise, &registry, &mut caps, &enterprise_cap);

    let provider_cap = direct_cap(
        &provider,
        &provider.identity.derived_agent_id(),
        {
            let mut a = escrow_actions();
            a.push("settlement.bind".into());
            a
        },
        Some(10_000),
    );
    let provider_grant = grant(&provider, &registry, &mut caps, &provider_cap);

    // Enterprise delegates bounded spend authority to autonomous agent.
    let agent_cap = direct_cap(
        &enterprise,
        &agent.identity.derived_agent_id(),
        vec!["settlement.settle".into()],
        Some(config.agent_max_spend),
    );
    let agent_delegate_grant = grant(&enterprise, &registry, &mut caps, &agent_cap);

    let mut ledger = BalanceLedger::new();
    ledger.fund_agent(&enterprise.identity.derived_agent_id(), 500_000);

    let mut settlements = SettlementStore::new();
    let now = 50u64;
    let ent_id = enterprise.identity.derived_agent_id();
    let prov_id = provider.identity.derived_agent_id();

    let ent_acct = new_account_binding(
        &ent_id,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        "enterprise-treasury-001",
        "AETHER_TEST",
        now,
        10_000,
    )
    .expect("enterprise account");
    let signed_ent = sign_account_binding(&ent_acct, &enterprise.signing_key, &ent_id).expect("sign");
    let enterprise_account = bind_account(
        &mut settlements,
        &registry,
        &caps,
        &signed_ent,
        Some(&enterprise_grant),
        now,
    )
    .expect("bind enterprise account");

    let prov_acct = new_account_binding(
        &prov_id,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        "provider-ledger-001",
        "AETHER_TEST",
        now,
        10_000,
    )
    .expect("provider account");
    let signed_prov =
        sign_account_binding(&prov_acct, &provider.signing_key, &prov_id).expect("sign");
    let provider_account = bind_account(
        &mut settlements,
        &registry,
        &caps,
        &signed_prov,
        Some(&provider_grant),
        now,
    )
    .expect("bind provider account");

    EnterpriseWorld {
        registry,
        caps,
        escrows: EscrowStore::new(),
        ledger,
        settlements,
        adapter: MockSettlementAdapterV0::new(),
        enterprise,
        agent,
        provider,
        enterprise_grant,
        provider_grant,
        agent_delegate_grant,
        enterprise_account,
        provider_account,
    }
}

/// Map protocol errors to the responsible layer for demo output.
pub fn protocol_layer(err: &Error) -> &'static str {
    match err {
        Error::InvalidPublicKey
        | Error::InvalidSignature
        | Error::SigningContextMismatch
        | Error::AgentIdMismatch
        | Error::IdentityNotFound
        | Error::IdentityAlreadyRegistered
        | Error::IdentityNotActive
        | Error::PermissionRootMismatch
        | Error::InvalidRootAuthority
        | Error::StaleRootVersion
        | Error::NonMonotonicRootVersion
        | Error::CapabilityNotFound
        | Error::CapabilityRevoked
        | Error::ParentMissing
        | Error::Escalation(_)
        | Error::ExcessiveDelegationDepth
        | Error::InvalidDelegationDepth
        | Error::UnexpectedAuthorisation
        | Error::Rejected(_)
        | Error::CapabilityDenied => "PROTO-0",
        Error::ChannelNotFound
        | Error::ChannelAlreadyExists
        | Error::InvalidChannelStatus
        | Error::UnilateralUpdate
        | Error::SequenceStale
        | Error::SequenceSkip
        | Error::StateCommitmentMismatch
        | Error::BalanceConservation
        | Error::ParticipantMismatch
        | Error::UnauthorizedTransition
        | Error::InvalidClose
        | Error::InvalidDisputeEvidence
        | Error::DisputeWindowOpen
        | Error::UntrustedTerminalOperation => "PROTO-1",
        Error::EscrowNotFound
        | Error::EscrowAlreadyExists
        | Error::InvalidEscrowStatus
        | Error::InsufficientBalance
        | Error::EscrowValueConservation
        | Error::InvalidReceipt
        | Error::ReceiptReplay
        | Error::InvalidEscrowTerms
        | Error::FeeBudgetExceeded
        | Error::InvalidEscrowEvidence => "PROTO-2",
        Error::AgentNotInDirectory
        | Error::SessionNotFound
        | Error::SessionAlreadyExists
        | Error::InvalidSessionStatus
        | Error::UnsupportedProtocolVersion
        | Error::MessageReplay
        | Error::DuplicateMessageId
        | Error::EnvelopeReceiverMismatch
        | Error::PayloadCommitmentMismatch
        | Error::SessionIdentityMismatch
        | Error::SessionExpired
        | Error::UnknownNetworkAgent => "PROTO-NET-0",
        Error::SettlementNotFound
        | Error::SettlementAlreadyExists
        | Error::InvalidSettlementStatus
        | Error::AccountBindingNotFound
        | Error::AccountBindingMismatch
        | Error::DuplicateSettlement
        | Error::InvalidSettlementEvidence
        | Error::SettlementProviderMismatch
        | Error::SettlementAmountMismatch
        | Error::AdapterFailure
        | Error::SettlementConflict => "PROTO-4",
        Error::MalformedCbor | Error::MalformedObject(_) => "PROTO (wire)",
    }
}
