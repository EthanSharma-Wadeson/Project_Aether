# Treasury Open Questions — Design Freeze + Phase 16 Resolutions

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_OPEN_QUESTIONS.md` |
| **Phase** | 15 design · **16 review resolutions** |
| **Status** | **BLOCKING questions RESOLVED** (Phase 16) — implementation still not authorised |
| **Date** | 2026-08-01 |
| **Related** | [TREASURY_ARCHITECTURE.md](TREASURY_ARCHITECTURE.md), [TREASURY_ARCHITECTURE_REVIEW.md](TREASURY_ARCHITECTURE_REVIEW.md) |

---

## How to use

| Tag | Meaning |
|-----|---------|
| **BLOCKING** | Was blocking implementation design — now resolved in Phase 16 |
| **IMPORTANT** | Resolved for v1 scope or held as go-to-market condition |
| **DEFER** | Explicitly deferred |

No question authorises code.

Canonical review: [TREASURY_ARCHITECTURE_REVIEW.md](TREASURY_ARCHITECTURE_REVIEW.md).

---

## Decision log

| ID | Decision | Date | Owner |
|----|----------|------|-------|
| Q1 | Hybrid persistence (CP façade + Treasury data plane; separate logical DB) | 2026-08-01 | Phase 16 review |
| Q2 | Treasury journal authorises spend; bank/custodian = custody; ERP = statutory | 2026-08-01 | Phase 16 review |
| Q3 | No automatic FX in v1 | 2026-08-01 | Phase 16 review |
| Q4 | No overdraft/credit in v1; hard fail | 2026-08-01 | Phase 16 review |
| Q5 | Constraints in Both; no protocol RFC for v1 | 2026-08-01 | Phase 16 review |
| Q6 | Spend-time intersection mandatory; PROTO-0 sync gated on Apply (or separate approval) | 2026-08-01 | Phase 16 review |
| Q7 | Per-org `observation_only` \| `treasury_enforced` | 2026-08-01 | Phase 16 review |
| Q8 | Single-org agent tenancy in v1 (deferred multi-org) | 2026-08-01 | Phase 16 review |
| Q9 | Customer/licensed custodian holds keys; not Aether-by-default | 2026-08-01 | Phase 16 review |
| Q10 | Customer = financial principal; counsel before production fiat | 2026-08-01 | Phase 16 review |
| Q11 | Velocity defaults configurable via impl ADR | 2026-08-01 | Phase 16 review |
| Q12 | Append-only + export hashes in v1; full chain optional | 2026-08-01 | Phase 16 review |
| Q13 | PostgreSQL at multi-tenant pilot | 2026-08-01 | Phase 16 review |
| Q14 | Console: budgets, allocations, journal, emergency freeze after backend | 2026-08-01 | Phase 16 review |
| Q15 | AETH never on-by-default | 2026-08-01 | Phase 16 review |

---

## Q1 — Persistence & tenancy (BLOCKING) — RESOLVED

**Decision: Hybrid (Option C).**

- Control Plane: auth, CSRF, RBAC, policy correlation, Apply gateway, API façade.  
- Treasury Data Plane: own journal/balances/reservations in a **separate logical database/schema** from day 1.  
- ERP is never spend-time authority.  
- In-process colocation allowed only if module + schema boundaries remain extractable.

Rejected: ERP-as-authority; chain/ledger-first; CP-only undifferentiated tables as the long-term home.

---

## Q2 — Balance authority vs bank truth (BLOCKING) — RESOLVED

**Decision: Yes — Treasury journal is authoritative for agent spend authorisation.**

- Authorise on journal `available` + successful reservation.  
- Bank / custodian = custody truth (async reconcile).  
- ERP = statutory / reporting peer.  
- Reconciliation breach above threshold → **block external payouts**.

---

## Q3 — FX / cross-asset (IMPORTANT) — RESOLVED

**Decision: No automatic FX in v1.** Same-asset movements only; explicit conversion journals are a later feature.

---

## Q4 — Overdraft / credit lines (IMPORTANT) — RESOLVED

**Decision: Hard fail.** No negative budgets / credit facilities in v1.

---

## Q5 — PROTO-0 constraint gap (BLOCKING) — RESOLVED

**Decision: Both — no protocol RFC for Treasury v1.**

- PROTO-0: `max_spend`, actions, expiry, freeze/revoke (existing).  
- Treasury: daily/tx/velocity, merchants, geo, categories, aggregate caps.  
- Binding rule: **intersection (minimum)**.

---

## Q6 — Allocation ↔ capability sync (BLOCKING) — RESOLVED

**Decision:**

1. Spend-time check **allocation ∩ capability ∩ policy** is mandatory whenever Treasury enforces spend.  
2. Lab / `observation_only`: allocation ledger may exist **without** PROTO-0 mutation.  
3. Production flows that **issue or resize** capabilities for `treasury_enforced` spend are **gated on Apply enablement** (or a separately approved CP→PROTO-0 mutation path). Treasury must not bypass PROTO-0.  
4. While Apply is disabled, stale capabilities are contained by intersection (high capability still capped by allocation; low capability caps spend below allocation).

---

## Q7 — Escrow funding source (IMPORTANT) — RESOLVED

**Decision:** Per-organisation mode flag:

- `observation_only` — compatible with current simulators; treasury optional  
- `treasury_enforced` — PROTO-2 fund paths used with product spend must reference treasury reservation  

---

## Q8 — Multi-org agents (DEFER) — AFFIRMED

**Decision:** Single-organisation agent tenancy in v1.

---

## Q9 — Digital asset custody (IMPORTANT) — RESOLVED

**Decision:** Customer or licensed custodian via adapter. Aether software is not the default key holder / money transmitter claim.

---

## Q10 — Money transmitter / licensing posture (IMPORTANT) — RESOLVED (GTM condition)

**Decision:** Architecture assumes **customer is the financial principal**. Counsel review is required before production fiat movement (Gate Condition C3).

---

## Q11 — Velocity limit defaults (IMPORTANT) — RESOLVED

**Decision:** Configurable; conservative defaults defined in the first implementation ADR (not in this doc).

---

## Q12 — Hash-chained journal vs DB audit log (IMPORTANT) — RESOLVED

**Decision:** v1 = append-only store + hashed export packages. Full cryptographic hash-chain = optional hardening.

---

## Q13 — PostgreSQL timing (DEFER) — AFFIRMED

**Decision:** PostgreSQL required for multi-tenant production pilot; design schema portably; SQLite lab-only.

---

## Q14 — Console UX scope (DEFER) — AFFIRMED

**Decision:** First UI slice after backend approval: budgets, allocations, journal explorer, emergency freeze.

---

## Q15 — AETH default enablement (DEFER) — AFFIRMED

**Decision:** AETH never on-by-default; opt-in asset registry only.
