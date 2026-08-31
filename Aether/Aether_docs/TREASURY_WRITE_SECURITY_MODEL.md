# Treasury Write Security Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_WRITE_SECURITY_MODEL.md` |
| **Phase** | 20 — Treasury Write Security Gate |
| **Status** | **DESIGN FROZEN** — no mutation implementation authorised |
| **Date** | 2026-08-01 |
| **Inputs** | [TREASURY_SECURITY_MODEL.md](TREASURY_SECURITY_MODEL.md), [TREASURY_CONTROL_PLANE_ARCHITECTURE.md](TREASURY_CONTROL_PLANE_ARCHITECTURE.md), [TREASURY_ACCOUNTING_MODEL.md](TREASURY_ACCOUNTING_MODEL.md), [TREASURY_READ_OBSERVATORY_IMPLEMENTATION.md](TREASURY_READ_OBSERVATORY_IMPLEMENTATION.md) |
| **Protocol impact** | **None** |
| **Accounting model impact** | **None** — journal semantics unchanged |
| **Apply impact** | **None** — `apply_enabled` remains false |

---

## Executive Summary

This document freezes the **security boundary** for future Control Plane → `aether-treasury` **write** operations (Slice D).

```text
Console (request / approve UX — future)
        │  JWT + CSRF + Origin
        ▼
Control Plane (authn/z · dual-control · audit · adapter)
        │  one authorised engine call per mutation
        ▼
aether-treasury (journal / allocations / reservations — accounting authority)
```

| Claim | Rule |
|-------|------|
| Who owns funds? | **Organisation** (title never agents) |
| Who owns accounting truth? | **`aether-treasury` journal** |
| Who authorises humans? | **Control Plane** (JWT, RBAC, approvals, CSRF) |
| Who may mutate PROTO-0? | **Nobody via treasury writes** — Apply remains separately gated |
| Phase 20 authorises code? | **No** |

**STOP:** No write routes, console mutation UI, Apply sync, custody/rails, or protocol changes are authorised by this document.

---

## 1. Authority Model

### 1.1 Ownership

| Concern | Owner | Notes |
|---------|-------|-------|
| Funds (legal / book title) | **Organisation** | Via funded treasury nodes |
| Budgets / hierarchy | **Organisation** (ops via CP) | Control state; money moves only via journal |
| Allocations (ceilings) | **Organisation** | Delegated spend authority — not agent equity |
| Accounting truth | **Treasury engine journal** | Append-only; balances are projections |
| Governance of operators | **Control Plane** | AuthN, RBAC, dual-control, CP audit |
| Agent identity / capabilities | **PROTO-0** | Observed only; never mutated by treasury adapter |
| Escrow / settlement contracts | **PROTO-2 / PROTO-4** | Treasury posts *accounting* around evidence; does not replace protocol |

### 1.2 Actors

| Actor | May do | Must not |
|-------|--------|----------|
| **Organisation** (principal) | Own funds; set risk appetite; recover unused | — |
| **Administrator / Security Officer** (`admin`) | Approve high-risk; freeze; execute dual-control completions | Self-approve own high-risk request (INV-TW08) |
| **Operator** (`operator`) | Request mutations; execute low-risk holds/releases within policy | Solo fund, adjust, chargeback, or unfreeze |
| **Auditor / Viewer** (`viewer`) | Read (per Phase 18/19 RBAC) | Any treasury write |
| **Agents** | Spend only under allocation ∩ capability ∩ policy (future) | Own assets; call treasury APIs; hold custody credentials |
| **Treasury adapter** | Map auth → org; call engine; emit audit | Invent balances; raw SQL in routes; PROTO-0 grant/revoke/freeze |
| **Browser / Console** | Call CP HTTP only | Open treasury DB; call engine directly |

### 1.3 Write invariants (INV-TW*)

