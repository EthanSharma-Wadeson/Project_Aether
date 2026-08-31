//! Apply error codes and PROTO-0 → Apply error mapping.
//!
//! See APPLY_PROTO_ADAPTER_SPEC.md §9.

use aether_core::error::Error as CoreError;
use aether_core::types::RejectReason;

/// Stable Apply adapter / pipeline error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApplyErrorCode {
    ApplyDisabled,
    /// Phase 8+ execution pipeline blocked before PROTO-0 mutation.
    ApplyExecutionDisabled,
    ApplyInvalidPolicy,
    ApplyUnsupportedOperation,
    AdapterNotImplemented,
    ApplyAgentNotFound,
    ApplyAgentNotActive,
    ApplyIdentityFrozen,
    ApplyIdentityRevoked,
    ApplyCapabilityNotFound,
    ApplyCapabilityRevoked,
    ApplyCapabilityDenied,
    ApplyCapabilityExpired,
    ApplyCapabilityNotYetValid,
    ApplyConstraintViolation,
    ApplyPermissionDenied,
    ApplyInvalidGrantSignature,
    ApplyProtoRejected,
    ApplyProtoConflict,
    ApplyProtoUnsupported,
    ApplyProtoInternal,
    ApplySignerNotConfigured,
    ApplySignerIssuerMismatch,
}

impl ApplyErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApplyDisabled => "APPLY_DISABLED",
            Self::ApplyExecutionDisabled => "APPLY_EXECUTION_DISABLED",
            Self::ApplyInvalidPolicy => "APPLY_INVALID_POLICY",
            Self::ApplyUnsupportedOperation => "APPLY_UNSUPPORTED_OPERATION",
            Self::AdapterNotImplemented => "ADAPTER_NOT_IMPLEMENTED",
            Self::ApplyAgentNotFound => "APPLY_AGENT_NOT_FOUND",
            Self::ApplyAgentNotActive => "APPLY_AGENT_NOT_ACTIVE",
            Self::ApplyIdentityFrozen => "APPLY_IDENTITY_FROZEN",
            Self::ApplyIdentityRevoked => "APPLY_IDENTITY_REVOKED",
            Self::ApplyCapabilityNotFound => "APPLY_CAPABILITY_NOT_FOUND",
            Self::ApplyCapabilityRevoked => "APPLY_CAPABILITY_REVOKED",
            Self::ApplyCapabilityDenied => "APPLY_CAPABILITY_DENIED",
            Self::ApplyCapabilityExpired => "APPLY_CAPABILITY_EXPIRED",
            Self::ApplyCapabilityNotYetValid => "APPLY_CAPABILITY_NOT_YET_VALID",
            Self::ApplyConstraintViolation => "APPLY_CONSTRAINT_VIOLATION",
            Self::ApplyPermissionDenied => "APPLY_PERMISSION_DENIED",
            Self::ApplyInvalidGrantSignature => "APPLY_INVALID_GRANT_SIGNATURE",
            Self::ApplyProtoRejected => "APPLY_PROTO_REJECTED",
            Self::ApplyProtoConflict => "APPLY_PROTO_CONFLICT",
            Self::ApplyProtoUnsupported => "APPLY_PROTO_UNSUPPORTED",
            Self::ApplyProtoInternal => "APPLY_PROTO_INTERNAL",
            Self::ApplySignerNotConfigured => "APPLY_SIGNER_NOT_CONFIGURED",
            Self::ApplySignerIssuerMismatch => "APPLY_SIGNER_ISSUER_MISMATCH",
        }
    }
}

