//! PROTO-4 settlement binding layer.
//!
//! Binds PROTO-2 terminal escrow outcomes to external settlement evidence via
//! a provider adapter. Aether escrow state remains authoritative; adapters are
//! evidence sources only. Mock `enterprise.ledger.v0` only — no live payment rails.

pub mod adapter;
pub mod binding;
pub mod model;
pub mod report;
pub mod store;
pub mod transition;
pub mod verify;

pub use adapter::{
    AdapterCapabilities, AdapterStatusResult, AdapterSubmitResult, MockSettlementAdapterV0,
    MockSubmitMode, SettlementAdapterV0,
};
pub use binding::{SettlementAccountBindingV0, SettlementBindingV0, SettlementIntentV0};
pub use model::{
    EconomicOutcome, SettlementStatus, ACTION_BIND, ACTION_CANCEL, ACTION_QUERY, ACTION_SETTLE,
    MSG_ACCOUNT_BIND, MSG_SETTLE_CANCEL, MSG_SETTLE_FINALIZE, MSG_SETTLE_QUERY, MSG_SETTLE_REQUEST,
    MSG_SETTLE_SUBMIT, PROVIDER_ENTERPRISE_LEDGER_V0, SETTLEMENT_PROTOCOL_VERSION,
    SETTLEMENT_SCHEMA_VERSION,
};
pub use report::{SettlementEvidencePackageV0, SettlementReportV0};
pub use store::SettlementStore;
pub use transition::{
    advance_mock_status, bind_account, cancel_settlement, finalize_settlement, intent_from_escrow,
    mark_disputed_external, new_account_binding, query_settlement, request_settlement,
    sign_account_binding, sign_settlement_binding, submit_settlement,
};
