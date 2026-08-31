# Treasury Architecture — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_ARCHITECTURE.md` |
| **Phase** | 15 — Agent Treasury & Financial Architecture |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Protocol impact** | **None** — does not change PROTO-0…4, Apply, or `aether-core` |
| **Related** | [TREASURY_ACCOUNTING_MODEL.md](TREASURY_ACCOUNTING_MODEL.md), [TREASURY_SECURITY_MODEL.md](TREASURY_SECURITY_MODEL.md), [TREASURY_THREAT_MODEL.md](TREASURY_THREAT_MODEL.md), [TREASURY_BUSINESS_CASE.md](TREASURY_BUSINESS_CASE.md), [TREASURY_OPEN_QUESTIONS.md](TREASURY_OPEN_QUESTIONS.md), [TREASURY_ARCHITECTURE_REVIEW.md](TREASURY_ARCHITECTURE_REVIEW.md) |

---

## 0. Design freeze statement

This document freezes the **product financial architecture** for enterprise agent spend under Aether.

**Out of scope for this phase (and not authorised):**

- wallets, bank/crypto integrations, payment APIs  
- settlement / accounting / funding code  
- HTTP routes, Control Plane changes, Apply enablement  
- protocol mutations or semantic changes  

**Invariants relative to v1.0:**

| Invariant | Meaning |
|-----------|---------|
| Agents never own money | Agents hold **delegated spending authority**, not asset title |
| Organisations own assets | Legal and ledger ownership sits on Organisation / Treasury accounts |
| Protocol remains authority for identity & capability | PROTO-0 grants/revokes/freezes stay the agent authority surface |
| PROTO-2/4 remain economic/settlement contracts | Escrow & settlement semantics unchanged; treasury **funds** and **constrains** them |
| Asset-agnostic core | Financial logic keyed by opaque `AssetId`, not currency-specific code |
| AETH optional | Never required; fiat + enterprise ledger first |
| No blockchain requirement | Digital assets are optional adapters, not protocol substrate |

---

## 1. Positioning in the Aether stack

Treasury is a **Control Plane / enterprise product layer** that sits **beside** the protocol, not inside it.

```text
 Organisation (legal principal)
        │
        ▼
 ┌──────────────────────────────────────────────┐
 │  Treasury Plane (NEW — design only)          │
 │  Org / Dept / Project treasuries             │
 │  Budgets, allocations, freezes, recovery     │
 │  Immutable financial journal                 │
 │  Regulatory extension points                 │
 └───────────────────┬──────────────────────────┘
                     │ maps authority into
                     ▼
 PROTO-0  CapabilityGrant (max_spend, actions, expiry, …)
                     │
                     ▼
 Agent (operational key) — spends under capability
                     │
                     ▼
 PROTO-2  Escrow (terms, receipts, dispute)     [unchanged]
                     │
                     ▼
 PROTO-4  Settlement binding → external rails   [unchanged adapters]
```

**Rule:** Treasury **never** becomes an alternate PROTO-0. When agent spend authority must change, future implementation still **requests** PROTO-0 grant/revoke/freeze through existing Apply/governance paths (Apply remains disabled until separately approved).

---

## 2. Core concepts

### 2.1 Organisation

Legal and commercial principal. Owns all treasuries under its tenancy. All financial events are attributable to exactly one Organisation.

### 2.2 Treasury hierarchy

| Level | Purpose | Typical owners |
|-------|---------|----------------|
| **Organisation Treasury** | Master pool; bank/ERP deposits land here | CFO / treasury ops |
| **Department Treasury** | Cost-centre partition of org funds | Department finance |
| **Project Treasury** | Time-bounded programme budget | Project owner + finance |
| **Agent Spending Allocation** | Non-owning budget slice delegated to an agent | Security + FinOps |
| **Temporary Funding** | Short-TTL top-ups (campaigns, incidents) | Operator + approver |
| **Reserve Accounts** | Non-spendable buffers (risk, FX, holdbacks) | Treasury ops |
| **Escrow Funding** | Funds earmarked to back PROTO-2 escrows | Finance + policy |

**Ownership rule:** Every node except the agent allocation **holds** balances. Agent Spending Allocations are **authorisations over balances**, not balance holders. Closing or freezing an allocation does not transfer title to the agent.

### 2.3 Identifiers (logical)

