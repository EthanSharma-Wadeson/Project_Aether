//! Settlement state transitions — authority pipeline before any mutation.

#![allow(clippy::too_many_arguments)]

use ed25519_dalek::SigningKey;

use crate::capability::grant::CapabilityGrant;
use crate::capability::model::CapabilityStore;
use crate::crypto::sha256;
use crate::crypto::signing::{sign_body, DOMAIN_TAG};
use crate::crypto::verify::SignedMessage;
use crate::error::{Error, Result};
use crate::escrow::store::{
    clear_hard_settlement_flag, promote_verified_hard_settlement, EscrowStore,
};
use crate::identity::registry::IdentityRegistry;
use crate::settlement::adapter::{
    AdapterStatusResult, MockSettlementAdapterV0, SettlementAdapterV0,
};
use crate::settlement::binding::{
    SettlementAccountBindingV0, SettlementBindingV0, SettlementIntentV0,
};
use crate::settlement::model::{
    EconomicOutcome, SettlementStatus, ACTION_BIND, ACTION_CANCEL, ACTION_QUERY, ACTION_SETTLE,
    MSG_ACCOUNT_BIND, MSG_SETTLE_REQUEST, SETTLEMENT_PROTOCOL_VERSION, SETTLEMENT_SCHEMA_VERSION,
};
use crate::settlement::report::{
    report_from_adapter, SettlementEvidencePackageV0, SettlementReportV0,
};
use crate::settlement::store::SettlementStore;
use crate::settlement::verify::{
    assert_canonical_account_body, escrow_terminal_commitment, require_capability,
    settleable_escrow_status, validate_destination_accounts, validate_settlement_against_escrow,
    verify_settlement_report, verify_signed_body,
};

fn signed_message(
    signing_key: &SigningKey,
    agent_id: &str,
    message_type: &str,
    protocol_version: u32,
    schema_version: u32,
    body: &[u8],
) -> SignedMessage {
    let (_, signature) = sign_body(
        signing_key,
        DOMAIN_TAG,
        protocol_version,
        schema_version,
        message_type,
        body,
    );
    SignedMessage {
        protocol_version,
        schema_version,
        message_type: message_type.into(),
        body: body.to_vec(),
        signer_key_id: agent_id.into(),
        signature: signature.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    }
}

pub fn sign_account_binding(
    binding: &SettlementAccountBindingV0,
    actor_key: &SigningKey,
    agent_id: &str,
) -> Result<SignedMessage> {
    Ok(signed_message(
        actor_key,
        agent_id,
        MSG_ACCOUNT_BIND,
        binding.protocol_version,
        binding.schema_version,
        &binding.encode()?,
    ))
}

pub fn sign_settlement_binding(
    binding: &SettlementBindingV0,
    actor_key: &SigningKey,
    agent_id: &str,
    message_type: &str,
) -> Result<SignedMessage> {
    Ok(signed_message(
        actor_key,
        agent_id,
        message_type,
        binding.protocol_version,
        binding.schema_version,
        &binding.encode()?,
    ))
}

/// Bind AgentId → external account (capability-gated).
pub fn bind_account(
    settle_store: &mut SettlementStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    signed: &SignedMessage,
    grant: Option<&CapabilityGrant>,
    now: u64,
) -> Result<SettlementAccountBindingV0> {
    // 1. Signature
    let entry = registry
        .get(&signed.signer_key_id)
        .ok_or(Error::IdentityNotFound)?;
    verify_signed_body(
        signed,
        &entry.identity.operational_public_key,
        MSG_ACCOUNT_BIND,
        SETTLEMENT_PROTOCOL_VERSION,
        SETTLEMENT_SCHEMA_VERSION,
    )?;

    // 2–3. Identity + capability
    require_capability(
        registry,
        caps,
        &signed.signer_key_id,
        grant,
        ACTION_BIND,
        None,
        None,
        now,
    )?;

    let binding = SettlementAccountBindingV0::decode(&signed.body)?;
    assert_canonical_account_body(&signed.body, &binding)?;
    binding.verify_id()?;

    if binding.agent_id != signed.signer_key_id {
        return Err(Error::AgentIdMismatch);
    }
    if binding.external_account_ref.is_empty() {
        return Err(Error::MalformedObject("empty external_account_ref"));
    }
    if now < binding.valid_after || now > binding.valid_before {
        return Err(Error::MalformedObject("binding not valid at now"));
    }
    if settle_store.get_account(&binding.binding_id).is_some() {
        return Err(Error::SettlementAlreadyExists);
    }

    settle_store.insert_account(binding.clone());
    Ok(binding)
}

