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
| Attestation profile for early receipts | Provisional | Project Lead | deterministic receipt-first for `PROTO-2` |
| Claim types for non-deterministic outputs | Provisional | Project Lead | out of objective-verification scope for v0 prototypes |
| In-memory simulator vs backend stub for channels | Provisional | Project Lead | in-process bilateral simulator for `PROTO-1` |
| Settlement backend shortlist / spike timing | Deferred | Project Lead | not part of `PROTO-0`–`PROTO-2` |

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

- [ ] First vertical for end-to-end prototype (which agent market)?
- [ ] Discovery/directory: protocol service or application layer only?
- [ ] Governance surface for parameters in phase 1?

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

## Links

- [CONSENSUS_AND_SETTLEMENT.md](../CONSENSUS_AND_SETTLEMENT.md)
- [RESEARCH_AGENDA.md](../RESEARCH_AGENDA.md)
- Sibling research notes in this folder
