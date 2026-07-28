//! In-memory escrow store — mutations only via transition functions.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::escrow::fee::{FeeLedgerEntryV0, FeeQuoteV0};
use crate::escrow::receipt::SettlementReceiptV0;
use crate::escrow::state::EscrowV0;

#[derive(Debug, Default)]
pub struct EscrowStore {
    pub(crate) escrows: HashMap<[u8; 32], EscrowRecord>,
}

#[derive(Debug, Clone)]
pub struct EscrowRecord {
    pub escrow: EscrowV0,
    pub fund_quote: FeeQuoteV0,
    pub release_quote: FeeQuoteV0,
    pub fee_ledger: Vec<FeeLedgerEntryV0>,
    pub last_receipt_nonce: u64,
    pub bound_receipt: Option<SettlementReceiptV0>,
}

impl EscrowStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, escrow_id: &[u8; 32]) -> Option<&EscrowRecord> {
        self.escrows.get(escrow_id)
    }

    /// Internal mutation only — escrow state must change via transition functions.
    pub(crate) fn insert_record(&mut self, escrow_id: [u8; 32], record: EscrowRecord) {
        self.escrows.insert(escrow_id, record);
    }
}

/// PROTO-4: promote hard settlement after verified finalize only.
///
/// Crate-private — must not be callable by external crates. Only
/// `settlement::finalize_settlement` may set hard=`true`.
pub(crate) fn promote_verified_hard_settlement(
    store: &mut EscrowStore,
    escrow_id: &[u8; 32],
) -> Result<EscrowV0> {
    set_hard_settlement_flag_inner(store, escrow_id, true)
}

/// PROTO-4: clear hard settlement on dispute / provider conflict.
///
/// Crate-private — dispute and query conflict paths may clear the flag.
pub(crate) fn clear_hard_settlement_flag(
    store: &mut EscrowStore,
    escrow_id: &[u8; 32],
) -> Result<EscrowV0> {
    set_hard_settlement_flag_inner(store, escrow_id, false)
}

fn set_hard_settlement_flag_inner(
    store: &mut EscrowStore,
    escrow_id: &[u8; 32],
    hard: bool,
) -> Result<EscrowV0> {
    let mut record = store.get(escrow_id).ok_or(Error::EscrowNotFound)?.clone();
    if !record.escrow.status.is_terminal() {
        return Err(Error::InvalidEscrowStatus);
    }
    record.escrow.finality.hard_settlement_placeholder = hard;
    let escrow = record.escrow.clone();
    store.insert_record(*escrow_id, record);
    Ok(escrow)
}
