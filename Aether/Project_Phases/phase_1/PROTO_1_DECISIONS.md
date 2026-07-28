# PROTO_1_DECISIONS.md — Phase 1

## Status

**Design resolution record — not implementation**

Resolves open questions from [PROTO_1_DESIGN.md](PROTO_1_DESIGN.md) §9 so acceptance tests and later coding are deterministic.

Owner: Project Lead  
Date: 2026-07-28

Status meanings:

| Status | Meaning |
|--------|---------|
| **Locked** | Accepted for PROTO-1 v0 simulator; change requires explicit decision revision |
| **Provisional** | Working rule for PROTO-1; review after tests / metrics |
| **Deferred** | Intentionally out of PROTO-1 |

PROTO-0 assumptions are preserved unless a decision below records a change. No PROTO-0 contradiction is introduced here.

---

## Decision index

| ID | Question | Status | Confidence |
|----|----------|--------|------------|
| P1-DEC-001 | Signer quorum for state updates | Locked | High |
| P1-DEC-002 | Open → Active trigger | Locked | High |
| P1-DEC-003 | Action identity / replay references | Locked | High |
| P1-DEC-004 | Minimal dispute evidence | Provisional | Medium |
| P1-DEC-005 | H1 baseline scope | Provisional | Medium |
| P1-DEC-006 | Abort / close / disagreement paths | Locked | Medium-High |
| P1-DEC-007 | v0 channel capability action tokens | Locked | High |

---

## P1-DEC-001 — Signer quorum

**Question:** Who must sign a channel state transition?

**Chosen approach (Locked):**

Normal accepted state updates require **both participants’ signatures** over the same canonical `StateUpdateV0` body (dual `SignedMessage<StateUpdateV0>` with identical body bytes).

Flow:

1. Either participant may **propose** an update (create canonical body + own signature).
2. The counterparty must **approve** by signing the **identical** canonical body.
3. The simulator **accepts** only when both verified signatures are present and PROTO-0 capability checks pass for both acting subjects as required by the operation.
4. On disagreement: the proposal is **not applied**; channel state remains at the last accepted sequence. Disagreement alone does not move status to `Disputed`.

Unilateral messages:

- A single signature may exist as a pending proposal in the harness, but is **not** channel state.
- Unilateral **dispute raise** is allowed only under P1-DEC-004 / P1-DEC-006 (presenting already dual-signed higher state, or challenging an invalid close), not for inventing new balances alone.

**Reasoning:** Bilateral agreement is the smallest honest model of off-chain channel safety. One-signature acceptance would allow a party to rewrite balances without counterparty consent.

**Security implications:**

- Prevents unilateral balance theft via forged “updates”
- Aligns with invariant 5: signatures prove agreement on bytes, not backend finality
- Increases message count vs unilateral channels — acceptable for PROTO-1

**Review trigger:** If H1 latency model shows dual-sign overhead dominates; or PROTO-2 needs asymmetric update patterns.

---

## P1-DEC-002 — Open → Active transition

**Question:** What triggers `Open → Active`?

**Chosen approach (Locked):**

```text
Open  = channel object created after dual open acknowledgements
Active = both participants have dual-signed the initial ChannelStateV0 (sequence = 0)
```

Required messages:

1. `ChannelOpenRequest` — initiator signs open material (participants, asset, opening balances, dispute_window).
2. `ChannelOpenAck` — counterparty signs the **same** open material.
3. After both open signatures + PROTO-0 checks → status `Open`, `sequence = 0` commitment to initial state may be pending activation.
4. `ChannelActivate` — both parties dual-sign `StateUpdateV0` / initial `ChannelStateV0` with `sequence = 0` equal to opening balances → status `Active`, `soft_local_agreement = true`.

Failure conditions (reject, remain uncreated or stay `Open` without activation):

- missing / invalid signature
- participant mismatch
- non-conserving opening balances
- failed PROTO-0 identity/capability checks
- frozen/revoked identity
- open material byte mismatch between request and ack

**Reasoning:** Separates “both agreed to parameters” (`Open`) from “both agreed to initial state bytes” (`Active`). Avoids ambiguous auto-activation.

**Security implications:** No balance-moving updates allowed before `Active`. Clear fail-closed points.

**Review trigger:** If tests show activate step is pure overhead; may merge Open+Activate later with an ADR.

---

## P1-DEC-003 — Action identity

**Question:** How are channel actions referenced for replay prevention and audit?

**Chosen approach (Locked):**

v0 uses **composition of identifiers**, not a separate free-form action UUID registry:

| Reference | Role |
|-----------|------|
| `channel_id` | Stable channel identity (hash of open material) |
| `sequence` | Monotonic per-channel state counter; primary replay key |
| `state_commitment` | `SHA-256(canonical_cbor(ChannelStateV0))` |
| `previous_state_commitment` | Links update to prior accepted state |
| `ReceiptV0` | Audit record of accepted/rejected transition |

Rules:

- Accept update iff `sequence == current_sequence + 1` (except activation at `sequence = 0` once).
- Reject `sequence <= current_sequence` (replay / stale).
- Reject if `previous_state_commitment` ≠ current commitment.
- Capability action tokens (`channel.open`, etc.) authorise the **class** of operation; they are not per-update IDs.

**Reasoning:** Sequence + commitments give deterministic replay protection and auditability without extra ID namespaces.

**Security implications:** Strong local replay resistance; does not solve distributed clock issues (still PROTO-0 logical `now`).

**Review trigger:** Multi-hop or multi-state-fork protocols requiring richer action IDs.

---

## P1-DEC-004 — Dispute evidence (minimal)

