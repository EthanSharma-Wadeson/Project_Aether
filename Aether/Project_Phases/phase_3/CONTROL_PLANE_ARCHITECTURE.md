# Control Plane Architecture

## Status

**Architecture design — not implementation**

The Control Plane is a human-facing interface for observing, configuring, and governing autonomous AI agents operating through Aether protocol layers.

**Authority invariant:** The Control Plane observes protocol state and configures policy through PROTO-0 capability issuance. It never bypasses, mutates, or replaces protocol authority.

---

## 1. User Types

### 1.1 Enterprise Operator

Responsible for governing agent fleets within an organisation.

| Need | Control Plane capability |
|------|------------------------|
| Agent inventory | View all registered agents, their status, and capabilities |
| Spending limits | Configure max_spend constraints on capability grants |
| Capability policies | Define templates for allowed actions, expiry windows, delegation depth |
| Settlement monitoring | Track escrow lifecycle, settlement finality, failure rates |
| Compliance visibility | Audit trails of capability issuance, revocation, and policy changes |

**Workflow:** Operator defines a policy → Control Plane issues PROTO-0 capabilities matching the policy → agents operate within those constraints → operator monitors outcomes.

### 1.2 AI Researcher

Studies agent behaviour, economic dynamics, and protocol performance.

| Need | Control Plane capability |
|------|------------------------|
| Agent behaviour analysis | Query historical event streams per agent |
| Reputation metrics | Explore `AgentMetricsV0` — completion rates, dispute rates, settlement latency |
| Economic activity graphs | Visualise escrow flows, settlement volumes, fee distribution |
| Protocol event exploration | Inspect individual `ReputationEventV0`, escrow terminals, settlement bindings |
| Experiments | Compare agent populations, simulate policy changes on historical data |

**Workflow:** Researcher queries read-only indexes → explores evidence → exports datasets → publishes findings. No write path.

### 1.3 Agent Developer

Builds and debugs agents that interact with Aether protocols.

| Need | Control Plane capability |
|------|------------------------|
| Capability debugging | Inspect why an action was rejected (capability chain, expiry, scope) |
| Receipt inspection | View `SettlementReceiptV0` details, verify signatures |
| Settlement tracing | Follow a settlement from escrow terminal through binding to finality |
| Network diagnostics | Inspect session establishment, envelope delivery, replay rejections |

**Workflow:** Developer deploys agent → observes protocol interactions → identifies issues → adjusts agent configuration.

---

## 2. Dashboard Views

### 2.1 Agent Observatory

Primary view for understanding individual agent state.

| Element | Source | Read/Write |
|---------|--------|------------|
| AgentId + identity status | PROTO-0 `IdentityRegistry` | Read |
| Active capabilities | PROTO-0 `CapabilityStore` | Read |
| Current escrows (non-terminal) | PROTO-2 `EscrowStore` | Read |
| Economic history (terminal escrows) | PROTO-2 `EscrowStore` | Read |
| Settlement bindings | PROTO-4 `SettlementStore` | Read |
| Reputation metrics | PROTO-3 `ReputationStore` | Read |
| Network sessions | PROTO-NET-0 `NetworkStore` | Read |

**No write operations.** Agent Observatory is purely observational.

### 2.2 Policy Console

Allows humans to define and manage agent policies.

| Operation | Protocol effect | Authority path |
|-----------|----------------|----------------|
| Create policy template | Stored in Control Plane DB | None (local) |
| Apply policy to agent | Issues `CapabilityGrant` via PROTO-0 | PROTO-0 `grant_capability` |
| Revoke capability | Revokes grant via PROTO-0 | PROTO-0 `revoke_capability` |
| Freeze agent | Freezes identity via PROTO-0 | PROTO-0 `freeze_identity` |
| Update spend limit | Issues new capability with updated constraints | PROTO-0 `grant_capability` |

**Critical design rule:** Policies are templates. Applying a policy translates to PROTO-0 capability operations. The Control Plane never bypasses capability enforcement — it uses the same API that agents use.

