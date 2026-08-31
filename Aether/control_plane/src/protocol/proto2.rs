use crate::error::{Error, Result};
use crate::models::escrows::EscrowView;
use crate::protocol::state::ProtocolState;

pub fn list_escrows(state: &ProtocolState) -> Vec<EscrowView> {
    state
        .index
        .escrow_ids
        .iter()
        .filter_map(|id| escrow_view(state, id).ok())
        .collect()
}

pub fn get_escrow(state: &ProtocolState, escrow_id: &[u8; 32]) -> Result<EscrowView> {
    escrow_view(state, escrow_id)
}

pub fn escrows_for_agent(state: &ProtocolState, agent_id: &str) -> Vec<EscrowView> {
    list_escrows(state)
        .into_iter()
        .filter(|e| e.payer == agent_id || e.provider == agent_id)
        .collect()
}

fn escrow_view(state: &ProtocolState, escrow_id: &[u8; 32]) -> Result<EscrowView> {
    let record = state
        .escrows
        .get(escrow_id)
        .ok_or_else(|| Error::NotFound("escrow".into()))?;
    let escrow = &record.escrow;
    Ok(EscrowView {
        escrow_id: hex::encode(escrow.escrow_id),
        status: escrow.status.as_str().into(),
        payer: escrow.terms.payer.clone(),
        provider: escrow.terms.provider.clone(),
        asset: escrow.terms.asset.clone(),
        principal_amount: escrow.terms.principal_amount,
        created_at: escrow.created_at,
        finalized_at: escrow.finalized_at,
        soft_finality: escrow.finality.finalized,
        hard_finality: escrow.finality.hard_settlement_placeholder,
        has_bound_receipt: record.bound_receipt.is_some(),
        dispute_state: if matches!(
            escrow.status,
            aether_core::escrow::state::EscrowStatus::Disputed
        ) {
            Some("disputed".into())
        } else {
            None
        },
    })
}