fn actor_pubkey<'a>(registry: &'a IdentityRegistry, agent_id: &str) -> Result<&'a [u8]> {
    let entry = registry.get(agent_id).ok_or(Error::IdentityNotFound)?;
    Ok(entry.identity.operational_public_key.as_slice())
}

/// Request settlement for a terminal PROTO-2 escrow outcome.
pub fn request_settlement(
    settle_store: &mut SettlementStore,
    escrow_store: &EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    intent: &SettlementIntentV0,
    actor_id: &str,
    actor_key: &SigningKey,
    grant: Option<&CapabilityGrant>,
    now: u64,
) -> Result<SettlementBindingV0> {
    // Capability before any settlement mutation / adapter effect.
    require_capability(
        registry,
        caps,
        actor_id,
        grant,
        ACTION_SETTLE,
        Some(intent.principal_amount),
        Some(&intent.asset),
        now,
    )?;

    let record = escrow_store
        .get(&intent.escrow_id)
        .ok_or(Error::EscrowNotFound)?;
    let escrow = &record.escrow;

    if escrow.terms.terms_version != intent.terms_version {
        return Err(Error::InvalidSettlementEvidence);
    }
    if escrow.status.as_str() != intent.aether_escrow_status {
        return Err(Error::InvalidSettlementEvidence);
    }
    settleable_escrow_status(escrow.status, intent.economic_outcome)?;

    if intent.principal_amount != escrow.terms.principal_amount
        || intent.fee_amount != escrow.fee_consumed
        || intent.asset != escrow.terms.asset
    {
        return Err(Error::SettlementAmountMismatch);
    }

    // Build binding then check idempotency via correlation.
    let escrow_commit = escrow_terminal_commitment(escrow_store, &intent.escrow_id)?;
    let mut binding = intent.to_proto_binding(
        escrow.terms.payer.clone(),
        escrow.terms.provider.clone(),
        now,
        [0u8; 32],
    )?;
    // evidence uses binding_id after compute
    let evidence = SettlementEvidencePackageV0 {
        settlement_binding_id: binding.binding_id,
        escrow_terminal_commitment: escrow_commit,
        capability_grant_ids: vec![],
        adapter_response_bytes: vec![],
        logical_time: now,
    };
    binding.evidence_commitment = evidence.commitment()?;

    // Idempotent retry
    if let Some(existing) = settle_store.get_by_correlation(&binding.correlation_id) {
        return Ok(existing.clone());
    }

    // Duplicate settlement for same escrow+outcome while active/final
    if let Some(active) =
        settle_store.active_for_escrow_outcome(&intent.escrow_id, intent.economic_outcome)
    {
        if active.correlation_id != binding.correlation_id {
            return Err(Error::DuplicateSettlement);
        }
        return Ok(active.clone());
    }

    validate_destination_accounts(settle_store, &binding, now)?;
    validate_settlement_against_escrow(escrow_store, &binding)?;

    if actor_id != binding.payer_agent_id && actor_id != binding.provider_agent_id {
        return Err(Error::ParticipantMismatch);
    }

    let signed = sign_settlement_binding(&binding, actor_key, actor_id, MSG_SETTLE_REQUEST)?;
    verify_signed_body(
        &signed,
        actor_pubkey(registry, actor_id)?,
        MSG_SETTLE_REQUEST,
        SETTLEMENT_PROTOCOL_VERSION,
        SETTLEMENT_SCHEMA_VERSION,
    )?;

    settle_store.insert_settlement(binding.clone());
    Ok(binding)
}

