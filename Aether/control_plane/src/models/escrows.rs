use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct EscrowView {
    pub escrow_id: String,
    pub status: String,
    pub payer: String,
    pub provider: String,
    pub asset: String,
    pub principal_amount: u64,
    pub created_at: u64,
    pub finalized_at: Option<u64>,
    pub soft_finality: bool,
    pub hard_finality: bool,
    pub has_bound_receipt: bool,
    pub dispute_state: Option<String>,
}
