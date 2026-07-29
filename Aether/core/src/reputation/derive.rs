//! Event derivation pipeline — derives ReputationEventV0 from protocol artifacts.
//!
//! The indexer reads from PROTO-0/1/2/4 stores but never writes to them.
//! Per P3-SEC-001: `capability.denied` is excluded from v0 (no signed evidence source).

use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::escrow::state::{EscrowStatus, EscrowV0};
use crate::identity::registry::IdentityRegistry;
use crate::settlement::binding::SettlementBindingV0;
use crate::settlement::model::SettlementStatus;
use crate::types::AgentId;

use super::model::{
    Attribution, EventType, EvidenceRefType, EvidenceRefV0, EvidenceSourceProto, ReputationEventV0,
    REPUTATION_PROTOCOL_VERSION, REPUTATION_SCHEMA_VERSION,
};
use super::store::ReputationStore;

/// Read-only indexer that derives reputation events from protocol artifacts.
#[derive(Debug)]
pub struct ReputationIndexer;

impl ReputationIndexer {
    /// Derive events from a terminal escrow and append to store.
    /// Returns the number of new events appended.
    pub fn ingest_escrow(
        store: &mut ReputationStore,
        escrow: &EscrowV0,
        registry: &IdentityRegistry,
    ) -> Result<usize> {
        if !escrow.status.is_terminal() {
            return Err(Error::MalformedObject(
                "cannot derive reputation from non-terminal escrow",
            ));
        }

        let payer = escrow.terms.payer.clone();
        let provider = escrow.terms.provider.clone();

        // Reject if either identity is unknown.
        Self::check_identity_known(registry, &payer)?;
        Self::check_identity_known(registry, &provider)?;

        let escrow_commitment = sha256(&escrow.encode()?);

        let mut count = 0;

        let principal = escrow.terms.principal_amount;

        match escrow.status {
            EscrowStatus::Released | EscrowStatus::ResolvedReleased => {
                // Provider credit.
                let evt = Self::build_event(
                    EventType::EscrowReleased,
                    &provider,
                    Some(&payer),
                    escrow.finalized_at.unwrap_or(escrow.created_at),
                    Attribution::Credit,
                    Some(principal),
                    vec![EvidenceRefV0 {
                        ref_type: EvidenceRefType::EscrowTerminal,
                        commitment: escrow_commitment,
                        source_proto: EvidenceSourceProto::Proto2,
                        locator: Some(escrow.escrow_id),
                    }],
                )?;
                if store.append(evt)? {
                    count += 1;
                }

                // Payer neutral (completed payment).
                let evt = Self::build_event(
                    EventType::EscrowReleased,
                    &payer,
                    Some(&provider),
                    escrow.finalized_at.unwrap_or(escrow.created_at),
                    Attribution::Neutral,
                    Some(principal),
                    vec![EvidenceRefV0 {
                        ref_type: EvidenceRefType::EscrowTerminal,
                        commitment: escrow_commitment,
                        source_proto: EvidenceSourceProto::Proto2,
                        locator: Some(escrow.escrow_id),
                    }],
                )?;
                if store.append(evt)? {
                    count += 1;
                }
            }

            EscrowStatus::Refunded | EscrowStatus::ResolvedRefunded => {
                // Provider fault signal.
                let evt = Self::build_event(
                    EventType::EscrowRefunded,
                    &provider,
                    Some(&payer),
                    escrow.finalized_at.unwrap_or(escrow.created_at),
                    Attribution::Fault,
                    Some(principal),
                    vec![EvidenceRefV0 {
                        ref_type: EvidenceRefType::EscrowTerminal,
                        commitment: escrow_commitment,
                        source_proto: EvidenceSourceProto::Proto2,
                        locator: Some(escrow.escrow_id),
                    }],
                )?;
                if store.append(evt)? {
                    count += 1;
                }

                // Payer credit (funds returned).
                let evt = Self::build_event(
                    EventType::EscrowRefunded,
                    &payer,
                    Some(&provider),
                    escrow.finalized_at.unwrap_or(escrow.created_at),
                    Attribution::Credit,
                    Some(principal),
                    vec![EvidenceRefV0 {
                        ref_type: EvidenceRefType::EscrowTerminal,
                        commitment: escrow_commitment,
                        source_proto: EvidenceSourceProto::Proto2,
                        locator: Some(escrow.escrow_id),
                    }],
                )?;
                if store.append(evt)? {
                    count += 1;
                }
            }

            EscrowStatus::Expired => {
                // Attribution depends on terms — for v0, provider fault.
                let evt = Self::build_event(
                    EventType::EscrowExpired,
                    &provider,
                    Some(&payer),
                    escrow.finalized_at.unwrap_or(escrow.created_at),
                    Attribution::Fault,
                    Some(principal),
                    vec![EvidenceRefV0 {
                        ref_type: EvidenceRefType::EscrowTerminal,
                        commitment: escrow_commitment,
                        source_proto: EvidenceSourceProto::Proto2,
                        locator: Some(escrow.escrow_id),
                    }],
                )?;
                if store.append(evt)? {
                    count += 1;
                }
            }

            EscrowStatus::Cancelled => {
                // Cooperative cancel — neutral for both.
                let evt = Self::build_event(
                    EventType::EscrowRefunded,
                    &payer,
                    Some(&provider),
                    escrow.finalized_at.unwrap_or(escrow.created_at),
                    Attribution::Neutral,
                    None,
                    vec![EvidenceRefV0 {
                        ref_type: EvidenceRefType::EscrowTerminal,
                        commitment: escrow_commitment,
                        source_proto: EvidenceSourceProto::Proto2,
                        locator: Some(escrow.escrow_id),
                    }],
                )?;
                if store.append(evt)? {
                    count += 1;
                }
            }

            _ => {}
        }

        Ok(count)
    }

