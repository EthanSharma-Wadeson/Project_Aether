# Apply State Machine

**Document type:** Frozen state machine specification  
**Version:** `aether.cp.apply.v1`  
**Date:** 2026-07-30  
**Related:** [APPLY_PROTOCOL_SPECIFICATION.md](APPLY_PROTOCOL_SPECIFICATION.md), [APPLY_FAILURE_MODEL.md](APPLY_FAILURE_MODEL.md)

---

## 1. Overview

Apply involves **four orthogonal state domains**. Implementations must track all domains; terminal failure in any domain blocks mutation.

| Domain | Store | Purpose |
|--------|-------|---------|
| **Policy template** | `policy_templates` | CP lifecycle (pre-existing) |
| **Dry-run attestation** | `dry_run_attestations` | Pre-flight evidence |
| **Apply approval** | `apply_approvals` | Human execution authorisation |
| **Replay operation** | `signed_operation_replay` | One-shot execution guard |

---

## 2. Policy template states (existing)

```text
draft ──submit──► pending_review ──approve──► approved ──archive──► archived
                       │                         │
                       └──reject──► rejected ────┘
                                      │
                       draft ◄──update (new version)
```

### Apply-relevant policy states

| State | Apply allowed? |
|-------|----------------|
| `draft` | **No** |
| `pending_review` | **No** |
| `approved` | **Precondition only** (template approved; Apply approval still required) |
| `rejected` | **No** |
| `archived` | **No** |

**Transition invalidating Apply bindings:** Any policy update incrementing `policy_version` or changing content **invalidates** all `active` apply approvals and unattested dry-runs for that policy (lazy invalidation on verification).

---

## 3. Dry-run attestation states

```text
                    ┌─────────────────┐
                    │  (no record)    │
                    └────────┬────────┘
                             │ POST dry-run
                             ▼
                    ┌─────────────────┐
              ┌────►│ dry_run_pending │ (transient, in-flight)
              │     └────────┬────────┘
              │              │
              │     ┌────────┴────────┐
              │     ▼                 ▼
              │ ┌──────────┐   ┌──────────────┐
              │ │dry_run_  │   │ dry_run_     │
              │ │failed    │   │ generated    │
              │ └──────────┘   │ (executable=  │
              │                │  false)      │
              │                └──────────────┘
              │                         │
              │                         │ executable=true
              │                         ▼
              │                ┌──────────────┐
              └────────────────│ dry_run_     │
                               │ executable   │◄─── bindable
                               └──────┬───────┘
                                      │ age > DRY_RUN_BINDING_MAX_AGE
                                      ▼
                               ┌──────────────┐
                               │ dry_run_     │
                               │ stale        │ (logical; reject on use)
                               └──────────────┘
```

| State | Description | Terminal? |
|-------|-------------|-----------|
| `dry_run_pending` | Request in flight | No |
| `dry_run_failed` | Validation/sign/sim failed | Yes |
| `dry_run_generated` | Completed but `executable = false` | Yes |
| `dry_run_executable` | Completed; bindable to Apply approval | No (until stale) |
| `dry_run_stale` | Older than 60 min; reject on bind/apply | Yes (for Apply) |

---

## 4. Apply approval states

```text
                         ┌─────────────────┐
                         │ (no approval)   │
                         └────────┬────────┘
                                  │ admin grant + SoD pass
                                  ▼
                         ┌─────────────────┐
                    ┌───►│ apply_approval_ │
                    │    │ active          │
                    │    └────────┬────────┘
                    │             │
         cancel     │    ┌────────┼────────┬──────────────┐
            │       │    ▼        ▼        ▼              ▼
            │       │ expire  consumed  cancel      (invalid bind
            │       │ (TTL)   (reserve) (admin)     at grant → reject)
            ▼       ▼        ▼        ▼
     ┌──────────┐ ┌──────┐ ┌────────┐ ┌───────────┐
     │cancelled │ │expired│ │consumed│ │ (no row)  │
     └──────────┘ └──────┘ └────────┘ └───────────┘
```

| State | Apply submit allowed? |
|-------|----------------------|
| `active` | **Yes** (if other gates pass) |
| `expired` | **No** |
| `cancelled` | **No** |
| `consumed` | **No** |

### Transitions

| From | Event | To | Side effect |
|------|-------|-----|-------------|
| — | `grant` | `active` | Set `expires_at = granted_at + 60m` |
| `active` | `time ≥ expires_at` | `expired` | None |
| `active` | `cancel` | `cancelled` | Audit |
| `active` | `reserve success` | `consumed` | Set `consumed_by_operation_id` |

---

## 5. Replay operation states (core execution)

