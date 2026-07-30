# Apply Sequence Diagrams

**Document type:** Frozen sequence specification  
**Version:** `aether.cp.apply.v1`  
**Date:** 2026-07-30  
**Related:** [APPLY_PROTOCOL_SPECIFICATION.md](APPLY_PROTOCOL_SPECIFICATION.md), [APPLY_STATE_MACHINE.md](APPLY_STATE_MACHINE.md), [APPLY_FAILURE_MODEL.md](APPLY_FAILURE_MODEL.md)

---

## 1. Participants

| Participant | Role |
|-------------|------|
| **Operator** | Human or automation with JWT |
| **Admin** | Operator with `admin` role (Apply approval) |
| **Control Plane (CP)** | API + validation + orchestration |
| **Signer** | `SigningGateway` — attests Apply payload |
| **Replay Store** | `signed_operation_replay` — one-shot guard |
| **PROTO-0** | Protocol authority — grant/revoke |
| **Audit** | `audit_log` + `mutation_audit` + `signer_audit` |

---

## 2. Normal execution (happy path)

### 2.1 Phase A — Dry-run (prerequisite)

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Sig as Signer
    participant Aud as Audit

    Op->>CP: POST /policies/{id}/dry-run<br/>JWT + CSRF + Origin
    CP->>CP: RBAC (operator+)
    CP->>CP: Policy status = approved
    CP->>CP: Simulate (read-only PROTO-0)
    CP->>CP: Compute execution_hash
    CP->>Sig: sign(dry_run payload, purpose=dry_run)
    Sig-->>CP: dry_run signature
    CP->>Aud: DRY_RUN_COMPLETED<br/>dry_run_id, execution_hash
    CP-->>Op: 200 executable=true<br/>dry_run_id, execution_hash
```

### 2.2 Phase B — Apply approval grant

```mermaid
sequenceDiagram
    autonumber
    participant Adm as Admin
    participant CP as Control Plane
    participant Aud as Audit

    Adm->>CP: POST /apply-approvals<br/>dry_run_id, execution_hash, confirm=true
    CP->>CP: RBAC (admin)
    CP->>CP: Verify attestation executable
    CP->>CP: Recompute execution_hash = match
    CP->>CP: SoD (approver ≠ dry-run operator)
    CP->>CP: Create apply_approval (active, TTL 60m)
    CP->>Aud: APPLY_APPROVAL_GRANTED
    CP-->>Adm: 201 apply_approval_id
```

### 2.3 Phase C — Apply submit (mutation)

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Sig as Signer
    participant Rep as Replay Store
    participant P0 as PROTO-0
    participant Aud as Audit

    Op->>CP: POST /apply<br/>confirm=true, operation_id,<br/>signature, apply_approval_id
    CP->>CP: G1–G10 gates (JWT, RBAC, CSRF, Origin, confirm)
    CP->>CP: Verify execution_hash + approval active
    CP->>Sig: verify(signature, purpose=apply)
    Sig-->>CP: valid
    CP->>Aud: APPLY_REQUESTED
    CP->>Rep: reserve(operation_id)
    Rep-->>CP: reserved (approval → consumed)
    CP->>Aud: APPLY_RESERVED
    CP->>CP: mandatory re-simulation (read-only)
    CP->>Rep: status = executing
    CP->>P0: grant_capability / revoke_capability
    P0-->>CP: accepted
    CP->>Rep: finalise(executed, success)
    CP->>Aud: APPLY_PROTO_SUCCESS, APPLY_COMPLETED
    CP-->>Op: 200 protocol_result=success
```

---

## 3. Replay attempt (duplicate operation_id)

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Rep as Replay Store
    participant P0 as PROTO-0
    participant Aud as Audit

    Op->>CP: POST /apply (same operation_id)
    CP->>CP: Gates + signature verify
    CP->>Rep: reserve(operation_id)
    alt status = executed
        Rep-->>CP: REPLAY_DUPLICATE (terminal success)
        CP->>Aud: APPLY_REPLAY_CACHED
        CP-->>Op: 200 cached result (no PROTO-0 call)
    else status = rejected
        Rep-->>CP: REPLAY_DUPLICATE (terminal reject)
        CP-->>Op: 409 cached rejection
    else status = reserved | executing | stuck
        Rep-->>CP: REPLAY_IN_PROGRESS
        CP->>Aud: APPLY_REPLAY_REJECTED
        CP-->>Op: 409 REPLAY_IN_PROGRESS (no PROTO-0 call)
    end
    Note over P0: PROTO-0 never called
```

---

## 4. Expired approval

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Rep as Replay Store
    participant P0 as PROTO-0
    participant Aud as Audit

    Op->>CP: POST /apply
    CP->>CP: Load apply_approval
    alt now >= expires_at OR status = expired
        CP->>CP: Reject APPLY_APPROVAL_EXPIRED
        CP->>Aud: APPLY_FAILED (approval expired)
        CP-->>Op: 403 APPLY_APPROVAL_EXPIRED
    end
    Note over Rep,P0: reserve() never called
```

---

## 5. Expired signature

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Rep as Replay Store
    participant Aud as Audit

    Op->>CP: POST /apply (expires_at in past)
    CP->>CP: Gates G1–G10 pass
    CP->>CP: now >= signature.expires_at
    CP->>Aud: APPLY_FAILED (signature expired)
    CP-->>Op: 403 APPLY_SIGNATURE_EXPIRED
    Note over Rep: No reserve()
