# research/unanswered_questions.md

## Purpose

Explicit backlog of undecided design questions. When a question is answered, record the decision, date, and move detail into the relevant design doc.

## Phase 0 Critical-Path Ownership

These are the Phase 0 implementation-readiness questions that required ownership before Phase 0 exit.

| Area | Status | Owner | Reference |
|------|--------|-------|-----------|
| Implementation language and runtime | Provisional | Project Lead | `DEC-001` |
| Repository layout | Provisional | Project Lead | `DEC-002` |
| Serialization format for protocol messages | Provisional | Project Lead | `DEC-003` |
| Signature scheme and signed-byte construction | Provisional | Project Lead | `DEC-004A` / `DEC-004B` |
| Agent ID derivation and identity commitment | Provisional | Project Lead | `DEC-005` |
| Minimal identity object | Provisional | Project Lead | `DEC-006` |
| Capability envelope schema | Provisional | Project Lead | `DEC-007` |
| Capability revocation approach | Provisional | Project Lead | `DEC-008` |

## Phase 1 Review Ownership

These items remain open or provisional for Phase 1 review, but do not block Phase 0 exit.

| Area | Status | Owner | Notes |
|------|--------|-------|-------|
| Attestation profile for early receipts | Provisional | Project Lead | deterministic receipt-first for `PROTO-2` — see [PROTO_2_DESIGN.md](../../Project_Phases/phase_1/PROTO_2_DESIGN.md) |
| Claim types for non-deterministic outputs | Provisional | Project Lead | out of objective-verification scope for v0 prototypes |
| In-memory simulator vs backend stub for channels | Provisional | Project Lead | in-process bilateral simulator for `PROTO-1` — see [PROTO_1_DESIGN.md](../../Project_Phases/phase_1/PROTO_1_DESIGN.md) |
| Settlement backend shortlist / spike timing | Deferred | Project Lead | not part of `PROTO-0`–`PROTO-2` |

### PROTO-1 design questions (resolve or provisional-lock before implementation)

- [x] Signer quorum for channel updates → **Locked: dual signatures to accept** — [PROTO_1_DECISIONS.md](../../Project_Phases/phase_1/PROTO_1_DECISIONS.md) P1-DEC-001
- [x] Open → Active trigger → **Locked: dual-signed sequence=0 activate** — P1-DEC-002
- [x] Exact v0 capability action tokens → **Locked:** `channel.open/activate/update/close/dispute` — P1-DEC-007
- [x] Minimal admissible dispute evidence → **Provisional:** `DisputeEvidenceV0` — P1-DEC-004
- [x] H1 simulated baseline parameters → **Provisional:** synthetic cost/latency units only — P1-DEC-005
- [x] Open → Finalized abort without activation → **Locked: allowed with dual-signed abort** — P1-DEC-006

Acceptance tests frozen: [PROTO_1_ACCEPTANCE_TESTS.md](../../Project_Phases/phase_1/PROTO_1_ACCEPTANCE_TESTS.md).

Remaining PROTO-1 risks (not pretend-solved): logical-time integrity, local revocation coherence, mid-channel key rotation, synthetic H1 limits, non-distributed disputes.

### PROTO-2 design questions (resolve or provisional-lock before implementation)

- [x] Escrow state ownership / store model → **Locked: separate `EscrowStore`, no public `get_mut`** — [PROTO_2_DECISIONS.md](../../Project_Phases/phase_1/PROTO_2_DECISIONS.md) P2-DEC-001
- [x] PROTO-1 integration model → **Locked: Option B separate ledger** — P2-DEC-002
- [x] Terms agreement quorum → **Locked: dual-signed `EscrowTermsV0`** — P2-DEC-003
- [x] Settlement receipt signer model → **Locked: provider only** — P2-DEC-004
- [x] Receipt-to-escrow binding → **Locked: escrow_id + terms_version + nonce** — P2-DEC-005
- [x] Release authority → **Provisional: either party after dispute window** — P2-DEC-006
- [x] Refund authority → **Locked** — P2-DEC-007
- [x] Timeout semantics → **Locked: logical `now` deadlines** — P2-DEC-008
- [x] Dispute resolver trust → **Provisional: local authorised participant** — P2-DEC-009
- [x] Escrow capability tokens → **Locked: `escrow.*` namespace** — P2-DEC-010
- [x] Economic finality definition → **Locked: simulator terminal only** — P2-DEC-011
- [x] Fee-budget model → **Provisional: quote/reserve/consume** — P2-DEC-012
- [x] H4/H5 prototype success criteria → **Provisional** — P2-DEC-013/014

Acceptance tests frozen: [PROTO_2_ACCEPTANCE_TESTS.md](../../Project_Phases/phase_1/PROTO_2_ACCEPTANCE_TESTS.md).