```text
OrganisationId
TreasuryId          # any node in the hierarchy
BudgetId            # spending envelope on a treasury or allocation
AssetId             # opaque asset key (see §3)
AllocationId        # agent-bound spending slice
PolicyId            # CP policy template / version
CapabilityId        # PROTO-0 capability (when mapped)
ApprovalId          # governance approval
OperationId         # Apply / mutation operation (when used)
EscrowId            # PROTO-2
SettlementBindingId # PROTO-4
RequestId           # correlation id for audits
JournalEntryId      # immutable accounting record
```

---

## 3. Asset model (multi-currency, asset-agnostic)

### 3.1 Asset abstraction

All balances, limits, and journals are parameterized by `AssetId` + `Amount` (integer minor units + scale metadata).

| Asset class | Examples | Required? |
|-------------|----------|-----------|
| Fiat | `GBP`, `USD`, `EUR` | Supported; enterprise default |
| Enterprise ledger | `ENTERPRISE_LEDGER` (internal unit / book money) | Supported |
| Stablecoin | `USDC` | Optional adapter |
| Protocol-labelled token | `AETH` | **Optional**; never required |
| Future | any registered `AssetId` | Extension registry |

**Asset registry (design):** each organisation registers which `AssetId`s are enabled, their scale, and which **custody adapters** may move them. Core treasury logic must not `match` on currency enums beyond validation against the registry.

### 3.2 Multi-asset rules

- A treasury may hold **many** assets (partitioned sub-balances).  
- Budgets and allocations are **per-asset** (or explicitly multi-asset with per-asset caps).  
- Cross-asset conversion is **out of core path** — requires an explicit FX/conversion journal + policy (see open questions).  
- PROTO-2 escrow `asset` labels must match registered `AssetId`s when treasury funding is attached.

### 3.3 What “holding” means

| Layer | Meaning of balance |
|-------|--------------------|
| Bank / payment provider | External custody (adapter) |
| Enterprise ledger | Internal book balance |
| Treasury Plane journal | Authoritative **control-plane** view of available / reserved / escrowed |
| PROTO-2 | Simulated or bound economic state — **not** org ownership ledger |

Soft vs hard finality (PROTO-4) remains unchanged: treasury may **reserve** on soft signals and **post** on hard settlement evidence.

---

## 4. Funding lifecycle

States for a **Budget** (spending envelope):

```text
 draft → funded → active ⇄ frozen → closed
              ↘ expired
```

| Operation | Effect | Agent money ownership? |
|-----------|--------|------------------------|
| **Create Treasury** | Create hierarchy node; zero balances | N/A |
| **Deposit Funds** | External → Org (or Dept) treasury available | No |
| **Allocate Budget** | Carve BudgetId / AllocationId from parent available → reserved-for-budget | No |
| **Increase Budget** | Move more parent available into budget | No |
| **Decrease Budget** | Return unused budget to parent (subject to reservations) | No |
| **Freeze Budget** | Block new spend / new reservations; existing escrows policy-dependent | No |
| **Close Budget** | Terminal; recover unused to parent | No |
| **Recover Funds** | Explicit pullback of unused / post-expiry | No |
| **Budget Expiry** | Auto-freeze or auto-close per policy; unused → parent or reserve | No |
| **Unused Funds** | Always attributable to parent treasury / org; never agent residual ownership | No |

**Temporary Funding:** TTL-bounded increase; on expiry, unused portion auto-recovers.

**Escrow Funding:** moves (or marks) funds from budget **available** → **reserved/escrowed** when a PROTO-2 escrow is funded; release/refund paths reverse the reservation per accounting model.

---

## 5. Spending authority model (frozen)

Agents spend only under **compound constraints**:

```text
 Organisation policy
   ∩ Treasury / Budget limits
   ∩ Agent Spending Allocation
   ∩ PROTO-0 Capability constraints
   ∩ Approval / attestation (when required)
```

### 5.1 Constraint dimensions

| Dimension | Description |
|-----------|-------------|
| Maximum spend | Lifetime or budget-ceiling for allocation |
| Daily limits | Rolling or calendar-day caps per asset |
| Per-transaction limits | Single operation / escrow / payment max |
| Allowed assets | Subset of org-registered `AssetId`s |
| Allowed merchants | Merchant / vendor allowlist or category codes |
| Allowed counterparties | Agent ids, org ids, account bindings |
| Geographic restrictions | Jurisdiction / region tags on counterparties or rails |
| Time restrictions | Windows, blackouts, expiry |
| Category restrictions | Spend categories (SaaS, compute, travel, …) |
| Delegation depth | Max hops from org root → agent (aligns with PROTO-0 narrowing) |
| Emergency stop | Org / treasury / agent kill-switch (maps to freeze budget + PROTO-0 freeze/revoke) |

