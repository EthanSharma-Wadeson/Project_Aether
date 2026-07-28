//! Shared helpers for PROTO-0 acceptance tests.

#![allow(dead_code)]

use aether_core::capability::grant::{grant_capability, CapabilityGrant};
use aether_core::capability::model::{CapabilityStore, CapabilityV0};
use aether_core::identity::agent_identity::IdentityBundle;
use aether_core::identity::registry::IdentityRegistry;
use aether_core::permission::root::{Constraints, RootAuthorityCapabilityV0};
use aether_core::types::{ActionRequest, ActionSelector, RateLimit, SubjectRef};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::{RngCore, SeedableRng};

pub const PROTOCOL_VERSION: u32 = 1;
pub const SCHEMA_VERSION: u32 = 1;

pub fn root_authority_template(
    actions: Vec<ActionSelector>,
    max_spend: Option<u64>,
) -> RootAuthorityCapabilityV0 {
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

pub fn fresh_agent(
    actions: Vec<ActionSelector>,
    max_spend: Option<u64>,
) -> (IdentityBundle, IdentityRegistry) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let root = root_authority_template(actions, max_spend);
    let bundle =
        IdentityBundle::create(signing_key, PROTOCOL_VERSION, SCHEMA_VERSION, root, 1, None)
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
                max_ops: 100,
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

pub fn channel_actions() -> Vec<ActionSelector> {
    vec![
        "channel.open".into(),
        "channel.activate".into(),
        "channel.update".into(),
        "channel.close".into(),
        "channel.dispute".into(),
    ]
}

pub fn register_agent(
    registry: &mut IdentityRegistry,
    actions: Vec<ActionSelector>,
    max_spend: Option<u64>,
) -> IdentityBundle {
    register_agent_seeded(registry, actions, max_spend, OsRng.next_u64())
}

pub fn register_agent_seeded(
    registry: &mut IdentityRegistry,
    actions: Vec<ActionSelector>,
    max_spend: Option<u64>,
    seed: u64,
) -> IdentityBundle {
    let mut csprng = rand::rngs::StdRng::seed_from_u64(seed);
    let signing_key = SigningKey::generate(&mut csprng);
    let root = root_authority_template(actions, max_spend);
    let bundle =
        IdentityBundle::create(signing_key, PROTOCOL_VERSION, SCHEMA_VERSION, root, 1, None)
            .expect("create identity");
    registry
        .register_bundle(&bundle, 0)
        .expect("register identity");
    bundle
}

pub fn grant_channel_caps(
    bundle: &IdentityBundle,
    registry: &IdentityRegistry,
    store: &mut CapabilityStore,
) -> CapabilityGrant {
    let cap = direct_capability(bundle, channel_actions(), Some(1_000));
    grant_and_store(bundle, registry, store, &cap)
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

pub fn grant_escrow_caps(
    bundle: &IdentityBundle,
    registry: &IdentityRegistry,
    store: &mut CapabilityStore,
) -> CapabilityGrant {
    let cap = direct_capability(bundle, escrow_actions(), Some(10_000));
    grant_and_store(bundle, registry, store, &cap)
}

pub fn sample_terms(
    payer_id: &str,
    provider_id: &str,
    principal: u64,
    max_fee: u64,
    fund_before: u64,
    receipt_before: u64,
    dispute_window: u64,
) -> aether_core::escrow::terms::EscrowTermsV0 {
    aether_core::escrow::terms::EscrowTermsV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        payer: payer_id.into(),
        provider: provider_id.into(),
        asset: "AETHER_TEST".into(),
        principal_amount: principal,
        max_protocol_fee: max_fee,
        claim_type: "compute".into(),
        required_result_code: "ok".into(),
        accept_output_commitment: false,
        expected_output_commitment: None,
        fund_before,
        receipt_before,
        dispute_window,
        terms_version: 1,
    }
}

pub struct EscrowPair {
    pub registry: IdentityRegistry,
    pub caps: CapabilityStore,
    pub escrows: aether_core::escrow::store::EscrowStore,
    pub ledger: aether_core::escrow::fee::BalanceLedger,
    pub payer: IdentityBundle,
    pub provider: IdentityBundle,
    pub grant_payer: CapabilityGrant,
    pub grant_provider: CapabilityGrant,
}

pub fn setup_escrow_pair() -> EscrowPair {
    setup_escrow_pair_seeded(OsRng.next_u64())
}

pub fn setup_escrow_pair_seeded(seed: u64) -> EscrowPair {
    let mut registry = IdentityRegistry::new();
    let payer = register_agent_seeded(&mut registry, escrow_actions(), Some(10_000), seed);
    let provider = register_agent_seeded(
        &mut registry,
        escrow_actions(),
        Some(10_000),
        seed.wrapping_add(1),
    );
    let mut caps = CapabilityStore::new();
    let grant_payer = grant_escrow_caps(&payer, &registry, &mut caps);
    let grant_provider = grant_escrow_caps(&provider, &registry, &mut caps);
    let mut ledger = aether_core::escrow::fee::BalanceLedger::new();
    ledger.fund_agent(&payer.identity.derived_agent_id(), 100_000);
    EscrowPair {
        registry,
        caps,
        escrows: aether_core::escrow::store::EscrowStore::new(),
        ledger,
        payer,
        provider,
        grant_payer,
        grant_provider,
    }
}

pub fn sample_receipt(
    terms: &aether_core::escrow::terms::EscrowTermsV0,
    escrow_id: [u8; 32],
    nonce: u64,
    logical_time: u64,
) -> aether_core::escrow::receipt::SettlementReceiptV0 {
    let receipt = aether_core::escrow::receipt::SettlementReceiptV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        receipt_id: [0u8; 32],
        escrow_id,
        terms_version: terms.terms_version,
        payer: terms.payer.clone(),
        provider: terms.provider.clone(),
        claim_type: terms.claim_type.clone(),
        result_code: terms.required_result_code.clone(),
        output_commitment: terms.expected_output_commitment.clone(),
        claimed_amount: terms.principal_amount,
        logical_time,
        receipt_nonce: nonce,
    };
    receipt.with_computed_id().expect("receipt id")
}

