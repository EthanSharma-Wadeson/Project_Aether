# PROTO-4 Security Review vs SECURITY_MODEL.md

## Status

**Phase 2 — evidence review complete; findings P4-SEC-001 / P4-SEC-002 remediated in code (awaiting fresh approval)**

Sources reviewed:

- `Aether_docs/SECURITY_MODEL.md`
- `Project_Phases/phase_2/PROTO_4_DESIGN.md`
- `Project_Phases/phase_2/PROTO_4_DECISIONS.md`
- `Project_Phases/phase_2/PROTO_4_ACCEPTANCE_TESTS.md`
- `Project_Phases/phase_2/PROTO_4_RESULTS.md`
- Settlement module under `Aether/core/src/settlement/` (read-only at review time)
- `Aether/core/src/escrow/store.rs` (hard-flag helpers)
- Suites `settlement_tests.rs`, `settlement_adversarial_tests.rs` (P4-T/R/A/I/H/M)

Date: 2026-07-28

Independent code review also recorded findings via [Security Review](c6bea918-bf64-4f86-8542-d3b455e1093b).

**Remediation note (same day):** P4-SEC-001/002 addressed — crate-private `promote_verified_hard_settlement`, fresh adapter query on finalize, regressions P4-SEC-R01–R03. See [PROTO_4_RESULTS.md](PROTO_4_RESULTS.md) § Validated by PROTO-4 Remediation. This document’s original verdict stands as the pre-remediation record; a **fresh** security review is required before APPROVE.
---

## Verdict

**REMEDIATION REQUIRED**

PROTO-4 **correctly preserves** the core authority split on agent-facing transition paths:

- Adapter / provider responses are evidence only
- PROTO-2 escrow status, amounts, and conservation are **not** overwritten by provider state
- Capability checks run before settlement mutations and adapter side effects
- Soft ≠ hard finality fields remain distinct

However, **`hard_settlement_placeholder` integrity is not fully closed** relative to P4-DEC-004 and the frozen acceptance rule that hard finality requires Confirmed + verified binding evidence:

| ID | Severity | Finding |
|----|----------|---------|
| P4-SEC-001 | Medium | Public `set_hard_settlement_flag` can set hard=`true` on any terminal escrow without settlement/capability/adapter evidence |
| P4-SEC-002 | Medium | `finalize_settlement` accepts a caller-supplied (possibly stale) report without a live adapter re-query; also forces report status to `Confirmed` before verify |

No high/critical path was found where a capability-bound agent alone overwrites PROTO-2 economics.

Passing tests are **not** evidence of production banking security, legal settlement, or adversarial live-adapter honesty.

**Do not** begin a second settlement backend, reputation, marketplace, open agent economy, or production integrations until remediation is accepted (or findings explicitly waived with documented residual risk).

---

## 1. Authority Boundary

### Confirmed

| Question | Result | Evidence |
|----------|--------|----------|
| Aether remains authoritative for escrow? | **Yes** | Settlement mutates escrow only via hard-flag helper; no `release`/`refund`/ledger rewrite from adapter paths |
| Adapter responses are evidence only? | **Yes (transitions)** | `submit`/`query` update `SettlementBindingV0` / reports; P4-I06–I08, P4-A09, P4-A11 |
| Provider state cannot overwrite escrow truth? | **Yes** | Escrow status/conservation unchanged under fail/conflict (P4-H003, P4-A09) |
| Hard settlement requires verified evidence on intended path? | **Mostly** | Intended path: binding `Confirmed`/`Finalized` + `verify_settlement_report` + finalize (P4-T007, T014, A01) |

### Bypass / gap paths

```text
P4-SEC-001 (library API):
  set_hard_settlement_flag(&mut EscrowStore, escrow_id, true)
        → hard_settlement_placeholder = true
  (terminal escrow only; no settlement.bind/settle, no adapter proof)

P4-SEC-002 (protocol path):
  Confirmed binding + cached SettlementReportV0
        → adapter reverses (binding still Confirmed until query)
        → finalize_settlement(stale report)
        → hard flag true without live Confirmed attestation
```

