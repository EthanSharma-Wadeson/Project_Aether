# PROTO_1_ACCEPTANCE_TESTS.md — Phase 1

## Status

**Frozen acceptance-test specification — required before PROTO-1 implementation**

Sources of truth:

- [PROTO_1_DESIGN.md](PROTO_1_DESIGN.md)
- [PROTO_1_DECISIONS.md](PROTO_1_DECISIONS.md)
- [PROTO_0_ACCEPTANCE_TESTS.md](PROTO_0_ACCEPTANCE_TESTS.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)

This document does **not** contain implementation.
It does **not** authorise networking, settlement, tokens, or PROTO-2 escrow logic.

---

## 1. Purpose

PROTO-1 acceptance tests freeze what the bilateral channel simulator must prove:

- authenticated participants open and activate a channel under PROTO-0 authority
- dual-signed state updates advance monotonically and conserve balances
- replays, forgeries, stale sequences, and unauthorised transitions fail closed
- close, abort, and simulated dispute paths behave as decided
- soft local agreement is distinguishable from hard settlement placeholder (H6)
- H1 metrics are recorded against the provisional simulated baseline (not overclaimed)

---

## 2. Minimal fixtures / harness assumptions

- two agents `A`, `B` registered via PROTO-0
- capabilities issued for required action tokens (`channel.open`, `channel.activate`, `channel.update`, `channel.close`, `channel.dispute`)
- logical time `now` injected by harness
- in-memory channel store only
- dual signatures required for accepted updates (P1-DEC-001)

Test IDs: `P1-T###` (lifecycle / functional), `P1-A##` (adversarial), `P1-I##` (PROTO-0 integration).

---

## 3. Channel lifecycle tests

### PASS

| ID | Case | Expected |
|----|------|----------|
| P1-T001 | Create/open material dual-signed by A and B with valid caps | Channel enters `Open` |
| P1-T002 | Activate with dual-signed initial `ChannelStateV0` sequence=0 matching opening balances | Status `Active`; commitment set; `soft_local_agreement=true` |
| P1-T003 | Dual-signed update sequence=1 conserving total deposit | Accepted; `sequence=1`; balances updated |
| P1-T004 | Multiple sequential updates 1..N | Each accepted iff seq = previous+1 |
| P1-T005 | Cooperative close from `Active` with dual-signed close on latest state | `Closing` then `Finalized`; `finalized=true` |
| P1-T006 | Abort from `Open` without activation (dual-signed abort) | `Open → Finalized`; no Active history |
| P1-T007 | Inspect finality after Active dual-signed state | `soft_local_agreement=true`, `hard_settlement_placeholder=false` |
| P1-T008 | Emit `ReceiptV0` on accepted update | Receipt references channel_id, sequence, commitments, result=accepted |

### FAIL

| ID | Case | Expected |
|----|------|----------|
| P1-T010 | Open with only one signature | Reject; no channel |
| P1-T011 | Activate with mismatched opening balances | Reject; remain `Open` |
| P1-T012 | Update while status=`Open` (not Active) | Reject |
| P1-T013 | Close from `Finalized` | Reject |
| P1-T014 | Update after `Finalized` | Reject |

---

## 4. Security / adversarial tests

| ID | Case | Expected |
|----|------|----------|
| P1-A01 | Forged state update (garbage / wrong-key signature) | Reject |
| P1-A02 | Invalid signature on one of two required signers | Reject |
| P1-A03 | Replayed older update after higher sequence accepted | Reject |
| P1-A04 | Stale sequence (`sequence <= current`) | Reject |
| P1-A05 | Skipped sequence (`current+2`) | Reject |
| P1-A06 | Participant mismatch (third agent or swapped unauthorized party) | Reject |
| P1-A07 | Unauthorized transition (e.g. `Disputed → Active`) | Reject |
| P1-A08 | Invalid close request (single signature, wrong state, stale seq) | Reject |
| P1-A09 | Balance inflation / non-conservation | Reject |
| P1-A10 | Negative balance attempt | Reject |
| P1-A11 | Asset mutation mid-channel | Reject |
| P1-A12 | `previous_state_commitment` mismatch | Reject |
| P1-A13 | Dual signatures over **different** body bytes presented as one update | Reject |
| P1-A14 | Treat soft agreement as hard settlement in finality assertion helper | Test asserts fields remain distinct (`hard_settlement_placeholder=false`) |
| P1-A15 | Dispute with higher valid dual-signed state during Closing | Enter `Disputed` then finalize to highest valid sequence |
| P1-A16 | Dispute evidence with only one signature | Reject raise / no status change |