Policy parameters:

| Parameter | Maps to |
|-----------|---------|
| `max_spend` | `CapabilityV0.constraints.max_spend` |
| `allowed_actions` | `CapabilityV0.actions` |
| `valid_before` | `CapabilityV0.constraints.valid_before` |
| `valid_after` | `CapabilityV0.constraints.valid_after` |
| `counterparties` | `CapabilityV0.constraints.counterparties` |
| `rate_limit` | `CapabilityV0.constraints.rate_limit` |
| `max_delegation_depth` | `CapabilityV0.delegation_depth` |
| `asset` | `CapabilityV0.constraints.asset` |

### 2.3 Economic Monitor

Real-time and historical view of economic activity.

| View | Data source | Granularity |
|------|-------------|-------------|
| Active escrows | PROTO-2 `EscrowStore` (non-terminal) | Per-escrow |
| Terminal outcomes | PROTO-2 `EscrowStore` (Released/Refunded/Expired) | Per-escrow |
| Settlement lifecycle | PROTO-4 `SettlementStore` | Per-binding |
| Fee analysis | PROTO-2 `FeeLedger` | Per-escrow |
| Finality status | `EconomicFinalityViewV0` | Per-escrow |
| Asset breakdown | Escrow terms + settlement bindings | Per-asset |

Aggregate views:

- Total value in escrow (by asset)
- Settlement success rate (per agent, per asset, per time window)
- Average settlement latency
- Dispute rate trends
- Fee efficiency

### 2.4 Reputation Explorer

Surfaces PROTO-3 reputation data without introducing subjective scores.

| View | Data source |
|------|-------------|
| Agent metrics | `AgentMetricsV0` via `ReputationQueryV0` |
| Event history | `ReputationEventV0` list per agent |
| Evidence references | `EvidenceRefV0` commitments per event |
| Counterparty diversity | `dyad_counts` / `max_dyad_concentration()` |
| Trust evolution | Metrics over time via `as_of` queries |

**No global trust score.** The explorer presents deterministic metrics. Interpretation is the user's responsibility.

Enterprise mode: when PROTO-3 runs in `LocalOnly` mode, the Reputation Explorer shows only the local index. No public data is exposed.

### 2.5 Research Lab

Experimentation and analysis environment.

| Capability | Implementation approach |
|-----------|----------------------|
| Historical queries | Time-bounded queries against event indexes |
| Agent comparison | Side-by-side metrics for multiple agents |
| Protocol event inspection | Drill into individual escrow/settlement/channel events |
| Dataset export | Export filtered event streams as structured data |
| Policy simulation | "What if" analysis: apply policy rules to historical data (read-only) |

The Research Lab has **no write path** to any protocol store.

---

## 3. Data Architecture

### 3.1 Read Paths

```text
Control Plane
    │
    ├── PROTO-0 IdentityRegistry     → agent status, identity details
    ├── PROTO-0 CapabilityStore       → active/revoked capabilities
    ├── PROTO-1 ChannelStore          → channel states, dispute outcomes
    ├── PROTO-2 EscrowStore           → escrow lifecycle, terminals
    ├── PROTO-2 FeeLedger             → fee consumption
    ├── PROTO-4 SettlementStore       → settlement bindings, finality
    ├── PROTO-3 ReputationStore       → events, metrics, evidence refs
    └── PROTO-NET-0 NetworkStore      → sessions, directories
```

All reads are against **immutable snapshots or append-only logs**. The Control Plane cannot modify these stores.

### 3.2 Write Paths

```text
Control Plane
    │
    ▼
Policy Decision (local to Control Plane)
    │
    ▼
PROTO-0 API
    ├── grant_capability()
    ├── revoke_capability()
    ├── freeze_identity()
    ├── revoke_identity()
    └── update_permission_root()
```

The Control Plane's **only write path** to the Aether protocol stack is through PROTO-0. Every write operation is subject to the same authorisation rules as any other caller:

