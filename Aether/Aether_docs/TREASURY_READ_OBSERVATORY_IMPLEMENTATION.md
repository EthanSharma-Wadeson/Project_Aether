# Treasury Read Observatory — Phase 19 Implementation

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_READ_OBSERVATORY_IMPLEMENTATION.md` |
| **Phase** | 19 — Treasury Read-Only Integration |
| **Date** | 2026-08-01 |
| **Design basis** | [TREASURY_CONTROL_PLANE_ARCHITECTURE.md](TREASURY_CONTROL_PLANE_ARCHITECTURE.md) |

---

## Implementation summary

Control Plane exposes **read-only** treasury observation via a `TreasuryAdapter` over `aether-treasury`. The Enterprise Governance Console renders dashboard, budget hierarchy, reservations, journal, and admin security views.

- No treasury mutation HTTP methods  
- No Apply / protocol / `aether-core` changes  
- Accounting model unchanged  
- Tenancy: deployment-bound `CP_TREASURY_ORG_ID` (default `org-default`)

---

## Files changed

### Backend

| Path | Role |
|------|------|
| `treasury/src/engine/queries.rs` | Read queries + integrity check |
| `control_plane/src/treasury/*` | Adapter, DTOs, errors, mappers |
| `control_plane/src/routes/treasury.rs` | HTTP handlers |
| `control_plane/src/routes/mod.rs` | Route registration + `AppState.treasury` |
| `control_plane/src/config/mod.rs` | `treasury_db_path`, org id, bootstrap flag |
| `control_plane/src/lib.rs` | Adapter bootstrap on run |
| `control_plane/Cargo.toml` | `aether-treasury` dependency |
| `control_plane/tests/phase19_treasury_read_tests.rs` | Security / integrity tests |

### Frontend

| Path | Role |
|------|------|
| `frontend/src/pages/TreasuryPages.tsx` | Dashboard, budgets, reservations, journal, security, node |
| `frontend/src/api/client.ts` | Typed treasury GETs |
| `frontend/src/App.tsx` / `Layout.tsx` | Routes + nav |

---

## API routes

All require JWT. Audit actions `observation.treasury.*`.

| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/treasury` | Org overview + totals |
| GET | `/api/treasury/:id` | Node + balances |
| GET | `/api/treasury/:id/allocations` | Allocations |
| GET | `/api/treasury/:id/reservations` | Reservations (`read_only: true`) |
| GET | `/api/treasury/:id/journal` | Immutable lines; operator filters adjustments/chargebacks |
| GET | `/api/treasury/:id/settlements` | Treasury `settlement_post` rows labelled `treasury_journal` |
| GET | `/api/treasury/:id/security` | **Admin only** metrics |

No POST/PUT/DELETE treasury routes.

---

## Security model

- JWT auth middleware on `/api/*`  
- Org tenancy: resources whose `organisation_id` ≠ caller binding → **404**  
- Security view → admin  
- UI cannot open treasury DB  
- CP cannot invent balances; reads engine only  

---

## Tests

`phase19_treasury_read_tests.rs`: viewer/operator/admin list · journal · admin security / operator forbidden · missing/invalid JWT · cross-tenant 404 · settlement labelling · journal↔balance integrity · mutation methods rejected · reservations read_only.

---

## Known limitations

- Single org binding per CP deployment (not per-operator org claims)  
- Demo bootstrap seeds sample funds (lab)  
- Operator journal redaction is event-type filter only  
- Viewer/Auditor still share `viewer` role  

---

## Remaining roadmap

Write APIs (separate gate) · Console funding UX · Postgres · Apply↔allocation sync · custody/ERP · multi-tenant operator org claims  

**STOP for writes until separately approved.**
