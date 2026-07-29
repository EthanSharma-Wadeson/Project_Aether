# PROTO-3 Acceptance Tests — Phase 2

## Status

**Frozen acceptance-test specification — required before PROTO-3 implementation**

Sources of truth:

- [PROTO_3_DESIGN.md](PROTO_3_DESIGN.md)
- [PROTO_3_DECISIONS.md](PROTO_3_DECISIONS.md)
- [PROTO_3_THREAT_MODEL.md](PROTO_3_THREAT_MODEL.md)
- [PROTO_2_RESULTS.md](../phase_1/PROTO_2_RESULTS.md)
- [PROTO_4_RESULTS.md](PROTO_4_RESULTS.md)
- [PROTO_NET_0_RESULTS.md](PROTO_NET_0_RESULTS.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)

This document does **not** contain implementation.  
It does **not** authorise marketplace, dashboard, governance, AETH, or modifications to PROTO-0/1/2/4/NET-0.

---

## 1. Purpose

PROTO-3 acceptance tests freeze what the reputation indexer prototype must prove:

- reputation derives **only** from verifiable protocol evidence
- event derivation is **deterministic** and **idempotent**
- metrics recompute identically from event log + evidence
- dispute and fault attribution follow PROTO-2/1 outcomes
- self-rating and unsigned events are rejected
- wash/collusion scenarios are detected or down-weighted per policy
- revoked identities stop accruing positive credit
- settlement failures do not produce success credit
- timeouts and expiry produce attributed events
- cross-adapter settlement events remain consistent
- enterprise-local indexes work without public exposure
- PROTO-3 never mutates authority layers

---

## 2. Security boundaries (normative)

### PROTO-3 owns

- `ReputationEventV0` derivation rules
- `AgentMetricsV0` computation
- Event log integrity (`event_id` dedup)
- Query response shape with evidence refs

### PROTO-3 does NOT own

- Identity, capabilities, escrow, settlement state
- Transport authorisation
- Any write path to PROTO-0/1/2/4/NET-0 stores

### Forbidden

```text
reputation query     →  grant capability
reputation metric    →  release escrow
unsigned event       →  positive metric credit
Confirmed-only       →  settlement.finalized credit (P3-DEC-008)
self-asserted review →  any event
```

---

## 3. Harness assumptions

- Reuse PROTO-0/2/4 test harness patterns (in-memory stores)
- Deterministic logical `now`
- Indexer under test: `ReputationIndexerV0` (name for spec only — not implemented)
- Reference policy: `aether.reputation.policy.v0`
- Optional NET fixtures for replay-reject events
- Enterprise mode flag: `local_only_index`

Test ID prefixes:

| Prefix | Category |
|--------|----------|
| `P3-T###` | Lifecycle / generation |
| `P3-R###` | Recomputation / determinism |
| `P3-A###` | Adversarial / anti-gaming |
| `P3-I###` | PROTO integration (read-only) |
| `P3-E###` | Enterprise-local index |
| `P3-V###` | Protocol version / adapter |
| `P3-M###` | Metrics correctness |

---

## 4. Lifecycle tests (`P3-T###`)

### PASS — successful reputation generation

| ID | Case | Expected |
|----|------|----------|
| P3-T001 | Ingest happy-path escrow release + receipt | `escrow.released` event for provider; `escrow_completed` +1 |
| P3-T002 | Ingest refund terminal path | `escrow.refunded` events; fault attribution per rules |
| P3-T003 | Ingest PROTO-4 finalized settlement after hard finality | `settlement.finalized` for both parties |
| P3-T004 | Ingest cooperative channel close | `channel.closed_cooperative` for participants |
| P3-T005 | Multiple escrows same agent | Cumulative metrics; distinct `event_id`s |
| P3-T006 | Dispute raised and resolved (provider fault) | `dispute_lost` metric for provider |
| P3-T007 | Dispute resolved (payer fault) | `dispute_lost` metric for payer |
| P3-T008 | Escrow fund timeout / expiry | `escrow.expired` with correct attribution |
| P3-T009 | Settlement failed before finalize | `settlement.failed`; no finalized credit |
| P3-T010 | Identity revoked after history | Historical events retained; no new positive events |

### FAIL — invalid generation

| ID | Case | Expected |
|----|------|----------|
| P3-T011 | Ingest non-terminal escrow as released | Reject event derivation |
| P3-T012 | Ingest forged release signature | Reject |
| P3-T013 | Ingest settlement Confirmed without Finalized | No `settlement.finalized` credit |
| P3-T014 | Ingest event with tampered evidence commitment | Reject |
| P3-T015 | Duplicate ingest same terminal escrow | Idempotent — single event |
| P3-T016 | Ingest mismatched payer/provider in evidence | Reject |
| P3-T017 | Ingest after forged capability grant | Reject (artifact invalid) |

---

## 5. Recomputation tests (`P3-R###`)

| ID | Case | Expected |
|----|------|----------|
| P3-R001 | Recompute metrics from empty log | Zero metrics |
| P3-R002 | Recompute from full event log | Identical to materialised cache |
| P3-R003 | Recompute after policy version change | Deterministic per policy id |
| P3-R004 | Two indexers same inputs | Identical `event_log_root` and metrics |
| P3-R005 | Partial re-ingest after crash | Idempotent; no duplicate events |
| P3-R006 | Recompute `as_of` before last event | Metrics exclude future events |
| P3-R007 | Evidence spot-check 10 random events | All verify against PROTO artifacts |
| P3-R008 | Rebuild index from exported evidence bundle | Same metrics as live index |

