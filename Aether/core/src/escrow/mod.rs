//! PROTO-2 escrow simulator.
//!
//! Local deterministic receipt-based conditional settlement.
//! Separate ledger from PROTO-1 channels. No real money or settlement backends.

pub mod dispute;
pub mod fee;
pub mod model;
pub mod receipt;
pub mod state;
pub mod store;
pub mod terms;
pub mod transition;
pub mod verify;

pub use dispute::{raise_dispute, resolve_dispute, DisputeEvidenceV0};
pub use fee::{BalanceLedger, FeeLedgerEntryV0, FeeQuoteV0};
pub use model::{
    DualSignedCancel, DualSignedTerms, EscrowCancelV0, EscrowFundingV0, EscrowRefundV0,
    EscrowReleaseV0, ACTION_CANCEL, ACTION_CREATE, ACTION_DISPUTE, ACTION_FUND, ACTION_REFUND,
    ACTION_RELEASE, ACTION_RESOLVE, ACTION_SUBMIT_RECEIPT, MSG_ESCROW_CANCEL, MSG_ESCROW_CREATE,
    MSG_ESCROW_DISPUTE, MSG_ESCROW_FUND, MSG_ESCROW_REFUND, MSG_ESCROW_RELEASE, MSG_ESCROW_RESOLVE,
    MSG_ESCROW_SUBMIT_RECEIPT,
};
pub use receipt::{SettlementEvidenceV0, SettlementReceiptV0};
pub use state::{EconomicFinalityViewV0, EscrowOutcome, EscrowStatus, EscrowV0};
pub use store::{EscrowRecord, EscrowStore};
pub use terms::EscrowTermsV0;
pub use transition::{
    cancel_escrow, create_escrow, expire_for_refund, fund_escrow, refund_escrow, release_escrow,
    sign_cancel_dual, sign_fund, sign_receipt, sign_refund, sign_release, sign_terms_dual,
    submit_receipt,
};