PROTO-2 implementation **complete** — see [PROTO_2_RESULTS.md](../../Project_Phases/phase_1/PROTO_2_RESULTS.md) (183 tests green).

## Phase 2 Architecture Gate Ownership

These items are open for mainframe design review. See [Project_Phases/phase_2/](../../Project_Phases/phase_2/).

| Area | Status | Owner | Reference |
|------|--------|-------|-----------|
| Agent discovery mechanism | Open | Project Lead | DEC-P2-001 |
| Network trust model | Open | Project Lead | DEC-P2-002 |
| Settlement philosophy (first adapter) | Open | Project Lead | DEC-P2-003 |
| Reputation architecture | Provisional | Project Lead | DEC-P2-004 |
| Permission scaling (Merkle roots) | Provisional | Project Lead | DEC-P2-005 |
| First networked prototype selection | Open | Project Lead | DEC-P2-006 |
| Revocation propagation | Open | Project Lead | DEC-P2-008 |
| Business wedge / design partner | Open | Project Lead | BUSINESS_ALIGNMENT.md |

### Phase 2 architecture questions

- [ ] What transport/session primitives are required before networked PROTO-1? → **Designed: MP-01..03** — [MAINFRAME_ARCHITECTURE.md](../../Project_Phases/phase_2/MAINFRAME_ARCHITECTURE.md)
- [ ] Is discovery protocol or application layer? → **Provisional: protocol primitive, marketplace is app** — DEC-P2-010
- [ ] When do Merkle permission roots become mandatory? → **Provisional: before public federation** — DEC-P2-005
- [ ] What is the first settlement backend for PROTO-4 spike? → **Provisional: enterprise.ledger.v0 stub** — [PROTO_4_DECISIONS.md](../../Project_Phases/phase_2/PROTO_4_DECISIONS.md) P4-DEC-003
- [ ] Hub-and-spoke vs mesh default? → **Open** — DEC-P2-007
- [ ] Max stale-grant window after revoke? → **Open** — DEC-P2-008
- [ ] Enterprise vs marketplace vs ecosystem first wedge? → **Decided: Enterprise Agent Spend Control** — [PHASE_2_WEDGE_DECISION.md](../../Project_Phases/phase_2/PHASE_2_WEDGE_DECISION.md)
- [ ] When may hard_settlement_placeholder become true? → **Locked design: only after external Confirmed + verified binding** — PROTO_4_DESIGN.md / P4-DEC-004

## Consensus & Settlement

- [x] Dedicated chain vs. modular settlement vs. certificates on existing L1/L2? → **Modular, settlement-agnostic protocol with pluggable backends (v0.1).** See [CONSENSUS_AND_SETTLEMENT.md](../CONSENSUS_AND_SETTLEMENT.md) §2.
- [x] Concrete fault model and finality definition? → **Layered fault model; multi-stage finality (PROPOSED → ECONOMIC_FINAL).** See [CONSENSUS_AND_SETTLEMENT.md](../CONSENSUS_AND_SETTLEMENT.md) §4–6.
- [ ] Dispute window parameters and delay assumptions? → Task-class-dependent; examples in ADR §8; exact bounds TBD.
- [x] Data availability path for dispute evidence? → **Split model: on-chain commitments, off-chain storage with availability obligations.** See [CONSENSUS_AND_SETTLEMENT.md](../CONSENSUS_AND_SETTLEMENT.md) §10–11.
- [x] Multi-backend settlement behind one identity layer? → **Yes. Aether Agent ID independent of settlement addresses; SettlementBinding model.** See [CONSENSUS_AND_SETTLEMENT.md](../CONSENSUS_AND_SETTLEMENT.md) §12.

### Still open (consensus & settlement)

- [ ] What settlement backends should be included in the first prototype?
- [ ] One backend initially or multiple adapters?
- [ ] Verifier committee sizes per task class; selection and conflict-of-interest rules
- [ ] Minimum and maximum allowed dispute periods
- [ ] Evidence retention duration and availability audit mechanism
- [ ] Backend-specific finality representation in protocol schema
- [ ] Cross-backend settlement effects on reputation
- [ ] Measurable conditions that would justify a native Aether chain

## Identity & Capabilities

- [ ] Exact `AgentId` derivation and multi-key commitment scheme? → **Provisional: stable material only (DEC-005); see [V0_WIRE_CRYPTO_GROUP.md](../Project_Phases/phase_0/V0_WIRE_CRYPTO_GROUP.md)**
- [ ] Serialization format (canonical binary / CBOR / other)? → **Provisional: schema-locked CBOR (DEC-003); experiment in `experiments/decision/`**
- [ ] Capability revocation mechanism (expiry-heavy vs. status lists vs. accumulators)? → **Provisional: expiry + explicit revoke + identity status (DEC-008)**
- [ ] Recovery model for principals without breaking headless ops?
- [ ] DID method adoption vs. custom mapping?

