# PROTO-4 Security Review — Post-Remediation

## Status

**Phase 2 — post-remediation evidence review complete (read-only; no implementation changes)**

Date: 2026-07-28

This review supersedes the pre-remediation verdict in the prior edition of this file for gate purposes. The historical findings P4-SEC-001 / P4-SEC-002 remain documented below as closed.

---

## Sources Reviewed

- `Aether_docs/SECURITY_MODEL.md`
- `Project_Phases/phase_2/PROTO_4_DESIGN.md`
- `Project_Phases/phase_2/PROTO_4_DECISIONS.md`
- `Project_Phases/phase_2/PROTO_4_ACCEPTANCE_TESTS.md`
- `Project_Phases/phase_2/PROTO_4_RESULTS.md`
- Prior `PROTO_4_SECURITY_REVIEW.md` (pre-remediation REMEDIATION REQUIRED)
- `Aether/core/src/settlement/` (all modules)
- `Aether/core/src/escrow/store.rs`, `escrow/mod.rs`, `escrow/state.rs`
- Suites: `settlement_tests.rs`, `settlement_adversarial_tests.rs`, `settlement_security_remediation_tests.rs`
- Repository search: `hard_settlement_placeholder`, `promote_verified_hard_settlement`, `set_hard_settlement_flag`, `clear_hard_settlement_flag`

---

## Verdict

**APPROVE WITH DOCUMENTED LIMITATIONS**

Within the declared **local mock / in-process prototype** scope:

- **P4-SEC-001 is closed** — no public API can promote `hard_settlement_placeholder = true`
- **P4-SEC-002 is closed** — `finalize_settlement` requires a Confirmed caller report **and** a fresh adapter `Confirmed` query before promotion; caller status is not overwritten
- Adapter evidence cannot overwrite PROTO-2 escrow economics
- `cargo fmt --check`, **274** tests, and `clippy -D warnings` are clean
- Remediation regressions **P4-SEC-R01 / R02 / R03** pass

Remaining limitations are **out of prototype scope** (production banking, live adapters, concurrency, persistence) and must not be treated as validated.

---

## Executive Summary

PROTO-4 settlement binding preserves the intended authority split after remediation:

```text
PROTO-2 Escrow Truth
        → Settlement Binding
        → External Adapter Evidence
        → Verified Finalization (+ fresh query)
        → hard_settlement_placeholder = true
```

Hard finality promotion is confined to crate-private `promote_verified_hard_settlement`, invoked only from `finalize_settlement` after capability, escrow, destination, caller-report, and live-adapter checks. Dispute/conflict paths may **clear** the flag only.

This does **not** prove production payment security, legal finality, or honesty of a compromised live adapter.

---

## P4-SEC-001 Review

| Check | Result |
|-------|--------|
| Public API can set hard=`true`? | **No** |
| `promote_verified_hard_settlement` visibility | `pub(crate)` in `escrow/store.rs`; **not** re-exported from `escrow/mod.rs` |
| `set_hard_settlement_flag` public export | **Removed** (inner helper is private `set_hard_settlement_flag_inner`) |
| Promotion only via validated finalize? | **Yes** — sole call site is `finalize_settlement` |
| Clear-only on dispute paths? | **Yes** — `clear_hard_settlement_flag` only |
| Test helpers create production bypass? | **No** — integration tests cannot call `pub(crate)` promote; helpers use `finalize_settlement` |
| Indirect store write-back of mutated clones? | **No** — `EscrowStore::insert_record` is `pub(crate)`; `get` is immutable |

**Conclusion:** P4-SEC-001 closed for external/public callers.

**Residual (documented, not a defect in scope):** any code compiled **inside** `aether_core` could theoretically call `promote_verified_hard_settlement`. No such call exists outside `finalize_settlement`. The crate is treated as a single TCB.

---

## P4-SEC-002 Review

`finalize_settlement` (`settlement/transition.rs`) before `promote_verified_hard_settlement`:

| # | Requirement | Evidenced? |
|---|-------------|------------|
| 1 | Valid PROTO-0 authority / identity | Yes — via `require_capability` → `authorise_action` |
| 2 | `settlement.settle` capability | Yes |
| 3 | Valid terminal PROTO-2 escrow | Yes — `validate_settlement_against_escrow` + settleable status |
| 4 | Valid settlement binding | Yes — loaded from store; status Confirmed (or already Finalized) |
| 5 | Caller report status already `Confirmed` | Yes — explicit check; **no status overwrite** |
| 6 | Fresh adapter query | Yes — `adapter.query(&ext)` |
| 7 | Fresh status still `Confirmed` | Yes |
| 8 | Matching settlement identity | Yes — `verify_settlement_report` on caller + live reports |
| 9 | Matching correlation ID | Yes |
| 10 | Matching external reference | Yes — binding, caller report, live result |
| 11 | Matching provider identity | Yes — report vs `adapter.provider_id()` and binding provider |
| 12 | Matching amount | Yes — binding / caller / live |
| 13 | Matching destination binding | Yes — `validate_destination_accounts` (binding ↔ account bindings; adapter has no destination field) |
| 14 | Valid settlement evidence / proof | Yes — mock `proof_token` via `verify_settlement_report` |
| 15 | No conflict/reversal on fresh query | Yes — non-Confirmed live status → reject |

