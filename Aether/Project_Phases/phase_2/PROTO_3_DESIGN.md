# PROTO-3 Design — Evidence-Based Reputation Layer

## Status

**Design complete — acceptance tests frozen — implementation not authorised**

PROTO-3 defines how autonomous agents evaluate counterparty trust using **only deterministic, cryptographically attributable protocol evidence**. It is **not** social reputation, reviews, voting, or marketplace ranking.

References:

- [PROTO_3_DECISIONS.md](PROTO_3_DECISIONS.md)
- [PROTO_3_ACCEPTANCE_TESTS.md](PROTO_3_ACCEPTANCE_TESTS.md)
- [PROTO_3_THREAT_MODEL.md](PROTO_3_THREAT_MODEL.md)
- [REPUTATION_SYSTEM.md](../../Aether_docs/REPUTATION_SYSTEM.md)
- [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) — DEC-P2-004

**Stop:** No Rust until explicit implementation approval. PROTO-3 must not modify PROTO-0, PROTO-1, PROTO-2, PROTO-4, or PROTO-NET-0.

---

## 1. Primary Question

> How can autonomous agents evaluate whether another agent is trustworthy using only protocol evidence?

**Answer:** Counterparties query a **reputation view** computed from **verified `ReputationEventV0` records** derived from signed protocol artifacts (escrow terminals, settlement bindings, channel closes, capability outcomes). The view exposes **deterministic metrics** and **evidence commitments** — not opaque scores or human opinions. Applications may apply policy thresholds; PROTO-3 does not mandate a single global trust number.

---

## 2. Architectural Position

```text
┌─────────────────────────────────────────────────────────────┐
│  Application / Control Plane (policy thresholds)            │
└───────────────────────────┬─────────────────────────────────┘
                            │ queries
┌───────────────────────────▼─────────────────────────────────┐
│  PROTO-3 — Reputation Indexer (read-only)                   │
│  ingest · verify · derive events · compute metrics          │
└───────────────────────────┬─────────────────────────────────┘
                            │ consumes (never mutates)
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   PROTO-0            PROTO-1/2            PROTO-4 / NET-0
   identity           escrow/channel       settlement / transport
   capabilities       terminals            evidence
```

### Authority rule (normative)

| Layer | Role |
|-------|------|
| **PROTO-0** | **Sole authority** for identity, capabilities, authorisation |
| PROTO-1/2/4 | Economic and settlement truth |
| **PROTO-3** | **Non-authoritative indexer** — derives views from evidence |

PROTO-3 **never** grants capabilities, releases escrow, or promotes settlement finality.

---

## 3. Design Goals

PROTO-3 answers operational questions:

| Question | Metric / evidence |
|----------|-------------------|
| Has this agent completed work? | `escrow_completed_count`, terminal `Released` events |
| How often does it dispute? | `dispute_initiated_rate`, `dispute_lost_count` |
| Does it timeout? | `escrow_expired_count`, `fund_timeout_count` |
| Does it honour settlements? | `settlement_finalized_rate`, `settlement_failed_count` |
| Reliable for delegation? | Composite policy on metrics + `identity_status` from PROTO-0 |

### Non-goals

- Star ratings, likes, followers, popularity
- Manual reviews or human jury default
- Consensus-canonical global score (v0)
- Modifying any lower-layer protocol object

---

## 4. Reputation Event Model

### 4.1 Principle

Protocols **do not emit reputation events directly**. Indexers **derive** `ReputationEventV0` when verifiable protocol artifacts satisfy event rules.

```text
Signed protocol artifact  →  verification  →  ReputationEventV0  →  metrics
```

### 4.2 Event categories (v0)

| Event type | Derived from | Subject agent(s) |
|------------|--------------|-------------------|
| `escrow.funded` | Funded `EscrowV0` + signed funding | payer |
| `escrow.receipt_accepted` | ReceiptAccepted + signed receipt | provider |
| `escrow.released` | Terminal `Released` + release signature | payer (action), provider (credit) |
| `escrow.refunded` | Terminal `Refunded` | payer (credit), provider (fault signal context) |
| `escrow.expired` | Expired/forfeit per PROTO-2 rules | attributed per terms |
| `escrow.dispute_raised` | Dispute record | initiator |
| `escrow.dispute_resolved` | Resolution outcome | winner/loser attribution |
| `settlement.requested` | `SettlementBindingV0` Requested | request signer |
| `settlement.finalized` | Finalized binding + hard finality | payer/provider |
| `settlement.failed` | Failed / `DisputedExternal` | binding parties |
| `channel.closed_cooperative` | PROTO-1 cooperative close | participants |
| `channel.dispute_lost` | PROTO-1 dispute outcome | fault party |
| `capability.denied` | Observed reject (optional ingest) | actor |
| `identity.revoked` | PROTO-0 registry status change | revoked agent |
| `integrity.replay_rejected` | PROTO-2/NET-0 replay reject (optional) | offender |

