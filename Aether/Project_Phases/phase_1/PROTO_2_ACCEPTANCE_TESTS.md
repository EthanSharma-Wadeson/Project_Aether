# PROTO_2_ACCEPTANCE_TESTS.md — Phase 1

## Status

**Frozen acceptance-test specification — required before PROTO-2 implementation**

Sources of truth:

- [PROTO_2_DESIGN.md](PROTO_2_DESIGN.md)
- [PROTO_2_DECISIONS.md](PROTO_2_DECISIONS.md)
- [PROTO_1_ACCEPTANCE_TESTS.md](PROTO_1_ACCEPTANCE_TESTS.md)
- [PROTO_0_ACCEPTANCE_TESTS.md](PROTO_0_ACCEPTANCE_TESTS.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)

This document does **not** contain implementation.
It does **not** authorise networking, settlement, tokens, or live backends.

---

## 1. Purpose

PROTO-2 acceptance tests freeze what the escrow + receipt simulator must prove:

- authorised agents create, fund, and settle escrows under PROTO-0 authority
- provider-signed settlement receipts bind to escrows and satisfy deterministic terms
- release, refund, timeout, and dispute paths are deterministic and fail closed
- fee quotes, reservations, and budgets enforce H5 constraints
- value conservation and mutual exclusivity of terminal outcomes
- H4/H5 metrics recorded without overclaiming production security

---

## 2. Minimal fixtures / harness assumptions

- two agents `P` (payer), `R` (provider) registered via PROTO-0
- capabilities for `escrow.*` action tokens (P2-DEC-010)
- logical time `now` injected by harness
- in-memory `EscrowStore` only (P2-DEC-001)
- separate from PROTO-1 `ChannelStore` (P2-DEC-002)
- simulated payer balance ledger for funding/fee debits
- dual signatures on `EscrowTermsV0` at create (P2-DEC-003)
- provider-only settlement receipt signatures (P2-DEC-004)

Test IDs:

| Prefix | Category |
|--------|----------|
| `P2-T###` | Lifecycle / normal behaviour |
| `P2-R###` | Receipt validation |
| `P2-E###` | Escrow economic rules |
| `P2-A###` | Adversarial / abuse |
| `P2-I###` | PROTO-0 / PROTO-1 integration |
| `P2-H###` | H4 / H5 measurement |
| `P2-M###` | Determinism / measurement hooks |

---

## 3. Lifecycle tests (`P2-T###`)

### PASS

| ID | Case | Expected |
|----|------|----------|
| P2-T001 | Dual-signed terms create escrow | Status `Proposed`; `escrow_id` deterministic |
| P2-T002 | Payer funds with valid `escrow.fund` | Status `Funded`; principal locked; fee reserved |
| P2-T003 | Provider submits valid receipt | Status `ReceiptAccepted`; receipt bound |
| P2-T004 | Release after dispute window | Status `Released`; provider credited; fee consumed |
| P2-T005 | Cooperative refund before receipt | Status `Refunded`; payer credited |
| P2-T006 | Timeout without receipt | Status `Expired` → `Refunded` |
| P2-T007 | Cancel unfunded escrow (dual-signed) | Status `Cancelled` |
| P2-T008 | Dispute → resolve with valid receipt | Status `ResolvedReleased` |
| P2-T009 | Dispute → resolve without valid receipt | Status `ResolvedRefunded` |
| P2-T010 | `EconomicFinalityViewV0.finalized=true` on terminal | `hard_settlement_placeholder=false` |

### FAIL

| ID | Case | Expected |
|----|------|----------|
| P2-T011 | Create with only one party signature | Reject; no escrow |
| P2-T012 | Fund before create / unknown escrow | Reject |
| P2-T013 | Submit receipt before fund | Reject |
| P2-T014 | Release before fund | Reject |
| P2-T015 | Release after `Refunded` | Reject |
| P2-T016 | Refund after `Released` | Reject |
| P2-T017 | Any transition from terminal status | Reject |
| P2-T018 | Fund after `fund_before` deadline | Reject |
| P2-T019 | Receipt after `receipt_before` deadline | Reject |
| P2-T020 | Release before dispute window elapsed (when non-zero) | Reject |