No path found: `adapter success alone → rewrite escrow Released/Refunded balances`.

---

## 2. Capability Enforcement

### Operation matrix

| Operation | Signature | Identity | Capability | Escrow/settlement rules | Mutate |
|-----------|-----------|----------|------------|-------------------------|--------|
| `bind_account` | Yes (incoming) | Via `authorise_action` | `settlement.bind` | Canonical body, non-empty ref | Account store |
| `request_settlement` | Self-sign after auth | Via authorise | `settlement.settle` (+ spend) | Terminal + outcome + amounts + accounts | Settlement store |
| `submit_settlement` | N/A (store binding) | Via authorise | `settlement.settle` | Re-validate escrow/accounts; then adapter | Settlement store |
| `query_settlement` | N/A | Via authorise | `settlement.query` | Re-validate escrow; adapter query | Settlement view (+ hard clear on dispute) |
| `finalize_settlement` | N/A | Via authorise | `settlement.settle` | Escrow + binding status + report verify | Settlement + hard flag |
| `cancel_settlement` | N/A | Via authorise | `settlement.cancel` | Pre-Confirmed only | Settlement (+ adapter cancel) |

Pipeline on agent-facing paths matches the frozen order (capability before adapter on submit — P4-I03).

### Residual notes (non-blocking alone)

- `request_settlement` / `submit` / `finalize` / `query` / `cancel` do not require an externally supplied signed settle message; they rely on PROTO-0 grant + actor key where signing occurs. Acceptable under “SignedMessage if applicable.”
- `mark_disputed_external` is public and ungated (clears hard flag). Host-trust / test helper; lower impact than false promotion.

---

## 3. Settlement Evidence (`SettlementReportV0`)

### What it proves (local mock trust model)

- Binding identity: `settlement_id`, `correlation_id`, `external_reference` match stored binding
- Adapter identity string matches binding provider / adapter `provider_id()`
- Amount matches binding principal
- For Confirmed/Finalized: deterministic mock `proof_token` matches `MockSettlementAdapterV0::proof_for`

### What it does **not** prove

- Legal or banking finality
- Solvency, KYC, or account ownership quality
- That the external ledger has not since reversed (without a fresh query)
- Honesty of a compromised production adapter
- Payload/economic “correctness” beyond structured field checks

### Explicit invariants

```text
signature ≠ correctness
adapter confirmation ≠ legal / payment finality
hard_settlement_placeholder ≠ production settlement finality
```

Aligned with SECURITY_MODEL and P4-DEC-002/004/006.

---

## 4. Finality Safety

### Required conditions (design / P4-DEC-004)

Hard finality may become true only after:

1. Valid PROTO-2 terminal escrow (settleable status + outcome)
2. Valid settlement binding
3. Verified adapter confirmation evidence
4. Matching amount
5. Matching destination (at bind/request)
6. Valid capabilities

### Intended path (`finalize_settlement`)

| Check | Present? |
|-------|----------|
| Capability `settlement.settle` | Yes |
| Live escrow consistency | Yes (`validate_settlement_against_escrow`) |
| Binding status Confirmed/Finalized | Yes |
| Amount match in report | Yes (`verify_settlement_report`) |
| Destination re-check at finalize | No (immutable intent; checked at request/submit) |
| Fresh live adapter query | **No — P4-SEC-002** |
| Caller report status respected | **No — overwritten to Confirmed before verify** |

### Skip paths

- **P4-SEC-001:** Direct public hard-flag set skips all settlement checks
- **P4-SEC-002:** Stale Confirmed report can finalize after adapter reversal until next query clears via `DisputedExternal`

Confirmed alone (`query` → Confirmed) does **not** set hard flag (P4-T014) — good.

---

## 5. Failure Analysis

