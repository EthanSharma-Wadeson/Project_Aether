# Treasury Write API Specification — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_WRITE_API_SPEC.md` |
| **Phase** | 20 — Treasury Write Security Gate |
| **Status** | **DESIGN FROZEN** — specification only; **no routes implemented** |
| **Date** | 2026-08-01 |
| **Related** | [TREASURY_WRITE_SECURITY_MODEL.md](TREASURY_WRITE_SECURITY_MODEL.md), [TREASURY_CONTROL_PLANE_ARCHITECTURE.md](TREASURY_CONTROL_PLANE_ARCHITECTURE.md) (read surface remains authoritative for GETs) |

---

## Executive Summary

Future treasury **mutations** are exposed only under `/api/treasury/...` via the Control Plane adapter. This document defines request/response shapes, permissions, audit actions, and errors for Phase 21 implementation **after separate approval**.

**Not implemented in Phase 20.** Apply remains disabled. No custody/payment endpoints.

---

## 1. Common conventions

### 1.1 Authentication & mutation guards

| Header / field | Requirement |
|----------------|-------------|
| `Authorization: Bearer <access_jwt>` | Required |
| `X-CSRF-Token` | Required on all `POST`/`PATCH`/`DELETE` below |
| `Origin` / `Referer` | Subject to `CP_ALLOWED_ORIGINS` |
| `Idempotency-Key` | Required on execute and single-shot mutations |
| `X-Request-Id` | Optional; server generates if absent; echoed |

### 1.2 Dual-control flow (high-risk)

```text
1. POST /api/treasury/mutations          → request_id, status=pending
2. POST /api/treasury/mutations/:id/approve  → approval_id (approver ≠ requester)
3. POST /api/treasury/mutations/:id/execute  → CSRF consume + engine + journal_batch_id
```

Low-risk operations may use **single-shot** endpoints (reserve/release/settlement_post) without steps 1–2.

### 1.3 Envelope

**Success (execute / single-shot):**

```json
{
  "request_id": "…",
  "outcome": "success",
  "operation": "fund",
  "journal_batch_id": "…",
  "resource": { },
  "approval_id": null,
  "duplicate": false
}
```

**Duplicate idempotency:** same body with `"duplicate": true` and original ids.

**Error:**

```json
{
  "error": "human readable",
  "code": "TREASURY_INSUFFICIENT_FUNDS",
  "request_id": "…",
  "retryable": false
}
```

### 1.4 Stable error codes (map from `TreasuryError`)

| `code` | HTTP | Engine / cause |
|--------|------|----------------|
| `UNAUTHORIZED` | 401 | JWT |
| `FORBIDDEN` | 403 | RBAC / SoD / viewer write |
| `CSRF_FAILED` | 403 | CSRF |
| `NOT_FOUND` | 404 | Unknown or wrong-org resource |
| `VALIDATION` | 400 | `Validation`, negative amount, etc. |
| `TREASURY_FROZEN` | 409 | `TreasuryFrozen` |
| `TREASURY_CLOSED` | 409 | `TreasuryClosed` |
| `INSUFFICIENT_FUNDS` | 409 | `InsufficientFunds` |
| `INSUFFICIENT_ALLOCATION` | 409 | `InsufficientAllocation` |
| `ALLOCATION_EXPIRED` | 409 | `AllocationExpired` |
| `RESERVATION_NOT_ACTIVE` | 409 | `ReservationNotActive` |
| `APPROVAL_REQUIRED` | 400 | Missing approval on H op |
| `APPROVAL_EXPIRED` | 409 | TTL |
| `APPROVAL_SOD` | 403 | requester == approver |
| `PAYLOAD_MISMATCH` | 409 | hash bind fail |
| `UNSUPPORTED_ASSET` | 400 | |
| `CONFLICT_IDEMPOTENCY` | 409 | Only if product chooses not to return original — **prefer success+duplicate** |
| `UNAVAILABLE` | 503 | DB |

---

## 2. Mutation request resource

### `POST /api/treasury/mutations`

Create a pending high-risk mutation request.

**Permissions:** `operator` or `admin`  
**CSRF:** required (not consumed)  
**Audit:** `treasury.mutation.request`

