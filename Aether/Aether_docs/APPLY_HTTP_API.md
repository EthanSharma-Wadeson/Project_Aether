# Apply HTTP API

**Phase:** 12 — Execution surface  
**Status:** Implemented behind `apply_enabled() == false`  
**Crate:** `aether-control-plane`

---

## Overview

Authenticated REST surface for the existing Apply engine:

| Concern | Owner |
|---------|--------|
| AuthN / AuthZ | JWT middleware + RBAC helpers |
| Origin / CSRF | `guard_mutation` |
| Request ID | middleware + write guard |
| Pipeline | `execute_apply` / `prepare_signature` / reconcile primitives |

**Production Apply remains impossible.** All responses include `"apply_enabled": false` and `"protocol_mutated": false` (or equivalent). Execute completes the pipeline and returns `APPLY_EXECUTION_DISABLED`.

Base path: `/api` (router nested under `/api` in process; integration tests hit paths without the prefix when using `api_router` directly).

---

## Authentication

| Requirement | Detail |
|-------------|--------|
| JWT | `Authorization: Bearer <access_token>` on every route |
| Roles | Prepare / status: Operator or Admin. Execute / reconcile: **Admin only** |
| Origin | Checked on mutating routes (`CP_ALLOWED_ORIGINS`) |
| CSRF | `X-CSRF-Token` on mutating routes; **consumed** on execute and reconcile abort |
| Confirm | `confirm: true` **mandatory** on `POST /apply` |
| Request ID | Optional client `X-Request-Id` / middleware; echoed in responses |

---

## Endpoints

### `POST /api/apply/prepare`

Prepare a signed Apply operation (no execution).

**Auth:** Operator or Admin + CSRF (not consumed).

**Request**

```json
{
  "approval_id": "...",
  "dry_run_id": "...",
  "execution_hash": "...",
  "policy_id": "...",
  "policy_version": 1,
  "operation_intent": "FreezeIdentity",
  "operation_id": null,
  "target_agent": "optional-agent-id"
}
```

**Response `200`**

```json
{
  "operation_id": "...",
  "payload_hash": "...",
  "execution_hash": "...",
  "approval_id": "...",
  "signature": { "algorithm": "Ed25519", "value": "...", "status": "prepared", "...": "..." },
  "apply_enabled": false,
  "protocol_mutated": false
}
```

---

### `POST /api/apply`

Run the full Apply execution pipeline.

**Auth:** Admin + CSRF (**consumed**) + `confirm: true`.

**Request**

```json
{
  "operation_id": "...",
  "approval_id": "...",
  "dry_run_id": "...",
  "execution_hash": "...",
  "confirm": true
}
```

**Response `200` (disabled)**

```json
{
  "outcome_code": "APPLY_EXECUTION_DISABLED",
  "code": "APPLY_EXECUTION_DISABLED",
  "apply_enabled": false,
  "protocol_mutated": false,
  "protocol_unchanged": true,
  "phase": "rejected",
  "replay": { "status": "rejected", "...": "..." },
  "request_id": "...",
  "audit_correlation_id": "..."
}
```

Never bypasses validation, re-simulation, or replay.

---

### `GET /api/apply/operations/{operation_id}`

Read-only status of signed operation + replay row.

**Auth:** Operator or Admin.

**Response `200`**

```json
{
  "operation_id": "...",
  "status": "prepared|reserved|executing|rejected|...",
  "signed_operation": { "...": "..." },
  "replay": { "...": "..." },
  "terminal": false,
  "failure_reason": null,
  "apply_enabled": false,
  "protocol_mutated": false
}
```

---

### `GET /api/apply/reconcile`

Admin scan of reserved / executing / stuck (runs timeout sweeps; **no PROTO-0 retry**).

### `GET /api/apply/reconcile/{operation_id}`

Admin status for one replay row.

### `POST /api/apply/reconcile/{operation_id}/abort`

Admin abort (`stuck` / `reserved` / `executing` → `aborted`). CSRF consumed.

**Request:** `{ "reason": "optional" }`

---

## Schemas (summary)

| Field | Type | Notes |
|-------|------|-------|
| `operation_id` | string | Stable Apply operation id |
| `execution_hash` | string (64 hex) | Content-addressed binding |
| `confirm` | bool | Must be `true` on execute |
| `outcome_code` | string | Stable Apply / pipeline code |

---

## Error model

HTTP errors use the Control Plane envelope:

```json
{ "error": "<message>" }
```

Pipeline / signature failures typically map to:

| Status | When |
|--------|------|
| `401` | Missing / invalid JWT |
| `403` | RBAC, CSRF, Origin |
| `400` | Validation, confirm missing, replay duplicate, stale artefacts |
| `404` | Unknown operation / approval |

Stable codes appear inside `error` strings (e.g. `REPLAY_DUPLICATE: ...`, `APPLY_EXECUTION_CONFIRM_REQUIRED: ...`).

Success path while disabled uses `outcome_code: APPLY_EXECUTION_DISABLED`.

---

## Security model

1. JWT required on all Apply routes.
2. Execute and reconcile are admin-only.
3. Mutating routes use `guard_mutation` (Origin + CSRF).
4. Execute consumes CSRF and requires `confirm: true`.
5. Engine path is unchanged: validation → resim → atomic reserve/consume → proto0 boundary.
6. `apply_enabled()` remains hard-false; adapter refuses mutations.
7. Reconcile never retries PROTO-0 and never assumes success after timeout.

---

## Examples

```bash
# Prepare (operator)
curl -X POST "$CP/api/apply/prepare" \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $CSRF" \
  -H "Origin: https://control.example.com" \
  -H "Content-Type: application/json" \
  -d '{"approval_id":"...","dry_run_id":"...","execution_hash":"...","policy_id":"...","policy_version":1,"operation_intent":"FreezeIdentity"}'

# Execute (admin) — returns APPLY_EXECUTION_DISABLED
curl -X POST "$CP/api/apply" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "X-CSRF-Token: $CSRF" \
  -H "Origin: https://control.example.com" \
  -H "Content-Type: application/json" \
  -d '{"operation_id":"...","approval_id":"...","dry_run_id":"...","execution_hash":"...","confirm":true}'
```

---

## Operational notes

### Idempotency

`operation_id` is one-shot via the replay store. Duplicate execute → `REPLAY_DUPLICATE` / `REPLAY_IN_PROGRESS`.

### Retry behaviour

- Safe to retry **prepare** with a new `operation_id` if the first failed before persistence.
- Do **not** retry execute after a successful disabled finalisation for the same `operation_id`.
- Stuck operations: admin abort only; never blind PROTO-0 retry.

### Failure semantics

| Failure | Effect |
|---------|--------|
| Auth / CSRF / confirm | No pipeline side effects |
| Validation / resim fail | No replay row, approval not consumed |
| Duplicate execute | Fail closed; prior terminal state retained |
| Disabled success | Replay `rejected` with `APPLY_EXECUTION_DISABLED`; protocol unchanged |

---

## Related docs

- `APPLY_ENABLEMENT_READINESS_REVIEW.md`
- `APPLY_IMPLEMENTATION_SECURITY_GATE.md`
- `APPLY_PROTOCOL_SPECIFICATION.md`
