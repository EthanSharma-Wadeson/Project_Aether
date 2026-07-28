# PROTO_1_DESIGN.md — Phase 1

## Status

**Design resolved + acceptance tests frozen — implementation not started**

PROTO-0 is complete and reviewed. PROTO-1 implementation may begin only after this package is accepted:

- state model in this document
- decisions in [PROTO_1_DECISIONS.md](PROTO_1_DECISIONS.md)
- frozen tests in [PROTO_1_ACCEPTANCE_TESTS.md](PROTO_1_ACCEPTANCE_TESTS.md)

This document does **not** itself authorise networking, settlement, or PROTO-2 work.

References:

- [PHASE_1_KICKOFF.md](../phase_0/PHASE_1_KICKOFF.md) §5
- [PROTO_0_ACCEPTANCE_TESTS.md](PROTO_0_ACCEPTANCE_TESTS.md)
- [PROTO_0_SECURITY_REVIEW.md](PROTO_0_SECURITY_REVIEW.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
- [PROTOCOL_DESIGN.md](../../Aether_docs/PROTOCOL_DESIGN.md)
- Phase 0 invariants in [CONTEXT.md](../phase_0/CONTEXT.md)

---

## 1. Purpose

PROTO-1 explores **bilateral agent interaction primitives** through a simulated channel model.

It asks:

> Can two authenticated, capability-authorised agents safely maintain an evolving shared state without immediate settlement infrastructure?

PROTO-1 must prove that agents can:

- open a bilateral interaction under PROTO-0 authority checks
- exchange signed state updates with monotonic sequencing
- reject replayed, forged, stale, or unauthorised updates
- expose machine-readable soft vs harder finality placeholders
- close cooperatively or via a simulated dispute path using the highest valid signed state

It maps to hypotheses:

- **H1** — channels beat direct on-chain micropayments for high-frequency dust updates (simulated cost/latency comparison)
- **H6** — soft vs hard finality can be made machine-rational when exposed explicitly

---

## 2. Scope

### In scope

- in-memory bilateral channel simulator
- two-party identity and capability checks via PROTO-0
- signed state updates
- sequence / nonce monotonicity
- balance conservation (simulated units)
- replay and stale-state rejection
- cooperative close
- disputed close using highest valid signed state
- explicit finality-field exposure (local agreement vs settlement placeholder vs dispute-window state)
- deterministic unit and adversarial tests
- H1 / H6 measurement harnesses (local only)

### Out of scope / non-goals

PROTO-1 is **not**:

- a blockchain channel
- a production Lightning-style payment channel
- a live settlement backend integration
- a networking / P2P protocol
- a native token or wallet
- a reputation system
- an AI reasoning runtime
- a marketplace
- a multiparty channel mesh
- production liquidity management

Do **not** introduce:

- native token
- real payments
- blockchain integration
- settlement backend
- reputation system
- AI model logic
- networking layer

---

## 3. Relationship to PROTO-0

PROTO-1 **consumes** PROTO-0. It must not reimplement or weaken identity/capability enforcement.

| Concern | Owner |
|---------|-------|
| Agent identity authentication | PROTO-0 |
| Capability grant verification | PROTO-0 |
| Delegation narrowing / revoke / freeze / stale root | PROTO-0 |
| Channel state machine, sequencing, conservation | PROTO-1 |
| Simulated economic gate before channel open/update/close | PROTO-0 `authorise_action` + PROTO-1 transition rules |

### Mandatory coupling rules

1. Opening a channel requires both participants to be registered, `active`, and to present valid capabilities for the open action.
2. Each state update that moves simulated value or commits a new channel state must pass capability checks **before** the channel accepts the transition (Phase 0 invariant 11).
3. Identity freeze/revoke or capability revoke must fail closed for new channel opens/updates.
4. A valid channel update signature is **not** sufficient without capability authorisation (Phase 0 invariant 5 / security model).
5. PROTO-0 code is modified only if a contradiction is discovered and recorded; prefer PROTO-1 adapters over changing PROTO-0 semantics.

### What PROTO-1 adds

- bilateral shared state
- monotonic sequence / nonce
- dual-party agreement model for updates
- soft/hard/dispute-window finality placeholders
- close and dispute transition logic

---

## 4. Assumptions

Inherited and provisional:

1. PROTO-0 trust assumptions remain in force (coherent local state, trusted logical `now`, current-state root validation, subject binding, canonical CBOR, local revoke).
2. Exactly **two** participants per channel in v0.
3. Simulated asset units are abstract integers; no real asset custody.
4. Both parties (or an agreed signing quorum of both) must sign accepted state updates unless a transition explicitly defines a unilateral challenge path.
5. Logical time is injected; dispute-window expiry uses the same logical clock model as PROTO-0.
6. “Hard settlement” in PROTO-1 is a **placeholder status**, not a backend call.
7. H1 baseline “direct settlement” is a simulated cost/latency model, not a live chain.
8. Channel capability action selectors will be a minimal v0 set (exact tokens TBD in acceptance tests), e.g. `channel.open`, `channel.update`, `channel.close`.

---

## 5. Logical Data Models

Logical structures only. Wire CBOR field order will be frozen with acceptance tests / schema notes before implementation.

### 5.1 `ChannelV0`

```text
ChannelV0 {
  protocol_version: u32,
  schema_version: u32,
  channel_id: bytes32,                 // commitment to opening material
  participants: [AgentId; 2],          // ordered, canonical
  asset: string,                       // simulated unit id
  opening_balances: [u64; 2],          // aligns with participants order
  total_deposit: u64,                  // must equal sum(opening_balances)
  dispute_window: u64,                 // logical-time duration
  created_at: u64,                     // logical time
  status: ChannelStatus,
  sequence: u64,                       // highest accepted update sequence
  current_state_commitment: bytes32,   // hash of latest accepted ChannelStateV0
  finality: FinalityViewV0
}
```

`channel_id` provisional construction:

```text
channel_id = SHA-256(canonical_cbor(ChannelOpenMaterialV0))
```

where open material binds participants, asset, opening balances, dispute window, and creator nonces / open message ids (exact open-material fields to freeze with acceptance tests).

### 5.2 `ChannelStatus`

```text
ChannelStatus =
  | Open
  | Active
  | Closing
  | Disputed
  | Finalized
```

Notes:

- Kickoff used `open | active | closing | disputed | settled`.
- PROTO-1 uses **`Finalized`** as the terminal simulated status. It does **not** imply backend settlement finality.
- `Disputed` is retained for the challenge path before finalization.

### 5.3 `ChannelStateV0` (payload inside updates)

```text
ChannelStateV0 {
  channel_id: bytes32,
  sequence: u64,
  balances: [u64; 2],
  asset: string,
  status_hint: ChannelStatus,          // proposed status after this update
  metadata_commitment: optional bytes  // opaque; not privately revealing
}
```

Conservation rule:

```text
sum(balances) == channel.total_deposit
all balances >= 0
asset unchanged
participants unchanged
```

### 5.4 `StateUpdateV0`

```text
StateUpdateV0 {
  protocol_version: u32,
  schema_version: u32,
  channel_id: bytes32,
  previous_state_commitment: bytes32,
  new_state: ChannelStateV0,
  sequence: u64,                       // must equal new_state.sequence
  logical_time: u64,
  // signatures belong on SignedMessage envelopes, not as authoritative
  // duplicated fields inside the unsigned body — same boundary as CapabilityV0
}
```

Transport form:

```text
ChannelUpdateGrant = SignedMessage<StateUpdateV0>   // per required signer
```

PROTO-1 acceptance of an update requires the configured signer set (both participants for normal updates) after each envelope verifies.

### 5.5 `ReceiptV0` (local transition receipt)

PROTO-1 receipts record that a **channel transition was accepted by the simulator**. They are not PROTO-2 escrow work receipts.

```text
ReceiptV0 {
  protocol_version: u32,
  schema_version: u32,
  receipt_id: bytes32,                 // hash of canonical receipt body without id
  channel_id: bytes32,
  action: string,                      // open | update | close | dispute | finalize
  participants: [AgentId; 2],
  sequence: u64,
  state_commitment: bytes32,
  previous_state_commitment: optional bytes32,
  logical_time: u64,
  finality_snapshot: FinalityViewV0,
  result: accepted | rejected_reason_code
}
```

### 5.6 `FinalityViewV0`

Machine-readable fields for H6:

```text
FinalityViewV0 {
  soft_local_agreement: bool,          // both parties signed latest state
  dispute_window_open: bool,
  dispute_deadline: optional u64,      // logical time
  hard_settlement_placeholder: bool,   // always false until a backend exists
  finalized: bool                      // channel status == Finalized
}
```

Agents must not infer “funds irreversible on a backend” from `soft_local_agreement` alone.

---

## 6. State Machine

### 6.1 Allowed transitions

```text
Open
  → Active          // dual-signed initial ChannelStateV0 at sequence = 0 (P1-DEC-002)
  → Closing         // not used pre-activate; prefer abort
  → Finalized       // dual-signed abort without activation (P1-DEC-006)

Active
  → Active          // dual-signed StateUpdateV0 with sequence = current+1
  → Closing         // cooperative close initiated (dual-signed)
  → Disputed        // admissible DisputeEvidenceV0

Closing
  → Finalized       // cooperative close completed on latest agreed state
  → Disputed        // challenge during close / dispute window

Disputed
  → Finalized       // highest valid dual-signed sequence (simulated)

Finalized
  → ∅               // terminal
```

### 6.2 Invalid transitions (must reject)

- any transition from `Finalized`
- `Disputed → Active` (no silent un-dispute without finalize rules)
- skipping sequence numbers
- applying an update with `sequence <= current sequence`
- changing participants or asset mid-channel
- negative balances or non-conserving totals
- status_hint that is not allowed from current status
- accepting updates while identity frozen/revoked or capability invalid

### 6.3 Replay prevention

- store highest accepted `sequence`
- reject any update with `sequence <= highest`
- reject duplicate payloads that replay an older `previous_state_commitment` after a newer state exists

### 6.4 Stale state rejection

Reject if:

- `previous_state_commitment` ≠ current channel commitment
- update body sequence does not match claimed new state sequence
- logical_time rules violated (e.g. dispute after deadline — exact rule in acceptance tests)
- capability or root-version is stale under PROTO-0 current-state rules

### 6.5 Closure behaviour

**Cooperative close**

1. both parties authorised for `channel.close`
2. latest agreed state used
3. status → `Closing` then `Finalized`
4. `FinalityViewV0.finalized = true`
5. `hard_settlement_placeholder` remains false

**Disputed close**

1. challenger presents a signed state with higher valid sequence than currently applied, or contests an invalid close
2. status → `Disputed`
3. dispute window runs on logical time
4. resolution selects highest valid dual-signed (or rules-valid) state
5. status → `Finalized` with that state commitment

Exact challenge evidence object can be refined in acceptance tests; keep minimal.

---

## 7. Security Properties and Invariant Map

### 7.1 Security properties PROTO-1 must uphold

| Property | Requirement |
|----------|-------------|
| Authentication | Updates/open/close messages verify under participant keys |
| Authorization | PROTO-0 capability checks before accepting economic/state transitions |
| Integrity | Only monotonic, conserving, participant-bound states accepted |
| Replay resistance | Old sequences cannot replace newer state |
| Fail closed | Malformed, forged, unauthorised, stale → reject |
| Finality honesty | Soft agreement ≠ hard settlement placeholder |

### 7.2 Adversarial cases (design targets for later acceptance tests)

| Attack | Expected |
|--------|----------|
| Replay older update | Reject |
| Forge update signature | Reject |
| Unauthorized participant / capability bypass | Reject |
| Nonce/sequence manipulation (skip, rewind) | Reject |
| Participant mismatch | Reject |
| Balance inflation / non-conservation | Reject |
| Invalid status transition | Reject |
| Close with stale state after newer agreed state | Reject |
| Identity frozen/revoked mid-channel update | Reject new updates |
| Treat soft agreement as hard settlement in agent logic tests | H6 failure if agents cannot distinguish |

### 7.3 Phase 0 invariant mapping

| Invariant | PROTO-1 obligation |
|-----------|--------------------|
| 1 Capability envelope limits | Open/update/close require envelopes that cover the action and spend/limits |
| 2 / 12 Delegation bounds | Inherited from PROTO-0; channel must not invent wider authority |
| 3 Expiry/revoke | Revoked/expired caps and frozen identities cannot authorise new transitions |
| 4 Settlement not from identity alone | Channel open/update needs capability, not identity claim alone |
| 5 Signature ≠ correctness | Dual signatures prove agreement on bytes, not backend finality or task truth |
| 6 Identity continuity | Channel binds `AgentId`s; root/key rotation policy must not silently rebind participants without explicit rules |
| 7 Deterministic transitions | Same inputs → same accept/reject and next state |
| 8 Dispute reproducibility | Simulated dispute resolution must be deterministic from rules + admissible signed states |
| 9 Commitment ≠ disclosure | `metadata_commitment` / receipts must not force private payload exposure |
| 10 Backend isolation | No backend in path; channel state must not depend on settlement adapter failures |
| 11 Capability before economic auth | Enforce before accepting balance-moving updates and opens |

---

## 8. Operations (logical API)

Design-level operations for the future simulator:

| Operation | Meaning |
|-----------|---------|
| `OpenChannel` | Create channel after PROTO-0 checks for both parties |
| `ApplyUpdate` | Verify signatures + capabilities + conservation + sequence; advance state |
| `BeginClose` | Move to `Closing` from latest agreed state |
| `RaiseDispute` | Move to `Disputed` with competing signed evidence |
| `Finalize` | Terminalise after cooperative close or dispute resolution |
| `InspectFinality` | Return `FinalityViewV0` |

No network send/receive primitives.

---

## 9. Unresolved Questions

Design questions from the preparation draft are **resolved** in [PROTO_1_DECISIONS.md](PROTO_1_DECISIONS.md).

| Topic | Decision | Status |
|-------|----------|--------|
| Signer quorum | Dual signatures required to accept updates | Locked (P1-DEC-001) |
| Open → Active | Dual-signed initial sequence=0 state | Locked (P1-DEC-002) |
| Action identity | channel_id + sequence + commitments + receipts | Locked (P1-DEC-003) |
| Dispute evidence | Minimal `DisputeEvidenceV0` | Provisional (P1-DEC-004) |
| H1 baseline | Simulated costs only; no real-chain proof | Provisional (P1-DEC-005) |
| Abort / close / disagreement | Dual-sign close; Open abort allowed; dispute on conflict | Locked / Provisional (P1-DEC-006) |
| Action tokens | `channel.open/activate/update/close/dispute` | Locked (P1-DEC-007) |

### Remaining tracked risks (not pretend-solved)

- logical-time integrity for dispute windows
- local revocation coherence
- operational-key rotation mid-channel
- synthetic H1 parameters
- non-distributed dispute model

Acceptance tests: [PROTO_1_ACCEPTANCE_TESTS.md](PROTO_1_ACCEPTANCE_TESTS.md).

---

## 10. Implementation Boundary (for later)

When coding is approved:

- production-intent channel logic under `Aether/core/` (e.g. `channel/` module) depending on PROTO-0 identity/capability APIs
- metrics / H1 harness may live under `Aether/experiments/proto/` and must not become a hidden `core/` dependency
- do not modify PROTO-0 semantics unless a recorded contradiction appears

---

## 11. Path to Implementation

Acceptance tests are frozen in [PROTO_1_ACCEPTANCE_TESTS.md](PROTO_1_ACCEPTANCE_TESTS.md).

**Next task:** implement PROTO-1 in Rust against `P1-T###` / `P1-A##` / `P1-I##` IDs.

Do not weaken frozen tests to make code pass.

---

## 12. Freeze Statement

> PROTO-1 design decisions and acceptance tests are ready for implementation review.

Coding starts only when explicitly approved. Networking, settlement, tokens, reputation, and AI remain forbidden in PROTO-1.