## Payments

- [ ] Default topology: mesh vs. hub-and-spoke?
- [ ] Multiparty channels in scope for v1?
- [ ] Native asset only vs. multi-asset bonds/fees at launch?
- [ ] Watchtower protocol in-core or out-of-core?

## Reputation

- [ ] Which score aggregates are consensus-canonical vs. indexer-derived?
- [ ] Default decay and anti-wash parameters?
- [ ] Threshold proofs for reputation without full graph disclosure?

## Verification & Privacy

- [ ] v1 attestation profile: receipts only, TEE optional, ZK later?
- [ ] How to specify claims for nondeterministic model outputs?
- [ ] Default visibility of bond and settlement amounts?
- [ ] Anonymous credentials vs. stable pseudonyms as default posture?

## Economics

- [ ] Issue a native utility token at genesis or delay?
- [ ] Slash redistribution policy (burn, treasury, victim compensate)?
- [ ] Cold-start subsidies without farmable inflation?
- [ ] Minimum bond schedule vs. inclusive micro-agent access?

## Product / Scope Boundaries

- [ ] First vertical for end-to-end prototype (which agent market)? → **Under review** — [BUSINESS_ALIGNMENT.md](../../Project_Phases/phase_2/BUSINESS_ALIGNMENT.md)
- [ ] Discovery/directory: protocol service or application layer only? → **Provisional: protocol primitive** — DEC-P2-010
- [ ] Governance surface for parameters in phase 1? → **Deferred to Phase 2 governance layer** — MAINFRAME_ARCHITECTURE.md §Layer 6
- [ ] First networked prototype: transport vs settlement? → **Open** — DEC-P2-006

## Decision Log

| Date | Question | Decision | Doc updated |
|------|----------|----------|-------------|
| 2026-07-28 | Dedicated chain vs. modular vs. L1/L2 certificates? | v0.1 is settlement-agnostic agent trust protocol with pluggable settlement adapter layer; no native L1 at launch | CONSENSUS_AND_SETTLEMENT.md |
| 2026-07-28 | Concrete fault model? | Layered assumptions per component; agents fully adversarial; BFT threshold (n ≥ 3f+1) for verifier committees where applicable; settlement security inherited from backend | CONSENSUS_AND_SETTLEMENT.md |
| 2026-07-28 | Finality definition? | Five-stage model: PROPOSED → ACCEPTED → SETTLEMENT_FINAL → DISPUTE_WINDOW_FINAL → ECONOMIC_FINAL; machine-readable exposure required | CONSENSUS_AND_SETTLEMENT.md |
| 2026-07-28 | Data availability for dispute evidence? | Split evidence model: on-chain commitments/hashes; off-chain full evidence with availability obligations and economic penalties for non-availability | CONSENSUS_AND_SETTLEMENT.md |
| 2026-07-28 | Multi-backend settlement behind one identity? | Yes. Aether Agent ID is root; settlement accounts bound via SettlementBinding capabilities | CONSENSUS_AND_SETTLEMENT.md |
| 2026-07-28 | Serialization format (v0)? | Provisional: schema-locked CBOR primary wire; JSON audit fixtures secondary | V0_WIRE_CRYPTO_GROUP.md |
| 2026-07-28 | AgentId derivation (v0)? | Provisional: SHA-256 over stable material (schema_version + operational_public_key); excludes permission_root | V0_WIRE_CRYPTO_GROUP.md |
| 2026-07-28 | Signature primitive (v0)? | Provisional: Ed25519 (ed25519-dalek); signed-byte rules in DEC-004B now fixture-backed in Rust and Python, but group remains provisional | V0_WIRE_CRYPTO_GROUP.md |
| 2026-07-28 | Capability revocation (v0)? | Provisional: expiry + CapabilityRevoke + identity freeze/revoke | V0_WIRE_CRYPTO_GROUP.md |
| 2026-07-28 | Phase 2 begins? | Mainframe architecture gate; design only, no implementation | Project_Phases/phase_2/ |
| 2026-07-28 | Discovery layer boundary? | Provisional: `AgentDirectoryV0` is protocol; marketplace is app | DEC-P2-010 |
| 2026-07-28 | Reputation model for PROTO-3? | Provisional: indexer-derived from signed events with evidence refs | DEC-P2-004 |

## Links

- [CONSENSUS_AND_SETTLEMENT.md](../CONSENSUS_AND_SETTLEMENT.md)
- [RESEARCH_AGENDA.md](../RESEARCH_AGENDA.md)
- Sibling research notes in this folder
