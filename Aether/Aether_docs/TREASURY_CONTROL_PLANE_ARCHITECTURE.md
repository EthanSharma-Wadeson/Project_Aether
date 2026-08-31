# Treasury ↔ Control Plane Integration Architecture — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_CONTROL_PLANE_ARCHITECTURE.md` |
| **Phase** | 18 — Treasury Control Plane Integration Design Freeze |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Inputs** | Phase 15–17 treasury pack; Enterprise Governance Console; Control Plane authority model |
| **Protocol impact** | **None** |
| **Accounting model impact** | **None** — `aether-treasury` journal semantics unchanged |
| **Gate decision** | **PASS WITH CONDITIONS** |

---

## Executive Summary

`aether-treasury` is an independent **organisation money-control engine**. The Enterprise Governance Console and Control Plane must expose **treasury intelligence** to operators without becoming a second mint, a PROTO-0 bypass, or a UI-driven mutation surface.

This phase freezes the **integration boundary**:

```text
 Governance Console (UI)
        │  HTTPS / same-origin
        ▼
 Control Plane (authn/z, RBAC, audit correlation)
        │  Treasury Adapter (in-process library boundary)
        ▼
 aether-treasury (journal / hierarchy / allocations / reservations)
```

**First implementation slice (when separately approved):** **read-only** HTTP observation of treasury state, linked to existing audit/policy/agent identifiers.  
**Explicitly out of this freeze’s authorised follow-on:** funding mutations, payment rails, custody, ERP, Apply sync, UI-direct DB writes.

| Question | Answer |
|----------|--------|
| Does integration require protocol changes? | **No** |
| Can CP invent balances? | **No** — only `aether-treasury` posts journals |
| Can UI mutate treasury? | **No** — UI calls CP only; Phase-19+ writes (if any) go through adapter + auth, never from browser to DB |
| Can Treasury bypass PROTO-0? | **No** — INV-T03 preserved |
| Gate | **PASS WITH CONDITIONS** |

**STOP:** This document does not authorise APIs, routes, migrations, frontend pages, or treasury mutations.

---

## 1. Authority Model

### 1.1 Who owns what

| Concern | Owner (source of truth) | Notes |
|---------|-------------------------|-------|
| **Funds (title / available books)** | Organisation via **Treasury journal** | Agents never hold title (INV-T01/T02) |
| **Allocations** | **Treasury** (`allocations` table) | Delegated ceilings, not wallets |
| **Reservations** | **Treasury** (`reservations` + journal) | Holds against available/escrow accounts |
| **Accounting truth** | **`aether-treasury` journal** | Append-only; CP stores projections only if cached (never authoritative if divergent) |
| **Spending permissions (agent authority)** | **PROTO-0 capabilities** ∩ Treasury allocation ∩ CP policy | Intersection (INV-T04/T05) |
| **Policy templates** | Control Plane DB | No protocol effect until Apply (still disabled) |
| **Identity / freeze** | PROTO-0 | CP requests only via Apply path when enabled |
| **Escrow contract state** | PROTO-2 | Distinct from treasury escrow *reservation* |
| **Settlement finality evidence** | PROTO-4 | Soft ≠ hard; treasury posts only on hard/policy (INV-T06/T07) |
| **Operator session / RBAC** | Control Plane | JWT + roles |

### 1.2 Integration invariants (normative)

| ID | Invariant |
|----|-----------|
| **INV-I01** | Control Plane **MUST NOT** insert/update/delete treasury journal lines except by calling `aether-treasury` engine APIs. |
| **INV-I02** | Control Plane **MUST NOT** “create money” by writing `account_balances` or fabricating funding outside the engine. |
| **INV-I03** | The Governance Console **MUST NOT** open the treasury database or call `aether-treasury` directly — only CP HTTP APIs. |
| **INV-I04** | Treasury adapter **MUST NOT** grant/revoke/freeze PROTO-0 capabilities. |
| **INV-I05** | Read APIs **MUST** scope all results by authenticated tenant/`organisation_id` (no cross-tenant leakage). |
| **INV-I06** | Phase-18 follow-on implementation defaults to **read-only** treasury HTTP; write endpoints require a **separate** design+security gate. |
| **INV-I07** | Apply remains independently gated; integration **MUST NOT** enable `apply_enabled`. |
| **INV-I08** | Cached CP denormalisations of treasury data are **hints**; on conflict, **treasury engine** wins for balances/reservations. |
| **INV-I09** | UI role gates are advisory; **server RBAC** is authoritative. |
| **INV-I10** | Every treasury-related CP request appends an audit event with `request_id`. |