    /// Derive events from a terminal settlement binding.
    pub fn ingest_settlement(
        store: &mut ReputationStore,
        binding: &SettlementBindingV0,
        escrow: &EscrowV0,
        registry: &IdentityRegistry,
    ) -> Result<usize> {
        Self::check_identity_known(registry, &binding.payer_agent_id)?;
        Self::check_identity_known(registry, &binding.provider_agent_id)?;

        // Verify escrow_id matches.
        if binding.escrow_id != escrow.escrow_id {
            return Err(Error::MalformedObject(
                "settlement binding escrow_id does not match escrow",
            ));
        }

        let binding_commitment = sha256(&binding.encode()?);
        let escrow_commitment = sha256(&escrow.encode()?);

        let mut count = 0;

        match binding.settlement_status {
            SettlementStatus::Finalized => {
                // P3-DEC-008: require hard finality.
                if !escrow.finality.hard_settlement_placeholder {
                    return Err(Error::MalformedObject(
                        "settlement.finalized requires hard finality",
                    ));
                }

                let time = binding.finalized_at.unwrap_or(binding.requested_at);

                // Payer credit.
                let evt = Self::build_event(
                    EventType::SettlementFinalized,
                    &binding.payer_agent_id,
                    Some(&binding.provider_agent_id),
                    time,
                    Attribution::Credit,
                    Some(binding.principal_amount),
                    vec![
                        EvidenceRefV0 {
                            ref_type: EvidenceRefType::SettlementBinding,
                            commitment: binding_commitment,
                            source_proto: EvidenceSourceProto::Proto4,
                            locator: Some(binding.binding_id),
                        },
                        EvidenceRefV0 {
                            ref_type: EvidenceRefType::EscrowTerminal,
                            commitment: escrow_commitment,
                            source_proto: EvidenceSourceProto::Proto2,
                            locator: Some(escrow.escrow_id),
                        },
                    ],
                )?;
                if store.append(evt)? {
                    count += 1;
                }

                // Provider credit.
                let evt = Self::build_event(
                    EventType::SettlementFinalized,
                    &binding.provider_agent_id,
                    Some(&binding.payer_agent_id),
                    time,
                    Attribution::Credit,
                    Some(binding.principal_amount),
                    vec![
                        EvidenceRefV0 {
                            ref_type: EvidenceRefType::SettlementBinding,
                            commitment: binding_commitment,
                            source_proto: EvidenceSourceProto::Proto4,
                            locator: Some(binding.binding_id),
                        },
                        EvidenceRefV0 {
                            ref_type: EvidenceRefType::EscrowTerminal,
                            commitment: escrow_commitment,
                            source_proto: EvidenceSourceProto::Proto2,
                            locator: Some(escrow.escrow_id),
                        },
                    ],
                )?;
                if store.append(evt)? {
                    count += 1;
                }
            }

            SettlementStatus::Failed | SettlementStatus::DisputedExternal => {
                let time = binding
                    .finalized_at
                    .or(binding.confirmed_at)
                    .unwrap_or(binding.requested_at);

                let evt = Self::build_event(
                    EventType::SettlementFailed,
                    &binding.payer_agent_id,
                    Some(&binding.provider_agent_id),
                    time,
                    Attribution::Debit,
                    Some(binding.principal_amount),
                    vec![EvidenceRefV0 {
                        ref_type: EvidenceRefType::SettlementBinding,
                        commitment: binding_commitment,
                        source_proto: EvidenceSourceProto::Proto4,
                        locator: Some(binding.binding_id),
                    }],
                )?;
                if store.append(evt)? {
                    count += 1;
                }

                let evt = Self::build_event(
                    EventType::SettlementFailed,
                    &binding.provider_agent_id,
                    Some(&binding.payer_agent_id),
                    time,
                    Attribution::Debit,
                    Some(binding.principal_amount),
                    vec![EvidenceRefV0 {
                        ref_type: EvidenceRefType::SettlementBinding,
                        commitment: binding_commitment,
                        source_proto: EvidenceSourceProto::Proto4,
                        locator: Some(binding.binding_id),
                    }],
                )?;
                if store.append(evt)? {
                    count += 1;
                }
            }

            _ => {
                return Err(Error::MalformedObject(
                    "settlement not in terminal status for reputation",
                ));
            }
        }

        Ok(count)
    }

