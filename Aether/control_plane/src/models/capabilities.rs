use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityView {
    pub capability_id: String,
    pub issuer: String,
    pub subject_agent_id: String,
    pub actions: Vec<String>,
    pub max_spend: Option<u64>,
    pub asset: Option<String>,
    pub valid_after: Option<u64>,
    pub valid_before: Option<u64>,
    pub delegation_depth: u32,
    pub revoked: bool,
    pub granted_under_root_version: u64,
}