---

## 4. Receipt validation tests (`P2-R###`)

| ID | Case | Expected |
|----|------|----------|
| P2-R001 | Valid provider-signed receipt accepted | Bound to escrow |
| P2-R002 | Forged / wrong-key signature | Reject |
| P2-R003 | Receipt for wrong `escrow_id` | Reject |
| P2-R004 | Receipt with wrong `terms_version` | Reject |
| P2-R005 | Receipt with wrong `claim_type` | Reject |
| P2-R006 | Receipt with wrong `result_code` | Reject |
| P2-R007 | Receipt with wrong `output_commitment` when required | Reject |
| P2-R008 | Receipt replay (duplicate `receipt_nonce`) | Reject |
| P2-R009 | Receipt from non-provider signer | Reject |
| P2-R010 | Non-canonical receipt body | Reject |
| P2-R011 | Tampered body after signing | Reject |
| P2-R012 | Receipt `claimed_amount` > principal | Reject |
| P2-R013 | Payer-signed receipt (wrong signer model) | Reject |
| P2-R014 | Receipt without `escrow.submit_receipt` capability | Reject |

---

## 5. Escrow economic rules (`P2-E###`)

| ID | Case | Expected |
|----|------|----------|
| P2-E001 | Double fund same escrow | Reject second fund |
| P2-E002 | Fund amount ≠ `principal_amount` | Reject |
| P2-E003 | Fund beyond payer simulated balance | Reject |
| P2-E004 | Fee reservation > `max_protocol_fee` | Reject |
| P2-E005 | No value creation on release | Conservation holds |
| P2-E006 | No value creation on refund | Conservation holds |
| P2-E007 | Double release | Reject / impossible |
| P2-E008 | Double refund | Reject / impossible |
| P2-E009 | Fee consumed ≤ fee reserved | Ledger consistent |
| P2-E010 | Insufficient fee budget blocks release | Reject |
| P2-E011 | Terminal outcomes mutually exclusive | No Released+Refunded |
| P2-E012 | Provider cannot self-refund principal | Reject |

---

## 6. Adversarial tests (`P2-A###`)

| ID | Case | Expected |
|----|------|----------|
| P2-A01 | Unauthorised escrow create (no cap) | Reject |
| P2-A02 | Unauthorised fund | Reject |
| P2-A03 | Unauthorised release | Reject |
| P2-A04 | Unauthorised refund | Reject |
| P2-A05 | Unauthorised dispute | Reject |
| P2-A06 | Unauthorised resolve | Reject |
| P2-A07 | Third-party receipt submission | Reject |
| P2-A08 | Conflicting receipts (higher nonce wins; lower rejected) | Deterministic |
| P2-A09 | Dispute from invalid status (`Cancelled`) | Reject |
| P2-A10 | Invalid dispute evidence | Reject; no status change |
| P2-A11 | Resolver without `escrow.resolve` | Reject |
| P2-A12 | Identity-only release attempt | Reject |
| P2-A13 | Valid receipt sig without capability | Reject |
| P2-A14 | Escrow ID guessing without authority | Reject |
| P2-A15 | Release with superseded receipt nonce | Reject |
| P2-A16 | Fee limit exceeded via manipulated quote | Reject |

---

## 7. PROTO-0 / PROTO-1 integration (`P2-I###`)

| ID | Case | Expected |
|----|------|----------|
| P2-I01 | Unregistered payer cannot create | Reject |
| P2-I02 | Missing `escrow.create` capability | Reject |
| P2-I03 | Missing `escrow.fund` on fund | Reject |
| P2-I04 | Frozen identity cannot fund | Reject |
| P2-I05 | Revoked identity cannot submit receipt | Reject |
| P2-I06 | Expired capability cannot release | Reject |
| P2-I07 | Revoked capability after escrow create blocks fund | Reject |
| P2-I08 | Stale permission root blocks new escrow action | Reject |
| P2-I09 | Identity-only bypass on economic op | Reject |
| P2-I10 | PROTO-1 channel op still independent (no regression) | PROTO-1 tests remain green |
| P2-I11 | PROTO-1 `ReceiptV0` not accepted as settlement receipt | Reject |
| P2-I12 | Escrow module does not mutate `ChannelStore` | Isolation |