---

## 6. Adversarial tests (`P3-A###`)

| ID | Case | Expected |
|----|------|----------|
| P3-A01 | Self-rating: agent submits unsigned positive event | Reject |
| P3-A02 | Fake escrow terminal (wrong escrow_id hash) | Reject |
| P3-A03 | Wash trade: A↔B circular releases same principal | Diversity dampening; policy flags low trust |
| P3-A04 | Sybil farm: 100 new agents, 1 escrow each | Cold-start metrics; no instant high trust |
| P3-A05 | Collusion ring: clique only transacts internally | Elevated `dyad_concentration` signal |
| P3-A06 | Replay same receipt evidence for different event | Reject duplicate `event_id` |
| P3-A07 | Settlement spoof: adapter-only claim without binding | Reject |
| P3-A08 | Selective disclosure: query hides negative events | `event_log_root` mismatch on verify |
| P3-A09 | Bribery: off-protocol payment for inflated indexer response | Verifier recompute detects mismatch (OPEN indexer honesty) |
| P3-A10 | Whitewash: new AgentId after revoke | Empty history; old id retains revoke events |
| P3-A11 | Fake dispute win without PROTO-2 dispute record | Reject |
| P3-A12 | Capability abuse ingest without observed deny artifact | No event unless PROTO-0 reject logged |
| P3-A13 | NET-0 replay reject | `integrity.replay_rejected` if ingested |
| P3-A14 | Inflated principal in event vs escrow terms | Reject on verify |

---

## 7. Integration tests (`P3-I###`)

### PROTO-0

| ID | Case | Expected |
|----|------|----------|
| P3-I01 | Read identity Active | `identity_age` starts |
| P3-I02 | Read identity Revoked | Stop positive accrual |
| P3-I03 | Indexer cannot call `grant_capability` | No side effect on cap store |
| P3-I04 | Frozen identity escrow complete | Event ingested with status annotation |

### PROTO-2

| ID | Case | Expected |
|----|------|----------|
| P3-I05 | Terminal Released + receipt path | Full completion metrics |
| P3-I06 | Receipt replay reject (PROTO-2) | Optional integrity event |
| P3-I07 | Fee consumed reflected in settlement amount evidence | Consistent weight_hint |

### PROTO-4

| ID | Case | Expected |
|----|------|----------|
| P3-I08 | Finalized + hard flag | Positive settlement credit |
| P3-I09 | DisputedExternal after finalize attempt | `settlement.failed`; hard credit revoked in metrics |
| P3-I10 | Cross-check escrow terminal commitment in binding | Reject if mismatch |

### PROTO-NET-0

| ID | Case | Expected |
|----|------|----------|
| P3-I11 | Transport delivery metadata optional ingest | No economic credit from transport alone |
| P3-I12 | Session replay reject | Integrity event only |

---

## 8. Enterprise tests (`P3-E###`)

| ID | Case | Expected |
|----|------|----------|
| P3-E01 | `local_only_index` mode | No public query endpoint |
| P3-E02 | Aggregate export: rates only | No counterparty graph in export |
| P3-E03 | Enterprise disables PROTO-3 | System operates; hire uses IAM/policy |
| P3-E04 | Private index + signed checkpoint | External verifier recomputes aggregate |
| P3-E05 | Multi-tenant index isolation | Tenant A cannot query Tenant B events |

---

## 9. Version and adapter tests (`P3-V###`)

| ID | Case | Expected |
|----|------|----------|
| P3-V01 | Events from PROTO-2 schema v1 | Derived correctly |
| P3-V02 | Mixed settlement providers in history | Metrics tagged by provider |
| P3-V03 | Aggregate across GBP and USD escrows | Separate asset columns; no FX |
| P3-V04 | Protocol version upgrade N→N+1 | Old events still verify |
| P3-V05 | Unknown event schema version | Reject or quarantine bucket |

---

## 10. Metrics tests (`P3-M###`)

| ID | Case | Expected |
|----|------|----------|
| P3-M01 | `dispute_rate` = disputes / eligible jobs | Correct formula |
| P3-M02 | `settlement_success_rate` excludes in-flight | Only terminal settlement states |
| P3-M03 | `median_settlement_latency` | Correct ordering |
| P3-M04 | `last_active` updates on each event | Monotonic |
| P3-M05 | Reference policy scalar is deterministic | Same inputs → same optional score |
| P3-M06 | Zero division (no escrows) | Rates defined as 0 or null per policy |

---

## 11. Hypothesis tests (`P3-H###`) — measurement, not pass/fail gate

| ID | Hypothesis | Measure |
|----|------------|---------|
| P3-H01 | Wash trade ROI < honest work ROI under policy v0 | Simulated adversary budget |
| P3-H02 | Query latency p99 < 100ms for 10k events/agent | Benchmark |
| P3-H03 | Full recompute < 5s for 100k events | Benchmark |
| P3-H04 | Spot-verify 3 events < 50ms | Benchmark |

---

## 12. Exit criteria

PROTO-3 implementation may be proposed when:

1. All `P3-T`, `P3-R`, `P3-A`, `P3-I`, `P3-E`, `P3-V`, `P3-M` cases have defined expected behaviour (this document)
2. Explicit implementation approval granted
3. No changes required to frozen PROTO-0/1/2/4 acceptance suites

---

## Freeze Statement

> This file is the **acceptance contract** for PROTO-3 v0. Implementation must not change derivation rules without revising this document.
