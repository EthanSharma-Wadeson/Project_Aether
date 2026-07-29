//! Reputation query interface.

use crate::types::AgentId;

use super::metrics::AgentMetricsV0;
use super::model::{EvidenceRefV0, ReputationEventV0};
use super::store::{CompletenessQualification, ReputationStore, StoreMode};

#[derive(Debug, Clone)]
pub struct ReputationQueryV0 {
    pub subject: AgentId,
    pub as_of: Option<u64>,
    pub include_evidence: bool,
}

#[derive(Debug, Clone)]
pub struct ReputationQueryResultV0 {
    pub subject: AgentId,
    pub metrics: AgentMetricsV0,
    pub event_log_root: [u8; 32],
    pub indexer_id: String,
    pub computed_at: u64,
    pub evidence_refs: Vec<EvidenceRefV0>,
    pub completeness: CompletenessQualification,
}

impl ReputationQueryResultV0 {
    /// Execute a query against a reputation store.
    pub fn execute(query: &ReputationQueryV0, store: &ReputationStore, now: u64) -> Option<Self> {
        if matches!(store.mode(), StoreMode::LocalOnly) {
            return None;
        }

        let metrics = match query.as_of {
            None => store.get_metrics(&query.subject)?.clone(),
            Some(cutoff) => {
                let events = store.agent_events(&query.subject);
                let filtered: Vec<&ReputationEventV0> = events
                    .into_iter()
                    .filter(|e| e.logical_time <= cutoff)
                    .collect();
                AgentMetricsV0::compute(&query.subject, &filtered)
            }
        };

        let evidence_refs = if query.include_evidence {
            store
                .agent_events(&query.subject)
                .into_iter()
                .flat_map(|e| e.evidence_refs.clone())
                .collect()
        } else {
            Vec::new()
        };

        Some(Self {
            subject: query.subject.clone(),
            metrics,
            event_log_root: store.event_log_root(),
            indexer_id: store.indexer_id().to_string(),
            computed_at: now,
            evidence_refs,
            completeness: store.completeness_qualification(),
        })
    }
}
