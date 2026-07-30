# Apply Failure Model

**Document type:** Frozen failure-handling specification  
**Version:** `aether.cp.apply.v1`  
**Date:** 2026-07-30  
**Related:** [APPLY_PROTOCOL_SPECIFICATION.md](APPLY_PROTOCOL_SPECIFICATION.md), [APPLY_STATE_MACHINE.md](APPLY_STATE_MACHINE.md), [APPLY_SEQUENCE_DIAGRAMS.md](APPLY_SEQUENCE_DIAGRAMS.md)

---

## 1. Principles

| Principle | Rule |
|-----------|------|
| **Fail closed** | On uncertainty, do not mutate protocol state |
| **No blind retry** | Never auto-retry PROTO-0 for same `operation_id` |
| **Audit always** | Every failure path appends audit with `failure_reason` |
| **Replay truth** | Terminal replay row is authoritative for idempotent retries |
| **Operator visibility** | `stuck` states require admin-visible reconcile workflow |

---

## 2. Failure matrix by stage

Legend:

- **FC** = Fail closed (no PROTO-0)
- **Retry** = Automatic retry allowed?
- **Manual** = Operator/admin action required
- **Protocol** = PROTO-0 state after failure

| Stage | Failure | FC | Retry | Manual | Protocol | HTTP |
|-------|---------|----|-------|--------|----------|------|
| Auth | Missing/invalid JWT | Yes | No | Re-login | Unchanged | 401 |
| Auth | RBAC deny | Yes | No | Escalate role | Unchanged | 403 |
| Auth | CSRF fail | Yes | No | Fresh token | Unchanged | 403 |
| Auth | Origin fail | Yes | No | Fix origin | Unchanged | 403 |
| Gate | `confirm ≠ true` | Yes | No | Resubmit | Unchanged | 400 |
| Gate | Policy not approved | Yes | No | Approve template | Unchanged | 409 |
| Gate | Policy archived | Yes | No | — | Unchanged | 409 |
| Dry-run bind | Not executable | Yes | No | Fix policy | Unchanged | 409 |
| Dry-run bind | Stale (>60m) | Yes | No | New dry-run | Unchanged | 409 |
| Hash | Stale execution_hash | Yes | No | New dry-run | Unchanged | 409 |
| Approval | Missing | Yes | No | Grant approval | Unchanged | 403 |
| Approval | Expired | Yes | No | New approval | Unchanged | 403 |
| Approval | Cancelled | Yes | No | New approval | Unchanged | 403 |
| Approval | Consumed | Yes | No | New approval + op | Unchanged | 409 |
| Signer | Missing/unavailable | Yes | No | Fix signer | Unchanged | 503 |
| Signer | Invalid signature | Yes | No | Re-sign | Unchanged | 403 |
| Signer | Wrong purpose (dry-run) | Yes | No | Apply sign | Unchanged | 403 |
| Signer | Signature expired | Yes | No | Re-sign | Unchanged | 403 |
| Signer | Key rotated | Yes | No | Re-sign | Unchanged | 403 |
| Replay | Duplicate in progress | Yes | No | Wait/abort | Unchanged | 409 |
| Replay | Duplicate terminal | Yes | **Idempotent read** | None | Unchanged | 200/409 |
| Sim | Re-simulation fail | Yes | No | Investigate | Unchanged | 422 |
| PROTO | PROTO-0 reject | Yes | No | Fix + new dry-run | Unchanged | 422 |
| Network | Timeout during PROTO-0 | Yes | **No** | **Reconcile** | **Unknown** | 503 |
| Crash | After reserve | Yes | No | **Reconcile** | Unknown | — |
| Crash | After PROTO-0 | Yes | No | **Reconcile** | May have changed | — |
| Crash | Before audit | Yes | No | Backfill audit | Per replay row | — |

---

## 3. Stage-specific failure handling

### 3.1 Authentication / authorisation

**Fail closed:** Yes.  
**Retry:** Client may retry with valid credentials.  
**Manual recovery:** None.  
**Operator notification:** Generic error (no user enumeration).  
**Audit:** `APPLY_FAILED` with `reason=auth|rbac|csrf|origin`.  
**Protocol state:** Unchanged.

---

### 3.2 Policy / execution hash validation