- Signature verification
- Capability chain validation
- Permission root binding
- Delegation depth enforcement

### 3.3 Control Plane Internal Storage

The Control Plane maintains its own state for:

| Data | Purpose | Protocol interaction |
|------|---------|---------------------|
| Policy templates | Reusable capability configurations | None until applied |
| Audit log | Record of all policy changes and capability operations | Append-only |
| User accounts | Dashboard authentication and RBAC | None |
| Dashboard preferences | Layout, filters, saved queries | None |
| Alert rules | Threshold-based notifications | Read from protocol stores |

This internal state is **not protocol state**. It is application configuration. Loss of Control Plane state does not affect protocol operation.

### 3.4 Event Indexing

The Control Plane builds read-optimised indexes from protocol stores:

```text
Protocol stores (source of truth)
    │
    ▼
Event indexing pipeline (read-only)
    │
    ▼
Control Plane indexes (derived, rebuildable)
    │
    ▼
Dashboard queries
```

Indexes are derived and can be rebuilt from protocol stores at any time. They are cache, not authority.

---

## 4. Security Model

### 4.1 What the Control Plane CAN Do

| Action | Mechanism | Authority source |
|--------|-----------|-----------------|
| Display agent identity and status | Read PROTO-0 registry | None required (read) |
| Display escrow and settlement state | Read PROTO-2/4 stores | None required (read) |
| Display reputation metrics | Query PROTO-3 store | None required (read) |
| Create policy templates | Write to CP internal DB | CP authentication |
| Issue capabilities via policy | Call PROTO-0 `grant_capability` | Requires valid signing key + parent capability |
| Revoke capabilities | Call PROTO-0 revoke | Requires valid signing key |
| Freeze/revoke identity | Call PROTO-0 freeze/revoke | Requires valid signing key |
| Export datasets | Read protocol stores | None required (read) |
| Configure alerts | Write to CP internal DB | CP authentication |

### 4.2 What the Control Plane CANNOT Do

| Forbidden action | Why | Enforcement |
|------------------|-----|-------------|
| Bypass capability checks | PROTO-0 enforces capability chain before any action | Protocol-level; CP has no backdoor |
| Modify escrow balances | `EscrowStore` mutations only via transition functions | `pub(crate)` visibility on store internals |
| Change settlement status | `SettlementStore` mutations only via transition functions | `pub(crate)` visibility |
| Fabricate reputation events | `ReputationStore.append()` verifies `event_id` + requires evidence | Deterministic event_id + evidence requirement |
| Override dispute outcomes | PROTO-1/2 dispute resolution follows protocol rules | No CP input to resolution logic |
| Grant capabilities beyond its own authority | PROTO-0 delegation narrowing enforced | Children cannot expand parent scope |
| Create identities without valid keys | PROTO-0 requires Ed25519 key pair | Cryptographic enforcement |

### 4.3 Operator Authority Model

```text
Root Principal (human key holder)
    │
    ▼
Control Plane Operator (delegated capability)
    │
    ▼
Agent Fleet (further-delegated capabilities)
```

The Control Plane operator holds a **delegated capability** from the root principal. The operator can issue sub-capabilities to agents, but cannot exceed the scope of their own delegation. This is enforced by PROTO-0 delegation narrowing.

---

## 5. API Design (Conceptual)

These are **design-phase specifications only**. No implementation.

### 5.1 Agent APIs

```
GET  /agents                          → list registered agents
GET  /agents/{id}                     → agent identity, status, summary
GET  /agents/{id}/capabilities        → active capability grants
GET  /agents/{id}/escrows             → escrow history (filterable by status)
GET  /agents/{id}/settlements         → settlement bindings
GET  /agents/{id}/reputation          → AgentMetricsV0 + evidence refs
GET  /agents/{id}/events              → protocol event timeline
```

### 5.2 Policy APIs

```
GET    /policies                      → list policy templates
POST   /policies                      → create policy template
GET    /policies/{id}                 → policy details
PUT    /policies/{id}                 → update template
DELETE /policies/{id}                 → archive template
POST   /policies/{id}/apply           → issue capabilities to target agent(s)
```