/// Submit settlement to adapter (side effect only after auth + validation).
pub fn submit_settlement(
    settle_store: &mut SettlementStore,
    escrow_store: &EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    adapter: &mut dyn SettlementAdapterV0,
    binding_id: &[u8; 32],
    actor_id: &str,
    grant: Option<&CapabilityGrant>,
    now: u64,
) -> Result<SettlementBindingV0> {
    let existing = settle_store
        .get_settlement(binding_id)
        .ok_or(Error::SettlementNotFound)?
        .clone();

    require_capability(
        registry,
        caps,
        actor_id,
        grant,
        ACTION_SETTLE,
        Some(existing.principal_amount),
        Some(&existing.asset),
        now,
    )?;

    if existing.settlement_status != SettlementStatus::Requested {
        // Idempotent: already submitted
        if existing.external_settlement_ref.is_some() {
            return Ok(existing);
        }
        return Err(Error::InvalidSettlementStatus);
    }

    validate_settlement_against_escrow(escrow_store, &existing)?;
    validate_destination_accounts(settle_store, &existing, now)?;

    if adapter.provider_id() != existing.settlement_provider {
        return Err(Error::SettlementProviderMismatch);
    }

    let submit = match adapter.submit(&existing) {
        Ok(r) => r,
        Err(Error::AdapterFailure) => {
            // Policy: adapter error on submit → remain Requested; no hard flag.
            return Ok(existing);
        }
        Err(e) => return Err(e),
    };

    // Duplicate external ref against another binding
    if let Some(other) = settle_store.get_by_external_ref(&submit.external_settlement_ref) {
        if other.binding_id != existing.binding_id {
            return Err(Error::SettlementConflict);
        }
    }

    let mut updated = existing;
    updated.external_settlement_ref = Some(submit.external_settlement_ref.clone());
    updated.submitted_at = Some(now);
    updated.adapter_receipt_commitment = Some(sha256(&submit.receipt_bytes));

    if submit.amount != updated.principal_amount {
        updated.settlement_status = SettlementStatus::Failed;
        settle_store.update_settlement(updated.clone());
        return Ok(updated);
    }

    match submit.status {
        SettlementStatus::Submitted | SettlementStatus::Accepted | SettlementStatus::Confirmed => {
            updated.settlement_status = submit.status;
            if submit.status == SettlementStatus::Accepted {
                updated.accepted_at = Some(now);
            }
            if submit.status == SettlementStatus::Confirmed {
                // Only accept Confirmed if proof verifies.
                let report = report_from_adapter(
                    updated.settlement_id(),
                    updated.correlation_id,
                    adapter.provider_id(),
                    &submit.external_settlement_ref,
                    SettlementStatus::Confirmed,
                    submit.amount,
                    submit.proof_token,
                    vec![],
                );
                if verify_settlement_report(&updated, &report, adapter).is_err() {
                    updated.settlement_status = SettlementStatus::Failed;
                } else {
                    updated.confirmed_at = Some(now);
                    updated.evidence_commitment = SettlementEvidencePackageV0 {
                        settlement_binding_id: updated.binding_id,
                        escrow_terminal_commitment: escrow_terminal_commitment(
                            escrow_store,
                            &updated.escrow_id,
                        )?,
                        capability_grant_ids: vec![],
                        adapter_response_bytes: submit.receipt_bytes.clone(),
                        logical_time: now,
                    }
                    .commitment()?;
                }
            }
        }
        SettlementStatus::Failed => {
            updated.settlement_status = SettlementStatus::Failed;
        }
        _ => {
            return Err(Error::InvalidSettlementStatus);
        }
    }

    settle_store.update_settlement(updated.clone());
    Ok(updated)
}