```

---

## 6. PROTO-0 rejection

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Rep as Replay Store
    participant P0 as PROTO-0
    participant Aud as Audit

    Op->>CP: POST /apply
    CP->>CP: Gates + verify + reserve
    CP->>CP: re-simulation pass
    CP->>Rep: executing
    CP->>P0: grant_capability
    P0-->>CP: rejected (protocol rules)
    CP->>Rep: finalise(rejected, protocol_result=rejected)
    CP->>Aud: APPLY_PROTO_REJECTED, APPLY_FAILED
    CP-->>Op: 422 APPLY_PROTO_REJECTED
    Note over P0: No state change (PROTO rejected)
```

---

## 7. Re-simulation failure (stale protocol state)

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Rep as Replay Store
    participant P0 as PROTO-0
    participant Aud as Audit

    Op->>CP: POST /apply
    CP->>CP: reserve success
    CP->>CP: re-simulate (capability revoked externally)
    CP->>CP: executable = false
    CP->>Rep: finalise(rejected, APPLY_SIMULATION_FAILED)
    CP->>Aud: APPLY_SIMULATION_REJECTED
    CP-->>Op: 422 APPLY_SIMULATION_FAILED
    Note over P0: PROTO-0 never called
```

---

## 8. Network failure (timeout during PROTO-0)

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Rep as Replay Store
    participant P0 as PROTO-0
    participant Aud as Audit

    Op->>CP: POST /apply
    CP->>Rep: reserved → executing
    CP->>P0: grant_capability
    Note over CP,P0: Network timeout — outcome unknown
    CP->>CP: Do NOT auto-retry PROTO-0
    CP->>Rep: status = stuck (or remain executing → stuck via timer)
    CP->>Aud: APPLY_REPLAY_STUCK
    CP-->>Op: 503 APPLY_UNCERTAIN (if response not sent)
    Note over Op: Operator must reconcile manually
```

---

## 9. Crash recovery

### 9.1 Crash after reserve, before execute

```mermaid
sequenceDiagram
    autonumber
    participant Sys as System / Admin
    participant CP as Control Plane
    participant Rep as Replay Store
    participant P0 as PROTO-0
    participant Aud as Audit

    Note over Rep: Row: status=reserved, reserved_at set
    CP->>CP: Startup / timer: reserved_at + 5m elapsed
    CP->>Rep: transition reserved → stuck
    CP->>Aud: APPLY_REPLAY_STUCK
    Sys->>CP: GET reconcile status (operation_id)
    CP->>P0: read-only verify (no mutation)
    alt PROTO shows mutation did not occur
        Sys->>CP: POST abort(operation_id)
        CP->>Rep: stuck → aborted
        CP->>Aud: APPLY_REPLAY_ABORTED
    else PROTO shows mutation occurred
        Sys->>CP: POST reconcile-finalise(executed)
        CP->>Rep: stuck → executed (audit only)
        CP->>Aud: APPLY_RECONCILED
    end
```

### 9.2 Crash after PROTO-0 success, before finalise

```mermaid
sequenceDiagram
    autonumber
    participant Sys as System / Admin
    participant CP as Control Plane
    participant Rep as Replay Store
    participant P0 as PROTO-0
    participant Aud as Audit

    Note over Rep: Row: status=executing
    Sys->>P0: read-only query (did grant occur?)
    alt grant present
        Sys->>CP: reconcile-finalise(executed)
        CP->>Rep: executing → executed
        CP->>Aud: APPLY_RECONCILED
    else grant absent
        Sys->>CP: abort → rejected
        CP->>Rep: executing → rejected
        CP->>Aud: APPLY_REPLAY_ABORTED
    end
```

### 9.3 Crash before audit write

```mermaid
sequenceDiagram
    autonumber
    participant CP as Control Plane
    participant Rep as Replay Store
    participant Aud as Audit

    Note over CP: finalise committed to Replay Store
    CP--xAud: crash before audit append
    CP->>CP: On recovery: replay row is source of truth
    CP->>Aud: append missing APPLY_COMPLETED (idempotent by operation_id)
```

**Rule:** Replay store terminal state takes precedence; audit backfill is idempotent on `operation_id`.

---

## 10. Dry-run signature replay at Apply (forbidden)

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Aud as Audit

    Op->>CP: POST /apply (dry_run signature, purpose=dry_run)
    CP->>CP: purpose ≠ apply
    CP->>Aud: APPLY_FAILED (invalid signature purpose)
    CP-->>Op: 403 APPLY_INVALID_SIGNATURE_PURPOSE
```

---

## 11. Stale policy / execution hash

```mermaid
sequenceDiagram
    autonumber
    participant Op as Operator
    participant CP as Control Plane
    participant Aud as Audit

    Op->>CP: POST /apply (old execution_hash)
    CP->>CP: Load live policy version N+1
    CP->>CP: Recompute execution_hash' ≠ submitted
    CP->>Aud: APPLY_FAILED (stale execution hash)
    CP-->>Op: 409 APPLY_STALE_EXECUTION_HASH
```

---

## 12. Message ordering invariants

1. **Audit** `APPLY_REQUESTED` after gate pass, before `reserve`.
2. **Replay** `reserve` before re-simulation and before PROTO-0.
3. **Re-simulation** after `reserve`, before `executing`.
4. **PROTO-0** at most once per `operation_id`.
5. **Finalise** always called after PROTO-0 returns or simulation fails post-reserve.
6. **Signer verify** before `reserve`.

---

## 13. Freeze statement

> Sequence ordering in this document is normative. Implementations must not reorder `reserve`, re-simulation, PROTO-0, and `finalise`.