```text
                              ┌─────────────┐
                              │  (no row)   │
                              └──────┬──────┘
                                     │ reserve() success
                                     ▼
                              ┌─────────────┐
                         ┌───►│  reserved   │◄───┐
                         │    └──────┬──────┘    │ timeout 5m
                         │           │           │ (no executing)
                         │           │ execute() │
                         │           ▼           ▼
                         │    ┌─────────────┐ ┌────────┐
                         │    │  executing  │ │ stuck  │──► manual reconcile
                         │    └──────┬──────┘ └────────┘
                         │           │
                         │     ┌─────┴─────┐
                         │     ▼           ▼
                         │ ┌─────────┐ ┌──────────┐
                         │ │executed │ │ rejected │
                         │ └─────────┘ └──────────┘
                         │
              abort()    │    ┌─────────┐
              (admin/    └───►│ aborted │
               system)         └─────────┘

     duplicate operation_id ──► reject (no new row)
     signature expired ──────► reject before reserve
```

| State | PROTO-0 called? | Terminal? | Retry same `operation_id`? |
|-------|-----------------|-----------|----------------------------|
| `reserved` | **No** | No | Reject `REPLAY_IN_PROGRESS` |
| `executing` | **In flight** | No | Reject `REPLAY_IN_PROGRESS` |
| `executed` | Yes (once) | Yes | Return cached success |
| `rejected` | Maybe (PROTO said no) | Yes | Return cached rejection |
| `stuck` | Unknown | Yes (auto) | Reject; admin `abort` → `aborted` |
| `aborted` | No / unknown | Yes | New `operation_id` required |

---

## 6. Composite Apply lifecycle (operator view)

Normative happy path:

```text
PolicyTemplateApproved
        │
        ▼
DryRunExecutable ──────────────────────────────┐
        │                                      │ (failure branches:
        ▼                                      │  dry_run_failed,
ApplyApprovalActive                            │  dry_run_generated,
        │                                      │  stale)
        ▼
ReplayReserved
        │
        ▼
ReplayExecuting
        │
        ▼
ReplayExecuted (protocol mutation occurred)
```

### Failure terminators (no mutation)

| Composite state | Code |
|-----------------|------|
| Gate validation failed | `APPLY_*` / auth errors |
| `ReplayRejected` (simulation) | `APPLY_SIMULATION_FAILED` |
| `ReplayRejected` (PROTO-0) | `APPLY_PROTO_REJECTED` |
| `ReplayAborted` | `REPLAY_ABORTED` |
| Approval expired/cancelled | `APPLY_APPROVAL_*` |

---

## 7. Timeout rules

| Timer | Starts | Expires | On expiry |
|-------|--------|---------|-----------|
| Apply approval TTL | `granted_at` | +60 min | `active` → `expired`; reject Apply |
| Apply signature TTL | `issued_at` | +15 min | Reject before reserve |
| Dry-run max age | `completed_at` | +60 min | Reject bind/apply as stale |
| Reserve stuck TTL | `reserved_at` | +5 min | `reserved` → `stuck` |

**No automatic retry** extends TTLs. New dry-run + new approval + new `operation_id` required.

---

## 8. Retry rules

| Scenario | Automatic retry? | Operator action |
|----------|------------------|-----------------|
| Gate failure | **No** | Fix input; new request |
| Simulation failure | **No** | Fix policy/protocol; new dry-run |
| PROTO-0 rejection | **No** | Investigate; new dry-run if still desired |
| Network timeout during `executing` | **No** | Manual reconcile; treat as `stuck` |
| Duplicate `operation_id` in progress | **No** | Wait or use new `operation_id` after abort |
| Successful `executed` retry | **No mutation** | Return cached result |

---

## 9. Cancellation rules

| Object | Who | Effect |
|--------|-----|--------|
| Apply approval `active` | `admin` | `cancelled`; cannot Apply |
| Replay `reserved` | `admin` reconcile | `abort` → `aborted` |
| Replay `stuck` | `admin` | `abort` → `aborted`; verify PROTO-0 state manually |
| Dry-run attestation | — | Cannot cancel; becomes stale by time |
| In-flight HTTP Apply | Client disconnect | Server continues or fails closed to `stuck` if uncertain |

---

## 10. State ↔ audit event mapping

| Transition | Audit action |
|------------|--------------|
| Dry-run complete (executable) | `DRY_RUN_COMPLETED` |
| Apply approval granted | `APPLY_APPROVAL_GRANTED` |
| Apply approval cancelled | `APPLY_APPROVAL_CANCELLED` |
| Apply submit received | `APPLY_REQUESTED` |
| Reserve success | `APPLY_RESERVED` |
| Re-simulation fail | `APPLY_SIMULATION_REJECTED` |
| PROTO-0 success | `APPLY_PROTO_SUCCESS` |
| PROTO-0 reject | `APPLY_PROTO_REJECTED` |
| Finalise | `APPLY_COMPLETED` or `APPLY_FAILED` |
| Stuck detected | `APPLY_REPLAY_STUCK` |
| Abort | `APPLY_REPLAY_ABORTED` |

---

## 11. Freeze statement

> State machine transitions in this document are normative. Implementations must not introduce undocumented states or transitions.