### 5.3 Capability APIs

```
POST   /capabilities/grant            → issue capability (via PROTO-0)
POST   /capabilities/revoke           → revoke capability (via PROTO-0)
GET    /capabilities/{id}             → capability details and chain
```

### 5.4 Economic APIs

```
GET  /escrows                         → list escrows (filterable)
GET  /escrows/{id}                    → escrow details + finality view
GET  /settlements                     → list settlement bindings
GET  /settlements/{id}                → binding details + finality
GET  /economics/summary               → aggregate economic metrics
```

### 5.5 Reputation APIs

```
GET  /reputation/{agent_id}           → AgentMetricsV0
GET  /reputation/{agent_id}/events    → ReputationEventV0 list
GET  /reputation/{agent_id}/evidence  → EvidenceRefV0 list
GET  /reputation/compare              → side-by-side metrics (query params)
```

### 5.6 Research APIs

```
GET  /research/events                 → filtered protocol event stream
GET  /research/export                 → dataset export (CSV/JSON)
GET  /research/simulate               → policy simulation against historical data
```

### 5.7 System APIs

```
GET  /system/health                   → Control Plane health
GET  /system/audit                    → audit log of CP operations
GET  /system/indexes/status           → index freshness and rebuild status
```

### Authentication

All API calls require Control Plane authentication (JWT or similar). Write operations additionally require a valid PROTO-0 signing key for the target capability chain.

---

## 6. Integration Map

### Protocol → Control Plane (read-only)

| Protocol layer | Data consumed | Update frequency |
|---------------|---------------|-----------------|
| PROTO-0 | Identity registry, capability store | On change (event-driven) |
| PROTO-1 | Channel states, dispute outcomes | On close/dispute |
| PROTO-2 | Escrow lifecycle, receipts, terminals | On state transition |
| PROTO-4 | Settlement bindings, finality | On status change |
| PROTO-3 | Reputation events, metrics | On ingest |
| PROTO-NET-0 | Session metadata (optional) | On session events |

### Control Plane → Protocol (write, PROTO-0 only)

| Operation | PROTO-0 function | When |
|-----------|-----------------|------|
| Apply policy | `grant_capability` | Operator action |
| Revoke capability | Capability store revoke | Operator action |
| Freeze agent | `freeze_identity` | Emergency or policy |
| Revoke identity | `revoke_identity` | Emergency |
| Update permission root | `update_permission_root` | Key rotation |

---

## 7. Deployment Modes

### Enterprise (default)

- Single-tenant Control Plane
- All data local
- PROTO-3 in `LocalOnly` mode
- No public APIs
- Integration with enterprise IAM (SSO/SAML)

### Research

- Read-only Control Plane
- No capability issuance
- Full protocol event access
- Dataset export enabled

### Developer

- Local instance
- Connected to test protocol stores
- Full read + limited write (test capabilities)
- Protocol event debugger

---

## 8. Implementation Scope (Frozen)

When implementation is approved, Phase 3 includes:

| Component | Scope |
|-----------|-------|
| API server | REST endpoints for all conceptual APIs |
| Protocol readers | Read-only adapters for each protocol store |
| Policy engine | Template → capability translation |
| Event indexer | Protocol event → dashboard-queryable index |
| Dashboard views | Agent Observatory, Policy Console, Economic Monitor, Reputation Explorer, Research Lab |
| Authentication | CP-level auth + PROTO-0 key integration |
| Audit logging | Append-only log of all write operations |

Phase 3 does **not** include:

- Marketplace
- Public agent directory
- Social reputation
- Governance DAO
- Token incentives
- AETH implementation
- Production cloud infrastructure

---

## Freeze Statement

> The Control Plane **observes protocol state** and **configures policy** through PROTO-0 capability issuance. It is never an authority layer. Loss of the Control Plane does not affect protocol operation. Implementation requires explicit approval.
