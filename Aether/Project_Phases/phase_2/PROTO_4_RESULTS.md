# PROTO-4 Results — Phase 2

## Status

**PROTO-4 implementation + P4-SEC-001/002 remediation complete — awaiting fresh security approval**

Command:

```bash
cd Aether/core && cargo fmt --check && cargo test && cargo clippy --all-targets -- -D warnings
```

Date: 2026-07-28

**STOP.** Do not begin a second settlement backend, reputation, marketplace, networked economic flows, or production payment integrations until PROTO-4 receives a **fresh** security approval after this remediation.

---

## Implemented Scope

Settlement-binding prototype under `Aether/core/src/settlement/`:

| Module | Role |
|--------|------|
| `model.rs` | `SettlementStatus`, `EconomicOutcome`, action/message constants |
| `binding.rs` | `SettlementAccountBindingV0`, `SettlementBindingV0`, `SettlementIntentV0` |
| `adapter.rs` | `SettlementAdapterV0` trait + deterministic `MockSettlementAdapterV0` (`enterprise.ledger.v0`) |
| `report.rs` | `SettlementReportV0`, evidence package / commitments |
| `store.rs` | `SettlementStore` (no public `get_mut`; indexes by correlation / escrow+outcome / external ref) |
| `verify.rs` | Signature → identity → capability → escrow → settlement evidence checks |
| `transition.rs` | bind / request / submit / query / finalize / cancel transitions |

Narrow PROTO-2 helpers (crate-private only):

- `promote_verified_hard_settlement` — sets hard=`true` (callable only from `finalize_settlement`)
- `clear_hard_settlement_flag` — clears hard on dispute/conflict paths

No public API can promote hard finality.

### Architecture rule (enforced)

```text
Aether protocol state remains authoritative.
External settlement providers are evidence sources only.
Never: provider state → overwrite → Aether economic truth
```

### Trust pipeline (enforced)

```text
signature verification
  → identity validation
  → capability authorisation
  → escrow validation
  → settlement validation
  → adapter call (if authorised)
  → state mutation
```

---

## Proven

Only claims demonstrated by the frozen acceptance suite (`PROTO_4_ACCEPTANCE_TESTS.md`):

| Area | Evidence |
|------|----------|
| Account binding | Capability-gated `SettlementAccountBindingV0`; empty ref rejected; deterministic `binding_id` |
| Settle request | Terminal Released/Refunded escrows only; outcome mismatch rejected; escrow snapshot integrity |
| Adapter lifecycle | Requested → Submitted → Accepted → Confirmed → Finalized via mock adapter |
| Hard finality | `hard_settlement_placeholder=true` only after Confirmed + verified report + finalize |
| Soft ≠ hard | Soft `finalized=true` can coexist with hard flag false pre-confirm |
| Idempotency | Same `correlation_id` returns existing binding; no second adapter debit |
| Fake confirmation | Missing/wrong proof token cannot finalize or set hard flag |
| Capability / identity | Missing grant, revoked cap, frozen identity, spend over limit, expired grant fail closed |
| Escrow isolation | Failed / conflicted settle leaves PROTO-2 status and conservation unchanged |
| Provider conflict | Confirmed/Finalized then adapter reverse → `DisputedExternal`; hard flag cleared |
| NET not authority | Envelope / session alone cannot authorise settle |
| Determinism | Same intent → same `binding_id` / `correlation_id` |

**Test counts:** 48 lifecycle/integrity/integration/hypothesis tests + 14 adversarial tests. Full suite: **271** tests green; `cargo fmt --check` and `clippy -D warnings` clean. Prior PROTO-0/1/2/NET-0 regressions remain green.

---

## Assumptions

- Trusted local `MockSettlementAdapterV0` (adapter can lie; protocol fails closed on missing proof)
- Logical time injected by harness
- No real payment network, banking API, or chain client
- In-memory stores only
- Enterprise ledger stub identity `enterprise.ledger.v0` only

---

## Not Proven

- Banking / payment-rail finality
- Legal settlement or accounting finality
- Fraud prevention against a malicious production adapter
- Enterprise adoption or multi-provider settlement
- Atomic multi-leg settlement across providers
- Live networked economic flows

Overclaim ban: PROTO-4 does **not** prove production payment security or adapter honesty beyond the stub trust model.

---

## Security protections exercised

| Threat | Result |
|--------|--------|
| Fake confirmation | Reject finalize; hard flag false |
| Forged settle signature | Reject |
| Wrong account / provider | Reject |
| Wrong amount / partial amount | Failed; hard flag false |
| Duplicate settlement | Idempotent or reject; no double debit |
| Replay adapter response across bindings | Reject |
| Revoked capability / frozen identity | Reject |
| Conflicting provider response | `DisputedExternal`; hard cleared |
| Adapter timeout / error | Remain Requested; escrow unchanged |
| Identity-only settle | Reject |

All failures fail closed.

---

## Validated by PROTO-4 Remediation

Remediation of [PROTO_4_SECURITY_REVIEW.md](PROTO_4_SECURITY_REVIEW.md) findings P4-SEC-001 / P4-SEC-002 (regressions P4-SEC-R01–R03):

| Claim | Evidence |
|-------|----------|
| Hard finality cannot be promoted without the validated settlement path | `promote_verified_hard_settlement` is `pub(crate)`; not re-exported; only `finalize_settlement` sets hard=`true` (P4-SEC-R01) |
| Stale adapter evidence cannot create finality | Finalize requires fresh `adapter.query` still `Confirmed`; reversed adapter + cached report rejected (P4-SEC-R02) |
| Fresh confirmation still finalizes | Caller Confirmed report + live Confirmed query → Finalized + hard true; escrow economics unchanged (P4-SEC-R03) |
| External settlement evidence remains separate from Aether protocol truth | Finalize / dispute paths do not rewrite PROTO-2 status or balances |
| Adapter responses remain evidence, not authority | Live query is attestation for hard promotion only; escrow authority unchanged |

Caller-supplied report status is **not** overwritten; it must already be `Confirmed`.

---

## Exit criteria checklist

1. Frozen `P4-T/R/A/I/H/M` behaviours covered — **yes**
2. `cargo test` green; clippy `-D warnings`; prior suites green — **yes**
3. Capability before adapter side effects — **yes** (`P4-I03`)
4. Hard flag never without Confirmed+verify+fresh query — **yes** (post-remediation)
5. Escrow never overwritten by adapter lies — **yes**
6. No live bank/chain/token/marketplace code — **yes**
7. `PROTO_4_RESULTS.md` written — **yes**
8. P4-SEC-001 / P4-SEC-002 remediated + regressions — **yes**
9. Fresh security approval after remediation — **pending**

**Next step:** Fresh PROTO-4 security review. Do not expand scope until APPROVE.