**Fail closed:** Yes.  
**Retry:** Only after new dry-run produces new `execution_hash`.  
**Manual recovery:** Operator updates policy or re-runs dry-run.  
**Audit:** `APPLY_FAILED` with `APPLY_STALE_EXECUTION_HASH` or `APPLY_STALE_POLICY_VERSION`.  
**Protocol state:** Unchanged.

**Stale policy scenarios:**

| Event | Behaviour |
|-------|-----------|
| Policy edited after dry-run | Hash mismatch → reject |
| Policy edited after Apply approval | Hash mismatch at submit → reject |
| Policy archived | Reject at gate |
| Version bump | All prior bindings invalid |

---

### 3.3 Apply approval

**Fail closed:** Yes.  
**Expired approval:** Lazy mark `expired`; reject submit.  
**Cancelled:** Reject submit.  
**Consumed:** Reject new submit with same approval.  
**Audit:** `APPLY_APPROVAL_EXPIRED`, `APPLY_APPROVAL_CANCELLED`, or `APPLY_APPROVAL_CONSUMED`.  
**Protocol state:** Unchanged.

---

### 3.4 Signer failures

**Fail closed:** Yes.  
**Missing signer:** Boot failure in production (`production_config_check`); runtime unavailable → 503.  
**Invalid signature:** Reject; no reserve.  
**Dry-run signature at Apply:** Reject `APPLY_INVALID_SIGNATURE_PURPOSE`.  
**Key rotation:** All signatures with old `signer_identity` rejected; operators must re-sign.  
**Audit:** `SIGN_VERIFY_FAILED` in `signer_audit`; `APPLY_FAILED` in `audit_log`.  
**Protocol state:** Unchanged.

**Verification order (normative):** See APPLY_PROTOCOL_SPECIFICATION §9.3.

---

### 3.5 Replay: reserve()

| Outcome | Behaviour |
|---------|-----------|
| Insert success | Proceed to re-simulation |
| Unique violation (`operation_id`) | Load existing row; idempotent branch |
| Approval already consumed | Reject `APPLY_APPROVAL_CONSUMED` |

**Audit:** `APPLY_RESERVED` or `APPLY_REPLAY_REJECTED`.  
**Protocol state:** Unchanged until `execute`.

---

### 3.6 Replay: execute() — re-simulation failure

**Fail closed:** Yes — PROTO-0 **not** called.  
**Effect:** `finalise(rejected, APPLY_SIMULATION_FAILED)`.  
**Retry:** New `operation_id` + unconsumed approval **not possible** (approval consumed). Operator must grant **new** Apply approval after new dry-run.  
**Audit:** `APPLY_SIMULATION_REJECTED`.  
**Protocol state:** Unchanged.

---

### 3.7 Replay: execute() — PROTO-0 rejection

**Fail closed:** Yes (no successful mutation).  
**Effect:** `finalise(rejected, protocol_result=rejected)`.  
**Retry:** Cached rejection for same `operation_id`; new attempt needs new approval chain.  
**Audit:** `APPLY_PROTO_REJECTED`, `APPLY_FAILED`.  
**Protocol state:** Unchanged.

---

### 3.8 Network timeout during PROTO-0

**Fail closed:** Yes — assume uncertain.  
**Effect:** Do not call PROTO-0 again. Mark `stuck` (or remain `executing` until stuck timer).  
**Retry:** **Forbidden** automatic retry.  
**Manual recovery:**

1. Admin queries PROTO-0 read-only (did mutation occur?).
2. If yes → `reconcile-finalise(executed)`.
3. If no → `abort` → `aborted`; operator may start new flow with new IDs.

**Operator notification:** `APPLY_REPLAY_STUCK` alert / admin UI flag.  
**Audit:** `APPLY_REPLAY_STUCK`, then `APPLY_RECONCILED` or `APPLY_REPLAY_ABORTED`.  
**Protocol state:** Unknown until reconcile; **never guess**.

---

### 3.9 Crash after reserve, before execute

**Fail closed:** Yes.  
**Timer:** `REPLAY_RESERVE_TTL` = 5 minutes → `stuck`.  
**Manual recovery:** Same as §3.8.  
**Protocol state:** Unchanged if PROTO-0 not called.

---

### 3.10 Crash after PROTO-0, before finalise

**Fail closed:** Treat as uncertain until reconcile.  
**Manual recovery:** Read PROTO-0; align replay row with reality; backfill audit.  
**Protocol state:** May have changed — **PROTO-0 is source of truth** for protocol facts.

