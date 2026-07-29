# PROTO-3 Threat Model — Reputation Layer

## Status

**Threat analysis only — extends [MAINFRAME_THREAT_MODEL.md](MAINFRAME_THREAT_MODEL.md) and [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)**

Classifies reputation-specific threats as **VALIDATED** (mitigated by design rules aligned with implemented protos) vs **OPEN** (requires PROTO-3 implementation evidence or external assumptions).

References:

- [PROTO_3_DESIGN.md](PROTO_3_DESIGN.md)
- [PROTO_3_DECISIONS.md](PROTO_3_DECISIONS.md)
- [PROTO_3_ACCEPTANCE_TESTS.md](PROTO_3_ACCEPTANCE_TESTS.md)

---

## 1. Scope

PROTO-3 introduces **no new authority**. Threats target:

- Indexer correctness and honesty
- Event derivation integrity
- Metric gaming and economic abuse
- Privacy leakage via reputation queries
- Misuse of reputation as pseudo-authority

---

## 2. Assets

| Asset | Why it matters |
|-------|----------------|
| Event log integrity | Wrong events → wrong hire decisions |
| Evidence commitments | Forgery poisons trust views |
| Metric accuracy | Autonomous delegation depends on rates |
| Counterparty privacy | Graph disclosure leaks business relationships |
| PROTO-0 authority boundary | Reputation must not bypass capabilities |

---

## 3. Adversaries

| Class | Goal |
|-------|------|
| **Reputation farmer** | Inflate metrics cheaply |
| **Sybil operator** | Many identities, fake volume |
| **Colluding cartel** | Mutual positive events |
| **Malicious indexer** | Lie about metrics |
| **Selective discloser** | Hide negative history |
| **Whitewasher** | Abandon bad identity, start fresh |
| **Briber** | Pay indexer for false scores |
| **Enterprise insider** | Exfiltrate or poison local index |

---

## 4. Threat Catalogue

### 4.1 Spoofing and forgery

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-001 | Forge `ReputationEventV0` without valid evidence | **VALIDATED** (design) | Require `EvidenceRefV0` + signature re-verification (P3-DEC-005) |
| P3-THR-002 | Forge escrow terminal state in indexer only | **VALIDATED** (design) | Re-verify against PROTO-2 store snapshot / commitment |
| P3-THR-003 | Fake settlement success without Finalized binding | **VALIDATED** (design) | P3-DEC-008; align PROTO-4 hard finality |
| P3-THR-004 | Impersonate another agent in event subject | **VALIDATED** (design) | Evidence must name parties matching signatures |

### 4.2 Collusion and wash activity

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-010 | A↔B circular escrows for volume | **OPEN** | Dyad dampening, diversity metrics (P3-A03, P3-A05) |
| P3-THR-011 | Cartel only hires cartel members | **OPEN** | Graph analysis; economic weight; bonds (deferred) |
| P3-THR-012 | Self-dealing via shared principal | **OPEN** | Same-principal heuristic; enterprise IAM external |
| P3-THR-013 | Low-value spam escrows | **OPEN** | `weight_hint` by principal; minimum economic threshold in policy |

### 4.3 Sybil

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-020 | Mass cheap identities | **OPEN** | Bonds + registration cost (external); cold-start metrics (P3-A04) |
| P3-THR-021 | Sybil split work across ids | **OPEN** | Graph linking heuristics; not solved by PROTO-3 alone |
| P3-THR-022 | Farm then exit scam | **OPEN** | Time-decay policies; bond lock (deferred) |

### 4.4 Indexer and disclosure

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-030 | Malicious indexer inflates metrics | **OPEN** | `event_log_root` + spot recompute (P3-DEC-006) |
| P3-THR-031 | Selective disclosure omits failures | **OPEN** | Verifier full recompute from evidence bundle (P3-A08) |
| P3-THR-032 | Indexer censorship of negative events | **OPEN** | Multi-indexer comparison; evidence export |
| P3-THR-033 | Bribery of indexer operator | **OPEN** | Independent verification; no single mandated indexer |

### 4.5 Whitewashing and identity lifecycle

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-040 | New AgentId after bad history | **VALIDATED** (design) | History tied to `AgentId`; new id starts cold (P3-A10) |
| P3-THR-041 | Revoked identity continues earning | **VALIDATED** (design) | Stop accrual on revoke (P3-T010, P3-I02) |
| P3-THR-042 | Key rotation erases history | **VALIDATED** (design) | Same `AgentId` persists across rotation |

