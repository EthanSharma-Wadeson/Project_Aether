//! In-memory settlement store — mutations only via transition functions.

use std::collections::HashMap;

use crate::settlement::binding::{SettlementAccountBindingV0, SettlementBindingV0};
use crate::settlement::model::{EconomicOutcome, SettlementStatus};

#[derive(Debug, Default)]
pub struct SettlementStore {
    pub(crate) accounts: HashMap<[u8; 32], SettlementAccountBindingV0>,
    pub(crate) settlements: HashMap<[u8; 32], SettlementBindingV0>,
    /// correlation_id → settlement binding_id
    pub(crate) by_correlation: HashMap<[u8; 32], [u8; 32]>,
    /// (escrow_id, outcome) → active settlement binding_id
    pub(crate) by_escrow_outcome: HashMap<([u8; 32], String), [u8; 32]>,
    /// external_settlement_ref → settlement binding_id
    pub(crate) by_external_ref: HashMap<String, [u8; 32]>,
}

impl SettlementStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_account(&self, binding_id: &[u8; 32]) -> Option<&SettlementAccountBindingV0> {
        self.accounts.get(binding_id)
    }

    pub fn get_settlement(&self, binding_id: &[u8; 32]) -> Option<&SettlementBindingV0> {
        self.settlements.get(binding_id)
    }

    pub fn get_by_correlation(&self, correlation_id: &[u8; 32]) -> Option<&SettlementBindingV0> {
        let id = self.by_correlation.get(correlation_id)?;
        self.settlements.get(id)
    }

    pub fn get_by_external_ref(&self, external_ref: &str) -> Option<&SettlementBindingV0> {
        let id = self.by_external_ref.get(external_ref)?;
        self.settlements.get(id)
    }

    pub fn active_for_escrow_outcome(
        &self,
        escrow_id: &[u8; 32],
        outcome: EconomicOutcome,
    ) -> Option<&SettlementBindingV0> {
        let key = (*escrow_id, outcome.as_str().to_string());
        let id = self.by_escrow_outcome.get(&key)?;
        self.settlements.get(id)
    }

    pub(crate) fn insert_account(&mut self, binding: SettlementAccountBindingV0) {
        self.accounts.insert(binding.binding_id, binding);
    }

    pub(crate) fn insert_settlement(&mut self, binding: SettlementBindingV0) {
        let binding_id = binding.binding_id;
        let corr = binding.correlation_id;
        let key = (
            binding.escrow_id,
            binding.economic_outcome.as_str().to_string(),
        );
        if let Some(ref ext) = binding.external_settlement_ref {
            self.by_external_ref.insert(ext.clone(), binding_id);
        }
        if binding.settlement_status.is_active()
            || matches!(
                binding.settlement_status,
                SettlementStatus::Finalized | SettlementStatus::Confirmed
            )
        {
            self.by_escrow_outcome.insert(key, binding_id);
        } else if matches!(
            binding.settlement_status,
            SettlementStatus::Failed | SettlementStatus::Cancelled
        ) {
            // Clear active index only if this binding owns it.
            if self.by_escrow_outcome.get(&key) == Some(&binding_id) {
                self.by_escrow_outcome.remove(&key);
            }
        }
        self.by_correlation.insert(corr, binding_id);
        self.settlements.insert(binding_id, binding);
    }

    pub(crate) fn update_settlement(&mut self, binding: SettlementBindingV0) {
        // Drop stale external ref index entry if changed.
        if let Some(prev) = self.settlements.get(&binding.binding_id) {
            if let Some(ref old_ext) = prev.external_settlement_ref {
                if Some(old_ext) != binding.external_settlement_ref.as_ref() {
                    self.by_external_ref.remove(old_ext);
                }
            }
        }
        self.insert_settlement(binding);
    }
}