Hard promotion occurs **only after** live report verification (`promote_verified_hard_settlement` is the last mutating step for the hard flag).

**Conclusion:** P4-SEC-002 closed. Regression **P4-SEC-R02** evidences stale Confirmed + reversed adapter → reject, hard remains false, escrow unchanged.

---

## Hard-Finality Write-Path Audit

Every write to escrow `hard_settlement_placeholder`:

| File | Function | Visibility | Preconditions | Promote / Clear | Public reachable? |
|------|----------|------------|---------------|-----------------|-------------------|
| `escrow/store.rs` | `set_hard_settlement_flag_inner` | private | Terminal escrow | Either | No (only via crate helpers) |
| `escrow/store.rs` | `promote_verified_hard_settlement` | `pub(crate)` | Terminal escrow | **Promote** | No (external crates); only called from `finalize_settlement` |
| `escrow/store.rs` | `clear_hard_settlement_flag` | `pub(crate)` | Terminal escrow | **Clear** | No directly; used by dispute/query conflict + `mark_disputed_external` |
| `settlement/transition.rs` | `finalize_settlement` → promote | `pub` | Full finalize pipeline | **Promote** | Yes — **only** validated path |
| `settlement/transition.rs` | `apply_adapter_status` conflict | private | Confirmed/Finalized + failed/cancelled/disputed live | **Clear** | Via `query_settlement` |
| `settlement/transition.rs` | `apply_adapter_status` DisputedExternal | private | Live disputed | **Clear** | Via `query_settlement` |
| `settlement/transition.rs` | `mark_disputed_external` | `pub` | Binding exists | **Clear** | Yes — clear only |
| `escrow/state.rs` | `EconomicFinalityViewV0::for_status` | `pub` | PROTO-2 transitions | Sets **false** on construction | N/A (initial/soft views) |

**CBOR decode** of finality fields can construct an in-memory struct with hard=`true`, but cannot update `EscrowStore` without `pub(crate) insert_record`.

**PROTO-1 channel** also has a `hard_settlement_placeholder` field (separate object); PROTO-4 escrow promotion paths above are the settlement gate under review.

---

## Fresh-Evidence Audit

```text
capability
 → escrow consistency + destination accounts
 → binding status Confirmed
 → caller report.status == Confirmed (unchanged)
 → verify_settlement_report(caller)
 → adapter.query(external_ref)
 → live.status == Confirmed + amount/ref match
 → verify_settlement_report(live)
 → mutate settlement → Finalized
 → promote_verified_hard_settlement
```

Confirmed via `query_settlement` / `submit_settlement` alone does **not** set hard (P4-T014, P4-SEC-R01, P4-I09).

---

## Authority-Boundary Review

| Domain | Owner | Evidenced |
|--------|-------|-----------|
| Escrow status, release/refund, conservation | PROTO-2 | Settlement never calls release/refund/ledger rewrite; P4-I06–I08, P4-A09, P4-SEC-R02/R03 |
| Settlement bindings, adapter evidence, hard view bit | PROTO-4 | Transitions + store |
| Account balances / payment success | External adapter (mock) | Evidence only |

```text
Adapter evidence ≠ protocol authority
hard_settlement_placeholder ≠ legal or production settlement finality
signature ≠ correctness
```

---

## Adversarial Analysis

### Attack A — Direct Hard-Finality Promotion

**Result:** No reachable public bypass. Promote is crate-private; store mutations are transition-gated. **P4-SEC-R01** passes.

### Attack B — Stale Confirmation

**Result:** Finalisation rejected; hard remains false; escrow economics unchanged. **P4-SEC-R02** passes.

### Attack C — Conflicting Fresh Evidence

| Fresh adapter condition | Behaviour |
|-------------------------|-----------|
| Failed / reversed (non-Confirmed) | Reject finalize (`InvalidSettlementEvidence`) — R02 |
| Amount mismatch | `SettlementAmountMismatch` |
| Different external reference | `InvalidSettlementEvidence` |
| Provider / identity mismatch | Fail in `verify_settlement_report` |
| Missing/wrong proof | Fail in `verify_settlement_report` |
| Malformed / query error | Fail closed |

Destination is validated against Aether account bindings, not an adapter destination field (adapter model has none). Fail closed for binding/account mismatch at finalize.

Dedicated unit tests do not enumerate every malformed variant; **code paths** fail closed. Covered by R02 + verify logic + prior A01/A05/A12 suites.