**Question:** What evidence exists for simulated disputes?

**Chosen approach (Provisional):**

Admissible evidence for PROTO-1 simulated dispute:

```text
DisputeEvidenceV0 {
  channel_id,
  claimed_state: ChannelStateV0,
  dual_signed_update_bodies: canonical StateUpdateV0 bytes,
  signature_A,
  signature_B,
  logical_time_raised
}
```

Verifier may inspect:

- both signatures over identical canonical update bytes
- sequence vs currently applied sequence
- conservation / participant / asset rules
- PROTO-0 identity status at raise time (fail closed if frozen/revoked where required)

Intentionally deferred:

- watchtowers
- network delivery proofs
- slash execution
- backend settlement proofs
- subjective “who was online” evidence
- multi-state DAG conflict beyond highest valid dual-signed sequence

Resolution rule (simulated): select the **highest sequence** among admissible dual-signed states that pass validation; then `Disputed → Finalized`.

**Reasoning:** Enough to test invariant 8 locally; avoids building a real dispute system.

**Security implications:** Local-only; missing signatures or lower sequences lose. No claim of production dispute safety.

**Review trigger:** PROTO-2 escrow disputes; first watchtower design; backend settlement spike.

---

## P1-DEC-005 — H1 baseline

**Question:** What can PROTO-1 claim about “channels beat direct settlement”?

**Hypothesis:** Channels beat direct settlement for high-frequency agent interactions.

**Chosen approach (Provisional):**

PROTO-1 **can** test:

- simulated per-update cost units (e.g. constant `C_channel` vs `C_direct`)
- simulated latency units (e.g. dual-sign local verify time vs modelled direct-settlement delay)
- update throughput under local in-memory conditions
- whether agents can read `FinalityViewV0` and treat soft ≠ hard (H6 companion)

PROTO-1 **cannot** prove:

- real L2 fee markets
- real congestion
- real dispute infrastructure cost
- production safety of channels vs chains

Future measurements required (post-PROTO-1 / PROTO-4):

- real backend fee and finality samples
- dispute/watchtower overhead
- failure rate under partitions

Default simulation parameters (provisional, override in harness config):

| Parameter | Provisional value |
|-----------|-------------------|
| `C_channel_update` | 1 cost unit |
| `C_direct_settlement` | 100 cost units |
| `L_channel_update` | measured local verify time |
| `L_direct_settlement` | 1000 logical time units |

**Reasoning:** Keeps H1 falsifiable inside the simulator without fake “blockchain proven” claims.

**Security implications:** None directly; prevents overclaiming in SECURITY_MODEL / results.

**Review trigger:** First live backend spike (PROTO-4); Phase 1 exit H1 write-up.

---

## P1-DEC-006 — Abort / close / disagreement paths

**Question:** Who closes, when is final, what happens on disagreement?

**Chosen approach (Locked for lifecycle; dispute details Provisional via P1-DEC-004):**

### Status set

`Open | Active | Closing | Disputed | Finalized`

### Who can request close

- Either participant with valid `channel.close` capability and active identity may request cooperative close.
- Cooperative close requires **both** signatures on the close/final state (same dual-sign rule).

### When close becomes final

1. Dual-signed close on latest agreed state → `Closing`
2. Optional explicit finalize step or immediate transition → `Finalized` when both have signed close confirmation on that state
3. After `Finalized`: no further updates

### Abort from `Open` without activation

**Allowed:** dual-signed abort → `Open → Finalized` with opening balances unchanged and no `Active` history.

### Disagreement on updates

- Pending proposal discarded; remain on last accepted state; status unchanged.

### Disagreement on close / conflicting signed states

- Either party may raise `Disputed` with `DisputeEvidenceV0` during `Active` or `Closing` while dispute window logically open.
- After window / immediate sim resolve: highest valid dual-signed sequence wins → `Finalized`.

### Invalid close request

Reject if: wrong participants, missing dual signatures, capability fail, attempting close from `Finalized`, or closing on stale sequence when a higher dual-signed state exists in evidence.

**Reasoning:** Matches design machine; abort without activation avoids stuck Open channels.

**Security implications:** Prevents unilateral finalize; dispute path is simulated only.

**Review trigger:** Real dispute windows and slash (Phase 2).

---

## P1-DEC-007 — Capability action tokens

**Question:** Exact v0 action selectors for channel ops?

**Chosen approach (Locked):**

| Token | Authorises |
|-------|------------|
| `channel.open` | Participate in open request/ack |
| `channel.activate` | Dual-sign initial sequence-0 state |
| `channel.update` | Propose/approve balance state updates |
| `channel.close` | Request/approve cooperative close or abort |
| `channel.dispute` | Raise simulated dispute with admissible evidence |

Spend/limits on capabilities still apply via PROTO-0 constraints when updates move simulated balances (e.g. `max_spend` bounds cumulative outbound if tests define that policy). Minimal PROTO-1 tests may use per-action presence without cumulative spend accounting unless specified in an acceptance case.

**Review trigger:** First real payment semantics tying spend ceilings to channel deltas.

---

## Unresolved risks (tracked, not pretended solved)

1. Logical-time integrity for dispute windows (PROTO-0 carry-forward)
2. Local revocation coherence across verifiers
3. Operational-key rotation mid-channel
4. H1 parameters are synthetic
5. Dispute model is minimal and non-distributed

---

## Implementation boundary

After acceptance-test freeze:

- implement under `Aether/core/` channel module consuming PROTO-0
- no networking, settlement, tokens, reputation, AI
- do not modify PROTO-0 unless contradiction recorded in decisions