Pipeline order for every accepted escrow action:

```text
1. Verify SignedMessage envelope(s) where applicable
2. PROTO-0 identity status
3. PROTO-0 capability / root checks
4. Escrow transition rules (status, timeout, conservation, receipt)
5. Apply state / emit audit record
```

---

## 8. H4 / H5 measurement (`P2-H###`, `P2-M###`)

Not pass/fail security proofs; required evidence recordings:

| ID | Case | Expected artifact |
|----|------|-------------------|
| P2-H001 | N happy-path escrows fund→receipt→release | H4 completion rate in results |
| P2-H002 | N timeout escrows without receipt | H4 safe-refund rate |
| P2-H003 | Adversarial wrongful-release attempts | Count = 0 in results |
| P2-H004 | Bounded task flows complete without non-deterministic proofs | H4 qualitative note |
| P2-H005 | Agent loop with fee quote + budget | H5 completion without manual intervention |
| P2-H006 | Operations rejected for insufficient budget | H5 rejection count |
| P2-M001 | Same inputs → same terminal status (determinism) | Reproducible hashes |
| P2-M002 | Fee ledger matches consumed totals | Ledger audit |

Overclaim ban: results must state PROTO-2 does **not** prove real-market escrow safety or autonomous fee planning in production.

---

## 9. Invariant coverage matrix

| Phase 0 invariant | PROTO-2 tests |
|-------------------|---------------|
| 1 Envelope limits | P2-I02–I09, P2-A01–A06 |
| 2 / 12 Delegation | Inherited PROTO-0; P2-I08 |
| 3 Expiry/revoke | P2-I04–I07 |
| 4 No settlement from identity alone | P2-I09, P2-A12 |
| 5 Signature ≠ truth/finality | P2-R001–R013, P2-H003 |
| 6 Identity continuity | Terms bind AgentIds; P2-R009 |
| 7 Deterministic transitions | P2-T###, P2-M001 |
| 8 Dispute reproducibility | P2-T008–T009, P2-A10–A11 |
| 9 Commitment ≠ disclosure | P2-R007; no raw payload required |
| 10 Backend isolation | P2-I12 |
| 11 Capability before economic auth | Pipeline §7; P2-A12–A13 |
| 12 Child delegation bounds | Inherited PROTO-0 |

---

## 10. PROTO-2 exit criteria

PROTO-2 implementation is complete only when:

1. All `P2-T###` PASS and FAIL cases behave as specified  
2. All `P2-R###`, `P2-E###`, `P2-A###` cases pass or reject correctly  
3. All `P2-I###` integration cases pass  
4. H4/H5 measurement tests (`P2-H###`, `P2-M###`) recorded without overclaim  
5. `cargo test` green; `cargo clippy --all-targets -- -D warnings` clean  
6. Every economically meaningful operation is capability-gated  
7. Escrow transitions are deterministic; value conserved  
8. Double release/refund impossible; receipt replay rejected  
9. Terminal outcomes mutually exclusive  
10. `PROTO_2_RESULTS.md` written  
11. `SECURITY_MODEL.md` reviewed after implementation  
12. No Phase 0 invariant violation; PROTO-1 tests remain green  
13. No live network, token, chain, or real-money code introduced  

A passing suite is not global protocol security proof.

---

## 11. Freeze statement

> PROTO-2 acceptance tests are **frozen** for implementation review.

Changes require an explicit edit to this file and a note in `PROTO_2_DECISIONS.md`. Implementation must target this specification; the specification must not be silently weakened to make tests pass.

**Next step:** Design review and explicit approval to implement PROTO-2.