**Not events:** capability grants, session hellos, non-terminal escrow states, adapter queries without binding.

### 4.3 Event identity and versioning

Each `ReputationEventV0` includes:

| Field | Purpose |
|-------|---------|
| `protocol_version` / `schema_version` | Event schema version |
| `event_id` | `SHA-256(canonical body without event_id)` |
| `event_type` | Namespaced string, e.g. `aether.reputation.escrow.released.v0` |
| `subject_agent_id` | Agent whose reputation is affected |
| `counterparty_agent_id` | Optional other party |
| `logical_time` | From source artifact |
| `evidence_refs` | List of verifiable commitments (see §5) |
| `attribution` | `credit` \| `debit` \| `neutral` \| `fault` |
| `weight_hint` | Optional economic weight (principal amount) for aggregation |

**Versioning:** New event types are additive minor versions. Breaking attribution rules require new major version. Indexers declare supported event schema versions.

### 4.4 Deduplication

`event_id` is deterministic from evidence. Re-ingesting the same escrow terminal produces the same event — idempotent indexing.

---

## 5. Reputation Evidence

### 5.1 Evidence ref structure

```text
EvidenceRefV0 {
  ref_type:     escrow_terminal | settlement_binding | receipt | channel_close | identity_status
  commitment:   bytes[32]        // SHA-256 canonical snapshot or object id
  source_proto: PROTO-0 | PROTO-1 | PROTO-2 | PROTO-4 | PROTO-NET-0
  locator:      optional opaque  // escrow_id, binding_id, channel_id
}
```

### 5.2 What proves an event occurred

| Event | Minimum evidence |
|-------|------------------|
| `escrow.released` | Dual-signed terms + terminal `EscrowV0` + signed release + escrow_id commitment |
| `settlement.finalized` | `SettlementBindingV0` at Finalized + fresh adapter evidence hash + matching escrow terminal commitment |
| `escrow.dispute_resolved` | Dispute record + resolution signature + terminal status |
| `identity.revoked` | Registry snapshot or signed revoke notice per PROTO-0 rules |

Indexer **must** re-verify:

1. Signatures and domain separation
2. Identity active at event time (or record revoked-after)
3. Cross-object consistency (amounts, parties, outcomes)
4. No contradictory terminal state

### 5.3 Forbidden evidence

- Self-asserted JSON without signature
- Platform review text
- Off-protocol star ratings
- Unsigned adapter claims
- Events referencing non-terminal escrow as success

---

## 6. Reputation Store

### 6.1 Model: derived with materialised cache

| Approach | PROTO-3 v0 |
|----------|------------|
| Pure on-demand | Too slow for hire-time queries |
| **Materialised index + recomputation** | **Chosen** |
| Consensus-canonical store | Deferred (DEC-P2-004 upgrade path C) |

```text
Ingest pipeline:
  artifact batch  →  verify  →  derive events  →  append-only event log
                                                      ↓
                                            materialised AgentMetricsV0
```

### 6.2 Recomputation

Any party with the event log and source artifacts can **recompute** metrics deterministically:

```text
metrics(agent_id, as_of_time, policy_id) = fold(verified_events, policy)
```

**Recompute triggers:**

- New evidence ingested
- Policy version change
- Identity merge/split (none in v0 — see lifecycle)
- Manual full rebuild (auditor)

Materialised views are **cache**, not authority.

### 6.3 Indexer trust

Indexers compete on correctness and freshness. Query responses include:

- `indexer_id`
- `computed_at`
- `event_log_root` (Merkle or hash chain over event_ids)
- `metrics`
- `evidence_refs` for sample or full export

Verifiers spot-check by re-deriving events from cited evidence.

---

## 7. Trust Model — Metrics, Not Mandated Scores

### 7.1 No required single score

PROTO-3 exposes **`AgentMetricsV0`** — a vector of deterministic counters and rates:

| Metric | Definition |
|--------|------------|
| `escrow_completed` | Count `escrow.released` with verified receipt path |
| `escrow_refunded` | Count terminal refunds attributed as provider fault |
| `dispute_rate` | disputes / (completed + disputed) |
| `settlement_success_rate` | finalized / (finalized + failed) |
| `median_settlement_latency` | `finalize_time - release_time` |
| `total_settled_value` | Sum principal (per asset, separate columns) |
| `replay_reject_count` | Integrity rejects |
| `identity_age` | First seen event time |
| `last_active` | Latest event time |
| `protocol_versions_seen` | Set of versions in evidence |