| ID | Invariant |
|----|-----------|
| **INV-TW01** | Agents **MUST NOT** own assets or appear as balance-sheet equity holders. |
| **INV-TW02** | All money movement **MUST** be posted exclusively by `aether-treasury` engine APIs inside one engine transaction per mutation. |
| **INV-TW03** | Control Plane **MUST NOT** insert/update/delete journal lines, balances, or allocations except by calling the engine. |
| **INV-TW04** | Console / browser **MUST NOT** open the treasury database or call the engine directly. |
| **INV-TW05** | Treasury write paths **MUST NOT** grant, revoke, or freeze PROTO-0 capabilities or identities. |
| **INV-TW06** | Every write **MUST** be scoped to the authenticated deployment org (`organisation_id`); cross-org ids → **404** (no existence oracle). |
| **INV-TW07** | Viewers **MUST NOT** perform treasury mutations (`reject_viewer_writes`). |
| **INV-TW08** | High-risk operations **MUST** enforce separation of duties: **requester ≠ approver** (distinct `operator_id`). |
| **INV-TW09** | Successful engine mutation **MUST** append a CP audit event with reconstructible linkage (§6 / API spec). |
| **INV-TW10** | Mutations **MUST** carry client `idempotency_key`; duplicates return the original successful outcome (no double journal). |
| **INV-TW11** | CSRF + Origin checks **MUST** apply to all treasury mutating HTTP methods (align Apply `guard_mutation`). |
| **INV-TW12** | CP **MUST NOT** hold an open protocol write transaction while mutating treasury. |
| **INV-TW13** | `apply_enabled` **MUST** remain false; treasury writes **MUST NOT** enable Apply or sync allocations to capabilities. |
| **INV-TW14** | Lab/internal **Funding** acknowledges book balances only — **MUST NOT** claim bank/crypto custody movement occurred. |
| **INV-TW15** | Corrections **MUST** use compensating journal events (refund / chargeback / adjustment); never update/delete posted lines. |
| **INV-TW16** | Frozen or closed treasuries **MUST** reject funding, new allocations, and new reservations (engine `TreasuryFrozen` / `TreasuryClosed`). |
| **INV-TW17** | UI role gates are advisory; **server RBAC + dual-control store** are authoritative. |

---

## 2. Write Operations Catalogue

Risk classes:

| Class | Meaning |
|-------|---------|
| **L** Low | Operator may execute with CSRF + idempotency (no second human) |
| **H** High | Dual-control required (request → approve → execute) |
| **E** Emergency | Admin may solo-execute freeze; unfreeze remains **H** |
| **X** Excluded | Not in Phase 21 write surface (custody/rails/Apply) |

Thresholds (`TW_THRESHOLD_*`) are deployment config; defaults below are **design defaults**, not code.

| Operation | Engine API (existing) | Risk | Default dual-control | Notes |
|-----------|----------------------|------|----------------------|-------|
| **Funding** (internal book) | `fund` | **H** | Always | INV-TW14; not a bank deposit |
| **Allocation create** | `create_allocation` | **H** if ceiling ≥ threshold; else **L** with admin-or-operator | Ceiling ≥ `TW_THRESHOLD_ALLOCATION_MINOR` → H | Initial reserve amount included |
| **Allocation increase** | *(Phase 21 may add engine helper or fund+ceiling update ADR)* | **H** | Always above threshold | Must not invent money outside journal |
| **Allocation reduction / close** | `close_allocation` + optional release | **L**/`H` | H if remaining reserved & recovery to parent | Cannot strand escrow-reserved without release path |
| **Allocation expire** | `expire_allocation_if_needed` | **L** (system/admin) | No | Sweeper / explicit ops |
| **Reservation create** | `reserve` / `escrow_reserve` | **L** | No | Capped by allocation remaining + available |
| **Reservation release** | `release_reservation` | **L** | No | |
| **Settlement posting** | `settlement_post` | **L** admin/operator | No | Journal only; requires active reservation; **not** PROTO-4 mutation |
| **Refund** | `refund` | **H** | Always above `TW_THRESHOLD_REFUND_MINOR`; always H if no prior settlement binding | Compensating entries |
| **Chargeback** | `chargeback` | **H** | Always | Suspense path |
| **Adjustment** | `adjustment_to_suspense` | **H** | Always | Reason code mandatory |
| **Emergency freeze** | `freeze_treasury` | **E** | Solo admin allowed | Audit `treasury.freeze`; no journal delete |
| **Unfreeze** | *(status → active; engine/ADR)* | **H** | Always | Break-glass; distinct from freezer preferred |
| **Close treasury** | `close_treasury` | **H** | Always | No silent balance wipe |
| **Create treasury node** | `create_treasury` | **H** | Admin approve | Hierarchy only; zero balances until fund |
| **Register asset** | `register_asset` | **H** | Always | Lab allowlist; not a rail |
| **Recovery — stuck reservation** | release + audit | **H** if force | Admin dual-control for force-break | Prefer TTL sweeper (TR-THR-003) |
| **Recovery — external payout** | — | **X** | — | Custody / rails out of scope |
| **Apply ↔ allocation sync** | — | **X** | — | Separate Apply gate |
| **Bank/crypto deposit confirm** | — | **X** | — | Needs custody adapter + C3 + TR-THR-062 |

