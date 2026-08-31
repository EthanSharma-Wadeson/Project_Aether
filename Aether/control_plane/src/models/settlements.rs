use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SettlementView {
    pub binding_id: String,
    pub escrow_id: String,
    pub agent_id: String,
    pub settlement_provider: String,
    pub external_settlement_ref: Option<String>,
    pub settlement_status: String,
    pub economic_outcome: String,
    pub amount: u64,
    pub asset: String,
    pub soft_finality: bool,
    pub hard_finality: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize)]
pub struct AccountBindingView {
    pub binding_id: String,
    pub agent_id: String,
    pub settlement_provider: String,
    pub external_account_ref: String,
    pub asset: String,
}