**Request**

```json
{
  "operation": "fund",
  "treasury_id": "tr_…",
  "payload": {
    "asset_id": "GBP",
    "amount_minor": 100000,
    "funding_kind": "internal_acknowledgment",
    "memo": "lab seed top-up"
  },
  "idempotency_key": "req-…"
}
```

`operation` enum (high-risk subset):

`fund` | `allocation_create` | `allocation_increase` | `allocation_reduce` | `refund` | `chargeback` | `adjustment` | `treasury_create` | `treasury_close` | `asset_register` | `unfreeze` | `force_release_reservation`

**Response `201`**

```json
{
  "mutation_id": "tm_…",
  "status": "pending",
  "operation": "fund",
  "payload_hash": "sha256:…",
  "requested_by": "op_…",
  "expires_at": "2026-08-01T12:00:00Z",
  "request_id": "…"
}
```

### `POST /api/treasury/mutations/:mutation_id/approve`

**Permissions:** `admin` and `operator_id` ≠ requester  
**CSRF:** required (not consumed)  
**Audit:** `treasury.mutation.approve`

**Request**

```json
{
  "decision": "approve",
  "reason": "within weekly funding policy"
}
```

(`decision`: `approve` | `reject`)

**Response `200`**

```json
{
  "mutation_id": "tm_…",
  "status": "approved",
  "approval_id": "ta_…",
  "approved_by": "op_…",
  "expires_at": "2026-08-01T12:15:00Z",
  "request_id": "…"
}
```

### `POST /api/treasury/mutations/:mutation_id/execute`

**Permissions:** `admin` (default); optionally `operator` if policy flag `TW_ALLOW_OPERATOR_EXECUTE_AFTER_APPROVAL=true`  
**CSRF:** **consumed**  
**Body:** `{ "confirm": true, "idempotency_key": "exec-…" }`  
**Audit:** `treasury.mutation.execute` (+ outcome)

Calls engine; returns common success envelope with `journal_batch_id` when applicable.

### `POST /api/treasury/mutations/:mutation_id/cancel`

**Permissions:** requester or admin  
**Audit:** `treasury.mutation.cancel`  
Pending only.

---

## 3. Single-shot low-risk endpoints

### `POST /api/treasury/:treasury_id/reservations`

Create hold (`reserve` or `escrow_reserve`).

**Permissions:** `operator` | `admin`  
**Risk class:** L  
**CSRF:** consumed  
**Audit:** `treasury.mutation.reserve`

**Request**

```json
{
  "kind": "budget",
  "allocation_id": "alloc_…",
  "asset_id": "GBP",
  "amount_minor": 5000,
  "escrow_id": null,
  "idempotency_key": "…",
  "confirm": true
}
```

`kind`: `budget` | `escrow` (maps to engine `reserve` / `escrow_reserve`).

**Response `200`:** success envelope + `resource.reservation`.

### `POST /api/treasury/reservations/:reservation_id/release`

**Permissions:** `operator` | `admin`  
**Audit:** `treasury.mutation.release`  
**Request:** `{ "idempotency_key": "…", "confirm": true }`

### `POST /api/treasury/reservations/:reservation_id/settlement-post`

Treasury journal settlement recognition only.

**Permissions:** `operator` | `admin`  
**Audit:** `treasury.mutation.settlement_post`  
**Request**

```json
{
  "amount_minor": 5000,
  "asset_id": "GBP",
  "evidence_ref": "optional-proto4-or-ops-ref",
  "idempotency_key": "…",
  "confirm": true
}
```

**Must not** mutate PROTO-4 state.

---

## 4. Emergency control endpoints

### `POST /api/treasury/:treasury_id/freeze`

**Permissions:** `admin` only  
**Risk:** E (solo allowed)  
**CSRF:** consumed  
**Audit:** `treasury.mutation.freeze`  
**Request:** `{ "reason": "…", "confirm": true, "idempotency_key": "…" }`

### Unfreeze

**Not single-shot.** Use mutation flow: `operation=unfreeze` (high-risk, dual-control).

---