Applications define thresholds: e.g. `escrow_completed >= 5 AND dispute_rate < 0.05`.

### 7.2 Optional reference scorer

A **reference policy** (`aether.reputation.policy.v0`) may produce a weighted index for convenience. It is **not normative** — alternate policies are allowed if they use the same event set.

---

## 8. Anti-Gaming (Design)

| Attack | Mitigation |
|--------|------------|
| Self-transactions | Same-principal graph detection; zero weight for same `principal_root` (OPEN heuristic) |
| Fake agents | Events require valid PROTO-0 signatures |
| Circular settlements | Counterparty diversity metrics; dampen repeated dyad cycles |
| Replay | Event dedup by `event_id`; integrity events |
| Collusion | Economic weight by principal; bond requirements (future BondV0) |
| Sybil | Cold-start limits; bonds external to PROTO-3; diversity signals |

See [PROTO_3_THREAT_MODEL.md](PROTO_3_THREAT_MODEL.md) for VALIDATED vs OPEN.

---

## 9. Reputation Lifecycle

| Event | Effect |
|-------|--------|
| Identity revoked | Stop new positive events; mark `identity_status=revoked`; historical events remain |
| Key rotation | No reputation transfer — same `AgentId` continues |
| Capability expiry | No direct reputation change |
| Protocol version upgrade | Indexer supports N and N-1 event schemas; old events remain valid |
| Settlement adapter differs | Metrics tagged by `settlement_provider`; cross-adapter aggregation is policy |

**Whitewashing:** New `AgentId` starts empty — by design. Cost to appear trusted = real completed work.

---

## 10. Privacy

| Data | Default exposure |
|------|------------------|
| Aggregate metrics | Public query API |
| Per-event evidence refs | Available to querier with evidence bundle |
| Full escrow terms | **Not** in reputation API — commitment only |
| Enterprise fleet | May use **private index** with aggregate export |

Enterprises may expose only: `completion_rate`, `dispute_rate` without counterparty graph.

Selective disclosure (PROTO-5 stretch): prove `metric >= threshold` without full history.

---

## 11. Enterprise vs Open Network

| Mode | Behaviour |
|------|-----------|
| **Enterprise (default)** | Reputation **disabled** or **local-only** index behind VPN; no public queries |
| **Open network** | Public indexer(s); agents query before hire |
| **Hybrid** | Export signed metric checkpoint from private index |

PROTO-3 is **optional** for enterprise spend control wedge (per PHASE_2_WEDGE_DECISION).

---

## 12. Integration Map

### PROTO-0 (read-only)

- Identity registry status (`Active`, `Frozen`, `Revoked`)
- Capability deny outcomes (optional ingest)
- **Never writes** to registry or capability store

### PROTO-1 (read-only)

- Cooperative channel close records
- Dispute outcomes
- Terminal channel balances as evidence context

### PROTO-2 (read-only)

- Terminal escrow states: `Released`, `Refunded`, `Resolved*`
- Signed receipts, releases, disputes
- `EconomicFinalityViewV0` snapshots

### PROTO-4 (read-only)

- `SettlementBindingV0` lifecycle through `Finalized` / `Failed` / `DisputedExternal`
- Evidence commitments; **hard finality** required for `settlement.finalized` credit

### PROTO-NET-0 (read-only)

- Message delivery metadata (optional)
- Replay rejection records
- **Never** treats transport as economic truth

---

## 13. Data Models (Design)

### `ReputationEventV0`

Canonical derived event (see §4.3).

### `AgentMetricsV0`

Materialised metric snapshot per `(agent_id, policy_id, as_of)`.

### `ReputationQueryV0`

```text
subject: AgentId
as_of: logical_time
policy_id: optional
dimensions: [metric names]
include_evidence: bool
```

### `ReputationQueryResultV0`

```text
subject, metrics, event_log_root, indexer_id, computed_at
evidence_refs[]  // mandatory if include_evidence
```

---

## 14. What PROTO-3 Does Not Solve

- Output correctness or AI safety
- Legal enforceability of work quality
- Global Sybil resistance without bonds (external)
- Trust in indexer honesty without verification
- Cross-platform identity merge
- Subjective “good provider” judgement

---

## 15. Implementation Gate

Implementation requires:

1. Frozen acceptance tests ([PROTO_3_ACCEPTANCE_TESTS.md](PROTO_3_ACCEPTANCE_TESTS.md))
2. Explicit approval after enterprise wedge acceptance
3. Adversarial suite green
4. SECURITY_MODEL.md reputation section update

---

## Freeze Statement

> PROTO-3 is a **read-only evidence indexer**. Authority remains PROTO-0. Reputation is **optional** for enterprise deployments. Metrics are **deterministic**; scores are **application policy**, not protocol law.