pub fn create_escrow_ready(
    pair: &mut EscrowPair,
    principal: u64,
    max_fee: u64,
    fund_before: u64,
    receipt_before: u64,
    dispute_window: u64,
    now: u64,
) -> (aether_core::escrow::terms::EscrowTermsV0, [u8; 32]) {
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let terms = sample_terms(
        &payer_id,
        &provider_id,
        principal,
        max_fee,
        fund_before,
        receipt_before,
        dispute_window,
    );
    let escrow_id = terms.escrow_id().expect("escrow id");
    let dual = aether_core::escrow::transition::sign_terms_dual(
        &terms,
        &pair.payer.signing_key,
        &pair.provider.signing_key,
        &payer_id,
        &provider_id,
    )
    .expect("sign terms");
    aether_core::escrow::transition::create_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &dual,
        &pair.grant_payer,
        &pair.grant_provider,
        now,
    )
    .expect("create");
    (terms, escrow_id)
}

pub fn fund_escrow_ready(
    pair: &mut EscrowPair,
    escrow_id: [u8; 32],
    now: u64,
) -> aether_core::escrow::state::EscrowV0 {
    let record = pair.escrows.get(&escrow_id).expect("escrow");
    let fee_reservation = record.fund_quote.quoted_fee;
    let principal = record.escrow.terms.principal_amount;
    let payer_id = pair.payer.identity.derived_agent_id();
    let funding = aether_core::escrow::model::EscrowFundingV0 {
        escrow_id,
        payer: payer_id.clone(),
        amount: principal,
        fee_reservation,
        logical_time: now,
    };
    let signed = aether_core::escrow::transition::sign_fund(
        &funding,
        &pair.payer.signing_key,
        &payer_id,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
    )
    .expect("sign fund");
    aether_core::escrow::transition::fund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        now,
    )
    .expect("fund")
}

pub fn submit_receipt_ready(
    pair: &mut EscrowPair,
    terms: &aether_core::escrow::terms::EscrowTermsV0,
    escrow_id: [u8; 32],
    nonce: u64,
    now: u64,
) -> aether_core::escrow::state::EscrowV0 {
    let receipt = sample_receipt(terms, escrow_id, nonce, now);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = aether_core::escrow::transition::sign_receipt(
        &receipt,
        &pair.provider.signing_key,
        &provider_id,
    )
    .expect("sign receipt");
    aether_core::escrow::transition::submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        now,
    )
    .expect("submit receipt")
}

