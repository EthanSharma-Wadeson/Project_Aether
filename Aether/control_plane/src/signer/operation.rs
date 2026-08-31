//! Canonical governance operation — the unit of work to be signed.
//!
//! Does NOT mutate protocol or Control Plane policy state by itself.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::operators::OperatorRole;

/// Authorised governance intent awaiting cryptographic attestation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceOperation {
    pub operation_id: String,
    pub request_id: String,
    pub policy_id: Option<String>,
    pub policy_version: Option<i64>,
    pub operator_id: String,
    pub operator_role: String,
    pub action: String,
    pub target: Option<String>,
    pub payload_hash: String,
    pub created_at: DateTime<Utc>,
}

impl GovernanceOperation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: impl Into<String>,
        operator_id: impl Into<String>,
        operator_role: OperatorRole,
        action: impl Into<String>,
        payload: &[u8],
    ) -> Self {
        Self {
            operation_id: Uuid::new_v4().to_string(),
            request_id: request_id.into(),
            policy_id: None,
            policy_version: None,
            operator_id: operator_id.into(),
            operator_role: operator_role.as_str().into(),
            action: action.into(),
            target: None,
            payload_hash: hex::encode(Sha256::digest(payload)),
            created_at: Utc::now(),
        }
    }

    pub fn with_policy(mut self, policy_id: impl Into<String>, version: i64) -> Self {
        self.policy_id = Some(policy_id.into());
        self.policy_version = Some(version);
        self
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Stable canonical serialization for signing and hashing.
    ///
    /// Uses a fixed field order via `serde` struct serialization (not
    /// `HashMap`), so the byte sequence is deterministic for equal values.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&CanonicalSignBody {
            operation_id: &self.operation_id,
            request_id: &self.request_id,
            policy_id: self.policy_id.as_deref(),
            policy_version: self.policy_version,
            operator_id: &self.operator_id,
            operator_role: &self.operator_role,
            action: &self.action,
            target: self.target.as_deref(),
            payload_hash: &self.payload_hash,
            created_at: self.created_at.to_rfc3339(),
        })
    }

    /// Hash of the canonical operation body (distinct from `payload_hash`).
    pub fn operation_hash(&self) -> Result<String, serde_json::Error> {
        Ok(hex::encode(Sha256::digest(self.canonical_bytes()?)))
    }
}

/// Explicit field order for stable signing bytes.
#[derive(Serialize)]
struct CanonicalSignBody<'a> {
    operation_id: &'a str,
    request_id: &'a str,
    policy_id: Option<&'a str>,
    policy_version: Option<i64>,
    operator_id: &'a str,
    operator_role: &'a str,
    action: &'a str,
    target: Option<&'a str>,
    payload_hash: &'a str,
    created_at: String,
}

/// Result of a successful sign — never includes private key material.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedGovernanceOperation {
    pub operation: GovernanceOperation,
    pub signer_identity: String,
    /// Hex-encoded Ed25519 signature over `operation.canonical_bytes()`.
    pub signature: String,
    pub signed_at: DateTime<Utc>,
}