| Case | Result | Test / note |
|------|--------|-------------|
| Forged / fake adapter confirmation (no proof) | Fail closed; hard false | P4-A01, P4-H004 |
| Wrong account / swapped bindings | Reject | P4-A03, P4-R008 |
| Wrong / partial amount | Failed; hard false | P4-R004, P4-A12 |
| Duplicate / in-flight settle | Idempotent or reject; no double debit | P4-T009, P4-A04 |
| Replayed report across bindings | Reject | P4-A05 |
| Revoked capability | Reject | P4-A06, P4-H002 |
| Frozen identity | Reject | P4-A07 |
| Conflicting provider (after finalize + query) | `DisputedExternal`; hard cleared; escrow unchanged | P4-A09 |
| Adapter timeout / error | Remain Requested; escrow unchanged | P4-A14 |
| Stale report finalize after reverse | **Gap — P4-SEC-002** | Not covered by current suite |
| Direct `set_hard_settlement_flag(true)` | **Gap — P4-SEC-001** | Not blocked |

All covered adversarial cases fail closed as specified.

---

## Validated by PROTO-4

Only locally evidenced claims:

| Area | Evidenced behaviour |
|------|---------------------|
| Authority split (transitions) | Adapter cannot rewrite PROTO-2 status/conservation |
| Capability before side effects | Bind/settle/query/cancel gated; adapter not called on auth failure |
| Soft ≠ hard fields | Distinct; Confirmed alone does not set hard |
| Fake confirmation (mock proof) | Missing/wrong proof rejected |
| Duplicate / cross-binding replay | Rejected or idempotent without second debit |
| Conflict after finalize+query | Dispute + hard clear |
| NET not authority | Session/envelope alone insufficient |
| Deterministic binding IDs | Same intent → same ids |

---

## Not Yet Validated

- Production banking / payment-rail security
- Legal settlement finality
- Live-adapter honesty / Byzantine provider
- Freshness of confirmation at finalize (until P4-SEC-002 remediated)
- Integrity of hard flag against host API misuse (until P4-SEC-001 remediated)
- Multi-provider settlement, reputation, marketplace, open agent economy
- Hostile network or real wall-clock time

---

## Security Assumptions

- Trusted local `MockSettlementAdapterV0` (deterministic proof is not a secrecy boundary)
- Logical time harness-injected
- No hostile network
- No real banking environment
- In-process host treated as trusted for store handles (**weakened by P4-SEC-001 until remediated**)

---

## Recommended Remediation

### P4-SEC-001 — Restrict hard-flag API

1. Change `set_hard_settlement_flag` to `pub(crate)` (remove from public `escrow` re-exports), **or**
2. Split into crate-private `set_hard_settlement_flag_true` callable only from `finalize_settlement`, and allow public/crate clear-only if needed for dispute paths.

Add a regression test that public crates cannot promote hard without finalize (or document intentional test-only feature gate).

### P4-SEC-002 — Fresh evidence at finalize

1. `finalize_settlement` must `adapter.query(external_ref)` (or equivalent) and require live status `Confirmed` before setting hard flag.
2. Do **not** overwrite caller `report.status` to `Confirmed`; require report status ∈ {Confirmed} and match live query.
3. Add adversarial test: Confirmed → adapter reverse → finalize(stale report) → **reject**; hard remains false.

### Optional (low)

- Gate or crate-privatize `mark_disputed_external`.

---

## SECURITY_MODEL / README impact

Until remediation is merged and re-reviewed:

- Treat PROTO-4 as **implemented with residual hard-finality gaps**
- Do not treat `hard_settlement_placeholder=true` as a fully integrity-protected control-plane signal
- Documentation updates in this change set record the review outcome and caveats; architecture documents unchanged (no contradiction requiring redesign)

---

## Stop

After this review: **STOP.**

Do not start:

- second settlement backend
- reputation
- marketplace
- open agent economy
- production payment integrations

until remediation is accepted (or findings explicitly waived).
