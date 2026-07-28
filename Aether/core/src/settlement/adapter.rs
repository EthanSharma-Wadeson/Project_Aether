//! Settlement adapter abstraction and deterministic mock provider.

use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::settlement::binding::SettlementBindingV0;
use crate::settlement::model::{SettlementStatus, PROVIDER_ENTERPRISE_LEDGER_V0};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterCapabilities {
    pub supports_hold: bool,
    pub supports_release: bool,
    pub supports_refund: bool,
    pub supports_cancel: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSubmitResult {
    pub external_settlement_ref: String,
    pub status: SettlementStatus,
    pub amount: u64,
    pub receipt_bytes: Vec<u8>,
    pub proof_token: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterStatusResult {
    pub external_settlement_ref: String,
    pub status: SettlementStatus,
    pub amount: u64,
    pub receipt_bytes: Vec<u8>,
    pub proof_token: Option<[u8; 32]>,
}

/// Provider-agnostic settlement boundary. Core validates results, not provider SDKs.
pub trait SettlementAdapterV0 {
    fn provider_id(&self) -> &str;
    fn capabilities(&self) -> AdapterCapabilities;

    fn validate_account(&self, account_ref: &str) -> Result<()>;

    fn submit(&mut self, binding: &SettlementBindingV0) -> Result<AdapterSubmitResult>;

    fn query(&self, external_ref: &str) -> Result<AdapterStatusResult>;

    fn cancel(&mut self, external_ref: &str) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MockSubmitMode {
    #[default]
    Success,
    Fail,
    PartialAmount(u64),
    Timeout,
}

/// Deterministic in-process `enterprise.ledger.v0` stub.
#[derive(Debug, Default)]
pub struct MockSettlementAdapterV0 {
    pub submit_mode: MockSubmitMode,
    pub reverse_on_query: bool,
    pub force_confirmed_without_proof: bool,
    pub corrupt_ref_on_query: bool,
    /// Count of successful debit/submit side effects.
    pub submit_success_count: u64,
    pub query_count: u64,
    pub cancel_count: u64,
    /// Tracks issued external refs for uniqueness.
    issued_refs: HashSet<String>,
    records: HashMap<String, AdapterStatusResult>,
}

impl MockSettlementAdapterV0 {
    pub fn new() -> Self {
        Self {
            submit_mode: MockSubmitMode::Success,
            ..Self::default()
        }
    }

    pub fn deterministic_ref(settlement_id: &[u8; 32]) -> String {
        format!("mock-ref-{}", hex::encode(settlement_id))
    }

    pub fn proof_for(settlement_id: &[u8; 32], external_ref: &str, amount: u64) -> [u8; 32] {
        let mut preimage = Vec::new();
        preimage.extend_from_slice(b"mock.proof.v0");
        preimage.extend_from_slice(settlement_id);
        preimage.extend_from_slice(external_ref.as_bytes());
        preimage.extend_from_slice(&amount.to_le_bytes());
        sha256(&preimage)
    }

    fn receipt_bytes(
        settlement_id: &[u8; 32],
        external_ref: &str,
        status: SettlementStatus,
        amount: u64,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(settlement_id);
        out.extend_from_slice(external_ref.as_bytes());
        out.extend_from_slice(status.as_str().as_bytes());
        out.extend_from_slice(&amount.to_le_bytes());
        out
    }
}

impl SettlementAdapterV0 for MockSettlementAdapterV0 {
    fn provider_id(&self) -> &str {
        PROVIDER_ENTERPRISE_LEDGER_V0
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            supports_hold: true,
            supports_release: true,
            supports_refund: true,
            supports_cancel: true,
        }
    }

    fn validate_account(&self, account_ref: &str) -> Result<()> {
        if account_ref.is_empty() {
            return Err(Error::MalformedObject("empty account ref"));
        }
        Ok(())
    }

    fn submit(&mut self, binding: &SettlementBindingV0) -> Result<AdapterSubmitResult> {
        match self.submit_mode {
            MockSubmitMode::Fail => return Err(Error::AdapterFailure),
            MockSubmitMode::Timeout => return Err(Error::AdapterFailure),
            MockSubmitMode::Success | MockSubmitMode::PartialAmount(_) => {}
        }

        let settlement_id = binding.settlement_id();
        let external_ref = Self::deterministic_ref(&settlement_id);
        if !self.issued_refs.insert(external_ref.clone()) {
            return Err(Error::SettlementConflict);
        }

        let amount = match self.submit_mode {
            MockSubmitMode::PartialAmount(a) => a,
            _ => binding.principal_amount,
        };

        let status = if matches!(self.submit_mode, MockSubmitMode::PartialAmount(_)) {
            SettlementStatus::Failed
        } else if self.force_confirmed_without_proof {
            SettlementStatus::Confirmed
        } else {
            SettlementStatus::Submitted
        };

        let proof = if self.force_confirmed_without_proof {
            None
        } else {
            Some(Self::proof_for(&settlement_id, &external_ref, amount))
        };

        let receipt_bytes = Self::receipt_bytes(&settlement_id, &external_ref, status, amount);
        let result = AdapterSubmitResult {
            external_settlement_ref: external_ref.clone(),
            status,
            amount,
            receipt_bytes: receipt_bytes.clone(),
            proof_token: proof,
        };

        self.records.insert(
            external_ref,
            AdapterStatusResult {
                external_settlement_ref: result.external_settlement_ref.clone(),
                status: result.status,
                amount: result.amount,
                receipt_bytes,
                proof_token: proof,
            },
        );
        self.submit_success_count = self.submit_success_count.saturating_add(1);
        Ok(result)
    }

    fn query(&self, external_ref: &str) -> Result<AdapterStatusResult> {
        // query_count is incremented via interior mutation pattern in transition by
        // calling mark_query; here we only read. Use a cell would be cleaner —
        // transition will call `note_query` via inherent method.
        let mut result = self
            .records
            .get(external_ref)
            .cloned()
            .ok_or(Error::SettlementNotFound)?;

        if self.corrupt_ref_on_query {
            return Err(Error::InvalidSettlementEvidence);
        }

        if self.reverse_on_query
            && matches!(
                result.status,
                SettlementStatus::Confirmed
                    | SettlementStatus::Finalized
                    | SettlementStatus::Accepted
            )
        {
            result.status = SettlementStatus::Failed;
            result.proof_token = None;
        } else if !self.force_confirmed_without_proof {
            // Advance Submitted → Accepted → Confirmed on successive logical queries
            // when stored as Submitted/Accepted. Transition layer controls advancement.
        }

        Ok(result)
    }

    fn cancel(&mut self, external_ref: &str) -> Result<()> {
        let rec = self
            .records
            .get_mut(external_ref)
            .ok_or(Error::SettlementNotFound)?;
        if !rec.status.allows_cancel() {
            return Err(Error::InvalidSettlementStatus);
        }
        rec.status = SettlementStatus::Cancelled;
        self.cancel_count = self.cancel_count.saturating_add(1);
        Ok(())
    }
}

impl MockSettlementAdapterV0 {
    pub fn note_query(&mut self) {
        self.query_count = self.query_count.saturating_add(1);
    }

    pub fn advance_record(
        &mut self,
        external_ref: &str,
        status: SettlementStatus,
        amount: u64,
        settlement_id: &[u8; 32],
    ) -> Result<AdapterStatusResult> {
        let rec = self
            .records
            .get_mut(external_ref)
            .ok_or(Error::SettlementNotFound)?;
        rec.status = status;
        rec.amount = amount;
        rec.proof_token = Some(Self::proof_for(settlement_id, external_ref, amount));
        rec.receipt_bytes = Self::receipt_bytes(settlement_id, external_ref, status, amount);
        Ok(rec.clone())
    }

    pub fn inject_fake_confirmed(&mut self, external_ref: &str, amount: u64) -> Result<()> {
        let rec = self
            .records
            .get_mut(external_ref)
            .ok_or(Error::SettlementNotFound)?;
        rec.status = SettlementStatus::Confirmed;
        rec.amount = amount;
        rec.proof_token = None; // missing proof = fake confirmation
        Ok(())
    }

    pub fn set_reversed(&mut self, external_ref: &str) -> Result<()> {
        let rec = self
            .records
            .get_mut(external_ref)
            .ok_or(Error::SettlementNotFound)?;
        rec.status = SettlementStatus::Failed;
        rec.proof_token = None;
        Ok(())
    }

    pub fn has_ref(&self, external_ref: &str) -> bool {
        self.issued_refs.contains(external_ref)
    }
}
