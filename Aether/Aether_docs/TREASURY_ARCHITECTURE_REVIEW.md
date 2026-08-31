# Treasury Architecture Review — Phase 16

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_ARCHITECTURE_REVIEW.md` |
| **Phase** | 16 — Treasury Architecture Review & Open Question Resolution |
| **Nature** | Architecture review only — **no implementation authorised** |
| **Date** | 2026-08-01 |
| **Inputs** | Phase 15 design freeze pack (`TREASURY_*.md`) |
| **Protocol impact** | **None** — review confirms implementability **without** PROTO-0…4 / Apply / `aether-core` changes |
| **Gate decision** | **PASS WITH CONDITIONS** |

---

## Executive Summary

Phase 15 froze a coherent enterprise Treasury Plane: organisations own funds; agents receive delegated spending authority; the journal is append-only and asset-agnostic; PROTO-0 remains the agent authority surface; PROTO-2/4 remain economic/settlement contracts.

This review validates **internal consistency**, resolves **all BLOCKING open questions**, recommends a **persistence architecture**, reclassifies security findings, and issues an implementation gate.

| Question | Verdict |
|----------|---------|
| Can Treasury ship without protocol changes? | **Yes** |
| Is authority model sound? | **Yes** — invariants recorded below |
| Is accounting model consistent? | **Yes** — unchanged; clarified source-of-truth matrix |
| Are blocking open questions resolved? | **Yes** (Q1, Q2, Q5, Q6) |
| Gate | **PASS WITH CONDITIONS** — implementation still needs separate approval; conditions in §Gate Decision |

**STOP rule:** This document does not authorise wallets, ledgers, routes, schema, frontend, Apply enablement, or protocol edits.

---

## 1. Authority Review

### 1.1 Verified properties

| Claim | Status | Evidence |
|-------|--------|----------|
| Organisation owns all funds | **VALIDATED** | Hierarchy + journal asset accounts; agents have no balance-sheet ownership |
| Agents never own assets | **VALIDATED** | Allocations are authority ceilings, not asset accounts |
| Treasury never bypasses PROTO-0 | **VALIDATED** | Stack diagram; capability check mandatory for agent-initiated spend |
| Spend requires capability ∩ allocation ∩ policy | **VALIDATED** | Compound constraint model; intersection rule |
| Accounting never becomes protocol authority | **VALIDATED** | Journal is CP/product truth for **org money control**; PROTO-0/2/4 unchanged |

### 1.2 Frozen invariants (normative for future implementation)

| ID | Invariant |
|----|-----------|
| **INV-T01** | Legal/economic title to funds belongs to the Organisation (via Treasury accounts), never to an Agent. |
| **INV-T02** | An Agent Spending Allocation is a **delegated ceiling**, not a wallet or custody account. |
| **INV-T03** | Treasury MUST NOT grant, revoke, or freeze PROTO-0 capabilities by writing protocol stores directly. |
| **INV-T04** | Agent-initiated spend MUST fail closed unless **capability ∧ allocation ∧ applicable policy** all pass. |
| **INV-T05** | Where PROTO-0 and Treasury disagree numerically, the **intersection (minimum)** binds. |
| **INV-T06** | The Treasury journal MUST NOT be treated as PROTO-2 escrow state or PROTO-4 settlement finality. |
| **INV-T07** | Soft finality MAY reserve; only hard finality (or explicit policy) MAY post irreversible expense/outflow in books. |
| **INV-T08** | Apply remains independently gated; Treasury design MUST NOT require `apply_enabled=true` to be valid as a ledger of allocations. |
| **INV-T09** | Asset logic is keyed by opaque `AssetId`; AETH/crypto MUST NOT be required. |
| **INV-T10** | Compensating journals only — no silent mutation or deletion of posted lines. |

### 1.3 Authority parties (clarified)

```text
 Organisation (principal / title)
        │
        ├─► Treasury Plane     authorises & books org money movements
        ├─► Control Plane      authn/z, policy templates, audit, Apply gateway
        ├─► PROTO-0            authoritative agent capability / identity
        ├─► PROTO-2 / PROTO-4  economic contract + settlement evidence
        └─► External rails     bank / custodian / ERP (adapters)
