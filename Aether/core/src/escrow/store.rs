//! In-memory escrow store — mutations only via transition functions.

use std::collections::HashMap;

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
