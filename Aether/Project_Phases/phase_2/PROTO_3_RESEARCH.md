# PROTO-3 Research — Reputation Layer Background

## Status

**Research synthesis — informs design; not implementation**

Collects prior research, hypotheses, and open questions that shaped PROTO-3. Resolves items where design freeze allows; carries forward items requiring implementation evidence.

References:

- [REPUTATION_SYSTEM.md](../../Aether_docs/REPUTATION_SYSTEM.md)
- [research/ai_agents.md](../../Aether_docs/research/ai_agents.md)
- [experiments/hypotheses.md](../../Aether_docs/experiments/hypotheses.md)
- [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) — DEC-P2-004

---

## 1. Research Question

How should machine agents select trustworthy counterparties **without** opaque platform ratings — using only evidence from protocol participation?

---

## 2. Prior Art in Aether Docs

| Source | Finding |
|--------|---------|
| REPUTATION_SYSTEM.md | Evidence layer + score vectors; anti-wash; separate evidence from policy |
| BUSINESS_ALIGNMENT.md | Reputation high value for marketplaces; **not required** for enterprise wedge |
| MAINFRAME_ARCHITECTURE.md | `ReputationEventV0`, indexer prototype, bonds parallel track |
| DEC-P2-004 | Indexer-derived (Option A); upgrade path to canonical events (C) |
| experiments/hypotheses.md H3 | Bonded evidence resists trivial wash reputation |
| ai_agents.md | Collusion and wash farming are expected adversary behaviours |

---

## 3. Hypotheses

| ID | Hypothesis | Design response | Evidence status |
|----|------------|-----------------|-----------------|
| H-P3-01 | Reputation without signatures is farmable | Evidence refs mandatory | **VALIDATED** (design) |
| H-P3-02 | Single global score creates lock-in | Metrics vector + optional policy | **VALIDATED** (design) |
| H-P3-03 | Escrow terminal events are sufficient v0 signal | Core event set in PROTO_3_DESIGN §4 | **Provisional** |
| H-P3-04 | Settlement hard finality should gate credit | P3-DEC-008 | **VALIDATED** (PROTO-4) |
| H-P3-05 | Bonds required for open-network Sybil resistance | Deferred P3-DEC-013 | **OPEN** |
| H-P3-06 | Wash trades profitable under naive scoring | Diversity dampening | **OPEN** — P3-H01 |
| H-P3-07 | Indexer fragmentation confuses agents | Competing indexers + recompute | **OPEN** |
| H-P3-08 | Enterprise will disable public reputation | Local-only mode | **VALIDATED** (design) |

---

## 4. Alternatives Considered

### 4.1 Bond-only trust (DEC-P2-004 Option D)

**Pros:** Simple anti-Sybil.  
**Cons:** No quality signal; rich agents always win.  
**Outcome:** Rejected as sole model; bonds deferred as complement.

### 4.2 Consensus-canonical scores (Option B)

**Pros:** Portable trust state.  
**Cons:** Expensive; slow; overkill for v0.  
**Outcome:** Deferred (P3-DEC-014).

### 4.3 Social reviews on-chain or off-chain

**Pros:** Human-readable.  
**Cons:** Subjective; gamed; not protocol evidence.  
**Outcome:** **Rejected** for PROTO-3.

### 4.4 Platform rating import

**Pros:** Bootstrap history.  
**Cons:** Poisons evidence model; lock-in.  
**Outcome:** **Rejected** — off-protocol reviews never become events.

### 4.5 ML-based trust scores

**Pros:** May detect complex fraud.  
**Cons:** Non-deterministic; not independently verifiable.  
**Outcome:** Application layer only; not PROTO-3 core.

---

## 5. Event Set Research

### Minimum viable events (v0)

Research suggests these carry the most signal per verification cost:

1. `escrow.released` with receipt path
2. `escrow.refunded` / dispute outcomes
3. `settlement.finalized` with hard finality
4. `identity.revoked`

### Lower priority (v0 optional)

- `capability.denied` — noisy; may reflect policy not fault
- NET-0 delivery metadata — weak economic signal
- Channel events — valuable for payment-channel use cases; secondary for escrow wedge

### Excluded

- Likes, stars, NPS, follower counts
- Manual override flags without evidence

---

## 6. Metric Formula Research

### Dispute rate

```text
dispute_rate = disputes_initiated / max(1, escrow_completed + disputes_initiated)
```

Attribution split on `dispute_resolved` — not symmetric punishment.

### Settlement success rate

```text
settlement_success_rate = finalized_count / max(1, finalized_count + failed_count)
```

In-flight bindings excluded.

### Time decay

**OPEN:** Exponential decay vs windowed counts. Default v0: **no decay** (simpler recompute); policy may add decay in application layer.

---

## 7. Enterprise Research Findings

From PHASE_2_WEDGE_DECISION and ENTERPRISE_DEMO:

| Finding | Implication |
|---------|-------------|
| Spend control works without reputation | PROTO-3 optional for enterprise |
| Audit needs signed evidence | Local index can serve compliance |
| Public reputation may alarm CISO | Default off; aggregate export only |

---

## 8. Open Network Research Findings

| Finding | Implication |
|---------|-------------|
| Cold-start problem real | New agents have empty metrics — expected |
| Marketplace needs counterparty signal | PROTO-3 primary beneficiary segment |
| Wash trading expected | Adversarial suite mandatory before launch |

---

## 9. Privacy Research

| Approach | Status |
|----------|--------|
| Commitment-only queries | **Locked** P3-DEC-010 |
| Full graph public | Rejected default |
| ZK threshold proofs (“score ≥ θ”) | PROTO-5 stretch |
| Enterprise aggregate export | P3-E02 |

---

## 10. Dependencies

```text
PROTO-3 design
    requires ──► PROTO-2 terminal semantics (implemented)
    requires ──► PROTO-4 finalize evidence rules (implemented)
    optional ──► PROTO-1 channel disputes
    optional ──► PROTO-NET-0 integrity events
    deferred ──► BondV0 (Sybil)
    deferred ──► PROTO-5 selective disclosure
```

---

## 11. Resolved Research Questions

| Question | Resolution |
|----------|------------|
| Consensus-canonical vs indexer? | Indexer v0 (P3-DEC-003, P3-DEC-014) |
| Global score required? | No (P3-DEC-004) |
| Modify protos to emit events? | No (P3-DEC-002) |
| Enterprise mandatory reputation? | No (P3-DEC-007) |
| Settlement Confirmed enough? | No — Finalized + hard (P3-DEC-008) |

---

## 12. Open Research Questions

| Question | Owner | Gate |
|----------|-------|------|
| Default time decay function? | Policy working group | Post-v0 |
| Bond minimum for open network? | Economic design | BondV0 |
| Wash detection ROC at scale? | PROTO-3 adversarial | P3-H01 |
| Cross-backend reputation aggregation? | MULTI_ASSET_SETTLEMENT | Per-asset metrics |
| Threshold ZK without full graph? | PROTO-5 | Stretch |
| Indexer reputation (meta-trust)? | Application | Out of v0 |

---

## 13. Recommended Reading Order

1. [PROTO_3_DESIGN.md](PROTO_3_DESIGN.md)
2. [PROTO_3_DECISIONS.md](PROTO_3_DECISIONS.md)
3. [PROTO_3_THREAT_MODEL.md](PROTO_3_THREAT_MODEL.md)
4. [PROTO_3_ACCEPTANCE_TESTS.md](PROTO_3_ACCEPTANCE_TESTS.md)

---

## Freeze Statement

> Research informs design freeze. OPEN questions do not block design approval; they block **implementation completion** claims.
