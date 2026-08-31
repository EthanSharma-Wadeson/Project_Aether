//! Apply signature domain types.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Apply signature TTL — `APPLY_SIGNATURE_TTL` (15 minutes).
pub const DEFAULT_SIGNATURE_TTL: Duration = Duration::minutes(15);

pub const APPLY_PAYLOAD_SCHEMA: &str = "aether.cp.apply_payload.v1";
pub const APPLY_SIGN_BODY_SCHEMA: &str = "aether.cp.apply_sign_body.v1";
pub const SIGNATURE_ALGORITHM: &str = "Ed25519";
pub const PURPOSE_APPLY: &str = "apply";

/// Signature record lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureStatus {
    Prepared,
    Valid,
    Expired,
}

impl SignatureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Valid => "valid",
            Self::Expired => "expired",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "prepared" => Some(Self::Prepared),
            "valid" => Some(Self::Valid),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

/// Cryptographically bound Apply authorisation object (`SignedOperationV1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedOperationV1 {
    pub operation_id: String,
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub operation_intent: String,
    pub signer_id: String,
    pub signer_role: String,
    /// Enterprise signer identity used in the sign body (not the operator).
    pub signer_identity: String,
    /// Exact RFC3339 timestamp embedded in the sign body.
    pub signed_at: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub signature_algorithm: String,
    /// Hex-encoded Ed25519 signature.
    pub signature: String,
    pub purpose: String,
    pub payload_hash: String,
    pub request_id: String,
    pub status: SignatureStatus,
}

/// Apply payload hashed before signing (`APPLY_PAYLOAD_SCHEMA`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyPayloadV1 {
    pub schema: String,
    pub purpose: String,
    pub operation_id: String,
    pub request_id: String,
    pub apply_approval_id: String,
    pub dry_run_id: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub execution_hash: String,
    pub capability_intent: String,
    pub target_agent: Option<String>,
    pub confirm: bool,
    pub issued_at: String,
    pub expires_at: String,
}

/// Bytes signed by Ed25519 (`APPLY_SIGN_BODY_SCHEMA`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplySignBodyV1 {
    pub schema: String,
    pub purpose: String,
    pub operation_id: String,
    pub request_id: String,
    pub payload_hash: String,
    pub signer_identity: String,
    pub signed_at: String,
}

/// Inputs to prepare an Apply signature.
#[derive(Debug, Clone)]
pub struct PrepareSignatureRequest {
    pub operation_id: Option<String>,
    pub approval_id: String,
    pub dry_run_id: String,
    pub execution_hash: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub operation_intent: String,
    pub request_id: String,
    pub signer_id: String,
    pub signer_role: String,
    pub target_agent: Option<String>,
    pub purpose: String,
    pub ttl: Option<Duration>,
}
