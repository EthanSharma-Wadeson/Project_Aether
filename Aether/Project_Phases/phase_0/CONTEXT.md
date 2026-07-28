# CONTEXT.md — Phase 0: Pre-Implementation Readiness

## Status

**Closed — exit review passed; provisional review items carried into Phase 1**

Phase 0 existed to convert design documentation into an implementation-ready foundation. The Phase 0 exit review has passed; protocol implementation still begins only through the ordered Phase 1 sequence in [PHASE_1_KICKOFF.md](PHASE_1_KICKOFF.md).

Small, disposable **decision experiments** are permitted when required to evaluate a blocking implementation choice — see [Decision Experiments](#decision-experiments).

---

## Naming Convention: Phases vs Prototypes

**Project phases** and **implementation prototypes** use distinct identifiers to avoid ambiguity in issues, commits, tests, and reviews.

| Kind | IDs | Examples |
|------|-----|----------|
| **Project phases** | Phase 0, Phase 1, Phase 2, Phase 3 | Phase 0 exit criteria |
| **Prototypes** | PROTO-0 … PROTO-5 | PROTO-0 adversarial tests |

```text
Phase 0 → Pre-Implementation Readiness
Phase 1 → Primitives and Threat Foundations
Phase 2 → Networked Settlement and Economic Safety
Phase 3 → Open Agent Economy Substrate

PROTO-0 → Identity and Capability Prototype
PROTO-1 → Payment Channel Prototype
PROTO-2 → Escrow Prototype
PROTO-3 → Reputation Indexer Prototype
PROTO-4 → Settlement Backend Spike
PROTO-5 → Selective Disclosure Credential Demo (stretch)
```

Do not use `P0`–`P5` — these collide with phase numbers. Full definitions: [prototypes.md](../Aether_docs/experiments/prototypes.md).

---

## Position in the Project Lifecycle

```text
Phase 0  →  Phase 1  →  Phase 2  →  Phase 3
(this)      (code)      (network)    (substrate)
```

| Phase | Name | Focus |
|-------|------|-------|
| **0** | Pre-Implementation Readiness | Lock decisions, repo structure, implementation constraints |
| 1 | Primitives & Threat Foundations | PROTO-0–PROTO-2: identity, channels, escrow |
| 2 | Networked Settlement & Economic Safety | Live backends, bonds, disputes |
| 3 | Open Agent Economy Substrate | External builders, pluggable verification |

Phase 0 maps to the period **before** [phase_1.md](../Aether_docs/roadmap/phase_1.md) coding begins. Phase 1 roadmap objectives still apply once Phase 0 exits.

---

## Mission (unchanged)

Build decentralized trust, identity, and payment infrastructure tailored for autonomous AI agents.

Source of truth for project mission and non-goals: [Aether_docs/CONTEXT.md](../Aether_docs/CONTEXT.md).

---

## Decision Precedence

When project documents conflict, the following order applies:

1. Security invariants (this document, [Security Invariants](#security-invariants))
2. Accepted architecture decisions ([CONSENSUS_AND_SETTLEMENT.md](../Aether_docs/CONSENSUS_AND_SETTLEMENT.md), [DECISIONS.md](DECISIONS.md))
3. Protocol specifications ([PROTOCOL_DESIGN.md](../Aether_docs/PROTOCOL_DESIGN.md), [V0_WIRE_CRYPTO_GROUP.md](V0_WIRE_CRYPTO_GROUP.md))
4. Phase-specific implementation constraints ([IMPLEMENTATION_CONSTRAINTS.md](IMPLEMENTATION_CONSTRAINTS.md))
5. Experiments and prototypes ([prototypes.md](../Aether_docs/experiments/prototypes.md), [hypotheses.md](../Aether_docs/experiments/hypotheses.md))
6. Roadmaps and planning documents

Experiments may challenge an accepted decision but do not silently override it.

Any change to a locked architectural decision requires:

- new evidence
- an explicit decision record in [DECISIONS.md](DECISIONS.md)
- documented rationale
- identified risks
- a review trigger
- updates to all affected documentation

---

## Decision Ownership

A **decision owner** is responsible for:

- gathering relevant evidence
- identifying viable alternatives
- documenting trade-offs
- proposing a decision
- recording known risks
- defining a review trigger
- ensuring affected documentation is updated

Decision ownership does not imply unilateral authority to modify locked architectural decisions.

Changes to locked decisions must follow [Decision Precedence](#decision-precedence) and the architecture-change process above.

For a single-founder project, unassigned owners default to **Project Lead**.

---

## Security Invariants

The following properties **MUST** hold for Aether v0 unless changed through an explicit architecture decision supported by new evidence.

1. An agent **MUST NOT** exceed an active capability envelope.

2. A delegated capability **MUST NOT** grant authority greater than its parent capability.

3. Expired or revoked capabilities **MUST NOT** authorise new actions.

4. Settlement authority **MUST NOT** be inferred solely from an identity claim.

5. A signature proves control of an authorised key. It does not prove that an agent's output is correct, truthful, or safe.

6. Identity history **MUST** remain attributable across authorised operational-key rotation.

7. Canonical protocol state transitions **MUST** be deterministic.

8. Dispute outcomes **MUST** be reproducible from the applicable protocol rules and admissible evidence.

9. Recording an evidence commitment **MUST NOT** automatically expose private evidence.

10. A settlement-backend failure **MUST NOT** silently corrupt canonical Aether identity or capability state.

11. Capability enforcement **MUST** occur before an externally visible economic action is authorised.

12. A child delegation **MUST** remain bounded by the scope, expiry, limits, and delegation depth of its parent.

These invariants take priority over language, serialization format, settlement backend, and repository structure choices.

---

## What Phase 0 Is

Phase 0 is **decision-locking and preparation**, not protocol implementation.

It answers: *"What must we know and agree on before the first line of protocol implementation code?"*

Activities include:

- Resolving blocking open questions (or recording explicit provisional choices in [DECISIONS.md](DECISIONS.md))
- Choosing implementation language, repo layout, and tooling
- Freezing v0 wire-format assumptions needed for PROTO-0
- Defining how code, specs, and experiments relate to each other
- Establishing contribution and testing norms for the implementation repo
- Mapping security invariants to planned Phase 1 tests

---

## Decision Experiments

No protocol implementation begins until Phase 0 exit criteria are met.

Small, disposable **decision experiments** are permitted when required to evaluate a blocking implementation choice.

Examples: comparing serialization formats, cross-language encoding checks, signature-library validation, capability-schema feasibility, dependency behaviour.

Decision experiments **MUST**:

- remain isolated from protocol implementation
- avoid establishing production behaviour
- avoid becoming an undocumented implementation foundation
- document the question being tested
- record the result and decision impact in [DECISIONS.md](DECISIONS.md)
- be deleted, archived, or explicitly promoted after the decision

A decision experiment does **not** constitute the beginning of Phase 1.

---

## What Phase 0 Is Not

- Writing protocol prototypes (that is Phase 1 / PROTO-0–PROTO-2)
- Selecting a production settlement backend (Phase 2 framing may begin in Phase 1; selection remains Phase 2)
- Launching a token or network
- Building agent UIs, LLM wrappers, or marketplaces
- Pretending all design questions are answered

---

## Architectural Context (Already Decided)

These decisions are locked and should not be re-litigated during Phase 0 unless new evidence contradicts them. See [CONSENSUS_AND_SETTLEMENT.md](../Aether_docs/CONSENSUS_AND_SETTLEMENT.md).

| Decision | Summary |
|----------|---------|
| No native L1 at v0.1 | Settlement-agnostic agent trust protocol with pluggable backends |
| Fault model | Agents adversarial; settlement security inherited from backend |
| Finality | Five stages: PROPOSED → ACCEPTED → SETTLEMENT_FINAL → DISPUTE_WINDOW_FINAL → ECONOMIC_FINAL |
| Evidence | On-chain commitments; off-chain storage with availability obligations |
| Identity | Aether Agent ID is root; settlement via SettlementBinding |

---

## Blocking Questions (Must Resolve or Provisionally Lock)

Phase 0 cannot exit while these remain unowned. Each must be either **decided** or **explicitly marked provisional** in [DECISIONS.md](DECISIONS.md) with a review trigger.

### Critical path (blocks PROTO-0)

| # | Question | Owner | Status |
|---|----------|-------|--------|
| 1 | Implementation language and runtime | Project Lead | Provisional (DEC-001; experiment PASS) |
| 2 | Repository layout (monorepo vs packages; where code lives under `Aether/`) | Project Lead | Provisional (DEC-002; scaffold created) |
| 3 | Serialization format for protocol messages | Project Lead | Provisional (DEC-003; see wire group) |
| 4 | Signature scheme (e.g. Ed25519-class) | Project Lead | Provisional (DEC-004A/B; 004A PASS) |
| 5 | Agent ID derivation and multi-key commitment scheme | Project Lead | Provisional (DEC-005; after DEC-007) |
| 6 | Minimal viable identity object (fields for v0) | Project Lead | Provisional (DEC-006; see wire group) |
| 7 | Capability envelope schema for v0 (spend, rate, counterparty, expiry) | Project Lead | Provisional (DEC-007; see wire group) |
| 8 | Capability revocation approach for v0 | Project Lead | Provisional (DEC-008; see wire group) |

### Important (review during Phase 1 sequencing; do not block Phase 0 exit)

| # | Question | Owner | Status |
|---|----------|-------|--------|
| 9 | v1 attestation profile (receipts-first assumed; confirm or reject) | Project Lead | Provisional — deterministic receipt-first for PROTO-2 |
| 10 | Claim types for non-deterministic agent outputs | Project Lead | Provisional — out of objective-verification scope for v0 prototypes |
| 11 | In-memory/simulated ledger vs early backend stub for channels | Project Lead | Provisional — in-process bilateral simulator for PROTO-1 |
| 12 | Settlement backend shortlist / spike timing | Project Lead | Deferred — outside PROTO-0–PROTO-2 scope |

### Deferred to Phase 1+ (track, do not block Phase 0 exit)

- Native token timing
- Hub-and-spoke vs mesh topology
- Reputation canonical vs indexer-derived aggregates
- Governance surface
- First vertical agent market

Full backlog: [unanswered_questions.md](../Aether_docs/research/unanswered_questions.md).

---

## Provisional Defaults (Starting Points — Not Final)

Use these to unblock discussion. Record each in [DECISIONS.md](DECISIONS.md) with status, confidence, and review trigger.

| Area | Provisional default | Review trigger |
|------|---------------------|----------------|
| Attestation v0 | Deterministic output receipts + escrow timeouts | PROTO-2 dispute rate |
| Channel v0 | In-process bilateral simulator; no live chain | PROTO-1 metrics |
| Identity v0 | Ed25519 operational keys; AgentId = hash of canonical key material | PROTO-0 interop review |
| Serialization v0 | Schema-locked CBOR primary wire; JSON audit fixtures secondary | First additional non-Rust consumer |
| Testing | Adversarial unit tests for capability bypass (H2) before any network code | Phase 1 exit |

---

## Phase 0 Objectives

1. **Own the critical-path questions** — every blocking item has an owner and entry in DECISIONS.md
2. **Publish implementation constraints doc** — language, layout, deps, testing norms
3. **Freeze v0 identity + capability wire/crypto group** — [V0_WIRE_CRYPTO_GROUP.md](V0_WIRE_CRYPTO_GROUP.md)
4. **Define repo structure under `Aether/`** — where implementation, tests, and experiments live
5. **Align documentation** — Aether_docs, Aether_Notes, and Project_Phases stay consistent
6. **Write Phase 1 kickoff plan** — ordered task list for PROTO-0 → PROTO-1 → PROTO-2 with hypothesis mapping
7. **Map security invariants to planned Phase 1 tests**

---

## Scope In

- Decision records for implementation prerequisites ([DECISIONS.md](DECISIONS.md) first)
- v0 wire-format drafts for identity and capabilities
- Repo scaffolding plan (directories, naming, CI intent — not necessarily full CI yet)
- Provisional choice log with confidence levels and review triggers
- Phase 1 engineering plan document
- Hygiene on `unanswered_questions.md` (owners assigned to critical items)
- Isolated decision experiments for blocking choices

## Scope Out

- Protocol library implementation (Phase 1)
- Payment channel simulator (PROTO-1)
- Escrow loop (PROTO-2)
- Settlement backend integration (PROTO-4)
- Token design or deployment
- Production infrastructure

---

## Exit Criteria

Phase 0 is complete when **all** of the following are true:

- [x] Implementation language and repo layout decided and documented
- [x] v0 serialization and signature scheme decided (or provisional with review trigger)
- [x] Minimal identity object and capability envelope schema drafted
- [x] Agent ID derivation approach documented (even if provisional)
- [x] Critical-path open questions have named owners in `unanswered_questions.md`
- [x] Every critical-path decision in [DECISIONS.md](DECISIONS.md) includes:
  - selected option
  - alternatives considered
  - rationale
  - relevant evidence
  - known risks
  - decision status (`Locked` | `Provisional` | `Deferred` | `Blocked`)
  - review trigger
  - named owner
- [x] Security invariants reviewed and mapped to planned Phase 1 tests
- [x] Repo structure plan exists (directories under `Aether/` defined)
- [x] Phase 1 kickoff plan written (PROTO-0 → PROTO-1 → PROTO-2 sequence, hypothesis mapping)
- [x] No protocol implementation merged except scaffolding agreed in the repo plan (optional: empty package, lint config — team choice)

### Exit review result

**Result:** PASS (2026-07-28)

Phase 0 is formally closed with the following carried into Phase 1 as **Provisional review items**, not Phase 0 blockers:

- permission-root semantics and root-authority descriptor shape
- root-version update semantics
- time semantics
- current-state versus historical authority validation
- signed transport/container clarification follow-through in implementation

These items are sequenced explicitly in [PHASE_1_KICKOFF.md](PHASE_1_KICKOFF.md) and do not reopen Phase 0 by themselves.

---

## Deliverables

Create in this order:

| Order | Artifact | Location |
|-------|----------|----------|
| 1 | Decision log | [DECISIONS.md](DECISIONS.md) |
| 2 | Wire/crypto group (DEC-003–008) | [V0_WIRE_CRYPTO_GROUP.md](V0_WIRE_CRYPTO_GROUP.md) |
| 3 | Implementation constraints | [IMPLEMENTATION_CONSTRAINTS.md](IMPLEMENTATION_CONSTRAINTS.md) |
| 4 | v0 wire schema + fixtures | [schemas/v0/](../../schemas/v0/) |
| 5 | Phase 1 kickoff plan | [PHASE_1_KICKOFF.md](PHASE_1_KICKOFF.md) |
| — | Phase 0 context | [CONTEXT.md](CONTEXT.md) |
| — | Experiment results | [experiments/decision/RESULTS.md](../../experiments/decision/RESULTS.md) |

---

## Key Risks

| Risk | Mitigation |
|------|------------|
| Phase 0 becomes endless design | Time-box; use provisional defaults with review triggers |
| Coding starts before identity schema is drafted | Exit criteria gate Phase 1 |
| Re-litigating settled ADR decisions | Reference CONSENSUS_AND_SETTLEMENT.md; change only with evidence |
| Scope creep into tokenomics or UI | Enforce non-goals from project CONTEXT |
| Docs and code diverge | Spec changes require doc update in same change set (norm from Phase 1 onward) |
| Decision experiments become stealth implementation | Decision experiment rules; promote or delete after decision |

---

## Hypotheses Relevant to Phase 0

Phase 0 does not run protocol prototypes, but implementation choices should enable testing these in Phase 1:

| ID | Claim | First tested in |
|----|-------|-----------------|
| H2 | Capability envelopes contain principal loss | PROTO-0 |
| H1 | Channels beat on-chain micropayments | PROTO-1 |
| H6 | Soft vs hard finality is machine-rational | PROTO-1 |
| H4 | Receipt-based escrow is enough for early markets | PROTO-2 |
| H5 | Agents can budget protocol fees autonomously | PROTO-2 |

See [hypotheses.md](../Aether_docs/experiments/hypotheses.md).

---

## Document Map

| Resource | Role |
|----------|------|
| [Aether_docs/CONTEXT.md](../Aether_docs/CONTEXT.md) | Project mission and design philosophy |
| [Aether_docs/CONSENSUS_AND_SETTLEMENT.md](../Aether_docs/CONSENSUS_AND_SETTLEMENT.md) | Settlement architecture ADR |
| [Aether_docs/roadmap/phase_1.md](../Aether_docs/roadmap/phase_1.md) | First coding phase (after Phase 0) |
| [Aether_docs/experiments/prototypes.md](../Aether_docs/experiments/prototypes.md) | PROTO-0–PROTO-5 prototype definitions |
| [Aether_docs/research/unanswered_questions.md](../Aether_docs/research/unanswered_questions.md) | Open question backlog |
| [Aether_Notes/README.md](../../Aether_Notes/README.md) | Learning guide for contributors |

---

## Working Principles for Phase 0

1. **Decide or provisionally decide** — unowned questions block progress more than wrong early choices tested by prototypes
2. **Smallest honest stack** — v0 schema covers PROTO-0 needs only; do not spec Phase 3 features
3. **Headless-first** — all implementation norms assume agents, not humans, as primary consumers
4. **Evidence over preference** — language and format choices should cite constraints (team skill, interop, auditability), not taste alone
5. **Gate discipline** — Phase 1 protocol implementation starts only when exit criteria are met
6. **Invariants first** — no implementation choice may weaken security invariants without an explicit ADR

---

## Next

When exit criteria are met → begin Phase 1 per [phase_1.md](../Aether_docs/roadmap/phase_1.md), starting with **PROTO-0** (identity and capability prototype).

Proceed to [DECISIONS.md](DECISIONS.md) to record critical-path decisions before expanding the implementation plan.