### Attack D — Confirmed Without Finalisation

**Result:** Confirmed does not imply hard=`true`. **P4-SEC-R01**, P4-T014, P4-I09.

### Attack E — Repeated Finalisation

| Behaviour | Assessment |
|-----------|------------|
| Second finalize when already `Finalized` | Early `Ok` return; does **not** re-query; does **not** re-promote |
| Duplicate adapter debit | Submit is idempotent when already submitted; finalize does not call `submit` |
| Escrow economics | Unchanged on finalize (R03) |
| Concurrency / distributed races | **Not implemented / not claimed** |

**Limitation:** After Finalized, if the adapter later reverses, hard stays true until a `query_settlement` conflict path clears it (P4-A09). Idempotent finalize does not itself clear or re-attest. Acceptable for local single-process model; not crash-recovery or concurrent safety.

---

## Test Verification

Commands:

```bash
cd Aether/core && cargo fmt --check && cargo test && cargo clippy --all-targets -- -D warnings
```

| Check | Result |
|-------|--------|
| `cargo fmt --check` | Pass |
| `cargo test` | **274 passed**, **0 failed** |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| PROTO-0 / 1 / 2 / NET-0 / PROTO-4 suites | Green (included in 274) |
| `p4_sec_r01_direct_hard_flag_promotion_prevented` | Pass |
| `p4_sec_r02_stale_confirmation_cannot_finalize` | Pass |
| `p4_sec_r03_fresh_confirmation_finalizes` | Pass |

---

## Validated by PROTO-4

Locally evidenced only:

| Claim | Support |
|-------|---------|
| No public hard-promotion API | Visibility + exports + R01 |
| Hard promotion only via finalize pipeline | Call-graph audit + R03 |
| Stale Confirmed cannot finalize after reverse | R02 |
| Fresh live Confirmed required | Finalize source + R02/R03 |
| Soft ≠ hard | T003/T014/I09/M002/R01 |
| Adapter cannot rewrite PROTO-2 economics | I06–I08, A09, R02/R03 |
| Capability before adapter side effects | I01–I05, I03, A06–A08, A13 |
| Fake confirmation without proof fails | A01, H004 |
| NET not settlement authority | I11, I12 |

---

## Design Assumptions

- Trusted local `MockSettlementAdapterV0` (deterministic proof is not a secrecy boundary)
- Logical time harness-injected
- Single-process in-memory stores
- `aether_core` crate treated as one TCB for `pub(crate)` helpers
- No hostile network in this prototype
- Host does not patch process memory / bypass Rust visibility

---

## Not Yet Validated

- Real banking or payment-rail security
- Legal settlement finality
- Production adapter honesty / Byzantine providers
- Hostile networks
- Distributed revocation
- Real wall-clock integrity
- Crash recovery / persistent settlement state
- Concurrent or distributed settlement processing
- Production custody and account security
- Multi-provider settlement behaviour

Do **not** upgrade these because the mock adapter and local tests pass.

---

## Findings

| ID | Severity | Status | Notes |
|----|----------|--------|-------|
| P4-SEC-001 | Medium (historical) | **Closed** | Public promote removed; crate-private promote only from finalize |
| P4-SEC-002 | Medium (historical) | **Closed** | Fresh adapter Confirmed required; no caller status overwrite |
| — | — | **None open** within local prototype scope | |

---

## Required Remediation

**None** for this review gate within local prototype scope.

Optional future hardening (out of current gate): same-crate lint/restriction so only `settlement::transition` may call `promote_verified_hard_settlement`; re-query on idempotent Finalized if control-plane consumers need continuous attestation without separate query.

---

## Residual Risks

1. Compromised or lying production adapter can still attest false Confirmed under the trust model (explicitly assumed for mock).
2. Same-crate TCB: other `aether_core` modules could call promote if added later without review.
3. Post-Finalized reversal requires `query_settlement` (or equivalent) to clear hard; idempotent finalize does not re-attest.
4. `mark_disputed_external` is public and ungated (clear-only / downgrade).
5. No persistence, crash recovery, or concurrency claims.

---

## Gate Decision

**APPROVE WITH DOCUMENTED LIMITATIONS** for PROTO-4 as the Enterprise Spend Control settlement-binding prototype.

Accepted meaning:

- Remediation of P4-SEC-001 / P4-SEC-002 is verified
- Hard finality may be treated as integrity-protected **within the local mock / capability-gated transition model**
- Hard finality must **not** be marketed as legal or production settlement finality

Next direction (second backend, reputation, marketplace, networked economy, production rails) requires **explicit** product/security acceptance of this verdict — not implied by this document alone.

---

## Stop

After this review: **STOP.**

Do not begin:

- A second settlement backend
- Reputation
- Marketplace functionality
- Production payment integrations
- Open agent economy functionality
- New protocol layers

until this verdict is explicitly accepted and the next work item is authorised.