### 5.2 Mapping to PROTO-0 (non-mutating design)

Future implementation **projects** allocation constraints into capability fields already understood by PROTO-0 (e.g. `max_spend`, actions, expiry) **without expanding protocol semantics**. Constraints that PROTO-0 cannot express remain enforced in the Treasury Plane **before** escrow/settlement adapters are invoked.

| Constraint | PROTO-0 today | Treasury Plane |
|------------|---------------|----------------|
| max_spend | Capability field | Must not exceed allocation remaining |
| actions | Capability field | Must be subset of allowed categories/actions |
| expiry | Capability field | ≤ budget / temporary funding TTL |
| merchants, geo, daily caps | Not first-class in PROTO-0 | Treasury pre-check + policy |
| emergency stop | freeze / revoke | Freeze budget + request PROTO-0 freeze/revoke |

**Design freeze:** PROTO-0 is **not** extended in Phase 15. Richer constraints live in Treasury + policy until a separate protocol RFC is approved.

### 5.3 Authority parties

| Party | May |
|-------|-----|
| Organisation admin / treasury ops | Create treasuries, deposit (via adapter), allocate, freeze, recover |
| Department / project owner | Allocate within their treasury ceiling |
| Security officer | Emergency stop, freeze |
| Operator | Propose allocations / temporary funding (subject to approval) |
| Approver (finance) | Approve funding and high-risk allocations |
| Agent | Initiate spend only within live capability ∩ allocation |
| Auditor / viewer | Read journals and attributions |

---

## 6. Relationship to escrow & settlement

| Concern | Owner |
|---------|-------|
| Who may open/fund escrow | Capability + treasury reservation check |
| Escrow terms / dispute / release | PROTO-2 (unchanged) |
| Settlement binding / soft vs hard finality | PROTO-4 (unchanged) |
| Booking available → reserved → settled / refunded | Treasury journal |
| External bank/ERP movement | Custody / settlement adapters (future) |

**Escrow Funding account:** optional dedicated sub-ledger under a Project or Org treasury used only to collateralise open escrows. Prevents project operating budgets from being silently exhausted by disputed holds.

---

## 7. Enterprise integration interfaces (design)

Logical interfaces only — no API implementation in this phase.

| Interface | Direction | Purpose |
|-----------|-----------|---------|
| `ErpSyncPort` | Bi-directional | Cost centres, vendors, chart of accounts |
| `AccountingExportPort` | Out | Journal batches, period close |
| `BankReconciliationPort` | In/Out | Statement lines ↔ deposit/withdrawal journals |
| `PayrollPort` | In (optional) | Funding from payroll cost allocation |
| `ProcurementPort` | In | PO / vendor approval before merchant allow |
| `FinanceApprovalPort` | In | Dual-control for large allocations |
| `CustodyAdapter` | Out/In | Fiat / ledger / optional USDC / optional AETH |
| `RegulatoryHookPort` | Sync points | KYC/AML/sanctions/tax/reporting (see security/regulatory docs) |

---

## 8. Scalability (design targets)

| Dimension | Target class |
|-----------|--------------|
| Organisations | Hundreds |
| Agents | Thousands per large org; tens of thousands globally |
| Transactions / journal entries | Millions+ |
| Storage | Append-only journal; snapshot balances; future PostgreSQL |
| Compute | Stateless API workers; sharded by `OrganisationId` |
| Consistency | Per-org serialisable funding; eventual cross-org analytics |
| Protocol | Remains local/authoritative per deployment; treasury scales independently |

**Horizontal scaling sketch:** partition journals and balance materialisations by `OrganisationId`. Idempotency keys on all funding/spend intents. No global blockchain.

---

## 9. Explicit non-goals

- Designing a chain, L2, or consensus  
- Requiring AETH or any crypto asset  
- Replacing PROTO-2/4 with a general ledger inside `aether-core`  
- Giving agents wallets or legal title  
- Implementing compliance engines in Phase 15  

---

## 10. Design freeze checklist

- [x] Treasury hierarchy defined  
- [x] Multi-currency / asset abstraction defined  
- [x] Funding lifecycle defined  
- [x] Spending authority dimensions frozen  
- [x] Protocol boundary unchanged  
- [x] Apply remains disabled; no implementation authorised  

**Recommendation:** Accept this architecture as the frozen baseline. Implementation requires a **separate Phase 16+ approval** after open questions and security review acceptance.