    /// Derive identity revocation event.
    pub fn ingest_identity_revocation(
        store: &mut ReputationStore,
        agent_id: &AgentId,
        registry: &IdentityRegistry,
        logical_time: u64,
    ) -> Result<usize> {
        let entry = registry.get(agent_id).ok_or(Error::IdentityNotFound)?;
        if entry.status != crate::types::AgentStatus::Revoked {
            return Err(Error::MalformedObject("identity not revoked"));
        }

        let status_bytes = format!("revoked:{agent_id}:{logical_time}");
        let commitment = sha256(status_bytes.as_bytes());

        let evt = Self::build_event(
            EventType::IdentityRevoked,
            agent_id,
            None,
            logical_time,
            Attribution::Debit,
            None,
            vec![EvidenceRefV0 {
                ref_type: EvidenceRefType::IdentityStatus,
                commitment,
                source_proto: EvidenceSourceProto::Proto0,
                locator: None,
            }],
        )?;

        let mut count = 0;
        if store.append(evt)? {
            count += 1;
        }
        Ok(count)
    }

    fn check_identity_known(registry: &IdentityRegistry, agent_id: &AgentId) -> Result<()> {
        registry.get(agent_id).ok_or(Error::IdentityNotFound)?;
        Ok(())
    }

    fn build_event(
        event_type: EventType,
        subject: &AgentId,
        counterparty: Option<&AgentId>,
        logical_time: u64,
        attribution: Attribution,
        weight_hint: Option<u64>,
        evidence_refs: Vec<EvidenceRefV0>,
    ) -> Result<ReputationEventV0> {
        ReputationEventV0 {
            protocol_version: REPUTATION_PROTOCOL_VERSION,
            schema_version: REPUTATION_SCHEMA_VERSION,
            event_id: [0u8; 32],
            event_type,
            subject_agent_id: subject.clone(),
            counterparty_agent_id: counterparty.cloned(),
            logical_time,
            evidence_refs,
            attribution,
            weight_hint,
        }
        .with_computed_id()
    }
}
