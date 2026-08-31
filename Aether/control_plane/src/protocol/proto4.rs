use crate::error::{Error, Result};
use crate::models::settlements::{AccountBindingView, SettlementView};
use crate::protocol::state::ProtocolState;

pub fn list_settlements(state: &ProtocolState) -> Vec<SettlementView> {
    state
        .index
        .settlement_binding_ids
        .iter()
        .filter_map(|id| settlement_view(state, id).ok())
        .collect()
}

pub fn get_settlement(state: &ProtocolState, binding_id: &[u8; 32]) -> Result<SettlementView> {
    settlement_view(state, binding_id)
}

pub fn settlements_for_agent(state: &ProtocolState, agent_id: &str) -> Vec<SettlementView> {
    list_settlements(state)
        .into_iter()
        .filter(|s| {
            state
                .settlements
                .get_settlement(
                    &hex::decode(&s.binding_id)
                        .ok()
                        .and_then(|b| b.try_into().ok())
                        .unwrap_or([0u8; 32]),
                )
                .map(|b| b.payer_agent_id == agent_id || b.provider_agent_id == agent_id)
                .unwrap_or(false)
        })
        .collect()
}

pub fn list_account_bindings(state: &ProtocolState) -> Vec<AccountBindingView> {
    state
        .index
        .account_binding_ids
        .iter()
        .filter_map(|id| {
            state
                .settlements
                .get_account(id)
                .map(|b| AccountBindingView {
                    binding_id: hex::encode(b.binding_id),
                    agent_id: b.agent_id.clone(),
                    settlement_provider: b.settlement_provider.clone(),
                    external_account_ref: b.external_account_ref.clone(),
                    asset: b.asset.clone(),
                })
        })
        .collect()
}

fn settlement_view(state: &ProtocolState, binding_id: &[u8; 32]) -> Result<SettlementView> {
    let binding = state
        .settlements
        .get_settlement(binding_id)
        .ok_or_else(|| Error::NotFound("settlement binding".into()))?;

    let escrow = state
        .escrows
        .get(&binding.escrow_id)
        .ok_or_else(|| Error::NotFound("escrow for settlement".into()))?;

    Ok(SettlementView {
        binding_id: hex::encode(binding.binding_id),
        escrow_id: hex::encode(binding.escrow_id),
        agent_id: binding.payer_agent_id.clone(),
        settlement_provider: binding.settlement_provider.clone(),
        external_settlement_ref: binding.external_settlement_ref.clone(),
        settlement_status: binding.settlement_status.as_str().into(),
        economic_outcome: binding.economic_outcome.as_str().into(),
        amount: binding.principal_amount,
        asset: binding.asset.clone(),
        soft_finality: escrow.escrow.finality.finalized,
        hard_finality: escrow.escrow.finality.hard_settlement_placeholder,
        created_at: binding.requested_at,
        updated_at: binding
            .finalized_at
            .or(binding.confirmed_at)
            .or(binding.accepted_at)
            .or(binding.submitted_at)
            .unwrap_or(binding.requested_at),
    })
}