### 2.1 Per-operation security matrix

For each allowed operation:

| Field | Requirement |
|-------|-------------|
| **Allowed roles (request)** | See [RBAC](#3-rbac-and-dual-control) |
| **Approval** | Class H/E rules; approval TTL; requester ≠ approver |
| **Execute role** | Operator+ for L; Admin (or designated executor) for H after approval |
| **Audit** | `treasury.mutation.<op>` + linkage fields |
| **Idempotency** | Required header/body key; engine + CP approval consume |
| **Failure** | No journal on validation/auth failure; see Failure Model |

Detailed schemas: [TREASURY_WRITE_API_SPEC.md](TREASURY_WRITE_API_SPEC.md).

---

## 3. RBAC and Dual Control

### 3.1 Backend roles (unchanged)

| Persona | Backend role | Write? |
|---------|--------------|--------|
| Administrator | `admin` | Yes (approve / execute high-risk; freeze) |
| Security Officer | `admin` | Yes (freeze; approve unfreeze / recovery) |
| Operator | `operator` | Yes (request; execute low-risk) |
| Auditor | `viewer` | **No** |
| Viewer | `viewer` | **No** |

**No new role required for Phase 21.** Optional future claims (`finance_approver`) may refine SoD without protocol change — Condition **W-C2**.

### 3.2 Separation of duties

```text
[Request]  operator_id = A   →  treasury_mutation_request (pending)
[Approve]  operator_id = B≠A →  approval (approved, expires_at)
[Execute]  operator_id = A|B|C (policy) → CSRF consume → engine → audit
```

| Rule | Detail |
|------|--------|
| **SoD-1** | High-risk: approver `operator_id` ≠ requester `operator_id`. |
| **SoD-2** | Same JWT/session cannot approve its own request. |
| **SoD-3** | Viewer cannot request, approve, or execute. |
| **SoD-4** | Emergency freeze: admin solo OK; recorded as `break_glass=false` unless fleet-wide (fleet-wide freeze → dual-control **OPEN residual** TR-TW-041). |
| **SoD-5** | Unfreeze / recovery to available / adjustments: dual-control mandatory. |
| **SoD-6** | Execute binds `approval_id` + request payload hash; mutation of payload after approve → reject. |

### 3.3 Who may request / approve / execute / review

| Operation class | Request | Approve | Execute | Review (read) |
|-----------------|---------|---------|---------|---------------|
| Low-risk (reserve/release/settle post) | operator, admin | — | same | viewer+ |
| High-risk (fund, adjust, chargeback, …) | operator or admin | **other** admin (or admin if requester was operator) | admin (default) or operator if policy allows post-approval | viewer+ / admin journal |
| Emergency freeze | admin | — | admin | admin |
| Unfreeze / force recovery | admin | **other** admin | admin | admin |

---

## 4. Security Controls (normative for Phase 21)

| Control | Requirement |
|---------|-------------|
| **JWT** | Valid access token; required claims `iss`/`aud`/`typ`/`sub`/`role`; stale tokens → 401 |
| **RBAC** | Server-side per route; never trust UI |
| **CSRF** | `X-CSRF-Token` on all mutating methods; **consume** on execute (and on single-shot L mutations) |
| **Origin** | `CP_ALLOWED_ORIGINS` on mutations |
| **Idempotency** | `Idempotency-Key` (or body field) required; unique per org; store outcome |
| **Replay** | Consumed approval cannot re-execute; reserved CSRF one-shot on execute |
| **Txn boundary** | One engine mutation = one DB transaction inside `aether-treasury` |
| **CP audit ordering** | Prefer: engine success → append CP audit with same `request_id`; on audit failure → compensating `treasury.mutation.audit_gap` alert (ADR in Phase 21) |
| **Optimistic concurrency** | Engine serialises per treasury row / reservation row; HTTP may pass `expected_status` / `allocation_version` if added — optional Condition **W-C3** |
| **Approval expiry** | Default ≤ 15 minutes (config); expired → no execute |
| **confirm** | High-risk execute body requires `confirm: true` (align Apply) |
| **Emergency** | Freeze allowed without dual-control; all freezes page Security metrics |

---

## 5. Failure Model (summary)

| Failure | Behaviour |
|---------|-----------|
| AuthN / CSRF / Origin | 401/403; **no** engine call |
| RBAC / SoD violation | 403; audit `treasury.mutation.denied` |
| Validation / insufficient funds | 400 + stable `code`; no journal |
| Duplicate idempotency | 200 with original batch/resource (or 409 policy — **prefer return original**) |
| Approval expired / mismatched hash | 409/400; no engine call |
| Stale allocation / frozen treasury | Map `TreasuryError` → 409/400 |
| Engine DB failure mid-txn | Rollback engine txn; 503; no success audit |
| Adapter panic after engine success | Compensating audit gap event; operator reconcile via journal `request_id` |
| Operator cancel pending request | Mark cancelled; cannot execute |

Full detail: [TREASURY_WRITE_SECURITY_GATE.md](TREASURY_WRITE_SECURITY_GATE.md) § Failure Model companion + API error table.

---

## 6. Audit Reconstruction

Every successful (and denied high-risk) mutation **MUST** be reconstructible from CP audit + treasury journal using:

| Field | Required |
|-------|----------|
| `request_id` | Yes |
| `actor` (`operator_id`, role) | Yes |
| `organisation_id` | Yes |
| `treasury_id` | When applicable |
| `allocation_id` / `reservation_id` | When applicable |
| `agent_id` | When allocation-bound |
| `policy_id` / `policy_version` | When policy-gated (optional until policies bind) |
| `capability_id` | Only if future spend path checks PROTO-0 — null for Phase 21 book ops |
| `approval_id` | Yes for class H |
| `journal_batch_id` | Yes on engine success |
| `idempotency_key` | Yes |
| `timestamp` | Yes |
| `outcome` | `success` \| `denied` \| `failed` \| `duplicate` |

---

## 7. Implementation Boundaries

### In scope for a future Phase 21 (after separate approval)

- CP write routes + adapter methods calling existing engine APIs  
- Dual-control request/approve store (CP DB)  
- CSRF/idempotency/audit wiring  
- Console **request/approve** UX for allowed ops  
- Tests: SoD, IDOR, CSRF, idempotency, freeze, insufficient funds  

### Explicitly out of scope (still)

- Apply enablement or Apply↔allocation sync  
- Custody, banking, payment rails, ERP connectors  
- `aether-core` / PROTO changes  
- Changing journal double-entry semantics  
- Multi-tenant org claims / Postgres mandate  
- Hash-chained journal (remains **target**, not Phase 21 blocker if append-only + tests hold)  

---

## 8. Relationship to prior freezes

| Doc | Relationship |
|-----|----------------|
| Phase 15 security / threat | Extended with INV-TW* and write-specific threats TR-TW-* |
| Phase 18 INV-I06 | Satisfied: this gate **is** the separate write security gate |
| Phase 19 | Read paths unchanged; writes additive |

---

## 9. Freeze statement

Authority model, dual-control rules, control requirements, and operation risk classes in this document are **frozen** for Phase 20. Status changes require a version note in the Phase 21 gate — not silent drift.

**This document does not authorise implementation.**
