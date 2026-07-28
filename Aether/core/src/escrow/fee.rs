//! Fee quotes, ledger entries, and simulated balance ledger.

use std::collections::HashMap;

use ciborium::value::Value;

use crate::cbor::{encode_value, map, text, u32_value, u64_value};
use crate::error::{Error, Result};
use crate::types::AgentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeQuoteV0 {
    pub protocol_version: u32,
    pub schema_version: u32,
    pub operation: String,
    pub asset: String,
    pub quoted_fee: u64,
    pub valid_after: u64,
    pub valid_before: u64,
}

impl FeeQuoteV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (text("protocol_version"), u32_value(self.protocol_version)),
            (text("schema_version"), u32_value(self.schema_version)),
            (text("operation"), text(self.operation.clone())),
            (text("asset"), text(self.asset.clone())),
            (text("quoted_fee"), u64_value(self.quoted_fee)),
            (text("valid_after"), u64_value(self.valid_after)),
            (text("valid_before"), u64_value(self.valid_before)),
        ])
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        encode_value(&self.to_cbor_value())
    }

    pub fn is_valid_at(&self, now: u64) -> bool {
        now >= self.valid_after && now <= self.valid_before
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeLedgerEntryV0 {
    pub escrow_id: [u8; 32],
    pub agent_id: AgentId,
    pub operation: String,
    pub amount: u64,
    pub logical_time: u64,
    pub remaining_budget: u64,
}

impl FeeLedgerEntryV0 {
    pub fn to_cbor_value(&self) -> Value {
        map(vec![
            (
                text("escrow_id"),
                crate::cbor::bytes(self.escrow_id.to_vec()),
            ),
            (text("agent_id"), text(self.agent_id.clone())),
            (text("operation"), text(self.operation.clone())),
            (text("amount"), u64_value(self.amount)),
            (text("logical_time"), u64_value(self.logical_time)),
            (text("remaining_budget"), u64_value(self.remaining_budget)),
        ])
    }
}

/// Simulated payer/provider balances and protocol treasury (abstract units).
#[derive(Debug, Default, Clone)]
pub struct BalanceLedger {
    balances: HashMap<AgentId, u64>,
    pub treasury: u64,
}

impl BalanceLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn credit(&mut self, agent: &str, amount: u64) -> Result<()> {
        let entry = self.balances.entry(agent.into()).or_insert(0);
        *entry = entry
            .checked_add(amount)
            .ok_or(Error::EscrowValueConservation)?;
        Ok(())
    }

    pub fn debit(&mut self, agent: &str, amount: u64) -> Result<()> {
        let balance = self.balance(agent);
        if balance < amount {
            return Err(Error::InsufficientBalance);
        }
        let entry = self
            .balances
            .get_mut(agent)
            .ok_or(Error::InsufficientBalance)?;
        *entry = entry
            .checked_sub(amount)
            .ok_or(Error::EscrowValueConservation)?;
        Ok(())
    }

    pub fn balance(&self, agent: &str) -> u64 {
        self.balances.get(agent).copied().unwrap_or(0)
    }

    pub fn fund_agent(&mut self, agent: &str, amount: u64) {
        let _ = self.credit(agent, amount);
    }

    pub fn consume_fee(&mut self, amount: u64) -> Result<()> {
        self.treasury = self
            .treasury
            .checked_add(amount)
            .ok_or(Error::EscrowValueConservation)?;
        Ok(())
    }
}

pub fn default_fee_quotes(
    protocol_version: u32,
    schema_version: u32,
    asset: &str,
    max_fee: u64,
    fund_before: u64,
) -> (FeeQuoteV0, FeeQuoteV0) {
    let fund_fee = max_fee / 2;
    let release_fee = max_fee - fund_fee;
    let fund_quote = FeeQuoteV0 {
        protocol_version,
        schema_version,
        operation: "escrow.fund".into(),
        asset: asset.into(),
        quoted_fee: fund_fee,
        valid_after: 0,
        valid_before: fund_before,
    };
    let release_quote = FeeQuoteV0 {
        protocol_version,
        schema_version,
        operation: "escrow.release".into(),
        asset: asset.into(),
        quoted_fee: release_fee,
        valid_after: 0,
        valid_before: u64::MAX,
    };
    (fund_quote, release_quote)
}

pub fn quote_for_operation<'a>(
    fund_quote: &'a FeeQuoteV0,
    release_quote: &'a FeeQuoteV0,
    operation: &str,
) -> Result<&'a FeeQuoteV0> {
    if operation == "escrow.fund" {
        Ok(fund_quote)
    } else if operation == "escrow.release" || operation == "escrow.resolve" {
        Ok(release_quote)
    } else {
        Err(Error::FeeBudgetExceeded)
    }
}