/// Map `aether_core::error::Error` to a stable Apply code.
pub fn map_core_error(err: &CoreError) -> ApplyErrorCode {
    match err {
        CoreError::IdentityNotFound => ApplyErrorCode::ApplyAgentNotFound,
        CoreError::IdentityNotActive => ApplyErrorCode::ApplyAgentNotActive,
        CoreError::IdentityAlreadyRegistered => ApplyErrorCode::ApplyProtoConflict,
        CoreError::InvalidSignature | CoreError::SigningContextMismatch => {
            ApplyErrorCode::ApplyInvalidGrantSignature
        }
        CoreError::MalformedCbor | CoreError::MalformedObject(_) | CoreError::AgentIdMismatch => {
            ApplyErrorCode::ApplyInvalidPolicy
        }
        CoreError::PermissionRootMismatch
        | CoreError::InvalidRootAuthority
        | CoreError::StaleRootVersion
        | CoreError::NonMonotonicRootVersion => ApplyErrorCode::ApplyPermissionDenied,
        CoreError::CapabilityNotFound => ApplyErrorCode::ApplyCapabilityNotFound,
        CoreError::CapabilityRevoked => ApplyErrorCode::ApplyCapabilityRevoked,
        CoreError::ParentMissing => ApplyErrorCode::ApplyInvalidPolicy,
        CoreError::Escalation(_) | CoreError::ExcessiveDelegationDepth => {
            ApplyErrorCode::ApplyCapabilityDenied
        }
        CoreError::InvalidDelegationDepth => ApplyErrorCode::ApplyInvalidPolicy,
        CoreError::UnexpectedAuthorisation => ApplyErrorCode::ApplyProtoInternal,
        CoreError::Rejected(reason) => map_reject_reason(reason),
        CoreError::CapabilityDenied => ApplyErrorCode::ApplyCapabilityDenied,
        CoreError::InvalidPublicKey => ApplyErrorCode::ApplyInvalidGrantSignature,
        _ => ApplyErrorCode::ApplyProtoUnsupported,
    }
}

/// Map PROTO-0 `RejectReason` to Apply code.
pub fn map_reject_reason(reason: &RejectReason) -> ApplyErrorCode {
    match reason {
        RejectReason::IdentityFrozen => ApplyErrorCode::ApplyIdentityFrozen,
        RejectReason::IdentityRevoked => ApplyErrorCode::ApplyIdentityRevoked,
        RejectReason::CapabilityNotFound => ApplyErrorCode::ApplyCapabilityNotFound,
        RejectReason::CapabilityRevoked => ApplyErrorCode::ApplyCapabilityRevoked,
        RejectReason::CapabilityExpired => ApplyErrorCode::ApplyCapabilityExpired,
        RejectReason::CapabilityNotYetValid => ApplyErrorCode::ApplyCapabilityNotYetValid,
        RejectReason::Escalation | RejectReason::ExcessiveDelegationDepth => {
            ApplyErrorCode::ApplyCapabilityDenied
        }
        RejectReason::InvalidDelegationDepth => ApplyErrorCode::ApplyInvalidPolicy,
        RejectReason::ActionNotPermitted | RejectReason::MissingCapability => {
            ApplyErrorCode::ApplyCapabilityDenied
        }
        RejectReason::SpendExceeded
        | RejectReason::AssetNotPermitted
        | RejectReason::CounterpartyNotPermitted
        | RejectReason::RateLimitExceeded => ApplyErrorCode::ApplyConstraintViolation,
        RejectReason::InvalidSignature | RejectReason::SigningContextMismatch => {
            ApplyErrorCode::ApplyInvalidGrantSignature
        }
        RejectReason::MalformedObject => ApplyErrorCode::ApplyInvalidPolicy,
        _ => ApplyErrorCode::ApplyProtoRejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_identity_frozen_reject_reason() {
        assert_eq!(
            map_reject_reason(&RejectReason::IdentityFrozen),
            ApplyErrorCode::ApplyIdentityFrozen
        );
    }

    #[test]
    fn maps_capability_not_found_core_error() {
        assert_eq!(
            map_core_error(&CoreError::CapabilityNotFound),
            ApplyErrorCode::ApplyCapabilityNotFound
        );
    }

    #[test]
    fn maps_escalation_to_denied() {
        assert_eq!(
            map_core_error(&CoreError::Escalation("actions vs root")),
            ApplyErrorCode::ApplyCapabilityDenied
        );
    }

    #[test]
    fn maps_proto1_error_to_unsupported() {
        assert_eq!(
            map_core_error(&CoreError::ChannelNotFound),
            ApplyErrorCode::ApplyProtoUnsupported
        );
    }
}
