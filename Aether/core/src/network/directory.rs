//! AgentDirectoryV0 — local discovery simulation (AgentId → connection info).

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::identity::registry::IdentityRegistry;
use crate::network::verify::require_active_identity;
use crate::types::AgentId;

/// Simulated connection endpoint. Not a real network address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntryV0 {
    pub agent_id: AgentId,
    pub endpoint: String,
    pub registered_at: u64,
}

#[derive(Debug, Default)]
pub struct AgentDirectoryV0 {
    entries: HashMap<AgentId, DirectoryEntryV0>,
}

impl AgentDirectoryV0 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an agent endpoint. Requires active PROTO-0 identity.
    pub fn register(
        &mut self,
        registry: &IdentityRegistry,
        agent_id: &str,
        endpoint: &str,
        now: u64,
    ) -> Result<DirectoryEntryV0> {
        require_active_identity(registry, agent_id)?;
        if endpoint.is_empty() {
            return Err(Error::MalformedObject("empty endpoint"));
        }
        let entry = DirectoryEntryV0 {
            agent_id: agent_id.into(),
            endpoint: endpoint.into(),
            registered_at: now,
        };
        self.entries.insert(agent_id.into(), entry.clone());
        Ok(entry)
    }

    pub fn lookup(&self, agent_id: &str) -> Result<&DirectoryEntryV0> {
        self.entries.get(agent_id).ok_or(Error::AgentNotInDirectory)
    }

    pub fn contains(&self, agent_id: &str) -> bool {
        self.entries.contains_key(agent_id)
    }

    pub fn unregister(&mut self, agent_id: &str) -> Result<()> {
        if self.entries.remove(agent_id).is_none() {
            return Err(Error::AgentNotInDirectory);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
