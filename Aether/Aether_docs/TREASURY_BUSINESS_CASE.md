# Treasury Business Case — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_BUSINESS_CASE.md` |
| **Phase** | 15 — Agent Treasury & Financial Architecture |
| **Status** | **DESIGN FROZEN** — strategy only; no implementation authorised |
| **Date** | 2026-08-01 |
| **Related** | [TREASURY_ARCHITECTURE.md](TREASURY_ARCHITECTURE.md), [PHASE_2_WEDGE_DECISION.md](../Project_Phases/phase_2/PHASE_2_WEDGE_DECISION.md) |

---

## 1. Problem

Enterprises are deploying AI agents that need to **spend** (APIs, vendors, compute, procurement) without receiving:

- corporate card PAN ownership,  
- bank login credentials, or  
- unbounded cloud billing roles.

Today’s tools are human-centric (OAuth, virtual cards, SaaS admin roles). Security and finance block autonomy because **money ownership and spend authority are conflated**.

Aether v1.0 already separates **identity/capability** (PROTO-0) from **economic contracts** (PROTO-2/4) and provides an Enterprise Governance Console. What is missing is an explicit **organisation treasury plane**: who owns the money, how budgets are carved, and how agent authority is funded and recovered.

---

## 2. Opportunity

Extend the Phase 2 wedge (**Enterprise Agent Spend Control**) with a finance-grade layer that:

- keeps **legal ownership** with the organisation,  
- gives agents **delegated, revocable, auditable** spending authority,  
- supports **GBP / USD / EUR**, enterprise ledger, optional USDC/AETH,  
- integrates with ERP, bank reconciliation, and finance approval,  
- remains **regulator-flexible** via hooks (not baked-in compliance products).

This is a **product differentiator** on top of a stable protocol — not a protocol rewrite.

---

## 3. Buyers and users

| Persona | Need |
|---------|------|
| CFO / Treasury | Ownership, reserves, recovery, multi-currency books |
| FinOps | Budgets, daily caps, project envelopes |
| Security / GRC | Emergency stop, attribution, audit export |
| AI platform lead | Agents that can act without waiting on humans per payment |
| Finance approver | Dual control on large allocations |
| Auditor | Immutable journals with full linkage |

**Buyer type:** Enterprise platform + finance co-buy. Protocol remains substrate; treasury is the sellable control surface.

---

## 4. Value proposition

| Without Treasury Plane | With Treasury Plane (designed) |
|------------------------|--------------------------------|
| Capability max_spend floats without funding truth | Spend capped by **funded allocation** |
| Escrow/settlement demos ≠ enterprise books | Journal maps escrow/settlement to org GL views |
| Agents appear to “have wallets” | Agents never own; org recovers unused funds |
| Single-currency mental model | Asset-agnostic multi-currency |
| Crypto-first pressure | Fiat + ledger first; AETH optional |

---

## 5. Fit with current product

| Already shipped | Treasury adds (future) |
|-----------------|------------------------|
| PROTO-0 capabilities | Allocation → capability projection rules |
| PROTO-2/4 (simulators) | Funding & posting rules around them |
| Control Plane policies / Apply (disabled) | Finance approval + funding lifecycle |
| Governance Console | Treasury / budget screens (later phase) |

**Explicit non-claim:** This business case does **not** authorise live payments, Apply enablement, or marketing as a live agent bank.

---

## 6. Commercial packaging (indicative)

| Tier | Contents (conceptual) |
|------|------------------------|
| Observatory | Read-only protocol + audit (today) |
| Governance | Policies + Apply-disabled workflows (today) |
| Treasury (future) | Org/dept/project budgets, allocations, journals, ERP export |
| Regulated (future) | Hook integrations (KYC/AML/sanctions) with customer-supplied vendors |

---

## 7. Success metrics (post-implementation)

- % of agent spend with full attribution chain  
- Mean time to emergency stop  
- Unused funds recovery rate at budget expiry  
- Duplicate payment rate ≈ 0 (idempotency)  
- Auditor export completeness  

---

## 8. Risks to the business case

| Risk | Response |
|------|----------|
| Perceived as “crypto wallet product” | Fiat-first messaging; AETH optional |
| Competing with banks/ERP | Integrate, don’t replace GL |
| Premature Apply enablement | Keep mutation gates separate |
| Compliance scope creep | Extension points only until separately sold |

---

## 9. Recommendation

**Proceed to design freeze acceptance.** Do **not** start implementation until:

1. Open questions in `TREASURY_OPEN_QUESTIONS.md` are triage-decided,  
2. OPEN threats in the threat model have owners,  
3. A separate implementation phase is explicitly approved.

Treasury is the correct next **product** capability after Governance Console — still subordinate to protocol stability and Apply-disabled production posture.