/// Query adapter and refresh settlement status without mutating escrow economics.
pub fn query_settlement(
    settle_store: &mut SettlementStore,
    escrow_store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    adapter: &mut MockSettlementAdapterV0,
    binding_id: &[u8; 32],
    actor_id: &str,
    grant: Option<&CapabilityGrant>,
    now: u64,
) -> Result<(SettlementBindingV0, SettlementReportV0)> {
    let existing = settle_store
        .get_settlement(binding_id)
        .ok_or(Error::SettlementNotFound)?
        .clone();

    require_capability(
        registry,
        caps,
        actor_id,
        grant,
        ACTION_QUERY,
        None,
        None,
        now,
    )?;

    validate_settlement_against_escrow(escrow_store, &existing)?;

    let ext = existing
        .external_settlement_ref
        .clone()
        .ok_or(Error::InvalidSettlementEvidence)?;

    adapter.note_query();
    let status_result = adapter.query(&ext)?;

    apply_adapter_status(
        settle_store,
        escrow_store,
        adapter,
        existing,
        status_result,
        now,
    )
}

/// Advance mock provider through Submitted → Accepted → Confirmed (test helper path).
pub fn advance_mock_status(
    adapter: &mut MockSettlementAdapterV0,
    binding: &SettlementBindingV0,
    status: SettlementStatus,
) -> Result<AdapterStatusResult> {
    let ext = binding
        .external_settlement_ref
        .as_ref()
        .ok_or(Error::InvalidSettlementEvidence)?;
    adapter.advance_record(
        ext,
        status,
        binding.principal_amount,
        &binding.settlement_id(),
    )
}

fn apply_adapter_status(
    settle_store: &mut SettlementStore,
    escrow_store: &mut EscrowStore,
    adapter: &dyn SettlementAdapterV0,
    mut binding: SettlementBindingV0,
    status_result: AdapterStatusResult,
    now: u64,
) -> Result<(SettlementBindingV0, SettlementReportV0)> {
    let ext = binding
        .external_settlement_ref
        .clone()
        .ok_or(Error::InvalidSettlementEvidence)?;

    if status_result.external_settlement_ref != ext {
        return Err(Error::InvalidSettlementEvidence);
    }

    let report = report_from_adapter(
        binding.settlement_id(),
        binding.correlation_id,
        adapter.provider_id(),
        &ext,
        status_result.status,
        status_result.amount,
        status_result.proof_token,
        vec![sha256(&status_result.receipt_bytes)],
    );

    // Conflicting: was Confirmed/Finalized, adapter now reports failure/reverse
    if matches!(
        binding.settlement_status,
        SettlementStatus::Confirmed | SettlementStatus::Finalized
    ) && matches!(
        status_result.status,
        SettlementStatus::Failed | SettlementStatus::Cancelled | SettlementStatus::DisputedExternal
    ) {
        binding.settlement_status = SettlementStatus::DisputedExternal;
        settle_store.update_settlement(binding.clone());
        let _ = clear_hard_settlement_flag(escrow_store, &binding.escrow_id);
        return Ok((binding, report));
    }

    if status_result.amount != binding.principal_amount
        && !matches!(status_result.status, SettlementStatus::Failed)
    {
        binding.settlement_status = SettlementStatus::Failed;
        settle_store.update_settlement(binding.clone());
        return Ok((binding, report));
    }

    match status_result.status {
        SettlementStatus::Accepted => {
            if matches!(
                binding.settlement_status,
                SettlementStatus::Submitted | SettlementStatus::Requested
            ) {
                binding.settlement_status = SettlementStatus::Accepted;
                binding.accepted_at = Some(now);
            }
        }
        SettlementStatus::Confirmed => {
            verify_settlement_report(&binding, &report, adapter)?;
            binding.settlement_status = SettlementStatus::Confirmed;
            binding.confirmed_at = Some(now);
            binding.adapter_receipt_commitment = Some(sha256(&status_result.receipt_bytes));
            binding.evidence_commitment = SettlementEvidencePackageV0 {
                settlement_binding_id: binding.binding_id,
                escrow_terminal_commitment: escrow_terminal_commitment(
                    escrow_store,
                    &binding.escrow_id,
                )?,
                capability_grant_ids: vec![],
                adapter_response_bytes: status_result.receipt_bytes.clone(),
                logical_time: now,
            }
            .commitment()?;
        }
        SettlementStatus::Failed => {
            binding.settlement_status = SettlementStatus::Failed;
        }
        SettlementStatus::Submitted => {
            binding.settlement_status = SettlementStatus::Submitted;
        }
        SettlementStatus::Cancelled => {
            binding.settlement_status = SettlementStatus::Cancelled;
        }
        SettlementStatus::DisputedExternal => {
            binding.settlement_status = SettlementStatus::DisputedExternal;
            let _ = clear_hard_settlement_flag(escrow_store, &binding.escrow_id);
        }
        SettlementStatus::Finalized | SettlementStatus::Requested => {}
    }

    settle_store.update_settlement(binding.clone());
    Ok((binding, report))
}

