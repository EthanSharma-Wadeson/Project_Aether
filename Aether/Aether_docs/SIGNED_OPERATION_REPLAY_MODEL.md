# Signed Operation Replay Model

**Document type:** Frozen design (pre-Apply)  
**Status:** Design only — store not implemented  
**Date:** 2026-07-29  
**Related:** [CONTROL_PLANE_APPLY_SECURITY_MODEL.md](CONTROL_PLANE_APPLY_SECURITY_MODEL.md)

---

## Mandatory Invariant

```text
A signed operation can execute at most once.
```

Any second attempt with the same `operation_id` (or equivalent replay reference) must be rejected without performing another protocol mutation.

---

## Operation Identity

Each Apply-bound signed operation has:

| Field | Description |
|-------|-------------|
| `operation_id` | Globally unique UUID (primary replay key) |
| `purpose` | Must be `apply` (dry-run signatures are a different purpose and are never replayable as Apply) |
| `payload_hash` | Hash of canonical Apply payload (includes `execution_hash`) |
| `signer_identity` | Identity label of the signing backend |
| `issued_at` | UTC timestamp |
| `expires_at` | UTC expiry; hard reject after this time |
| `request_id` | Correlation id |
| `dry_run_id` | Bound dry-run attestation |
| `policy_id` / `policy_version` | Bound policy version |

Optional secondary nonce may be included; if present it is stored alongside `operation_id` but does not replace it.

---

## Storage Model (Future)

Proposed durable table: `signed_operation_replay` (name illustrative).

| Column | Type | Notes |
|--------|------|-------|
| `operation_id` | TEXT PK | Unique |
| `payload_hash` | TEXT NOT NULL | |
| `signer_identity` | TEXT NOT NULL | |
| `request_id` | TEXT NOT NULL | |
| `dry_run_id` | TEXT NOT NULL | |
| `policy_id` | TEXT NOT NULL | |
| `policy_version` | INTEGER NOT NULL | |
| `execution_hash` | TEXT NOT NULL | |
| `issued_at` | TEXT NOT NULL | RFC3339 |
| `expires_at` | TEXT NOT NULL | RFC3339 |
| `status` | TEXT NOT NULL | `reserved` \| `executed` \| `rejected` \| `expired` |
| `protocol_result` | TEXT NULL | Set on terminal Apply attempt |
| `created_at` | TEXT NOT NULL | Insert time |

### Lifecycle

1. **Reserve** — Before calling PROTO-0, insert row with `status=reserved` under a unique constraint on `operation_id`. Conflict → reject as replay.
2. **Execute** — Call protocol adapter once.
3. **Finalise** — Update to `executed` or `rejected` with `protocol_result`. Never delete rows (append-only semantics via status transitions).

If the process crashes after reserve but before finalise, recovery must treat `reserved` as non-retryable for mutation (fail closed / manual reconcile), never auto-retry PROTO-0 blindly.

---

## Expiry

- Signatures carry `expires_at` (recommended short TTL, e.g. minutes).
- At Apply time: if `now > expires_at` → reject (`expired`), record audit, do not mutate.
- Expired rows remain in storage for audit; may be archived offline later.

---

## Duplicate Handling

| Scenario | Behaviour |
|----------|-----------|
| Same `operation_id` presented again | Reject — replay |
| Different `operation_id`, identical payload | Allowed only if policy still valid; PROTO-0 may still reject as duplicate grant — CP still treats each op as one-shot |
| Concurrent inserts of same `operation_id` | DB unique constraint; loser rejects |
| Client retry after success | Return prior result / conflict; **no second mutation** |
| Client retry after reserved crash | Reject or operator-mediated recovery; **no automatic second mutation** |

---

## Replay Rejection

Reject when any of:

- `operation_id` already in store
- Signature expired
- Purpose ≠ `apply`
- Payload / `execution_hash` mismatch vs approval binding
- Signer missing / verification fails
- Approval revoked or cancelled

Audit every rejection with `request_id`, `operation_id`, and reason.

---

## Relation to Dry-Run

Dry-run may sign with purpose `dry_run`. Those signatures:

- Are **not** inserted into the Apply replay store as executable Apply ops
- **Must not** be accepted by Apply verification
- May be audited separately (`signer_audit`)

Apply always creates a **new** `operation_id` and Apply-purpose signature after human approval.

---

## Freeze Statement

> Replay protection design is frozen. Implementation of the replay store and Apply executor requires separate approval. Until then, no signed Apply operations exist and no protocol mutations are performed.
