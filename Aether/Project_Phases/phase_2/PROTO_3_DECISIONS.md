# PROTO-3 Decisions — Reputation Layer

## Status

**Design resolution record — not implementation**

Resolves architectural questions for PROTO-3 evidence-based reputation.

Owner: Project Lead  
Date: 2026-07-29

| Status | Meaning |
|--------|---------|
| **Locked** | Accepted for PROTO-3 v0 design |
| **Provisional** | Working rule; revisit after spike |
| **Deferred** | Out of PROTO-3 v0 |

Preserves: PROTO-0 sole authority; no modification of PROTO-0/1/2/4/NET-0; DEC-P2-004 indexer-derived model.

---

## Decision Index

| ID | Question | Status | Confidence |
|----|----------|--------|------------|
| P3-DEC-001 | Reputation authority model | Locked | High |
| P3-DEC-002 | Event derivation vs protocol emission | Locked | High |
| P3-DEC-003 | Store model (materialised + recomputable) | Locked | High |
| P3-DEC-004 | Score vs metrics exposure | Locked | High |
| P3-DEC-005 | Evidence ref requirements | Locked | High |
| P3-DEC-006 | Indexer trust model | Provisional | Medium |
| P3-DEC-007 | Enterprise opt-out / local index | Locked | High |
| P3-DEC-008 | Settlement finality for credit events | Locked | High |
| P3-DEC-009 | Self-dealing / wash detection | Provisional | Medium |
| P3-DEC-010 | Privacy default (commitments not payloads) | Locked | High |
| P3-DEC-011 | PROTO integration (read-only) | Locked | High |
| P3-DEC-012 | Acceptance-test freeze gate | Locked | High |
| P3-DEC-013 | Bond integration for Sybil | Deferred | Medium |
| P3-DEC-014 | Consensus-canonical events | Deferred | Low |

---

## P3-DEC-001 — Reputation authority model

**Question:** Can PROTO-3 grant trust or override PROTO-0?

**Chosen (Locked):** **No.** PROTO-3 is strictly non-authoritative. Capabilities, escrow, and settlement are authorised only by PROTO-0/2/4. Reputation informs **application policy** only.

**Rejected:** Reputation-gated capabilities in core protocol; reputation as consensus state.

---

## P3-DEC-002 — Event derivation vs protocol emission

**Question:** Should PROTO-0/1/2/4 emit reputation events?

**Chosen (Locked):** **Indexers derive** `ReputationEventV0` from verified protocol artifacts. Lower layers are **not modified** to emit reputation hooks.

**Rationale:** Avoids coupling; keeps reputation optional; single verification path.

**Rejected:** Inline event emission from `release_escrow`, `finalize_settlement`, etc.

---

## P3-DEC-003 — Store model

**Question:** Derived on demand, materialised, or canonical chain?

**Chosen (Locked):** **Append-only event log + materialised metrics cache**, with **deterministic recomputation** from events + evidence.

**Deferred:** Consensus-canonical reputation (DEC-P2-004 Option C).

---

## P3-DEC-004 — Score vs metrics

**Question:** Should PROTO-3 define a global trust score?

**Chosen (Locked):** Expose **deterministic metrics vector** (`AgentMetricsV0`). Optional **reference policy** may compute a scalar for convenience — not required or normative.

**Rejected:** Mandatory 0–100 score; star ratings; leaderboard protocol object.

---

## P3-DEC-005 — Evidence requirements

**Question:** What proves a reputation event?

**Chosen (Locked):** Every `ReputationEventV0` must cite one or more `EvidenceRefV0` with cryptographic commitments verifiable against PROTO-0/1/2/4 artifacts. Unsigned assertions rejected.

**Rejected:** Honor-system event submission; social attestations.

---

## P3-DEC-006 — Indexer trust model

**Question:** How do queriers trust an indexer?

**Chosen (Provisional):** **Competing indexers** return `event_log_root` + evidence refs; queriers **spot-verify** or fully recompute. No single mandated indexer.

**Upgrade path:** Signed indexer attestations (application layer); canonical event anchoring (P3-DEC-014).

---

## P3-DEC-007 — Enterprise opt-out

**Question:** Can enterprises disable reputation?

**Chosen (Locked):** **Yes.** Enterprise deployments may run **local-only** indexes, disable public queries, or omit PROTO-3 entirely. Spend-control wedge does not require reputation.

---

## P3-DEC-008 — Settlement finality for credit

**Question:** When does settlement count as successful reputation evidence?

**Chosen (Locked):** `settlement.finalized` credit requires `SettlementBindingV0` status `Finalized` **and** escrow `hard_settlement_placeholder=true` (or policy-equivalent verified hard path). `Confirmed` alone is insufficient.

**Aligns with:** PROTO-4 P4-DEC-004; P4-SEC-002 fresh finalize rule.

---

## P3-DEC-009 — Self-dealing / wash detection

**Question:** How are circular fake transactions handled?

**Chosen (Provisional):** Metrics include **counterparty diversity** and **dyad frequency** dampening. Same-principal detection is **heuristic** (OPEN). Economic weight scales with principal amount.

**Evidence required:** PROTO-3 adversarial suite wash ROI tests.

---

## P3-DEC-010 — Privacy default

**Question:** What is exposed in queries?

**Chosen (Locked):** Default queries return **metrics + evidence commitments**, not full escrow/channel payloads. Counterparty identities may be redacted in enterprise aggregate mode.

---

## P3-DEC-011 — PROTO integration

**Question:** How does PROTO-3 consume other layers?

**Chosen (Locked):** **Read-only ingestion** of signed artifacts and store snapshots. No write API to PROTO-0/1/2/4/NET-0.

---

## P3-DEC-012 — Acceptance-test freeze gate

**Question:** When may implementation start?

**Chosen (Locked):** After [PROTO_3_ACCEPTANCE_TESTS.md](PROTO_3_ACCEPTANCE_TESTS.md) frozen **and** explicit implementation approval — separate from this design freeze.

---

## P3-DEC-013 — Bond integration

**Question:** Are bonds required for reputation?

**Chosen (Deferred):** `BondV0` not in PROTO-3 v0. Anti-Sybil bonds are economic layer (future). Reputation metrics work without bonds but are more farmable.

---

## P3-DEC-014 — Consensus-canonical events

**Question:** Should events be anchored on-chain or BFT-agreed?

**Chosen (Deferred):** Indexer-derived only for v0. Revisit if indexer trust blocks adoption.

**Aligns with:** DEC-P2-004 Option A → C upgrade path.

---

## Cross-References

| Decision | Related |
|----------|---------|
| P3-DEC-001 | SECURITY_MODEL.md — authority |
| P3-DEC-008 | PROTO_4_DESIGN.md §4 |
| P3-DEC-007 | PHASE_2_WEDGE_DECISION.md |
| P3-DEC-009 | MAINFRAME_THREAT_MODEL.md §3.6 |

---

## Freeze Statement

> These decisions lock PROTO-3 v0 **design**. Implementation is a separate gate.