## 5. High-risk payload shapes (inside `mutations.payload`)

### Fund

```json
{
  "asset_id": "GBP",
  "amount_minor": 100000,
  "funding_kind": "internal_acknowledgment",
  "memo": "…"
}
```

`funding_kind` **MUST** be `internal_acknowledgment` in Phase 21. Values like `bank_deposit` / `custody` are **rejected** until custody gate.

### Allocation create

```json
{
  "agent_id": "agent_…",
  "asset_id": "GBP",
  "ceiling_minor": 50000,
  "initial_minor": 10000,
  "expires_at": "2026-09-01T00:00:00Z"
}
```

### Allocation increase / reduce

```json
{
  "allocation_id": "alloc_…",
  "new_ceiling_minor": 75000,
  "delta_available_minor": 5000
}
```

Phase 21 **MUST** implement via engine-safe journal paths (ADR if engine lacks dedicated method — extend `aether-treasury` without changing accounting principles).

### Refund

```json
{
  "reservation_id_or_settlement_ref": "…",
  "amount_minor": 1000,
  "asset_id": "GBP",
  "reason": "…"
}
```

### Chargeback

```json
{
  "amount_minor": 1000,
  "asset_id": "GBP",
  "treasury_id": "tr_…",
  "reason": "…",
  "prior_batch_id": "optional"
}
```

### Adjustment

```json
{
  "treasury_id": "tr_…",
  "asset_id": "GBP",
  "amount_minor": 100,
  "direction": "to_suspense",
  "reason_code": "RECON_MISMATCH",
  "memo": "…"
}
```

### Treasury create

```json
{
  "parent_treasury_id": "tr_org",
  "kind": "project",
  "name": "Agent Ops Q3"
}
```

### Force release reservation

```json
{
  "reservation_id": "res_…",
  "reason": "TTL exceeded / stuck"
}
```

---

## 6. Permissions summary

| Endpoint | viewer | operator | admin |
|----------|--------|----------|-------|
| POST mutations | ✗ | ✓ | ✓ |
| POST approve | ✗ | ✗ | ✓ (≠ requester) |
| POST execute | ✗ | policy | ✓ |
| POST cancel | ✗ | requester | ✓ |
| POST reservations | ✗ | ✓ | ✓ |
| POST release / settlement-post | ✗ | ✓ | ✓ |
| POST freeze | ✗ | ✗ | ✓ |

---

## 7. Audit events

| Action | When |
|--------|------|
| `treasury.mutation.request` | Pending created |
| `treasury.mutation.approve` | Approved |
| `treasury.mutation.reject` | Rejected |
| `treasury.mutation.cancel` | Cancelled |
| `treasury.mutation.execute` | Execute attempted |
| `treasury.mutation.reserve` | Single-shot reserve |
| `treasury.mutation.release` | Release |
| `treasury.mutation.settlement_post` | Settlement journal |
| `treasury.mutation.freeze` | Freeze |
| `treasury.mutation.denied` | AuthZ / SoD deny |
| `treasury.mutation.audit_gap` | Engine OK, audit append failed |

**Linkage fields (mandatory when known):**  
`request_id`, `actor`, `organisation_id`, `treasury_id`, `allocation_id`, `agent_id`, `reservation_id`, `approval_id`, `mutation_id`, `journal_batch_id`, `idempotency_key`, `payload_hash`, `outcome`, `timestamp`.

`policy_id` / `capability_id` / `operation_id` (Apply): **null** for Phase 21 book mutations.

---

## 8. Explicit non-endpoints (Phase 21)

Do **not** implement:

- `POST /api/treasury/payments/**`
- `POST /api/treasury/custody/**`
- `POST /api/treasury/apply-sync/**`
- Webhooks from banks
- Any route that sets `apply_enabled` or calls `proto0_write`

Read GETs remain as Phase 19.

---

## 9. Freeze statement

API paths, risk split (dual-control vs single-shot), error codes, and audit action names are **frozen** for Phase 20. Additive fields require a Phase 21 note; removals of SoD/CSRF/idempotency requirements are **forbidden** without a new security gate.

**No HTTP handlers are authorised by this document.**