```

**Finding:** No protocol change is required to uphold these invariants. Rich constraints (merchants, geo, daily caps) live in Treasury without extending PROTO-0 (see §5 / Q5).

---

## 2. Persistence Recommendation

### 2.1 Options evaluated

| Option | Summary | Verdict |
|--------|---------|---------|
| **A. Embedded CP storage only** | Journal tables inside today’s Control Plane SQLite | Acceptable for lab spike only; couples finance scale to CP |
| **B. Separate Treasury service + DB only** | Standalone service; CP unused | Rejected as sole path — loses shared auth/policy/Apply boundary |
| **C. Hybrid** | CP = façade (auth, RBAC, policy, audit correlation); Treasury = own data plane | **SELECTED** |
| **D. External ERP as authority** | ERP balances authorise agent spend | **Rejected** for spend-time auth — too slow/inconsistent; ERP is sync/export peer |
| **E. Ledger-first / chain** | Blockchain as system of record | **Rejected** — violates fiat-first, no-chain requirement |

### 2.2 Recommended architecture: Hybrid (C)

```text
 Operators / Console
        │
        ▼
 Control Plane (auth, CSRF, RBAC, policy IDs, audit correlation, Apply gateway)
        │  Treasury API façade (future)
        ▼
 Treasury Data Plane
        │  • own database (logical separation from day 1)
        │  • append-only journal + balance projections
        │  • reservation/allocation state machines
        ▼
 Adapters: Bank | Custodian | ERP | Regulatory hooks
        │
        └── read-only observation of PROTO-* (no store ownership)
```

**Why C:**

1. Preserves CP security model (JWT, CSRF, roles) without putting millions of journal rows on the critical path of protocol observation.  
2. Keeps protocol DBs / `aether-core` untouched.  
3. Allows PostgreSQL and org-keyed partitioning without rewriting CP.  
4. Avoids ERP-as-authority races.  
5. Allows a **first vertical slice** to run Treasury Data Plane **in-process** with CP **only if** it uses a **separate logical database/schema** and a stable internal API — extractable without redesign.

**Explicit non-recommendation:** Mixing treasury journals into PROTO-0/2/4 stores, or into undifferentiated CP tables without a Treasury module boundary.

### 2.3 Sequencing (implementation guidance — not authorised yet)

| Slice | Persistence |
|-------|-------------|
| Lab / design validation | Hybrid logically; may colocate process; **separate schema** |
| Multi-tenant pilot | Dedicated Treasury DB; PostgreSQL preferred |
| Scale-out | Partition by `OrganisationId`; stateless workers |

---

## 3. Accounting Review

### 3.1 Source of truth matrix

| Concern | System of record for **authorisation** | System of record for **external custody** | System of record for **statutory books** |
|---------|----------------------------------------|-------------------------------------------|------------------------------------------|
| Available / reserved balances | **Treasury Ledger (journal projections)** | — | ERP (after export/sync) |
| Agent spend permission | Treasury ∩ PROTO-0 ∩ policy | — | — |
| Bank cash | Reconciled mirror / custody adapter | **Bank** | ERP |
| Crypto / stablecoin | Reconciled mirror | **Custodian** | ERP / specialist |
| PROTO-2 escrow lifecycle | Protocol escrow store | — | Mapped via treasury reservations |
| PROTO-4 soft finality | Evidence only; may soft-reserve | Rail-dependent | Not hard close |
| PROTO-4 hard finality | Triggers treasury **post** | Rail settlement | ERP recognition |
| Reconciliation breaks | Treasury suspense + halt payouts | Bank/custodian statement | Adjust via compensating entries |

### 3.2 Relationship diagram

```text
 Bank / Custodian  ──async reconcile──►  Treasury Ledger  ──export──►  Enterprise ERP
      (custody truth)                 (spend-auth truth)              (statutory truth)

 PROTO-2/4 ──evidence & ids──► Treasury reservations / posts
 PROTO-0   ──capability check──► Spend gate (with allocation)