/// Finalize after Confirmed + verified evidence; sets hard_settlement_placeholder.
///
/// Requires a caller-supplied Confirmed report **and** a fresh adapter query that
/// still reports Confirmed. Does not overwrite caller report status.
pub fn finalize_settlement(
    settle_store: &mut SettlementStore,
    escrow_store: &mut EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    adapter: &dyn SettlementAdapterV0,
    binding_id: &[u8; 32],
    actor_id: &str,
    grant: Option<&CapabilityGrant>,
    report: &SettlementReportV0,
    now: u64,
) -> Result<SettlementBindingV0> {
    let mut binding = settle_store
        .get_settlement(binding_id)
        .ok_or(Error::SettlementNotFound)?
        .clone();

    require_capability(
        registry,
        caps,
        actor_id,
        grant,
        ACTION_SETTLE,
        Some(binding.principal_amount),
        Some(&binding.asset),
        now,
    )?;

    validate_settlement_against_escrow(escrow_store, &binding)?;
    validate_destination_accounts(settle_store, &binding, now)?;

    if !matches!(
        binding.settlement_status,
        SettlementStatus::Confirmed | SettlementStatus::Finalized
    ) {
        return Err(Error::InvalidSettlementStatus);
    }

    if binding.settlement_status == SettlementStatus::Finalized {
        return Ok(binding);
    }

    // Caller report must already claim Confirmed — never overwrite status.
    if report.status != SettlementStatus::Confirmed {
        return Err(Error::InvalidSettlementEvidence);
    }
    verify_settlement_report(&binding, report, adapter)?;

    // Fresh adapter attestation — stale reports cannot promote hard finality.
    let ext = binding
        .external_settlement_ref
        .clone()
        .ok_or(Error::InvalidSettlementEvidence)?;
    let live = adapter.query(&ext)?;
    if live.status != SettlementStatus::Confirmed {
        return Err(Error::InvalidSettlementEvidence);
    }
    if live.external_settlement_ref != ext {
        return Err(Error::InvalidSettlementEvidence);
    }
    if live.amount != binding.principal_amount || live.amount != report.amount {
        return Err(Error::SettlementAmountMismatch);
    }
    if live.external_settlement_ref != report.external_reference {
        return Err(Error::InvalidSettlementEvidence);
    }

    let live_report = report_from_adapter(
        binding.settlement_id(),
        binding.correlation_id,
        adapter.provider_id(),
        &ext,
        live.status,
        live.amount,
        live.proof_token,
        vec![sha256(&live.receipt_bytes)],
    );
    if live_report.status != SettlementStatus::Confirmed {
        return Err(Error::InvalidSettlementEvidence);
    }
    verify_settlement_report(&binding, &live_report, adapter)?;

    binding.settlement_status = SettlementStatus::Finalized;
    binding.finalized_at = Some(now);
    binding.adapter_receipt_commitment = Some(sha256(&live.receipt_bytes));
    settle_store.update_settlement(binding.clone());
    promote_verified_hard_settlement(escrow_store, &binding.escrow_id)?;
    Ok(binding)
}

