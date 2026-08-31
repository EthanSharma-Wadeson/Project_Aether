# Treasury Write Implementation — Phase 21

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_WRITE_IMPLEMENTATION.md` |
| **Phase** | 21 — Treasury Write Façade |
| **Date** | 2026-08-01 |
| **Gate** | [TREASURY_WRITE_SECURITY_GATE.md](TREASURY_WRITE_SECURITY_GATE.md) — PASS WITH CONDITIONS |
| **Apply** | Remains **disabled** |
| **Protocol** | Unchanged (`aether-core` not modified) |

---

## Summary

Control Plane exposes an **internal ledger governance** write façade over `aether-treasury`. All mutations go through the adapter/engine. Funding / custody / payment rails / Apply sync are **not** implemented. UI and API copy state: **“Internal ledger governance only. No external money movement.”**

```text
Console (/treasury/actions)
   → CP HTTP (JWT + CSRF + RBAC + SoD)
   → TreasuryWriteService
   → TreasuryAdapter.engine()
   → aether-treasury (append-only journal)
```

---

## Files created / updated

### Created

| Path | Role |
|------|------|
| `control_plane/src/treasury/write/mod.rs` | Module root |
| `control_plane/src/treasury/write/models.rs` | Operations, DTOs, ledger notice |
| `control_plane/src/treasury/write/errors.rs` | HTTP error envelope + codes |
| `control_plane/src/treasury/write/approval.rs` | Mutation request / approval store |
| `control_plane/src/treasury/write/audit.rs` | `treasury_mutation_audit` + CP audit |
| `control_plane/src/treasury/write/validation.rs` | Payload validation |
| `control_plane/src/treasury/write/service.rs` | Orchestration + engine calls |
| `control_plane/tests/phase21_treasury_write_tests.rs` | Security / SoD / CSRF / idempotency tests |
| `Aether_docs/TREASURY_WRITE_IMPLEMENTATION.md` | This document |

### Updated

| Path | Change |
|------|--------|
| `treasury/src/engine/mod.rs` | `unfreeze_treasury`, `increase_allocation`, `decrease_allocation` |
| `treasury/src/engine/queries.rs` | `get_allocation`, `get_reservation` |
| `control_plane/src/db/mod.rs` | Tables: `treasury_mutation_requests`, `treasury_mutation_approvals`, `treasury_mutation_audit` |
| `control_plane/src/treasury/mod.rs` | Export write module |
| `control_plane/src/routes/mod.rs` | Register write routes; `AppState.treasury_write` |
| `control_plane/src/routes/treasury.rs` | Write handlers |
| `control_plane/src/lib.rs` | Wire `TreasuryWriteService` |
| `control_plane/frontend/src/api/client.ts` | Write API client |
| `control_plane/frontend/src/pages/TreasuryPages.tsx` | Treasury Actions UI |
| Integration tests | `AppState` includes `treasury_write` |

---

## Architecture

### Dual-control (high-risk)

`allocation_create` | `allocation_increase` | `allocation_decrease` | `refund` | `chargeback` | `adjustment` | `unfreeze`

```text
POST /api/treasury/mutations
POST /api/treasury/mutations/:id/approve   (admin ≠ requester)
POST /api/treasury/mutations/:id/execute   (admin, CSRF consumed, confirm:true)
```

### Single-shot (low-risk / emergency)

| Op | Route | Role |
|----|-------|------|
| Reserve | `POST /api/treasury/:id/reservations` | operator+ |
| Release | `POST /api/treasury/reservations/:id/release` | operator+ |
| Settlement post | `POST /api/treasury/reservations/:id/settlement-post` | operator+ |
| Freeze | `POST /api/treasury/:id/freeze` | admin |

### Persistence (CP DB — governance metadata only)

- `treasury_mutation_requests` — request lifecycle + idempotency
- `treasury_mutation_approvals` — approval records (SoD)
- `treasury_mutation_audit` — reconstructible mutation timeline

Ledger lines remain exclusively in **aether-treasury** SQLite.

---

## Security controls

| Control | Implementation |
|---------|----------------|
| JWT | Existing auth middleware |
| RBAC | `require_operator` / `require_admin` / `reject_viewer_writes` |
| CSRF | `guard_mutation` (consume on execute / single-shot) |
| SoD | Approver `operator_id` ≠ requester |
| Idempotency | Unique `(organisation_id, idempotency_key)` |
| Tenant | Deployment org binding; unknown/cross-org → 404 |
| Concurrent approve/execute | Conditional `UPDATE … WHERE status = …` |
| Confirm | `confirm: true` on execute / single-shot |

---

## Audit model

Every request/approve/execute/single-shot appends:

1. Row in `treasury_mutation_audit` (`request_id`, actor, operation, journal_reference, outcome)
2. CP `audit_log` action `treasury.mutation.*`

Reconstruction: timeline API `GET /api/treasury/mutations/timeline` + journal by `batch_id` / `request_id`.

---

## Frontend

`/treasury/actions` — create allocation / unfreeze requests, approval queue, execute queue, history, timeline. Banner: internal ledger only.

---

## Known limitations / residuals

| Item | Notes |
|------|-------|
| No `fund` HTTP | Phase 21 scope excluded external/internal funding API (lab bootstrap still seeds via adapter) |
| Reservation TTL sweeper | TR-TW-060 — force paths exist; sweeper job still backlog |
| Fleet freeze dual-control | TR-TW-032 — single treasury freeze is admin solo |
| MFA / HSM | Lab residual |
| Apply sync | Explicitly out of scope |
| Hash-chained journal | Still target; append-only + tests hold |

---

## Tests

`cargo test --test phase21_treasury_write_tests` covers RBAC, CSRF, SoD, idempotency, dual-control execute, freeze/unfreeze, concurrent approval, cross-org 404, audit timeline.

---

## STOP

No custody, rails, Apply enablement, or `aether-core` changes in this phase.