### 1.3 Authority verification checklist

| Claim | Status |
|-------|--------|
| Treasury cannot bypass PROTO-0 | **VALIDATED** (INV-I04 / INV-T03) |
| Control Plane cannot create money | **VALIDATED** (INV-I01/I02) |
| Agents cannot own assets | **VALIDATED** (INV-T01/T02) |
| UI cannot directly mutate treasury state | **VALIDATED** (INV-I03/I06) |
| No authority conflicts with Phase 16 | **VALIDATED** |

---

## 2. Integration Architecture

### 2.1 Components

```text
┌─────────────────────────────────────────────────────────────┐
│ Enterprise Governance Console                               │
│  React pages (future): Treasury Dashboard, Journal, …       │
└──────────────────────────┬──────────────────────────────────┘
                           │ Bearer JWT + CSRF (mutations later)
┌──────────────────────────▼──────────────────────────────────┐
│ Control Plane                                               │
│  auth middleware · RBAC · CP audit_log                      │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Treasury Adapter                                         │ │
│  │  • maps AuthContext → organisation_id                    │ │
│  │  • calls aether_treasury::TreasuryEngine (read)          │ │
│  │  • shapes DTOs · enforces RBAC · emits audit             │ │
│  │  • NEVER exposes pool / raw SQL to routes                │ │
│  └──────────────────────────┬─────────────────────────────┘ │
└─────────────────────────────┼───────────────────────────────┘
                              │ Rust API (in-process)
┌─────────────────────────────▼───────────────────────────────┐
│ aether-treasury                                             │
│  separate SQLite (now) → PostgreSQL (pilot)                 │
│  journal, treasuries, allocations, reservations, assets     │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Adapter responsibilities

| Responsibility | In adapter | Out of adapter |
|----------------|------------|----------------|
| Authn context → org tenancy | ✓ | — |
| RBAC allow/deny | ✓ | — |
| Call engine list/get/balance | ✓ | — |
| DTO mapping + pagination | ✓ | — |
| CP `audit_log` correlation | ✓ | — |
| Join display names (agent labels) from protocol index | ✓ read-only | Mutate protocol |
| Post funding / reserve / settle | **Deferred** (future write gate) | — |
| PROTO-0 grant/revoke | — | Forbidden |
| Open browser CORS to treasury DB | — | Forbidden |

### 2.3 Read / write boundaries

| Direction | Phase-18 freeze | Future (separate approval) |
|-----------|-----------------|----------------------------|
| Console → CP | Read treasury routes | Optional write routes + CSRF |
| CP → Treasury engine | **Read-only** methods | Engine mutations via adapter only |
| CP → Protocol | Existing observation | Apply still separately gated |
| Treasury → Protocol | None | None (no bypass) |

### 2.4 Error handling

| Engine / adapter condition | HTTP (design) | Notes |
|----------------------------|---------------|-------|
| Not found | 404 | Unknown treasury/allocation/reservation |
| Forbidden (role/tenant) | 403 | Wrong org or insufficient role |
| Unauthenticated / bad JWT | 401 | Existing CP behaviour |
| Engine validation (if writes later) | 400 + stable `code` | Map `TreasuryError` |
| DB unavailable | 503 | No silent empty success |

Adapter **MUST NOT** convert missing tenant data into another org’s rows.

### 2.5 Transaction boundaries

- **Reads:** single engine queries; no distributed transactions.  
- **Writes (future):** one engine operation = one SQLite/PG transaction inside `aether-treasury`; CP audit append is **best-effort after success** or same logical `request_id` with compensating audit on failure (implementation ADR).  
- CP **MUST NOT** hold an open protocol write transaction while mutating treasury.

### 2.6 PostgreSQL migration path

Aligned with Phase 16 Hybrid recommendation:

1. Keep `aether-treasury` owning schema + migrations.  
2. CP configures `TREASURY_DATABASE_URL` (SQLite lab → Postgres pilot).  
3. Adapter depends only on `TreasuryEngine` trait/API — no SQL in routes.  
4. Partition key remains `organisation_id`.

---

## 3. API Design (read-only surface)

All routes under `/api/treasury/*`, authenticated.  
**Default permission:** see RBAC §4.  
**Audit:** every list/get appends `observation.treasury.*` with `request_id`.

### 3.1 `GET /api/treasury/overview`

| Field | Definition |
|-------|------------|
| **Purpose** | Org-level rollup: treasury counts, assets enabled, available/reserved totals, active allocations/reservations |
| **Request** | Optional `?asset_id=` |
| **Response** | `{ organisation_id, treasuries_count, assets[], totals[{asset_id, available, reserved, escrow_reserved}], active_allocations, active_reservations, as_of }` |
| **Permissions** | viewer+ (balances: see §4) |
| **Audit** | `observation.treasury.overview` |

### 3.2 `GET /api/treasury/treasuries`

| Field | Definition |
|-------|------------|
| **Purpose** | Budget / hierarchy explorer |
| **Request** | `?parent_id=&kind=&status=` |
| **Response** | `{ treasuries: [{ treasury_id, parent_treasury_id, kind, name, status, created_at }] }` |
| **Permissions** | viewer+ |
| **Audit** | `observation.treasury.treasuries.list` |

### 3.3 `GET /api/treasury/treasuries/:id`

| Field | Definition |
|-------|------------|
| **Purpose** | Single treasury + per-asset balances |
| **Request** | path id |
| **Response** | `{ treasury, balances[{asset_id, available, reserved, escrow_reserved}] }` |
| **Permissions** | viewer+; balance fields redacted for roles without balance permission |
| **Audit** | `observation.treasury.treasuries.get` |

### 3.4 `GET /api/treasury/allocations`

| Field | Definition |
|-------|------------|
| **Purpose** | Agent spending allocations |
| **Request** | `?agent_id=&treasury_id=&status=&asset_id=` |
| **Response** | `{ allocations: [{ allocation_id, agent_id, treasury_id, asset_id, ceiling_minor, remaining_minor, status, expires_at }] }` |
| **Permissions** | viewer+ |
| **Audit** | `observation.treasury.allocations.list` |

### 3.5 `GET /api/treasury/allocations/:id`

| Field | Definition |
|-------|------------|
| **Purpose** | Allocation detail + linked open reservations summary |
| **Permissions** | viewer+ |
| **Audit** | `observation.treasury.allocations.get` |

### 3.6 `GET /api/treasury/reservations`

| Field | Definition |
|-------|------------|
| **Purpose** | Reservation timeline / holds |
| **Request** | `?status=&kind=&treasury_id=&allocation_id=&escrow_id=` |
| **Response** | `{ reservations: [{ reservation_id, kind, status, amount_minor, asset_id, escrow_id, created_at, updated_at, … }] }` |
| **Permissions** | viewer+ |
| **Audit** | `observation.treasury.reservations.list` |

### 3.7 `GET /api/treasury/journal`

| Field | Definition |
|-------|------------|
| **Purpose** | Journal explorer (batches/lines) |
| **Request** | `?from=&to=&event_type=&treasury_id=&allocation_id=&request_id=&batch_id=&limit=&cursor=` |
| **Response** | `{ entries: [...], next_cursor? }` — include account_code, debit/credit, linkage ids |
| **Permissions** | **auditor/admin** for full journal; operator may see limited event types (see §4) |
| **Audit** | `observation.treasury.journal.list` |

### 3.8 `GET /api/treasury/settlements`

| Field | Definition |
|-------|------------|
| **Purpose** | Treasury-side settlement **posts** (journal `settlement_post`) correlated to protocol settlement ids when present in metadata |
| **Request** | `?from=&to=&asset_id=` |
| **Response** | `{ posts: [{ batch_id, reservation_id, amount_minor, asset_id, settlement_binding_id?, created_at }] }` |
| **Permissions** | viewer+ (amounts per §4) |
| **Audit** | `observation.treasury.settlements.list` |
| **Note** | Does **not** replace `GET /api/settlements` (PROTO-4 observation) |

### 3.9 `GET /api/treasury/security/metrics`

| Field | Definition |
|-------|------------|
| **Purpose** | Security-oriented counters: frozen treasuries, expired allocations, stuck/active reservation age, recent adjustment/chargeback counts |
| **Response** | `{ frozen_treasuries, expired_allocations, active_reservations, aged_reservations, recent_adjustments, recent_chargebacks, as_of }` |
| **Permissions** | **admin** (Security Officer persona) |
| **Audit** | `observation.treasury.security.metrics` |

### 3.10 Explicitly not in Phase-18 API surface

`POST/PUT/DELETE` funding, allocate, reserve, release, settle, refund, chargeback, adjust · payment webhooks · custody · ERP sync · Apply triggers.

---

## 4. RBAC Model

Backend roles remain: `admin` | `operator` | `viewer`.  
Personas map as in the Governance Console.

| Persona | Backend role | Balances | Allocations | Reservations | Full journal | Security metrics |
|---------|--------------|----------|-------------|--------------|--------------|------------------|
| Administrator | `admin` | ✓ | ✓ | ✓ | ✓ | ✓ |
| Security Officer | `admin` | ✓ | ✓ | ✓ | ✓ | ✓ |
| Operator | `operator` | ✓ (org) | ✓ | ✓ | Limited* | ✗ |
| Auditor | `viewer` | ✓ | ✓ | ✓ | ✓ | ✗ (use Security Centre protocol audit) |
| Viewer | `viewer` | Redacted** | ✓ status/ceiling | ✓ status | ✗ | ✗ |

\*Operator journal: `funding`, `allocation`, `reservation`, `release`, `escrow_reservation` only — not adjustments/chargebacks unless admin.  
\*\*Viewer: see hierarchy and allocation **status**; omit or mask `available`/`reserved` minor units unless product policy later elevates Auditor distinctly (today Auditor≡viewer → **Auditors get balances**; pure “Viewer” product label may use the same role — **CONDITION C2**: if product needs split, add role or claim without protocol change).

**Decision for freeze:** `viewer` receives balances and full journal (Auditor/Viewer combined as today). Masking is **DEFERRED** until a distinct auditor role exists.

---

## 5. Data Model Boundary

| Data | System of record | CP may | Console may |
|------|------------------|--------|-------------|
| Treasury balances / journal / reservations / allocations / assets | **aether-treasury** | Read via adapter; never raw SQL from routes | Read via CP API |
| Operators, sessions, CSRF, CP audit_log | **Control Plane** | R/W | Login / display |
| Policy templates | **Control Plane** | R/W (existing) | Existing UI |
| Capabilities, identity, freeze | **PROTO-0** | Observe; mutate only via Apply | Observe |
| Escrow lifecycle | **PROTO-2** | Observe | Observe |
| Settlement bindings | **PROTO-4** | Observe | Observe |
| Apply replay / approvals | **Control Plane Apply tables** | Existing | Existing |

**Settlements:** Protocol settlement list ≠ treasury settlement posts; UI should label both and link by id when metadata present.

---

## 6. Security Review

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| TR-CP-001 | Unauthorised balance visibility | **VALIDATED** | JWT + RBAC; tenant scope INV-I05 |
| TR-CP-002 | Tenant data leakage | **VALIDATED** | Adapter forces `organisation_id` from auth, not client body |
| TR-CP-003 | Fake treasury data via CP DB write | **VALIDATED** | INV-I01/I02; no CP writers to journal |
| TR-CP-004 | Journal tampering through API | **VALIDATED** (read-only slice); writes later need immutability tests | Engine append-only |
| TR-CP-005 | Privilege escalation (viewer→admin via treasury route) | **VALIDATED** | Server-side role checks on each route |
| TR-CP-006 | Confused deputy (CP uses treasury pool as admin for all orgs) | **VALIDATED** | Adapter binds engine calls to caller org only |
| TR-CP-007 | Stale projections if CP caches balances | **OPEN** | Prefer live engine reads; if cache, TTL + INV-I08 |
| TR-CP-008 | Race on future writes through CP | **DEFERRED** | Engine already serialises reserves; CP must not dual-write |
| TR-CP-009 | UI XSS exfiltrating journal | **OPEN** | Existing CP web controls; CSP hardening backlog |
| TR-CP-010 | IDOR on `:id` across orgs | **VALIDATED** | Adapter verifies resource.org == caller.org |

---

## 7. Audit Integration

### 7.1 Correlation fields

Every treasury observation/mutation (future) carries:

| Field | Source |
|-------|--------|
| `request_id` | CP generates per HTTP request |
| `organisation_id` | Auth context |
| `operator_id` / role | JWT |
| `treasury_id` / `allocation_id` / `reservation_id` | Resource |
| `agent_id` | Allocation / filter |
| `policy_id` | Optional query join when allocation metadata references policy |
| `capability_id` | Optional display join from protocol index (not stored as authority in treasury) |
| `escrow_id` / `settlement_binding_id` | Reservation / journal metadata when present |
| `operation_id` | Only if future Apply-linked write; null in read-only slice |

### 7.2 Reconstruction path

```text
CP audit_log (request_id, operator, action)
    → treasury journal_entries (request_id / batch_id)
    → reservation_id / allocation_id / treasury_id
    → optional PROTO-2 escrow_id / PROTO-4 binding_id (observation)
    → optional policy_id from CP policies (display)
    → optional capability snapshot from protocol index (display)
```

Operators can reconstruct **who looked at or (later) changed what**, and **which books moved**, without treating CP audit as the ledger.

---

## 8. Frontend Architecture (design only)

Navigation addition (future): **Treasury** section under Governance/Finance.

| Page | Route (design) | Data | Permissions |
|------|----------------|------|-------------|
| **Treasury Dashboard** | `/treasury` | overview + security sparkline | viewer+ (metrics admin) |
| **Budget Explorer** | `/treasury/budgets` | treasuries hierarchy + balances | viewer+ |
| **Allocation View** | `/treasury/allocations` | allocations table + agent link | viewer+ |
| **Reservation Timeline** | `/treasury/reservations` | reservations filtered | viewer+ |
| **Journal Explorer** | `/treasury/journal` | journal API + export | viewer (full) / per §4 |
| **Treasury Security Centre** | `/treasury/security` | security metrics | admin |

### 8.1 Components (reuse console system)

`PageHeader`, `Metric`, `DataTable`, `StatusBadge`, filters, CSV/JSON export (journal) — same patterns as Audit Explorer.  
Banner: **“Observation only — no funding or custody in this release.”**

### 8.2 Data requirements

TanStack Query keys: `['treasury', 'overview'|…]`.  
No direct SQLite. No client-side balance arithmetic as authority.

---

## 9. Implementation Plan (not authorised)

Ordered slices **after** explicit approval:

| Slice | Scope |
|-------|-------|
| **A** | CP dependency on `aether-treasury`; adapter; config `TREASURY_DATABASE_URL`; seed/lab wiring |
| **B** | Read-only routes §3 + audit actions + tests (RBAC, IDOR, tenant) |
| **C** | Console pages §8 (read-only) |
| **D** | (Separate gate) Write APIs + CSRF + dual-control rules |
| **E** | (Separate gate) Apply allocation↔capability sync |
| **F** | Postgres + custody/ERP — later |

**Must not** in slices A–C: enable Apply, modify `aether-core`, change treasury accounting semantics, add payment providers.

---

## 10. Remaining Risks

| Risk | Disposition |
|------|-------------|
| Viewer/Auditor role conflation for balance sensitivity | Condition C2 / DEFERRED split |
| Stale cache if introduced carelessly | TR-CP-007 OPEN |
| Operators mistaking treasury posts for PROTO-4 finality | UX labelling requirement |
| Lab SQLite multi-process sharing | Ops note; Postgres for pilot |
| Future write APIs expanding attack surface | Separate security gate |

---

## 11. Gate Decision

### Decision: **PASS WITH CONDITIONS**

Integration boundary is frozen, authority-consistent, and implementable **without** protocol or accounting-model changes. **Read-only** CP observation is the approved *design* target for the next implementation phase — still requiring a **separate implementation approval**.

### Conditions

| ID | Condition |
|----|-----------|
| **C1** | First implementation slice is **read-only**; no treasury mutation HTTP without a new design/security gate. |
| **C2** | If product requires Viewer without balance visibility, introduce an explicit role/claim before shipping masking — do not overload silently. |
| **C3** | Adapter must enforce org tenancy from auth context (no client-supplied org override). |
| **C4** | Do not enable Apply or modify PROTO-* / `aether-core` as part of treasury CP integration. |
| **C5** | UI must label soft protocol settlement vs treasury settlement posts distinctly. |

### FAIL criteria (none triggered)

- Requiring protocol changes  
- Allowing UI→DB treasury writes  
- Letting CP mint balances outside the engine  
- Bundling custody/payments into the integration freeze  

---

## 12. Checklist

- [x] Integration boundary frozen  
- [x] No authority conflicts  
- [x] No protocol changes required  
- [x] Treasury remains separate accounting authority  
- [x] Security model documented  
- [x] API surface defined (read-only)  
- [x] Frontend scope defined  

**STOP.** Implementation requires separate approval after this design freeze.