pub fn release_escrow_ready(
    pair: &mut EscrowPair,
    escrow_id: [u8; 32],
    now: u64,
) -> aether_core::escrow::state::EscrowV0 {
    let record = pair.escrows.get(&escrow_id).expect("escrow");
    let receipt_id = record.escrow.receipt_id.expect("receipt id");
    let payer_id = pair.payer.identity.derived_agent_id();
    let release = aether_core::escrow::model::EscrowReleaseV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        receipt_id,
        actor: payer_id.clone(),
        logical_time: now,
    };
    let signed = aether_core::escrow::transition::sign_release(&release, &pair.payer.signing_key)
        .expect("sign release");
    aether_core::escrow::transition::release_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        &payer_id,
        now,
    )
    .expect("release")
}

pub fn refund_escrow_ready(
    pair: &mut EscrowPair,
    escrow_id: [u8; 32],
    now: u64,
) -> aether_core::escrow::state::EscrowV0 {
    let payer_id = pair.payer.identity.derived_agent_id();
    let refund = aether_core::escrow::model::EscrowRefundV0 {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        escrow_id,
        actor: payer_id.clone(),
        reason: "cooperative".into(),
        logical_time: now,
    };
    let signed = aether_core::escrow::transition::sign_refund(&refund, &pair.payer.signing_key)
        .expect("sign refund");
    aether_core::escrow::transition::refund_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &mut pair.ledger,
        &signed,
        &pair.grant_payer,
        now,
    )
    .expect("refund")
}

pub fn happy_path_through_release(pair: &mut EscrowPair, principal: u64, max_fee: u64) -> [u8; 32] {
    let fund_before = 100;
    let receipt_before = 500;
    let dispute_window = 100;
    let (terms, escrow_id) = create_escrow_ready(
        pair,
        principal,
        max_fee,
        fund_before,
        receipt_before,
        dispute_window,
        10,
    );
    fund_escrow_ready(pair, escrow_id, 20);
    submit_receipt_ready(pair, &terms, escrow_id, 1, 30);
    let deadline = pair
        .escrows
        .get(&escrow_id)
        .unwrap()
        .escrow
        .finality
        .dispute_deadline
        .unwrap();
    release_escrow_ready(pair, escrow_id, deadline);
    escrow_id
}

pub fn happy_path_through_refund(pair: &mut EscrowPair, principal: u64, max_fee: u64) -> [u8; 32] {
    let fund_before = 100;
    let receipt_before = 500;
    let dispute_window = 100;
    let (_terms, escrow_id) = create_escrow_ready(
        pair,
        principal,
        max_fee,
        fund_before,
        receipt_before,
        dispute_window,
        10,
    );
    fund_escrow_ready(pair, escrow_id, 20);
    refund_escrow_ready(pair, escrow_id, 40);
    escrow_id
}

// --- PROTO-4 settlement helpers ---

pub fn settlement_actions() -> Vec<ActionSelector> {
    vec![
        "settlement.bind".into(),
        "settlement.settle".into(),
        "settlement.query".into(),
        "settlement.cancel".into(),
    ]
}

pub fn escrow_and_settlement_actions() -> Vec<ActionSelector> {
    let mut actions = escrow_actions();
    actions.extend(settlement_actions());
    actions
}

pub fn grant_settlement_caps(
    bundle: &IdentityBundle,
    registry: &IdentityRegistry,
    store: &mut CapabilityStore,
    max_spend: Option<u64>,
) -> CapabilityGrant {
    let cap = direct_capability(bundle, settlement_actions(), max_spend);
    grant_and_store(bundle, registry, store, &cap)
}

pub struct SettlementHarness {
    pub pair: EscrowPair,
    pub settlements: aether_core::settlement::SettlementStore,
    pub adapter: aether_core::settlement::MockSettlementAdapterV0,
    pub grant_settle_payer: CapabilityGrant,
    pub grant_settle_provider: CapabilityGrant,
    pub payer_account: aether_core::settlement::SettlementAccountBindingV0,
    pub provider_account: aether_core::settlement::SettlementAccountBindingV0,
}