---

## 5. PROTO-0 integration tests

| ID | Case | Expected |
|----|------|----------|
| P1-I01 | Open requires identity verification for both agents | Unregistered agent → reject |
| P1-I02 | Capability check before `channel.open` | Missing `channel.open` cap → reject |
| P1-I03 | Capability check before `channel.update` | Missing `channel.update` → reject |
| P1-I04 | Revoked identity cannot open or update | Reject |
| P1-I05 | Frozen identity cannot open or update | Reject |
| P1-I06 | Expired capability cannot authorise update | Reject |
| P1-I07 | Revoked capability cannot authorise update | Reject |
| P1-I08 | Identity-only attempt (no capability) on update | Reject (invariant 4/11) |
| P1-I09 | Valid channel signatures without capability still rejected | Reject |
| P1-I10 | Stale permission root under PROTO-0 rules blocks new channel actions | Reject |

Pipeline order for every accepted channel action:

```text
1. Verify SignedMessage envelope(s)
2. PROTO-0 identity status
3. PROTO-0 capability / root checks
4. Channel transition rules (sequence, conservation, status)
5. Apply state / emit receipt
```

---

## 6. Dispute path tests (minimal)

| ID | Case | Expected |
|----|------|----------|
| P1-T020 | Raise dispute with admissible `DisputeEvidenceV0` for higher dual-signed seq | Status `Disputed` |
| P1-T021 | Resolve dispute to highest valid sequence | Status `Finalized` on winning commitment |
| P1-T022 | Lower-sequence evidence cannot override higher accepted state | Reject or lose resolution |

---

## 7. H1 / H6 measurement hooks

Not pass/fail security tests; required evidence recordings:

| ID | Case | Expected artifact |
|----|------|-------------------|
| P1-M01 | Run N channel updates; record total simulated cost vs N×`C_direct_settlement` | Metrics in results log |
| P1-M02 | Record local update latency vs provisional `L_direct_settlement` | Metrics in results log |
| P1-M03 | Assert agent-visible finality API returns distinct soft vs hard fields | Boolean checks |

Overclaim ban: results must state PROTO-1 does **not** prove real-chain fee superiority.

---

## 8. Invariant coverage matrix

| Phase 0 invariant | PROTO-1 tests |
|-------------------|---------------|
| 1 Envelope limits | P1-I02, P1-I03, P1-I06–I09 |
| 2 / 12 Delegation | Inherited PROTO-0; no channel widening path |
| 3 Expiry/revoke | P1-I04–I07 |
| 4 No settlement from identity alone | P1-I08 |
| 5 Signature ≠ truth/finality | P1-A14, P1-M03 |
| 6 Identity continuity | Channel binds AgentIds; rotation mid-channel deferred |
| 7 Deterministic transitions | All lifecycle tests must be deterministic |
| 8 Dispute reproducibility | P1-T020–T022, P1-A15 |
| 9 Commitment ≠ disclosure | Receipts/commitments only in fixtures |
| 10 Backend isolation | No backend in harness |
| 11 Capability before economic auth | Pipeline §5; P1-I09 |

---

## 9. Measurable exit criteria for PROTO-1 implementation

PROTO-1 implementation is complete only when:

1. All `P1-T###` PASS cases accept and FAIL cases reject  
2. All `P1-A##` adversarial cases reject (or finalize correctly for A15)  
3. All `P1-I##` PROTO-0 integration cases pass  
4. Dispute minimal path tests pass  
5. P1-M01–M03 metrics recorded without overclaim  
6. No Phase 0 invariant violation remains in-scope  
7. Results written to `PROTO_1_RESULTS.md`  
8. Security findings reviewed against `SECURITY_MODEL.md` before PROTO-2  

A passing suite is not global protocol security proof.

---

## 10. Freeze statement

> PROTO-1 acceptance tests are **frozen** for implementation.

Changes require an explicit edit to this file and a note in `PROTO_1_DECISIONS.md` / Phase 1 evidence. Implementation must target this specification; the specification must not be silently weakened to make tests pass.

**Next step:** PROTO-1 Rust implementation against these IDs.
