use std::sync::Arc;

use aether_core::capability::grant::CapabilityGrant;
use aether_core::capability::model::{CapabilityStore, CapabilityV0};
use aether_core::escrow::store::EscrowStore;
use aether_core::identity::registry::IdentityRegistry;
use aether_core::reputation::derive::ReputationIndexer;
use aether_core::reputation::store::{ReputationStore, StoreMode};
use aether_core::settlement::store::SettlementStore;
use aether_enterprise_demo::enterprise_demo::harness::EnterpriseWorld;
use aether_enterprise_demo::enterprise_demo::runner::build_observatory_bootstrap;
use chrono::Utc;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ObservationIndex {
    pub agent_ids: Vec<String>,
    pub capability_ids: Vec<[u8; 32]>,
    pub escrow_ids: Vec<[u8; 32]>,
    pub settlement_binding_ids: Vec<[u8; 32]>,
    pub account_binding_ids: Vec<[u8; 32]>,
}

pub struct ProtocolState {
    pub registry: IdentityRegistry,
    pub caps: CapabilityStore,
    pub escrows: EscrowStore,
    pub settlements: SettlementStore,
    pub reputation: ReputationStore,
    pub index: ObservationIndex,
    pub loaded_at: chrono::DateTime<Utc>,
}

impl ProtocolState {
    pub fn bootstrap() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::bootstrap_owned()?))
    }

    /// Owned bootstrap for mutation tests / exclusive adapter calls.
    pub fn bootstrap_owned() -> Result<Self> {
        let bootstrap = build_observatory_bootstrap();
        let escrow_id_opt = bootstrap.escrow_id;
        let settlement_binding_id = bootstrap.settlement_binding_id;
        let mut state = Self::from_world(bootstrap.world)?;
        if let Some(escrow_id) = escrow_id_opt {
            state.index.escrow_ids.push(escrow_id);
            if let Some(record) = state.escrows.get(&escrow_id) {
                let _ = ReputationIndexer::ingest_escrow(
                    &mut state.reputation,
                    &record.escrow,
                    &state.registry,
                );
            }
        }
        if let Some(binding_id) = settlement_binding_id {
            state.index.settlement_binding_ids.push(binding_id);
            if let (Some(binding), Some(escrow_id)) =
                (state.settlements.get_settlement(&binding_id), escrow_id_opt)
            {
                if let Some(record) = state.escrows.get(&escrow_id) {
                    let _ = ReputationIndexer::ingest_settlement(
                        &mut state.reputation,
                        binding,
                        &record.escrow,
                        &state.registry,
                    );
                }
            }
        }
        Ok(state)
    }

    fn from_world(world: EnterpriseWorld) -> Result<Self> {
        let index = build_index(&world);
        Ok(Self {
            registry: world.registry,
            caps: world.caps,
            escrows: world.escrows,
            settlements: world.settlements,
            reputation: ReputationStore::new(
                "control-plane-indexer-0".into(),
                StoreMode::LocalOnly,
            ),
            index,
            loaded_at: Utc::now(),
        })
    }
}

fn build_index(world: &EnterpriseWorld) -> ObservationIndex {
    let agent_ids = vec![
        world.enterprise.identity.derived_agent_id(),
        world.agent.identity.derived_agent_id(),
        world.provider.identity.derived_agent_id(),
    ];

    let capability_ids = [
        &world.enterprise_grant,
        &world.provider_grant,
        &world.agent_delegate_grant,
    ]
    .iter()
    .filter_map(|grant| capability_id_from_grant(grant).ok())
    .collect();

    let account_binding_ids = vec![
        world.enterprise_account.binding_id,
        world.provider_account.binding_id,
    ];

    ObservationIndex {
        agent_ids,
        capability_ids,
        escrow_ids: Vec::new(),
        settlement_binding_ids: Vec::new(),
        account_binding_ids,
    }
}

fn capability_id_from_grant(grant: &CapabilityGrant) -> Result<[u8; 32]> {
    let cap =
        CapabilityV0::decode(&grant.message.body).map_err(|e| Error::Protocol(format!("{e:?}")))?;
    cap.capability_id()
        .map_err(|e| Error::Protocol(format!("{e:?}")))
}
