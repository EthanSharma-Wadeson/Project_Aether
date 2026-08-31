# Treasury Accounting Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_ACCOUNTING_MODEL.md` |
| **Phase** | 15 — Agent Treasury & Financial Architecture |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Related** | [TREASURY_ARCHITECTURE.md](TREASURY_ARCHITECTURE.md) |

---

## 1. Purpose

Define the **immutable financial record model** for organisation-owned funds under agent delegated spend.

This is a **control-plane journal**, not a replacement for PROTO-2 escrow or PROTO-4 settlement semantics.

---

## 2. Principles

| # | Principle |
|---|-----------|
| A1 | Append-only journal; corrections via compensating entries |
| A2 | Double-entry within an organisation’s books (every debit has a credit) |
| A3 | Balances are **projections** of the journal, never independent truth |
| A4 | Every entry carries full attribution (see §5) |
| A5 | Asset-agnostic: entries always include `AssetId` + minor units |
| A6 | Soft settlement evidence may **reserve**; hard finality may **post** |
| A7 | Agents never appear as balance-sheet owners |

---

## 3. Account types (logical chart)

Within each `OrganisationId`:

| Account class | Examples | Normal balance |
|---------------|----------|----------------|
| **External custody** | Bank GBP, Bank USD, USDC custody, Enterprise ledger | Asset |
| **Treasury available** | Org / Dept / Project available per asset | Asset (internal) |
| **Budget reserved** | Allocated but not yet spent | Asset (restricted) |
| **Escrow reserved** | Held against PROTO-2 escrow | Asset (restricted) |
| **Agent allocation control** | Memo / contra tracking of authority — **not** agent equity | Off-balance / control |
| **Payable / clearing** | In-flight settlement, refunds pending | Liability / clearing |
| **Expense / spend** | Recognised agent spend categories | Expense |
| **Reserve** | Risk / FX / holdback | Equity-like restricted |
| **Suspense** | Unmatched bank lines, chargebacks under review | Clearing |

**Agent Spending Allocation** updates control accounts and budget reserved — it does **not** create an agent-owned asset account.

---

## 4. Immutable event types

Each event produces one or more journal lines (`JournalEntryId` batch).

| Event | Typical effect |
|-------|----------------|
| **Funding** | External custody ↑ ; treasury available ↑ (or deposit clearing) |
| **Transfers** | Available move Org ↔ Dept ↔ Project (same asset) |
| **Reservations** | Available ↓ ; budget or escrow reserved ↑ |
| **Escrow** | Budget reserved → escrow reserved (on fund); reverse on cancel |
| **Settlement** | Escrow reserved → expense / external outflow on hard finality |
| **Refunds** | Reverse settlement path; restore available or reserved per policy |
| **Chargebacks** | Suspense ↑ ; investigate; later expense or restore |
| **Adjustments** | Compensating entries only; reason + approver required |
| **Reconciliations** | Match bank/ERP lines to journal; no balance inventing |
| **Exports** | Snapshot / batch pointer; export itself is an audit event, not a balance change |

### 4.1 State machine vs journal

Budget lifecycle (`active`, `frozen`, `closed`, …) is **control state**. Money movement is **only** via journal events. Freezing must not silently delete reserved escrow lines.

### 4.2 Idempotency

All ingest events carry `IdempotencyKey` / `RequestId`. Duplicate delivery must attach to the same `JournalEntryId` batch (no double posting).

---

## 5. Attribution schema (mandatory)

Every financial event / journal batch **must** link:

| Field | Required | Notes |
|-------|----------|-------|
| `organisation_id` | Yes | Tenancy |
| `treasury_id` | Yes | Node owning or funding the movement |
| `budget_id` | When applicable | Envelope |
| `allocation_id` | When agent-bound | Spending slice |
| `agent_id` | When agent-initiated | Never owner |
| `policy_id` + `policy_version` | When policy-gated | CP template |
| `capability_id` | When PROTO-0 checked | Grant used |
| `approval_id` | When approval required | Governance |
| `operation_id` | When Apply/ops involved | May be null while Apply disabled |
| `escrow_id` | When PROTO-2 involved | |
| `settlement_binding_id` | When PROTO-4 involved | |
| `request_id` | Yes | Correlation |
| `asset_id` | Yes | |
| `actor_operator_id` | When human-initiated | |
| `evidence_refs` | Recommended | Bank ref, ERP doc, receipt hash |

Missing required fields → event rejected at design boundary (future validation).

---

## 6. Balance projections

Materialised views (rebuildable from journal):

```text
available(treasury, asset)
reserved_budget(budget, asset)
reserved_escrow(escrow, asset)
allocation_remaining(allocation, asset)   # derived authority ceiling
```

**Invariant:**  
`sum(available + reserved_budget + reserved_escrow + suspense_clearing)` reconciles to external custody ± in-flight payables per asset (within defined timing windows).

---

## 7. Soft vs hard finality posting rules

| Signal | Accounting action |
|--------|-------------------|
| PROTO-2 fund escrow | Reservation only |
| PROTO-4 soft finality | Optional soft-reserve / annotate; **no** irreversible expense recognition as hard cash |
| PROTO-4 hard finality | Post settlement; release escrow reservation to expense/outflow |
| Adapter failure / timeout | Compensating release or suspense — never silent drop |

This preserves existing protocol soft≠hard distinction in the books.

---

## 8. Exports & period close

- Period close produces an immutable **export package** (JSON/CSV/ERP batch) with hash + `RequestId`.  
- Re-export allowed; mutating historical journal lines **forbidden**.  
- Adjustments after close require explicit reopening policy + compensating entries in the new period.

---

## 9. Non-goals

- Full statutory GL replacement for the enterprise  
- Tax engine or FX trading ledger  
- Implementing payment rails  

---

## 10. Freeze statement

Accounting event taxonomy, attribution schema, and soft/hard posting rules are **frozen** for design. Implementation requires separate approval.