/// Cancel before Confirmed.
pub fn cancel_settlement(
    settle_store: &mut SettlementStore,
    escrow_store: &EscrowStore,
    registry: &IdentityRegistry,
    caps: &CapabilityStore,
    adapter: &mut dyn SettlementAdapterV0,
    binding_id: &[u8; 32],
    actor_id: &str,
    grant: Option<&CapabilityGrant>,
    now: u64,
) -> Result<SettlementBindingV0> {
    let mut binding = settle_store
        .get_settlement(binding_id)
        .ok_or(Error::SettlementNotFound)?
        .clone();

    require_capability(
        registry,
        caps,
        actor_id,
        grant,
        ACTION_CANCEL,
        None,
        None,
        now,
    )?;

    if !binding.settlement_status.allows_cancel() {
        return Err(Error::InvalidSettlementStatus);
    }

    validate_settlement_against_escrow(escrow_store, &binding)?;

    if let Some(ref ext) = binding.external_settlement_ref {
        adapter.cancel(ext)?;
    }

    binding.settlement_status = SettlementStatus::Cancelled;
    settle_store.update_settlement(binding.clone());
    let _ = now;
    Ok(binding)
}

/// Mark disputed after external conflict (also used by tests).
pub fn mark_disputed_external(
    settle_store: &mut SettlementStore,
    escrow_store: &mut EscrowStore,
    binding_id: &[u8; 32],
) -> Result<SettlementBindingV0> {
    let mut binding = settle_store
        .get_settlement(binding_id)
        .ok_or(Error::SettlementNotFound)?
        .clone();
    binding.settlement_status = SettlementStatus::DisputedExternal;
    settle_store.update_settlement(binding.clone());
    clear_hard_settlement_flag(escrow_store, &binding.escrow_id)?;
    Ok(binding)
}

/// Build a settle intent from live escrow + account bindings.
pub fn intent_from_escrow(
    escrow_store: &EscrowStore,
    escrow_id: &[u8; 32],
    outcome: EconomicOutcome,
    settlement_provider: &str,
    payer_account_binding_id: [u8; 32],
    provider_account_binding_id: Option<[u8; 32]>,
) -> Result<SettlementIntentV0> {
    let record = escrow_store.get(escrow_id).ok_or(Error::EscrowNotFound)?;
    settleable_escrow_status(record.escrow.status, outcome)?;
    Ok(SettlementIntentV0 {
        escrow_id: *escrow_id,
        terms_version: record.escrow.terms.terms_version,
        economic_outcome: outcome,
        principal_amount: record.escrow.terms.principal_amount,
        fee_amount: record.escrow.fee_consumed,
        asset: record.escrow.terms.asset.clone(),
        settlement_provider: settlement_provider.into(),
        payer_account_binding_id,
        provider_account_binding_id,
        aether_escrow_status: record.escrow.status.as_str().into(),
    })
}

pub fn new_account_binding(
    agent_id: &str,
    settlement_provider: &str,
    external_account_ref: &str,
    asset: &str,
    now: u64,
    valid_before: u64,
) -> Result<SettlementAccountBindingV0> {
    SettlementAccountBindingV0 {
        protocol_version: SETTLEMENT_PROTOCOL_VERSION,
        schema_version: SETTLEMENT_SCHEMA_VERSION,
        binding_id: [0u8; 32],
        agent_id: agent_id.into(),
        settlement_provider: settlement_provider.into(),
        external_account_ref: external_account_ref.into(),
        asset: asset.into(),
        scope: "escrow.settle".into(),
        binding_version: 1,
        commitments: vec![],
        valid_after: 0,
        valid_before,
        created_at: now,
    }
    .with_computed_id()
}