```

### 3.3 Lifecycle consistency (validated)

| Stage | Money / control effect | Notes |
|-------|------------------------|-------|
| **Budget** | Envelope on treasury funds | State: draft→funded→active⇄frozen→closed/expired |
| **Allocation** | Agent ceiling over budget | No title transfer |
| **Reservation** | Available → reserved (budget or escrow) | Atomic; idempotent |
| **Escrow** | Reserved against `EscrowId` | PROTO-2 owns contract state |
| **Settlement** | Soft: annotate/soft-reserve; Hard: post outflow/expense | INV-T07 |
| **Recovery** | Unused allocation/budget → parent available | Never to agent |
| **Expiry** | Auto-freeze/close + recover unused | Temporary funding TTL |
| **Refund** | Compensating restore via suspense-safe path | Serialised |
| **Chargeback** | Suspense first; then expense or restore | No double-credit |

**Finding:** Lifecycle is internally consistent with the frozen accounting model. **No change** to `TREASURY_ACCOUNTING_MODEL.md` is required; this review clarifies authority boundaries only.

### 3.4 Accounting findings (summary)

1. Treasury Ledger is **authoritative for agent spend authorisation** (Q2 resolved).  
2. Bank/Custodian remain **custody truth**; divergence beyond threshold **blocks external payouts**.  
3. ERP is **downstream** for statutory reporting — never spend-time authority.  
4. Protocol stores never invent organisation GL balances.

---

## 4. Constraint Model Review

| Constraint class | Where enforced | PROTO-0 change needed? |
|------------------|----------------|------------------------|
| `max_spend`, actions, expiry | **Both** — PROTO-0 fields ∩ Treasury allocation/budget | **No** |
| Daily / per-tx / velocity | **Treasury** | **No** |
| Merchants, counterparties, geo, categories | **Treasury** (+ policy templates) | **No** |
| Delegation depth | PROTO-0 narrowing ∩ Treasury depth policy | **No** |
| Emergency stop | Treasury freeze + **request** PROTO-0 freeze/revoke via CP/Apply | **No** |

**Trade-offs:**

| Approach | Pros | Cons |
|----------|------|------|
| PROTO-0 only | Portable across implementations | Cannot express merchant/geo/daily without RFC |
| Treasury only | Fast to ship; rich enterprise rules | Non-Aether consumers won’t see those limits |
| **Both (selected)** | Least privilege intersection; no protocol churn | Dual maintenance; sync discipline (Q6) |

**Resolution (Q5):** Enforce rich constraints in **Treasury**; keep using PROTO-0 for existing fields; **no protocol RFC** for Treasury v1.

---

## 5. Regulatory Boundaries

| Responsibility | Aether (software) | Enterprise customer | Payment provider | Custodian | Compliance platform |
|----------------|-------------------|---------------------|------------------|-----------|---------------------|
| Product controls, journals, caps | ✓ | Configures | | | |
| Legal ownership of funds | | ✓ principal | | may hold | |
| KYC/AML/sanctions decisions | Hook only | Accountable | May perform | May perform | ✓ engines |
| Money transmission license | Must not assume | Counsel per market | Often licensed | Often licensed | |
| Tax filing | Export hooks | ✓ | | | Optional |
| Audit retention | Provides exports/holds | Sets policy | | | |
| Protocol semantics | Stable | Consumes | Unrelated | Unrelated | Unrelated |

**Finding:** Regulatory logic stays in **extension points**. No legal regime is embedded into `aether-core`.

---

## 6. Security Review

Review statuses for implementation planning: **VALIDATED** (design adequate), **OPEN** (must close in implementation ADRs), **DEFERRED** (adapter/legal/later tier).

| ID | Threat | Phase 15 | Phase 16 |
|----|--------|----------|----------|
| TR-THR-001 | Budget abuse | MITIGATED (design) | **VALIDATED** |
| TR-THR-002 | Temp funding no expiry | MITIGATED (design) | **VALIDATED** |
| TR-THR-003 | Orphaned / fake reservations | OPEN | **VALIDATED** — require reservation TTL + sweeper + audited break-glass |
| TR-THR-004 | Aggregate overspend | MITIGATED (design) | **VALIDATED** |
| TR-THR-010 | Runaway agent drain | MITIGATED (design) | **VALIDATED** |
| TR-THR-011 | Many small escrows | OPEN | **VALIDATED** — require velocity + open-escrow caps (defaults in impl ADR) |
| TR-THR-012 | Cross-asset bypass | MITIGATED (design) | **VALIDATED** (same-asset v1; no auto-FX) |
| TR-THR-020–021 | Insider / journal delete | MITIGATED (design) | **VALIDATED** |
| TR-THR-022 | Allowlist self-pay | OPEN | **VALIDATED** — delayed activation + change-control |
| TR-THR-030–032 | Dup / replay / double-spend | MITIGATED (design) | **VALIDATED** |
| TR-THR-033 | Refund/chargeback race | OPEN | **VALIDATED** — suspense + serialised posting |
| TR-THR-040 | Exhaustion DoS | ACCEPTED | **VALIDATED** as residual ops risk |
| TR-THR-041 | Freeze storm | OPEN | **VALIDATED** — dual control for fleet-wide freeze |
| TR-THR-050–052 | Policy/protocol bypass | MITIGATED (design) | **VALIDATED** |
| TR-THR-053 | Manual pretend funds (Apply off) | ACCEPTED | **VALIDATED** residual — see Condition C2 |
| TR-THR-060–061 | Compromised op/signer | MITIGATED (design) | **VALIDATED** |
| TR-THR-062 | Compromised custody creds | OPEN | **OPEN** — adapter implementation phase |
| TR-THR-063 | Recovery abuse | MITIGATED (design) | **VALIDATED** |
| TR-THR-070–071 | Sanctions / retention wipe | MITIGATED (design) | **VALIDATED** (hooks) |
| TR-THR-072 | Over-collection PII | OPEN | **DEFERRED** — jurisdiction DPA guidelines |
| — | FX inconsistencies | (implied) | **VALIDATED** mitigated by **no auto-FX in v1** |
| — | Stale allocations | (implied) | **VALIDATED** — expiry + spend-time remaining check |
| — | Race on reserve | (implied) | **VALIDATED** — atomic reserve per org+asset |

### Security findings (summary)

1. Design-level controls cover overspend, duplicate reservation, replay, and intersection bypass **without protocol changes**.  
2. Former OPEN design gaps (orphan reservations, velocity, allowlist delay, refund races, fleet freeze) are **closed at design** with mandatory implementation acceptance criteria.  
3. **Still OPEN:** custody adapter credential security (TR-THR-062).  
4. **Residual accepted:** Apply-disabled operational drift (TR-THR-053); exhaustion as availability issue.

---

## 7. Scalability Review

| Target | Assessment |
|--------|------------|
| Millions of journal entries | **Feasible** with append-only storage, async projection rebuild, org partition |
| Hundreds of orgs / thousands of agents | **Feasible** under Hybrid + `OrganisationId` sharding |
| Many assets | **Feasible** — per-asset sub-balances; no cross-asset hot path in v1 |
| SQLite | Acceptable for single-tenant lab only |
| PostgreSQL | **Required** before multi-tenant production pilot (Q13) |
| Distributed execution | Stateless API + partitioned DB; no global chain |

**Finding:** Scalability path does not require protocol changes or distributed consensus inside Aether core.

---

## 8. Open Questions Resolution

### BLOCKING — resolved

| ID | Decision | Rationale |
|----|----------|-----------|
| **Q1 Persistence** | **Hybrid (C)** — CP façade + Treasury data plane with separate logical DB from day 1 | Isolation, CP security reuse, extractability, no ERP-as-auth |
| **Q2 Accounting authority** | **Treasury journal authorises spend**; bank/custodian = custody truth; ERP = statutory peer; reconcile async; breach → block payouts | Matches INV-T06/T07; avoids bank latency on every agent action |
| **Q5 Constraints** | **Both** — PROTO-0 for existing fields; Treasury for rich rules; **no protocol RFC for v1** | Ship enterprise controls without semantic churn |
| **Q6 Allocation ↔ capability** | **Spend-time intersection is mandatory.** Lab may run allocation ledger without PROTO-0 mutation. **Production capability updates** for enforced spend remain gated on **Apply enablement (separate gate)** or an explicitly approved CP→PROTO-0 path — Treasury must not invent one | Preserves INV-T03/T08; avoids fake sync while Apply is off |

### IMPORTANT — resolved for v1 scope

| ID | Decision |
|----|----------|
| **Q3 FX** | **No automatic FX** in v1; same-asset only |
| **Q4 Overdraft** | **Hard fail**; no credit lines in v1 |
| **Q7 Escrow funding** | Per-org mode: `observation_only` \| `treasury_enforced` |
| **Q9 Custody** | Customer or licensed custodian via adapter; Aether software is not the default key holder |
| **Q10 Licensing** | Customer is financial principal; **counsel review before production fiat movement** (go-to-market condition) |
| **Q11 Velocity defaults** | Configurable; conservative defaults in implementation ADR |
| **Q12 Hash chain** | v1: append-only SQL + export package hashes; full hash-chain optional hardening |

### DEFER — affirmed

| ID | Decision |
|----|----------|
| **Q8 Multi-org agents** | Single-org tenancy in v1 |
| **Q13 PostgreSQL** | At multi-tenant pilot; portable schema now |
| **Q14 Console UX** | Budgets, allocations, journal explorer, emergency freeze — after backend approval |
| **Q15 AETH default** | Never on-by-default |

Decision log is mirrored in [TREASURY_OPEN_QUESTIONS.md](TREASURY_OPEN_QUESTIONS.md).

---

## 9. Remaining Risks

| Risk | Severity | Disposition |
|------|----------|-------------|
| Apply disabled → capability/allocation operational drift | Medium | **Condition C2** — production enforced mode waits on Apply gate or approved mutation path |
| Custody adapter compromise (TR-THR-062) | High (when adapters exist) | **OPEN** until adapter security ADR |
| Legal/MT licensing in target markets | High (GTM) | **Condition C3** — counsel before fiat movement |
| ERP/bank reconcile lag → false confidence | Medium | Threshold halt on payouts; monitoring |
| Soft≠hard timing gaps | Medium | Accepted; books use suspense |
| First-slice process colocation misuse (blurring module boundary) | Low | **Condition C1** — separate schema + API boundary mandatory |

---

## 10. Implementation Recommendation

**Do not implement yet.** When a future phase is approved, implement in this order:

1. Treasury Data Plane skeleton (journal, budgets, allocations) behind CP façade — **no** payment rails.  
2. Spend-gate library enforcing INV-T04/T05 in `observation_only` / dry paths.  
3. Reservation TTL + sweeper + idempotency tests.  
4. Console read/write for budgets (after backend).  
5. Adapters + regulatory hooks only after C3 and adapter security ADR.  
6. Capability sync automation only after Apply enablement gate (or explicit alternate approval).

**Must not:** modify `aether-core`, PROTO semantics, or set `apply_enabled=true` as part of Treasury work.

---

## 11. Gate Decision

### Decision: **PASS WITH CONDITIONS**

The Treasury design is internally consistent, protocol-compatible, and ready for a **future implementation proposal**. It is **not** an implementation authorisation.

### Conditions

| ID | Condition |
|----|-----------|
| **C1** | Persistence MUST follow Hybrid: separate Treasury logical database/schema; CP remains façade for auth/policy. |
| **C2** | Production `treasury_enforced` flows that require new/changed PROTO-0 capabilities MUST NOT ship until Apply enablement (or a separately approved PROTO-0 mutation path) is complete. Allocation-only / observation modes may proceed earlier under implementation approval. |
| **C3** | Production fiat/crypto movement requires legal review of customer-principal posture in target jurisdictions. |
| **C4** | Implementation MUST include acceptance tests for: intersection spend gate, atomic reservation, idempotency, reservation TTL/orphan sweeper, refund serialisation, no agent balance ownership. |
| **C5** | OPEN item TR-THR-062 (custody credentials) MUST be closed before enabling live custody adapters. |

### What would be FAIL

- Requiring protocol semantic changes to ship Treasury v1  
- Making agents asset owners  
- Using ERP/bank as spend-time authority without reconcile controls  
- Embedding compliance engines into `aether-core`  
- Enabling Apply inside a Treasury phase without its own gate  

None of those are present in the frozen design as reviewed.

---

## 12. Review checklist

- [x] Blocking questions resolved  
- [x] Architecture internally consistent  
- [x] Persistence recommendation clear (Hybrid)  
- [x] Security review completed  
- [x] Remaining risks documented  
- [x] Implementation recommendation issued  
- [x] No protocol / Apply / code changes made in this phase  

**STOP.** Treasury implementation requires separate approval after this review.