pub fn setup_settlement_harness(seed: u64) -> SettlementHarness {
    use aether_core::settlement::{
        bind_account, new_account_binding, sign_account_binding, PROVIDER_ENTERPRISE_LEDGER_V0,
    };

    let mut registry = IdentityRegistry::new();
    let payer = register_agent_seeded(
        &mut registry,
        escrow_and_settlement_actions(),
        Some(10_000),
        seed,
    );
    let provider = register_agent_seeded(
        &mut registry,
        escrow_and_settlement_actions(),
        Some(10_000),
        seed.wrapping_add(1),
    );
    let mut caps = CapabilityStore::new();
    let grant_payer = grant_and_store(
        &payer,
        &registry,
        &mut caps,
        &direct_capability(&payer, escrow_and_settlement_actions(), Some(10_000)),
    );
    let grant_provider = grant_and_store(
        &provider,
        &registry,
        &mut caps,
        &direct_capability(&provider, escrow_and_settlement_actions(), Some(10_000)),
    );
    let grant_settle_payer = grant_payer.clone();
    let grant_settle_provider = grant_provider.clone();

    let mut ledger = aether_core::escrow::fee::BalanceLedger::new();
    ledger.fund_agent(&payer.identity.derived_agent_id(), 100_000);

    let pair = EscrowPair {
        registry,
        caps,
        escrows: aether_core::escrow::store::EscrowStore::new(),
        ledger,
        payer,
        provider,
        grant_payer,
        grant_provider,
    };

    let mut settlements = aether_core::settlement::SettlementStore::new();
    let now = 50u64;
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();

    let payer_acct = new_account_binding(
        &payer_id,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        "acct-payer-001",
        "AETHER_TEST",
        now,
        10_000,
    )
    .expect("payer acct");
    let signed_payer =
        sign_account_binding(&payer_acct, &pair.payer.signing_key, &payer_id).expect("sign");
    let payer_account = bind_account(
        &mut settlements,
        &pair.registry,
        &pair.caps,
        &signed_payer,
        Some(&grant_settle_payer),
        now,
    )
    .expect("bind payer");

    let provider_acct = new_account_binding(
        &provider_id,
        PROVIDER_ENTERPRISE_LEDGER_V0,
        "acct-provider-001",
        "AETHER_TEST",
        now,
        10_000,
    )
    .expect("provider acct");
    let signed_provider =
        sign_account_binding(&provider_acct, &pair.provider.signing_key, &provider_id)
            .expect("sign");
    let provider_account = bind_account(
        &mut settlements,
        &pair.registry,
        &pair.caps,
        &signed_provider,
        Some(&grant_settle_provider),
        now,
    )
    .expect("bind provider");

    SettlementHarness {
        pair,
        settlements,
        adapter: aether_core::settlement::MockSettlementAdapterV0::new(),
        grant_settle_payer,
        grant_settle_provider,
        payer_account,
        provider_account,
    }
}

pub fn request_release_settlement(
    h: &mut SettlementHarness,
    escrow_id: [u8; 32],
    now: u64,
) -> aether_core::settlement::SettlementBindingV0 {
    use aether_core::settlement::{intent_from_escrow, request_settlement, EconomicOutcome};

    let intent = intent_from_escrow(
        &h.pair.escrows,
        &escrow_id,
        EconomicOutcome::ReleaseToProvider,
        aether_core::settlement::PROVIDER_ENTERPRISE_LEDGER_V0,
        h.payer_account.binding_id,
        Some(h.provider_account.binding_id),
    )
    .expect("intent");
    let payer_id = h.pair.payer.identity.derived_agent_id();
    request_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &intent,
        &payer_id,
        &h.pair.payer.signing_key,
        Some(&h.grant_settle_payer),
        now,
    )
    .expect("request")
}