---

### 3.11 Crash before audit

**Fail closed:** N/A (mutation may have completed).  
**Recovery:** On startup, scan `executing`/`reserved` rows; reconcile. Backfill audit idempotently using `operation_id` as dedup key.  
**Protocol state:** Per PROTO-0 query.

---

### 3.12 Duplicate submission

| Case | HTTP | Mutation |
|------|------|----------|
| Same `operation_id`, `executed` | 200 + cached | No |
| Same `operation_id`, `rejected` | 409 + cached | No |
| Same `operation_id`, in progress | 409 | No |
| New `operation_id`, consumed approval | 409 | No |
| New `operation_id`, valid new approval | Normal flow | At most once |

---

## 4. abort() specification

**Who may call:** `admin` or system timer (stuck transition).

**Preconditions:** `status ∈ {reserved, executing, stuck}`.

**Effects:**

| Prior status | PROTO-0 called? | New status | PROTO-0 retry? |
|--------------|-----------------|------------|----------------|
| `reserved` | No | `aborted` | No |
| `executing` | Unknown | `aborted` after read-only check | No |
| `stuck` | Unknown | `aborted` after reconcile | No |

**Audit:** `APPLY_REPLAY_ABORTED` with `failure_reason`.

**After abort:** New Apply requires new `operation_id`, new Apply approval, and new Apply signature.

---

## 5. Manual reconciliation protocol

### 5.1 Admin reconcile query

`GET /api/apply-operations/{operation_id}/reconcile-status`

Returns:

- `replay_status`
- `protocol_observation` (read-only)
- `recommended_action`: `finalise_executed` | `finalise_rejected` | `abort` | `pending`

### 5.2 Admin reconcile action

`POST /api/apply-operations/{operation_id}/reconcile`

Body: `{ "action": "finalise_executed" | "finalise_rejected" | "abort", "confirm": true, "reason": "..." }`

- Requires `admin` + CSRF + audit.
- Must include read-only PROTO-0 evidence in audit metadata.

---

## 6. Operator notification (normative minimum)

| Event | Channel | Content |
|-------|---------|---------|
| Apply success | API response + audit | `operation_id`, `protocol_result` |
| Apply rejected (PROTO) | API 422 | PROTO reason |
| Replay stuck | API 503 + admin alert | `operation_id`, reconcile URL |
| Approval expiring soon | Optional (future) | `apply_approval_id`, `expires_at` |

MVP: API response codes + audit sufficient; admin UI SHOULD surface `stuck` rows.

---

## 7. Audit records per failure

Every failure **must** write at minimum:

```json
{
  "request_id": "<string>",
  "operation_id": "<uuid|null>",
  "dry_run_id": "<uuid|null>",
  "execution_hash": "<hex|null>",
  "apply_approval_id": "<uuid|null>",
  "signer_identity": "<string|null>",
  "policy_id": "<string>",
  "policy_version": <int|null>,
  "operator_id": "<string>",
  "event": "APPLY_FAILED",
  "failure_code": "<stable code>",
  "failure_reason": "<human readable>",
  "replay_status": "<terminal or null>",
  "protocol_result": "not_applicable|rejected",
  "timestamp": "<RFC3339>"
}
```

Plus corresponding `mutation_audit` row with `protocol_result = rejected` or `not_applicable`.

---

## 8. Immutable audit chain (normative)

1. All audit tables are **append-only** (no UPDATE/DELETE on audit rows).
2. Each Apply attempt produces ≥1 `audit_log` row and exactly 1 `mutation_audit` row.
3. `signer_audit` row for every sign verify attempt on Apply path.
4. Replay terminal transition produces `APPLY_COMPLETED` or `APPLY_FAILED`.
5. Reconcile backfill uses `operation_id` deduplication — duplicate append forbidden.
6. Future enhancement (SEC-CP-03): hash chain over `audit_log` — not required for MVP but recommended.

---

## 9. Rollback attacks

**Threat:** Operator attempts to undo mutation via CP.  
**Mitigation:** No rollback API. PROTO-0 mutations are forward-only; revoke is a **new** governed Apply flow.  
**Failure mode:** N/A — feature absent.

---

## 10. Freeze statement

> Failure handling in this document is normative. Implementations must not auto-retry PROTO-0 for the same `operation_id` or proceed without audit on any path.
