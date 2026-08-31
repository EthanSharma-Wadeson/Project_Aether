# Apply Enablement Readiness Review

**Phase:** 11 — Safety Hardening & Enablement Readiness  
**Date:** 2026-07-30  
**Scope:** Control Plane Apply pipeline only (`aether-control-plane`)  
**Non-goals:** HTTP execute routes, `apply_enabled=true`, `aether-core` changes, live protocol mutations

---

## Executive Summary

Phase 11 closes the remaining enablement blockers identified by the Phase 9 security gate and Phase 10 adapter work. Atomic reserve+consume, pre-PROTO-0 attestation/policy freshness, reconciliation primitives, audit reconstruction, and fail-closed enablement configuration are now in place.

**Decision: READY FOR FINAL GATE**

Apply remains impossible in production: `apply_enabled()` is hard-false, no execute HTTP routes exist, and production config rejects `CP_APPLY_ENABLED=true`.

---

## Current Architecture

```
Dry-run → Attestation → Approval → Signature
    → Validation (G1–G14)
    → Re-simulation          (C1: before reserve)
    → Atomic prepare         (C2: reserve + consume)
    → Begin executing
    → Freshness recheck      (C3/C4: approval, signature, attestation, policy)
    → proto0_write           (mutations only if apply_enabled)
    → APPLY_EXECUTION_DISABLED / APPLY_DISABLED
```

| Layer | Mutation? | Notes |
|-------|-----------|-------|
| Validation / resim | No | Observation only |
| Replay prepare | No | DB-only one-shot guard |
| `proto0_write::execute_shared` | No while disabled | Pipeline path |
| `proto0_write::execute` | Yes only if enabled | Test override only |

---

## Remaining Conditions

| ID | Status | Notes |
|----|--------|-------|
| **C1** | **Closed** | Normative order documented as validate → resim → reserve (safer; no approval burn on sim fail). Frozen docs historically said reserve→resim — readiness gate should amend docs. |
| **C2** | **Closed** | `reserve_and_consume_approval` in one `BEGIN IMMEDIATE` txn; rollback leaves neither reserved nor consumed. |
| **C3** | **Closed** | Pre-PROTO-0 recheck: approval TTL/status, signature TTL, attestation `get_valid_attestation`. |
| **C4** | **Closed** | Policy version + approval binding consistency at boundary; audit payloads include correlation + policy fields. |
| **C5** | Closed (Phase 10) | Unknown intent never defaults to Grant. |
| **C6** | **Closed** | Startup scan, reserved/executing timeout → stuck, admin abort, reconcile audit events. No PROTO-0 retry. |
| **C7** | **Closed** | Full execution audit chain + `reconstruct_audit_by_request_id`. |
| **C8** | **Closed** | Early pipeline guards; env cannot enable in production; runtime `apply_enabled()` hard-false. |
| **C9** | **Partial / closed for pre-route** | `confirm: true` required when enabled; HTTP execute + CSRF consume still deferred to route phase. |
| **C10** | **Closed** | Hardening tests added (see Test Coverage). |

---

## Atomicity Review

**Before:** `reserve` COMMIT then `consume_approval` — crash window could leave reserved + active approval.

**After:** `ReplayStore::reserve_and_consume_approval`:

1. `BEGIN IMMEDIATE`
2. Reject duplicate / in-progress replay
3. Require approval `active` and not expired
4. INSERT replay `reserved`
5. UPDATE approval `consumed`
6. `COMMIT` or `ROLLBACK`

Invariants:

- No consumed approval without reservation
- No reservation without successful consume
- No protocol mutation in prepare path

---

## Recovery Review

| Primitive | Behaviour |
|-----------|-----------|
| `startup_scan` | Sweeps reserved/executing timeouts → stuck; reports outstanding rows; audits `APPLY_RECONCILE_SCAN` |
| `sweep_reserved_timeouts` | `reserved` → `stuck` |
| `sweep_executing_timeouts` | `executing` → `stuck` (never assumes PROTO-0 success) |
| `admin_abort` | `stuck`/`reserved`/`executing` → `aborted`; audits `APPLY_RECONCILED` |

Wired at Control Plane process start (non-fatal on scan error).

**Explicit non-behaviour:** no automatic PROTO-0 retry; no auto-mark-executed.

---

## Audit Review

Execution events now carry:

`request_id`, `operation_id`, `policy_id`, `policy_version`, `execution_hash`, `actor`, `audit_correlation_id`, `phase`, `result`, `operation_intent`

Chain for a successful disabled Apply:

1. `APPLY_EXECUTION_STARTED`
2. `APPLY_VALIDATION_PASSED`
3. `APPLY_RESIMULATION_APPROVED`
4. `APPLY_REPLAY_RESERVED`
5. `APPLY_PROTO_BOUNDARY`
6. `APPLY_EXECUTION_BLOCKED`
7. `APPLY_EXECUTION_FINALISED`

Reconstruction: `reconstruct_audit_by_request_id`.

---

## Threat Review

| Threat | Mitigation |
|--------|------------|
| Partial prepare (reserve without consume) | Atomic txn (C2) |
| Stale attestation / signature / approval at execute | Pre-PROTO-0 recheck (C3) |
| Policy drift mid-flight | Version + binding check (C4) |
| Blind retry after crash | Stuck + admin abort only (C6) |
| Accidental enablement | Hard-false runtime + prod env reject (C8) |
| Execute without confirm | ConfirmRequired when enabled (C9) |
| Unknown intent → Grant | Reject unsupported (C5) |
| Live Apply via HTTP | No execute routes |

---

## Test Coverage

Added / extended:

- Atomic prepare rollback on missing approval
- Crash after reserve / during executing (persistence, no mutation)
- Approval expiry during prepare (no reserved row)
- Stale attestation rejection path
- Concurrent Apply preparation (one wins)
- Audit reconstruction by `request_id`
- Reconciliation scan + admin abort
- Executing timeout → stuck
- Confirm required when enabled
- Production config rejects `CP_APPLY_ENABLED`
- Enablement default disabled / ack required

---

## Remaining Risks

1. **HTTP execute surface not designed** — CSRF consume + confirm binding still needed when routes land (C9 residual).
2. **Frozen doc ordering** — historical reserve→resim text should be amended to match C1 code (documentation delta, not a runtime hole).
3. **Executing timeout uses `updated_at`** — adequate for Phase 11; dedicated `executing_at` column may be preferable later.
4. **Attestation mid-pipeline tests** may fail at G8/resim rather than boundary depending on TTL timing — both are fail-closed.
5. **mutation_audit table** not yet wired for Apply attempts (blueprint residual; CP `audit_log` reconstruction covers request trails).

None of these allow live protocol mutation while `apply_enabled()==false`.

---

## Enablement Recommendation

### READY FOR FINAL GATE

Proceed to a **final enablement gate** that must still require, before any flip:

1. Amend frozen docs for C1 ordering (or explicit waiver)
2. Design Apply execute HTTP route with CSRF + `confirm: true`
3. Wire Apply attempts into `mutation_audit` if required by blueprint
4. Operator runbook for stuck reconcile
5. Explicit dual-control change to flip `apply_enabled()` implementation

**Until that gate passes:** keep `apply_enabled()==false`, no execute routes, no production mutations.

---

## Verification

```bash
cargo fmt --check
cargo test   # aether-control-plane
cargo test   # aether-core
cargo clippy --all-targets -- -D warnings
```

Expected: all green; Apply disabled; no live protocol mutations outside explicit adapter unit tests with `ApplyEnabledGuard`.
