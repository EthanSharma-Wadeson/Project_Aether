# PROTO-3 Security Review

## Status

**APPROVE WITH DOCUMENTED LIMITATIONS**

Date: 2026-07-29  
Scope: Frozen PROTO-3 reputation architecture design (no implementation exists)  
Reviewer: Security review against SECURITY_MODEL.md, PROTO-0/1/2/4/NET-0 guarantees

This is a **design review** only. No Rust code was examined because none exists. The review evaluates whether the frozen architecture introduces authority leaks, violates existing protocol guarantees, or makes claims unsupported by the protocol stack.

References:

- [PROTO_3_DESIGN.md](PROTO_3_DESIGN.md)
- [PROTO_3_DECISIONS.md](PROTO_3_DECISIONS.md)
- [PROTO_3_THREAT_MODEL.md](PROTO_3_THREAT_MODEL.md)
- [PROTO_3_ACCEPTANCE_TESTS.md](PROTO_3_ACCEPTANCE_TESTS.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
- [PROTO_4_SECURITY_REVIEW.md](PROTO_4_SECURITY_REVIEW.md)

---

## Verdict

PROTO-3 design **preserves authority boundaries**. No architectural path exists from reputation queries to capability grants, escrow mutations, or settlement promotion. The design is read-only by construction, optional for enterprise, and explicitly non-authoritative.

Three findings are raised — none require remediation before implementation approval, but all require attention during implementation:

| ID | Severity | Summary |
|----|----------|---------|
| P3-SEC-001 | **Medium** | `capability.denied` event creates a negative-reputation side channel from PROTO-0 internals |
| P3-SEC-002 | **Low** | `event_log_root` verification relies on indexer cooperation; no protocol-enforced audit trail |
| P3-SEC-003 | **Low** | Counterparty graph leakage through `evidence_refs` even in commitment-only mode |

---

## 1. Authority Boundary Review

| Component | Result | Evidence |
|-----------|--------|----------|
| Reputation cannot grant capabilities | **PASS** | P3-DEC-001 (Locked): PROTO-3 non-authoritative; no write API to `CapabilityStore`. P3-I03 acceptance test: indexer cannot call `grant_capability`. No `CapabilityGrant` output in any PROTO-3 data model. |
| Reputation cannot approve settlements | **PASS** | PROTO-4 `request_settlement` / `submit_settlement` / `finalize_settlement` require `settlement.settle` capability grant (PROTO-0); no reputation parameter in any settlement function signature. P3-DEC-011: read-only integration. |
| Reputation cannot modify escrow | **PASS** | PROTO-2 `create_escrow` / `fund_escrow` / `release_escrow` require payer/provider capability grants. No PROTO-3 object is accepted by any PROTO-2 transition function. P3 acceptance test §2 Forbidden section explicitly prohibits `reputation metric → release escrow`. |
| Reputation cannot override identity state | **PASS** | PROTO-0 `IdentityRegistry` mutations require signed identity operations. PROTO-3 reads `Active`/`Frozen`/`Revoked` status (P3-I01, P3-I02) but has no write path. No `register_bundle`, `freeze_identity`, or `revoke_identity` in PROTO-3 design. |
| Reputation cannot become an alternative identity system | **PASS** | `AgentMetricsV0` references existing `AgentId`; it does not issue new identities. No `ReputationIdentity` object or trust-bootstrapped identity exists. `AgentId` derivation remains solely PROTO-0 (`SHA-256(schema_version ‖ operational_public_key)`). |
| PROTO-0 remains the only authority layer | **PASS** | Design §2 authority rule table: PROTO-0 = sole authority. P3-DEC-001 = Locked. No design object carries authorisation semantics. All economic operations (PROTO-1/2/4) continue to require PROTO-0 capability grants independently. |

**Boundary verdict:** No authority leak identified. The design is structurally incapable of escalating reputation into protocol authority, provided implementation follows the frozen acceptance tests (particularly P3-I03 and the §2 Forbidden block).

---

## 2. Evidence Integrity Review

| Claim | Status | Reason |
|-------|--------|--------|
| Every reputation event cites verifiable evidence | **Supported** | P3-DEC-005 (Locked): `EvidenceRefV0` mandatory with commitment hash. §5.3 forbids unsigned, self-asserted, or off-protocol evidence. P3-T012/T014 acceptance tests cover forged and tampered evidence rejection. |
| Commitments are sufficient for verification | **Supported with caveat** | `commitment = SHA-256(canonical snapshot)` matches PROTO-0/2/4 object id patterns. **Caveat:** verifier must have access to the source artifact to recompute the hash. Evidence availability is an external assumption — same as PROTO-4 (CONSENSUS_AND_SETTLEMENT §10–11). |
| Observer can independently verify metrics | **Supported** | P3-R002/R004/R007/R008: recompute from event log matches materialised cache; spot-check verifies against PROTO artifacts. **Requires** evidence bundle export — not enforced at protocol level (see P3-SEC-002). |
| Event IDs are deterministic | **Supported** | `event_id = SHA-256(canonical body without event_id)`. Same terminal escrow produces same event — P3-T015 tests idempotency. Consistent with PROTO-2 `escrow_id` and PROTO-4 `binding_id` patterns. |
| Settlement credit requires hard finality | **Supported** | P3-DEC-008 (Locked): `settlement.finalized` credit requires `Finalized` status **and** `hard_settlement_placeholder=true`. Aligns with P4-SEC-002 fresh finalize rule. P3-T013: Confirmed-only does not produce credit. |
| Non-terminal escrow cannot produce success events | **Supported** | P3-T011 rejects non-terminal ingest. Event types in §4.2 are derived only from terminal status (`Released`, `Refunded`, etc.). |
| Revoked identity stops positive accrual | **Supported** | P3-T010, P3-I02: revoke check before positive event creation. Historical events retained for honest reporting. |

---

## 3. Threat Review

### VALIDATED

These claims are supported by the frozen design **and** by existing validated protocol properties:

| Threat | Validation basis |
|--------|------------------|
| Forged events without PROTO-0 signatures rejected | P3-DEC-005 + PROTO-0 signature verification pipeline (SECURITY_MODEL §Validated by PROTO-0) |
| Fake settlement success without hard finality | P3-DEC-008 + PROTO-4 finalize path (P4-SEC-R02) |
| Reputation cannot write to authority layers | P3-DEC-001/011 + no write API in data models |
| Revoked identity stops earning | P3-T010 + PROTO-0 registry status check |
| Event replay dedup | Deterministic `event_id` (P3-T015) |
| Whitewash via new `AgentId` starts cold | Design §9: history tied to `AgentId`; new id = empty metrics |
| Enterprise local-only mode available | P3-DEC-007 + P3-E01/E03 |
| Stale adapter evidence cannot produce settlement credit | P3-DEC-008 + P4-SEC-002 remediation |
| Cross-adapter double-count blocked | One escrow terminal → one completion event set (P3-THR-051) |
| Receipt replay cannot boost reputation | PROTO-2 `ReceiptReplay` / `InvalidEscrowStatus` rejection; terminal-only events |

### OPEN

These remain **unvalidated** — they are acknowledged in the design and threat model but require implementation evidence:

| Threat | Why OPEN |
|--------|----------|
| Wash trading ROI | Dyad dampening is heuristic (P3-DEC-009 Provisional); P3-H01 requires adversarial simulation |
| Sybil resistance | No bond system exists (P3-DEC-013 Deferred); PROTO-0 identity registration has no economic cost |
| Collusion ring detection | Graph analysis not specified beyond `dyad_concentration` signal |
| Malicious indexer | Competing indexers + spot-verify (P3-DEC-006 Provisional); no enforced audit trail |
| Selective event disclosure | `event_log_root` mismatch detection depends on verifier cooperation |
| Same-principal self-dealing | Heuristic only; no deterministic block |
| Privacy-preserving threshold queries | Deferred to PROTO-5 |
| Optimal anti-wash parameters | No simulation evidence |

---

## 4. Security Claims Actually Supported by Architecture

The following are justified by design + existing protocol evidence:

1. **Authority isolation** — PROTO-3 cannot escalate to capability grants, escrow mutations, or settlement promotion.
2. **Evidence-first derivation** — every reputation event must cite signed, verifiable PROTO-0/1/2/4 artifacts.
3. **Deterministic recomputation** — metrics from the same event log and policy produce identical results.
4. **Settlement credit integrity** — only hard-finalized settlements produce `settlement.finalized` credit, aligned with PROTO-4 P4-SEC-002 remediation.
5. **Replay resistance** — deterministic `event_id` prevents duplicate event creation.
6. **Revocation respect** — PROTO-0 identity revocation stops positive accrual.
7. **Enterprise opt-out** — reputation is fully optional; spend-control wedge does not require it.
8. **No mandatory global score** — metrics vector, not opaque number; applications apply their own policy.

---

## 5. Design Assumptions

These must hold for the architecture to be sound. They are **not** validated by PROTO-3 itself:

| # | Assumption | Risk if false |
|---|-----------|---------------|
| A1 | Indexer implementation follows read-only contract | Authority leak if indexer gains write handle |
| A2 | Evidence availability — source artifacts accessible for verification | Metrics unverifiable without artifacts |
| A3 | `event_id` collision resistance (SHA-256) | Duplicate suppression fails |
| A4 | PROTO-0 identity registration cost is sufficient for open network | Sybil flood if free |
| A5 | Application developers do not treat metrics as capability gates | De facto authority leak at application layer |
| A6 | Logical time is consistent between indexer and source stores | Time-window metrics inconsistent |
| A7 | Canonical CBOR rules are enforced identically in PROTO-3 indexer and source protos | Commitment mismatch on verify |

---

## 6. Findings

### P3-SEC-001 — `capability.denied` event creates negative-reputation side channel

**ID:** P3-SEC-001  
**Severity:** Medium  
**Location:** PROTO_3_DESIGN.md §4.2 — `capability.denied` event type  
**Issue:** The `capability.denied` event is listed as "optional ingest" from "observed reject" of PROTO-0. PROTO-0 `authorise_action` returns `Rejected(reason)` as an in-process return value — it does not produce a signed exportable artifact. For an indexer to derive a `capability.denied` event, it must either:

(a) have direct access to PROTO-0 authorisation call results (coupling the indexer to the authority layer execution context), or  
(b) rely on an unsigned log entry from the caller (violating P3-DEC-005 signed evidence requirement).

Neither option produces a verifiable, independently recomputable evidence trail.

Additionally, negative events derived from capability denials could leak information about an agent's attempted but rejected actions — a privacy concern not addressed in the privacy model.

**Impact:** If ingested via unsigned logs, an indexer could fabricate denial events to damage an agent's reputation. If ingested via execution context access, the indexer is coupled to the authority layer at runtime.

**Recommendation:** Either (a) remove `capability.denied` from the v0 event set and defer until a signed capability-deny receipt exists, or (b) explicitly mark it as **unsigned-optional** with a distinct evidence quality flag that prevents it from affecting economic metrics. Acceptance test P3-A12 partially addresses this but does not resolve the unsigned-evidence contradiction with P3-DEC-005.

---

### P3-SEC-002 — `event_log_root` verification lacks protocol enforcement

**ID:** P3-SEC-002  
**Severity:** Low  
**Location:** PROTO_3_DESIGN.md §6.3; PROTO_3_DECISIONS.md P3-DEC-006  
**Issue:** The `event_log_root` (Merkle or hash chain) is produced by the indexer. There is no protocol mechanism forcing an indexer to include all derived events — the root is self-attested. A malicious indexer can produce a valid root over a curated subset of events (omitting negative history) and present it alongside partial evidence. Verifiers can detect this **only** if they independently ingest all source artifacts and recompute — but the design does not specify how a verifier discovers artifacts it has not already seen.

**Impact:** Selective omission is detectable in theory but may be impractical without a completeness guarantee on artifact visibility. In practice this is acceptable for v0 (P3-DEC-006 is Provisional with acknowledged upgrade path), but the design should not claim "spot-check detects selective disclosure" without qualifying the completeness assumption.

**Recommendation:** Add explicit assumption to PROTO_3_DESIGN.md §6.3: verifier completeness depends on independent artifact access; `event_log_root` proves internal consistency of the set the indexer chose to include, not completeness. This is honest for v0. Track the completeness gap as an open item for the canonical-events upgrade path (P3-DEC-014).

---

### P3-SEC-003 — Counterparty graph leakage via evidence refs

**ID:** P3-SEC-003  
**Severity:** Low  
**Location:** PROTO_3_DESIGN.md §10; PROTO_3_DECISIONS.md P3-DEC-010  
**Issue:** P3-DEC-010 states default queries return "metrics + evidence commitments, not full escrow/channel payloads." However, `EvidenceRefV0` includes `locator` (escrow_id, binding_id, channel_id) which — when combined with `counterparty_agent_id` in the `ReputationEventV0` — reveals the counterparty graph even without full payload disclosure.

Enterprise aggregate mode can "redact counterparty identities," but the open-network default exposes this graph. For agents operating across multiple providers, the evidence refs reveal business relationships.

**Impact:** Low — the graph is already partially inferable from dual-signed escrow creation on open networks. But the design should not claim privacy-by-default when evidence refs contain identifying locators.

**Recommendation:** Clarify in §10 that commitment-only mode protects **payload content**, not **relationship graph**. If graph privacy is required, `counterparty_agent_id` and `locator` must be redacted or hashed in query responses — a stronger privacy mode not currently specified. Track for PROTO-5 selective disclosure.

---

## 7. Remaining Risks

### Sybil resistance

PROTO-3 v0 has **no Sybil resistance** beyond cold-start metrics. PROTO-0 identity registration has no economic cost. An attacker can create unlimited `AgentId`s and build independent empty histories. Bond integration (P3-DEC-013) is deferred. **This is the single largest unsolved risk for open-network reputation.**

Honest assessment: Sybil resistance is NOT solved. It is acknowledged and deferred. Enterprise deployments are unaffected (reputation is optional or local-only).

### Indexer honesty

No protocol mechanism forces honest indexing. Competing indexers and spot-verification are the mitigation (P3-DEC-006 Provisional). This is comparable to trusting a block explorer — reasonable for v0 but not cryptographically enforced.

### Collusion

Agents who collude using **real economic transactions** (genuine escrows with real principal) produce **legitimate-looking** reputation events. PROTO-3 cannot distinguish "cooperative real work" from "cooperative fake work with real funds." Dyad dampening is a heuristic. This is a **fundamental limitation** of any evidence-based reputation system without subjective quality verification.

### Privacy

Open-network reputation inherently leaks relationship graphs (P3-SEC-003). Enterprises mitigate via local-only mode. Selective disclosure (PROTO-5) is required for graph-private reputation queries.

### Work quality verification

PROTO-3 proves that economic events occurred under signed authority. It does **not** prove that work was useful, correct, or honest. A provider can submit a valid receipt for worthless output. Reputation reflects **protocol compliance**, not **task quality**. This is explicitly out of scope (PROTO_3_DESIGN §14) and is an honest limitation.

---

## 8. Alignment with SECURITY_MODEL.md

| SECURITY_MODEL section | PROTO-3 alignment |
|------------------------|-------------------|
| Protected assets | Reputation adds "metric accuracy" and "counterparty privacy" — consistent with existing asset list |
| Adversary classes | Reputation farmer, Sybil operator, colluding cartel — already listed in SECURITY_MODEL §Adversary Classes |
| Signature semantics | PROTO-3 re-verifies signatures; does not introduce new signing semantics |
| PROTO-0 authority | Preserved — read-only consumption |
| PROTO-2 escrow guarantees | Preserved — terminal-only event derivation matches escrow status machine |
| PROTO-4 settlement guarantees | Preserved — hard finality required for settlement credit (P3-DEC-008 ↔ P4-SEC-002) |
| Reputation Security section | Currently "not validated by PROTO-0" — accurate; PROTO-3 design freeze advances status to "designed, not validated" |
| Not Yet Validated list | "reputation systems and anti-wash scoring" — remains correct; PROTO-3 is design only |

---

## 9. PROTO-3 Integration Safety

| Integration | Direction | Mutation? | Circular dependency? |
|-------------|-----------|-----------|---------------------|
| PROTO-0 | PROTO-3 reads registry status | No | No — PROTO-0 does not query PROTO-3 |
| PROTO-1 | PROTO-3 reads channel close records | No | No — PROTO-1 does not query PROTO-3 |
| PROTO-2 | PROTO-3 reads escrow terminals | No | No — PROTO-2 does not query PROTO-3 |
| PROTO-4 | PROTO-3 reads settlement bindings | No | No — PROTO-4 does not query PROTO-3 |
| PROTO-NET-0 | PROTO-3 reads replay reject records (optional) | No | No — NET-0 does not query PROTO-3 |

**No circular dependency.** No authority escalation path. No economic mutation from PROTO-3 to any lower layer.

---

## 10. Conclusion

PROTO-3 design is **architecturally sound** with respect to authority boundaries, evidence integrity, and protocol integration safety. The three findings are low-to-medium severity and do not block implementation approval — they require attention during implementation.

The design is **honest about its limitations**: Sybil resistance, collusion, indexer honesty, and work quality are explicitly OPEN. This is the correct posture for a v0 design freeze.

**Verdict: APPROVE WITH DOCUMENTED LIMITATIONS.**

Limitations:

1. Sybil resistance is NOT solved (P3-DEC-013 deferred; no bond system).
2. Indexer honesty is NOT enforced (P3-DEC-006 Provisional; spot-verify only).
3. Collusion with real economic transactions is NOT detectable by PROTO-3 alone.
4. Counterparty graph privacy is NOT provided in open-network default mode.
5. Work quality is NOT assessed — reputation reflects protocol compliance only.

---

## Stop Condition

PROTO-3 implementation must not begin until this review is **accepted**.

No Rust.  
No marketplace.  
No dashboard.  
No AETH implementation.  
No governance.  
No production reputation service.

Design review only.
