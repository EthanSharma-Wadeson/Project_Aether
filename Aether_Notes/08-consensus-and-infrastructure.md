# 08 — Consensus & Infrastructure

**Key takeaway:** Aether v0.1 is a settlement-agnostic agent trust protocol — not a new L1. Consensus touchpoints handle identity, bonds, settlements, and disputes via pluggable backends. Finality is five-stage and machine-readable.

> **Updated:** Reflects [CONSENSUS_AND_SETTLEMENT.md](../Aether_docs/CONSENSUS_AND_SETTLEMENT.md) ADR (2026-07-28).

---

## Architecture decision (v0.1)

**Decided:** No native L1 at launch. Aether defines agent semantics; settlement backends handle ordering and base security.

```
AI AGENTS
    ↓
AETHER PROTOCOL (identity, capabilities, tasks, attestations, disputes, reputation)
    ↓
SETTLEMENT ADAPTER LAYER (Backend A • Backend B • Future Aether Chain)
```

A native chain is only justified if prototypes prove existing backends cannot meet requirements.

---

## What goes on-chain vs. off-chain

| Requires settlement backend (global ordering) | Typically off-chain |
|-----------------------------------------------|---------------------|
| Identity register / freeze / revoke | Channel update stream |
| Bond lock / slash / release | Bilateral negotiation |
| Channel open / settle / dispute resolve | Local attestation verification |
| Evidence commitments (hashes, not full data) | Full task payloads, logs, model artefacts |
| Protocol parameter updates | Discovery gossip |

---

## Fault model (by layer)

| Component | Assumption |
|-----------|------------|
| Settlement backend | Security inherited from selected backend |
| Autonomous agents | May be malicious, buggy, compromised |
| Aether verifiers | May be unavailable, incorrect, malicious, colluding |
| Network | Delays, partitions, outages possible |
| Storage providers | May refuse to serve evidence |
| Principals | May misconfigure permissions |

For BFT verifier committees: conventional threshold `n ≥ 3f + 1` (fewer than one-third Byzantine). Not automatic for every workflow.

---

## Five-stage finality (not one "confirmed" flag)

```
PROPOSED → ACCEPTED → SETTLEMENT_FINAL → DISPUTE_WINDOW_FINAL → ECONOMIC_FINAL
```

| State | Meaning |
|-------|---------|
| Proposed | Submitted, not yet accepted |
| Accepted | Passed initial validation; not economically complete |
| Settlement Final | Final per backend rules |
| Dispute-Window Final | Dispute period expired without challenge |
| Economic Final | Funds, bonds, reputation applied — workflow complete |

Agents must receive this as **machine-readable protocol state**, not informal descriptions.

Channel updates remain bilateral soft finality off the hot path.

---

## Dispute windows (task-class dependent)

No universal dispute period. Examples (not final parameters):

| Task class | Illustrative window |
|------------|---------------------|
| Deterministic machine-verifiable | Seconds to minutes |
| Standard API/compute service | Minutes to hours |
| Complex AI-generated output | Hours to days |
| Human-reviewed / high-value | Days or longer |

Task contracts specify dispute config **before** work begins.

---

## Evidence model

**On-chain:** commitment/hash, content ID, timestamp, submitter, dispute status, retention deadline.

**Off-chain:** full outputs, logs, datasets, model artefacts, private materials.

> A hash proves integrity, not availability.

Participants must keep evidence available through task + dispute + retention period. Failure → adverse dispute outcome or economic penalty.

---

## Multi-backend identity

Aether Agent ID is **not** an Ethereum address. Settlement accounts bind via `SettlementBinding`:

- backend_id, account_identifier, authorised_key, capability_scope, expiry

One identity → multiple backends → portable reputation.

---

## Settlement backend interface (conceptual)

Operations: lock_escrow, update_escrow, submit_attestation, open_dispute, submit_evidence_commitment, resolve_dispute, release_funds, apply_slashing, get_finality.

Backends differ in cost, latency, finality, assets, privacy — Aether must expose these differences honestly.

---

## v0.1 consensus strategy

1. Define settlement-independent protocol semantics
2. Create backend adapter interface
3. Test multiple settlement environments
4. Compare cost, latency, finality, security, DX
5. Identify gaps existing infrastructure cannot fill

No native consensus algorithm selected for v0.1.

---

## Backend evaluation criteria

Cost, latency, finality clarity, security assumptions, scalability, interoperability, programmability, privacy, reliability under congestion, developer experience.

---

## Still open

- Which backend(s) for first prototype
- Verifier committee design per task class
- Exact dispute period bounds and evidence retention
- Backend-specific finality in protocol schema
- Cross-backend reputation effects

**Next:** [09-economics-and-security.md](09-economics-and-security.md)