pub fn full_finalize_release(
    h: &mut SettlementHarness,
    escrow_id: [u8; 32],
    now: u64,
) -> aether_core::settlement::SettlementBindingV0 {
    use aether_core::settlement::{
        advance_mock_status, finalize_settlement, query_settlement, submit_settlement,
        SettlementStatus,
    };

    let binding = request_release_settlement(h, escrow_id, now);
    let payer_id = h.pair.payer.identity.derived_agent_id();
    let submitted = submit_settlement(
        &mut h.settlements,
        &h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &binding.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        now + 1,
    )
    .expect("submit");

    advance_mock_status(&mut h.adapter, &submitted, SettlementStatus::Accepted).expect("accept");
    let (accepted, _) = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &submitted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        now + 2,
    )
    .expect("query accept");

    advance_mock_status(&mut h.adapter, &accepted, SettlementStatus::Confirmed).expect("confirm");
    let (confirmed, report) = query_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &mut h.adapter,
        &accepted.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        now + 3,
    )
    .expect("query confirm");

    finalize_settlement(
        &mut h.settlements,
        &mut h.pair.escrows,
        &h.pair.registry,
        &h.pair.caps,
        &h.adapter,
        &confirmed.binding_id,
        &payer_id,
        Some(&h.grant_settle_payer),
        &report,
        now + 4,
    )
    .expect("finalize")
}

// --- PROTO-NET-0 helpers ---

pub struct NetworkPair {
    pub registry: IdentityRegistry,
    pub directory: aether_core::network::AgentDirectoryV0,
    pub store_a: aether_core::network::SessionStore,
    pub store_b: aether_core::network::SessionStore,
    pub a: IdentityBundle,
    pub b: IdentityBundle,
}

pub fn setup_network_pair() -> NetworkPair {
    setup_network_pair_seeded(42)
}

pub fn setup_network_pair_seeded(seed: u64) -> NetworkPair {
    let mut registry = IdentityRegistry::new();
    let a = register_agent_seeded(&mut registry, vec!["net.session".into()], None, seed);
    let b = register_agent_seeded(
        &mut registry,
        vec!["net.session".into()],
        None,
        seed.wrapping_add(1),
    );
    let mut directory = aether_core::network::AgentDirectoryV0::new();
    let a_id = a.identity.derived_agent_id();
    let b_id = b.identity.derived_agent_id();
    directory
        .register(&registry, &a_id, "sim://agent-a", 1)
        .expect("register a");
    directory
        .register(&registry, &b_id, "sim://agent-b", 1)
        .expect("register b");
    NetworkPair {
        registry,
        directory,
        store_a: aether_core::network::SessionStore::new(),
        store_b: aether_core::network::SessionStore::new(),
        a,
        b,
    }
}

/// Full hello handshake: A initiates, B accepts, A completes. Returns matching session ids.
pub fn establish_session(
    pair: &mut NetworkPair,
    nonce_a: u64,
    nonce_b: u64,
    now: u64,
    ttl: u64,
) -> ([u8; 32], [u8; 32]) {
    use aether_core::network::{
        accept_hello, complete_hello, create_hello, initiate_hello, sign_hello, MSG_NET_HELLO,
        MSG_NET_HELLO_ACCEPT,
    };

    let a_id = pair.a.identity.derived_agent_id();
    let b_id = pair.b.identity.derived_agent_id();

    let hello_a = create_hello(&a_id, now, nonce_a, None);
    let signed_hello =
        sign_hello(&hello_a, &pair.a.signing_key, MSG_NET_HELLO).expect("sign hello");
    initiate_hello(
        &mut pair.store_a,
        &pair.registry,
        &a_id,
        &b_id,
        &signed_hello,
        now,
        ttl,
    )
    .expect("initiate");

    let accept_b = create_hello(&b_id, now + 1, nonce_b, None);
    let signed_accept =
        sign_hello(&accept_b, &pair.b.signing_key, MSG_NET_HELLO_ACCEPT).expect("sign accept");
    let sess_b = accept_hello(
        &mut pair.store_b,
        &pair.registry,
        &b_id,
        &signed_hello,
        &signed_accept,
        now + 1,
        ttl,
    )
    .expect("accept");

    let sess_a = complete_hello(
        &mut pair.store_a,
        &pair.registry,
        &a_id,
        &b_id,
        nonce_a,
        &signed_accept,
        now + 2,
    )
    .expect("complete");

    assert_eq!(sess_a.session_id, sess_b.session_id);
    (sess_a.session_id, sess_b.session_id)
}
