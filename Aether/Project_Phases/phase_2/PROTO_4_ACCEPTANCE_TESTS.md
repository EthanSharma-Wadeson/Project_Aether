# PROTO_4_ACCEPTANCE_TESTS.md — Phase 2

## Status

**Frozen acceptance-test specification — required before PROTO-4 implementation**

Sources of truth:

- [PROTO_4_DESIGN.md](PROTO_4_DESIGN.md)
- [PROTO_4_DECISIONS.md](PROTO_4_DECISIONS.md)
- [PHASE_2_WEDGE_DECISION.md](PHASE_2_WEDGE_DECISION.md)
- [PROTO_2_DESIGN.md](../phase_1/PROTO_2_DESIGN.md)
- [PROTO_2_RESULTS.md](../phase_1/PROTO_2_RESULTS.md)
- [PROTO_NET_0_SECURITY_REVIEW.md](PROTO_NET_0_SECURITY_REVIEW.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
- [CONSENSUS_AND_SETTLEMENT.md](../../Aether_docs/CONSENSUS_AND_SETTLEMENT.md) §2, §6–7, §12

This document does **not** contain implementation.  
It does **not** authorise live banking APIs, tokens, public chains, reputation, or marketplace work.

First spike adapter under test: deterministic `enterprise.ledger.v0` stub (P4-DEC-003) behind `SettlementAdapterV0`.

---

## 1. Purpose

PROTO-4 acceptance tests freeze what the settlement-binding prototype must prove for **Enterprise Agent Spend Control**:

- authorised agents bind `AgentId` → external accounts under PROTO-0
- authorised agents request settlement for PROTO-2 terminal outcomes
- adapter responses advance settlement status without rewriting escrow truth
- `hard_settlement_placeholder` becomes true **only** after verified external confirmation
- settlement failure / conflict fails closed and leaves PROTO-2 state coherent
- transport (PROTO-NET-0) is never an authority source

---

## 2. Contradiction / consistency review

Reviewed before freeze. Findings:

| Item | Assessment |
|------|------------|
| PROTO-2 always `hard_settlement_placeholder=false` | **Not a contradiction** — PROTO-2 scope ends at simulator finality; PROTO-4 is the only path that may set hard settlement (P4-DEC-004) |
| CONSENSUS `SettlementBinding` (account) vs PROTO-4 `SettlementBindingV0` (escrow settle) | **Resolved in design** via split: `SettlementAccountBindingV0` + `SettlementBindingV0` (P4-DEC-005). Tests use both names explicitly |
| Design “terminal or settleable” escrow | **Narrowed for freeze:** settle request requires PROTO-2 **terminal** status with matching outcome (`Released`/`ResolvedReleased` → release; `Refunded`/`ResolvedRefunded` → refund). Pre-terminal settle is out of v0 acceptance |
| Design status “Disputed” vs `DisputedExternal` | **Use `DisputedExternal`** as in design transition table |
| DEC-P2-003 “two backends” vs P4-DEC-003 one stub | **Not contradictory** — architecture stays agnostic; acceptance suite requires **one** stub; second adapter is optional stretch, not freeze-blocking |
| SECURITY_MODEL signature ≠ truth | **Preserved** — adapter confirmation proves provider-reported status under trusted-stub assumption, not legal finality |

No silent architectural overrides. If implementation needs pre-terminal settle, revise this file and P4-DEC explicitly.

---

## 3. Security boundaries (normative for tests)

### Aether owns

- `AgentId`, identity status, capabilities  
- Escrow state and conservation (PROTO-2)  
- Settlement **intent** and `SettlementBindingV0` / account bindings  
- Evidence commitment validation and finality **views**  

### External provider owns

- Account balances / holds  
- Money movement / ledger posts  
- External transaction status  

### Forbidden

```text
external provider state  →  overwrite Aether escrow truth
adapter success alone    →  hard_settlement_placeholder = true  (without binding verify)
settlement binding       →  mutate ChannelStore / invent escrow funds
```

---

## 4. Minimal fixtures / harness assumptions

- Agents `P` (payer), `R` (provider), optional `T` (treasury/bind issuer) via PROTO-0  
- Capabilities: `escrow.*` (for setup) + `settlement.bind` / `settlement.settle` / `settlement.query` / `settlement.cancel` (P4-DEC-009)  
- Logical `now` injected  
- In-memory `EscrowStore` + `SettlementStore` (no public `get_mut`)  
- `enterprise.ledger.v0` stub adapter (controllable success/fail/conflict for tests)  
- PROTO-2 happy-path or refund-path escrow brought to **terminal** before settle  
- Optional NET path: envelopes may carry settle messages but must not authorise  

Test IDs:

| Prefix | Category |
|--------|----------|
| `P4-T###` | Lifecycle |
| `P4-R###` | Binding integrity |
| `P4-A###` | Adversarial |
| `P4-I###` | PROTO-0 / PROTO-2 / NET integration |
| `P4-H###` | Hypothesis / measurement |
| `P4-M###` | Determinism |

---

## 5. Lifecycle tests (`P4-T###`)

### PASS

| ID | Case | Expected |
|----|------|----------|
| P4-T001 | Create `SettlementAccountBindingV0` for P with `settlement.bind` | Binding stored; `binding_id` deterministic from canonical body |
| P4-T002 | Create account binding for R (release path) | Provider account bound to same `settlement_provider` |
| P4-T003 | Request settle for terminal `Released` escrow | `SettlementBindingV0` status `Requested`; escrow unchanged; `hard_settlement_placeholder=false` |
| P4-T004 | Adapter submit success | Status `Submitted`; `external_settlement_ref` set |
| P4-T005 | Adapter accepts / holds | Status `Accepted` |
| P4-T006 | Adapter confirms full amount | Status `Confirmed`; evidence commitments present |
| P4-T007 | Finalize after Confirmed + verify | Status `Finalized`; `hard_settlement_placeholder=true`; PROTO-2 escrow status still terminal released |
| P4-T008 | Refund-path settle for terminal `Refunded` | Binding outcome `refund_to_payer`; lifecycle to Finalized with hard flag |
| P4-T009 | Idempotent retry same `correlation_id` | Returns existing binding; no second adapter debit |
| P4-T010 | Query refreshes status from adapter | `settlement.query` updates view without mutating escrow |

### FAIL

| ID | Case | Expected |
|----|------|----------|
| P4-T011 | Settle before escrow terminal | Reject; no binding |
| P4-T012 | Settle unknown `escrow_id` | Reject |
| P4-T013 | Settle with outcome mismatch (refund binding on Released) | Reject |
| P4-T014 | Promote hard settlement at `Requested`/`Submitted`/`Accepted` | Reject / flag remains false |
| P4-T015 | Cancel after `Confirmed` | Reject |
| P4-T016 | Second distinct `correlation_id` after Finalized for same escrow+outcome | Reject |
| P4-T017 | Account binding with empty `external_account_ref` | Reject |
| P4-T018 | Settle without payer (or required provider) account binding | Reject |

---

## 6. Binding integrity tests (`P4-R###`)

| ID | Case | Expected |
|----|------|----------|
| P4-R001 | Binding `escrow_id` matches live escrow | Accept |
| P4-R002 | Binding with wrong `escrow_id` | Reject |
| P4-R003 | Binding with wrong `terms_version` | Reject |
| P4-R004 | Binding amounts ≠ escrow principal/fee snapshot | Reject |
| P4-R005 | `correlation_id` deterministic for same intent inputs | Equal ids |
| P4-R006 | Different intents → different `correlation_id` | Distinct |
| P4-R007 | `external_settlement_ref` unique per successful submit (stub) | Duplicate ref rejected on second binding |
| P4-R008 | Swap payer/provider account binding ids | Reject |
| P4-R009 | Wrong `settlement_provider` vs account binding provider | Reject |
| P4-R010 | Tampered / wrong `evidence_commitment` | Reject promotion to Confirmed/Finalized |
| P4-R011 | Non-canonical binding body | Reject |
| P4-R012 | `aether_escrow_status` snapshot mismatch with store | Reject |

---

## 7. Adversarial tests (`P4-A###`)

| ID | Case | Expected |
|----|------|----------|
| P4-A01 | Fake adapter confirmation (status Confirmed without stub proof fields) | Reject finalize; hard flag false |
| P4-A02 | Forged settlement report / wrong signature on settle intent | Reject |
| P4-A03 | Wrong external account ref (not in account binding) | Reject |
| P4-A04 | Duplicate settlement request (new correlation, same escrow+outcome, in-flight or final) | Reject |
| P4-A05 | Replayed adapter response for different binding | Reject |
| P4-A06 | Settle after capability revoke | Reject (`CapabilityDenied`) |
| P4-A07 | Settle after identity freeze | Reject (`IdentityNotActive`) |
| P4-A08 | Settle exceeding authorised `max_spend` / grant constraints | Reject |
| P4-A09 | Conflicting provider state (Confirmed then stub reports reversed) | → `DisputedExternal`; hard flag cleared; escrow terminal unchanged |
| P4-A10 | Corrupted `external_settlement_ref` after submit | Query/finalize fail closed |
| P4-A11 | Adapter reports success for non-terminal escrow | Reject; escrow untouched |
| P4-A12 | Adapter partial amount ≠ binding | → `Failed`; hard flag false |
| P4-A13 | Identity-only settle (no grant) | Reject |
| P4-A14 | Adapter timeout / error on submit | → `Failed` or remain `Requested` per policy; no hard flag; escrow unchanged |

---

## 8. Integration tests (`P4-I###`)

### PROTO-0

| ID | Case | Expected |
|----|------|----------|
| P4-I01 | `settlement.bind` requires capability | Missing cap → reject |
| P4-I02 | `settlement.settle` requires capability | Missing cap → reject |
| P4-I03 | Capability checked **before** adapter side effect | Stub not invoked on auth failure |
| P4-I04 | Expired capability cannot settle | Reject |
| P4-I05 | Stale permission root cannot settle | Reject |

### PROTO-2

| ID | Case | Expected |
|----|------|----------|
| P4-I06 | Escrow terminal status remains authoritative after Failed settle | Escrow status unchanged |
| P4-I07 | Successful settle does not rewrite escrow balances/conservation | Conservation still holds; no double credit in PROTO-2 ledger |
| P4-I08 | Failed settle does not release/refund escrow a second time | No PROTO-2 transition |
| P4-I09 | Soft `finalized=true` can coexist with `hard_settlement_placeholder=false` pre-confirm | Distinct fields |
| P4-I10 | ChannelStore untouched by all settle ops | Isolation |

### PROTO-NET-0

| ID | Case | Expected |
|----|------|----------|
| P4-I11 | Optional envelope delivery of settle intent still requires PROTO-0 settle auth | Envelope alone insufficient |
| P4-I12 | Session establishment does not grant settlement authority | No settle without grant |

Pipeline order for every accepted settle action:

```text
1. Verify SignedMessage (if applicable)
2. PROTO-0 identity status
3. PROTO-0 capability / root checks
4. PROTO-2 escrow consistency
5. Settlement binding rules + idempotency
6. Adapter call (if authorised)
7. Mutate SettlementStore / finality view
```

---

## 9. Hypothesis tests (`P4-H###`, `P4-M###`)

Not global security proofs; required evidence recordings.

| ID | Hypothesis | Success criterion |
|----|------------|-------------------|
| P4-H001 | Enterprise: Aether can bind autonomous decisions to external settlement evidence | N terminal escrows complete Requested→Finalized with hard flag true; audit export contains binding + external_ref |
| P4-H002 | Enterprise: revoke stops further settlement | After revoke, settle rejected; prior Finalized unchanged |
| P4-H003 | Security: settlement failure does not corrupt protocol state | Adapter fail/conflict leaves escrow terminal + conservation intact; hard flag false |
| P4-H004 | Security: false confirmation cannot set hard settlement without evidence | P4-A01 style attempts count = 0 successes |
| P4-M001 | Same inputs → same `binding_id` / `correlation_id` | Deterministic hashes |
| P4-M002 | Soft vs hard finality fields remain distinguishable in all states | Assertions on both fields |

Overclaim ban: results must state PROTO-4 does **not** prove production banking security, legal finality, or adapter honesty beyond the stub trust model.

---

## 10. Invariant coverage matrix

| Concern | PROTO-4 tests |
|---------|---------------|
| Capability before economic side effect | P4-I01–I05, P4-A06–A08, P4-A13 |
| Escrow truth isolation | P4-I06–I10, P4-A09, P4-A11, P4-H003 |
| Soft ≠ hard finality | P4-T003, T007, T014, P4-I09, P4-M002 |
| Duplicate / replay settle | P4-T009, T016, P4-A04–A05, P4-R007 |
| Binding integrity | P4-R001–R012 |
| NET not authority | P4-I11–I12 |
| Signature ≠ correctness | P4-A01–A02; adapter confirm ≠ legal truth |

---

## 11. PROTO-4 exit criteria (implementation later)

PROTO-4 implementation is complete only when:

1. All `P4-T###` PASS/FAIL behaviours match this file  
2. All `P4-R###`, `P4-A###`, `P4-I###` pass or reject correctly  
3. `P4-H###` / `P4-M###` recorded without overclaim  
4. `cargo test` green; clippy `-D warnings`; prior PROTO-0/1/2/NET suites green  
5. Every settle/bind path capability-gated before adapter effects  
6. `hard_settlement_placeholder` never true without Confirmed+verify  
7. Escrow conservation and status never overwritten by adapter lies  
8. `PROTO_4_RESULTS.md` written  
9. `SECURITY_MODEL.md` reviewed post-implementation  
10. No live bank/chain/token/marketplace code in the spike  

A passing suite is not production payment security proof.

---

## 12. Freeze statement

> PROTO-4 acceptance tests are **frozen** for implementation review.

Changes require an explicit edit to this file and a note in `PROTO_4_DECISIONS.md`. Implementation must target this specification; the specification must not be silently weakened to make tests pass.

**Next step:** Explicit approval to implement PROTO-4. Do not write Rust until approved.