### 4.6 Fake settlements and adapter abuse

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-050 | Credit on adapter lie without binding | **VALIDATED** (design) | Evidence must include binding + PROTO-4 verify path |
| P3-THR-051 | Cross-adapter double-count same economic outcome | **VALIDATED** (design) | One escrow terminal → one completion event set |
| P3-THR-052 | Stale finalize counted as success | **VALIDATED** (PROTO-4) | P4-SEC-002; P3-DEC-008 |

### 4.7 Reputation as authority confusion

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-060 | App treats high metric as capability grant | **VALIDATED** (design) | P3-DEC-001; documentation; no core hook |
| P3-THR-061 | Marketplace hides PROTO-0 deny behind score | **OPEN** | Application anti-pattern; out of protocol scope |

### 4.8 Privacy

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-070 | Query reveals full counterparty graph | **OPEN** | Aggregate mode; commitment-only default (P3-DEC-010) |
| P3-THR-071 | Enterprise leak via public index | **VALIDATED** (design) | Local-only index (P3-DEC-007) |
| P3-THR-072 | Metadata deanonymisation | **OPEN** | Selective disclosure (PROTO-5 stretch) |

### 4.9 Replay and integrity

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| P3-THR-080 | Replay old terminal state as new event | **VALIDATED** (design) | Deterministic `event_id` dedup (P3-T015) |
| P3-THR-081 | PROTO-2 receipt replay as reputation boost | **VALIDATED** (PROTO-2) | Terminal state required; replay reject optional event |

---

## 5. VALIDATED vs OPEN Summary

### VALIDATED (design + existing proto evidence)

- Forged events without cryptographic evidence rejected by design
- Settlement credit requires verified hard finality path
- PROTO-3 cannot write authority layers
- Revoked identities stop positive accrual
- Event idempotency via deterministic `event_id`
- Whitewashing via new `AgentId` starts cold (by design)
- Enterprise local-only mode supported

### OPEN (requires PROTO-3 implementation + adversarial tests)

- Wash trading ROI vs honest work
- Sybil resistance without bonds
- Collusion ring detection accuracy
- Malicious indexer detection at scale
- Selective disclosure detection
- Privacy-preserving threshold queries
- Same-principal self-dealing detection
- Optimal diversity/dyad policy parameters

---

## 6. Trust Model Change With Optional Reputation

| Deployment | Trust model |
|------------|-------------|
| **Enterprise, PROTO-3 off** | IAM + PROTO-0 capabilities only — **unchanged** |
| **Enterprise, local index** | Internal audit supplement — not public trust |
| **Open network** | Queriers trust **indexer + spot verification**, not reputation as authority |

Optional reputation **adds information**; it does **not** replace PROTO-0 authorisation.

---

## 7. Security Invariants

1. **INV-P3-01:** No code path from reputation query to capability grant.
2. **INV-P3-02:** Every positive economic event cites verifiable PROTO-1/2/4 evidence.
3. **INV-P3-03:** `settlement.finalized` requires hard finality alignment.
4. **INV-P3-04:** Metrics recomputed from event log match materialised cache.
5. **INV-P3-05:** Unsigned social input never becomes `ReputationEventV0`.

---

## 8. Residual Risk Statement

PROTO-3 **does not** eliminate:

- Indexer dishonesty without independent verification
- Sybil floods without economic bonds
- Collusion with real economic work between conspirators
- Subjective work quality assessment
- Legal recourse for fraud

Goal: raise attack **cost** above expected **profit** for common farms — not perfect trust.

---

## 9. Verdict

**Design-phase verdict:** PROTO-3 threat model is **acceptable for v0 design freeze** provided:

- Reputation remains non-authoritative (P3-DEC-001)
- Evidence-first derivation (P3-DEC-005)
- OPEN items tracked in acceptance adversarial suite

**Implementation verdict:** Pending — requires P3-A### and P3-H01 results.

---

## Freeze Statement

> OPEN threats are explicit. PROTO-3 must not claim Sybil or wash resistance is **VALIDATED** until adversarial tests pass.
