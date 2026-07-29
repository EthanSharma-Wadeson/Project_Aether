//! Append-only reputation event log with materialised metrics cache.

use std::collections::HashMap;

use crate::crypto::sha256;
use crate::error::{Error, Result};
use crate::types::AgentId;

use super::metrics::AgentMetricsV0;
use super::model::ReputationEventV0;

/// Completeness qualification for `event_log_root` (P3-SEC-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletenessQualification {
    /// Indexer ingested all known artifacts.
    SelfAttested,
    /// Verifier confirmed against independent artifact source.
    IndependentlyVerified,
}

impl CompletenessQualification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SelfAttested => "self_attested",
            Self::IndependentlyVerified => "independently_verified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreMode {
    /// Public indexer — queries are open.
    Public,
    /// Enterprise local-only — no public query endpoint.
    LocalOnly,
}

#[derive(Debug)]
pub struct ReputationStore {
    events: Vec<ReputationEventV0>,
    by_event_id: HashMap<[u8; 32], usize>,
    by_agent: HashMap<AgentId, Vec<usize>>,
    metrics_cache: HashMap<AgentId, AgentMetricsV0>,
    event_log_root: [u8; 32],
    mode: StoreMode,
    indexer_id: String,
}

impl ReputationStore {
    pub fn new(indexer_id: String, mode: StoreMode) -> Self {
        Self {
            events: Vec::new(),
            by_event_id: HashMap::new(),
            by_agent: HashMap::new(),
            metrics_cache: HashMap::new(),
            event_log_root: [0u8; 32],
            mode,
            indexer_id,
        }
    }

    pub fn mode(&self) -> StoreMode {
        self.mode
    }

    pub fn indexer_id(&self) -> &str {
        &self.indexer_id
    }

    pub fn event_log_root(&self) -> [u8; 32] {
        self.event_log_root
    }

    /// `event_log_root` proves internal consistency of the included set,
    /// not completeness of all possible events (P3-SEC-002).
    pub fn completeness_qualification(&self) -> CompletenessQualification {
        CompletenessQualification::SelfAttested
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn events(&self) -> &[ReputationEventV0] {
        &self.events
    }

    pub fn get_event(&self, event_id: &[u8; 32]) -> Option<&ReputationEventV0> {
        self.by_event_id.get(event_id).map(|&i| &self.events[i])
    }

    pub fn agent_events(&self, agent_id: &AgentId) -> Vec<&ReputationEventV0> {
        match self.by_agent.get(agent_id) {
            Some(indices) => indices.iter().map(|&i| &self.events[i]).collect(),
            None => Vec::new(),
        }
    }

    pub fn get_metrics(&self, agent_id: &AgentId) -> Option<&AgentMetricsV0> {
        self.metrics_cache.get(agent_id)
    }

    pub fn all_agent_ids(&self) -> Vec<&AgentId> {
        self.by_agent.keys().collect()
    }

    /// Append verified event. Returns Ok(true) if new, Ok(false) if duplicate (idempotent).
    pub fn append(&mut self, event: ReputationEventV0) -> Result<bool> {
        event.verify_id()?;
        if event.evidence_refs.is_empty() {
            return Err(Error::MalformedObject("event has no evidence refs"));
        }

        if self.by_event_id.contains_key(&event.event_id) {
            return Ok(false);
        }

        let idx = self.events.len();
        let agent = event.subject_agent_id.clone();

        self.by_event_id.insert(event.event_id, idx);
        self.by_agent.entry(agent.clone()).or_default().push(idx);
        if let Some(ref cp) = event.counterparty_agent_id {
            self.by_agent.entry(cp.clone()).or_default().push(idx);
        }

        self.events.push(event);
        self.update_log_root();
        self.recompute_metrics_for(&agent);

        Ok(true)
    }

    fn update_log_root(&mut self) {
        // Incremental hash chain: H(prev_root || latest_event_id)
        let latest = &self.events.last().unwrap().event_id;
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&self.event_log_root);
        data.extend_from_slice(latest);
        self.event_log_root = sha256(&data);
    }

    fn recompute_metrics_for(&mut self, agent_id: &AgentId) {
        let events = self.agent_events(agent_id);
        let metrics = AgentMetricsV0::compute(agent_id, &events);
        self.metrics_cache.insert(agent_id.clone(), metrics);
    }

    /// Full recompute from event log — must match materialised cache.
    pub fn full_recompute(&self) -> HashMap<AgentId, AgentMetricsV0> {
        let mut result: HashMap<AgentId, AgentMetricsV0> = HashMap::new();
        let mut per_agent: HashMap<AgentId, Vec<&ReputationEventV0>> = HashMap::new();

        for event in &self.events {
            per_agent
                .entry(event.subject_agent_id.clone())
                .or_default()
                .push(event);
            if let Some(ref cp) = event.counterparty_agent_id {
                per_agent.entry(cp.clone()).or_default().push(event);
            }
        }

        for (agent_id, events) in &per_agent {
            result.insert(agent_id.clone(), AgentMetricsV0::compute(agent_id, events));
        }
        result
    }

    /// Recompute event_log_root from scratch.
    pub fn recompute_log_root(&self) -> [u8; 32] {
        let mut root = [0u8; 32];
        for event in &self.events {
            let mut data = Vec::with_capacity(64);
            data.extend_from_slice(&root);
            data.extend_from_slice(&event.event_id);
            root = sha256(&data);
        }
        root
    }
}
