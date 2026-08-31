# Regulatory Positioning — Phase 31.5

| Field | Value |
|-------|--------|
| **Document** | `REGULATORY_POSITIONING.md` |
| **Phase** | 31.5 — Regulatory Alignment & Agent Runtime Hardening Review |
| **Status** | **DESIGN REVIEW — FROZEN** |
| **Date** | 2026-08-01 |
| **Scope** | Positioning only — no code, providers, Apply, rails, or protocol changes |
| **Inputs** | Mills Review analysis (chat), Phases 26–31 implementations, treasury C3 |
| **Companions** | [AGENT_RUNTIME_HARDENING_REVIEW.md](AGENT_RUNTIME_HARDENING_REVIEW.md), [PROVIDER_INTEGRATION_SECURITY_GATE.md](PROVIDER_INTEGRATION_SECURITY_GATE.md) |

---

## Executive summary

Aether is positioned as **enterprise AI governance and control infrastructure**, not as a retail financial service, payment institution, custodian, or advice firm.

Current build posture (Phases 26–31) supports that claim: identity, sessions, tool decision gateway, E2 enforcement, sandbox simulation, treasury observation/ledger governance — with **Apply disabled**, **no external model providers**, **no real tool execution**, **no PROTO-0 mutation**, and **no money movement**.

The Mills Review (*AI and the future of retail financial services*, July 2026) is **not binding FCA Handbook**. It recommends that the FCA develop agentic-finance foundations. Aether’s controls align **directionally** with those foundations (identity, delegated authority, audit, fail-closed spend checks). They do **not** constitute FCA authorisation, Consumer Duty compliance evidence, or a “Mills-certified” product.

---

## 1. What Aether is

| Statement | Meaning |
|-----------|---------|
| **Governance control plane** | Answers “may this registered agent perform this action?” with ALLOW / DENY / REQUIRES_REVIEW |
| **Enterprise spend-control architecture** | Org treasury → allocation → caps → E2; agents never hold title to funds |
| **Agent runtime identity & session layer** | CP-local registry + sessions + L3 placeholder credentials |
| **Tool permission & decision gateway** | Registry-backed evaluation; `tools_executed = false` |
| **Lab sandbox** | Simulated Agent A / Agent B loops with audit reconstruction |
| **Audit substrate** | Correlated events for identity, session, tool, enforcement, sandbox |

```text
Organisation (accountable)
    → registers agent principal
    → allocates authority (caps / treasury allocation)
    → agent requests action
    → Aether gateway + E2 decide
    → (future) governed side effect only if separately authorised
```

---

## 2. What Aether is not

| Aether is not | Why it matters |
|---------------|----------------|
| An FCA-authorised firm (by virtue of this software alone) | Permissions attach to legal persons and activities, not codebases |
| A retail bank, EMI, payment institution, or custodian | No client money, safeguarding, or payment rails |
| A consumer advice / arranging product | No personalised regulated advice channel |
| An “agent wallet” or agent-owned balance | Treasury ownership rules forbid agent title |
| A payment initiator or open-banking TPP | No live payment APIs |
| A certified Consumer Duty / SM&CR evidence pack | Duty and SM&CR apply when a firm is in perimeter and serves consumers |
| A Mills Review compliance badge | Mills advises the FCA; it does not certify products |

**Comms rule:** Do not market lab budgets (“£1 for Claude”) as title, safeguarded funds, or FCA-approved agent money.

---

## 3. Regulatory boundary — governance vs financial execution

| Plane | Aether today | Crossing into financial execution |
|-------|--------------|-----------------------------------|
| **Governance** | Identity, session, tool decision, E2, audit, sandbox simulate | Remains governance |
| **Protocol authority** | PROTO-0 observation only | Apply / `proto0_write` when enabled |
| **Treasury** | Internal journal + allocations; sandbox uses `simulated_budget_minor` metadata | Fiat/crypto movement, custody, ERP settlement |
| **Tools** | Decision-only + simulated SUCCESS | Real HTTP / vendor / purchase / transfer handlers |
| **Models** | Not connected | Provider adapters that propose intents only |

**Hard distinction (normative):**

> Aether may **decide** and **record**. It must not **move money**, **custody assets**, or **execute regulated financial actions** unless a later phase obtains explicit product, security, **and legal** authorisation.

---

## 4. Features that trigger FCA / legal review

Any of the following **must** stop engineering and open a legal/regulatory gate (counsel + product) before implementation or go-live:

| Trigger ID | Feature / change | Likely concern areas |
|------------|------------------|----------------------|
| **REG-T01** | Live payments, open banking, card, ACH, FPS, crypto rails | PSR, EMI, payment initiation, fraud/APP |
| **REG-T02** | Custody, client money, safeguarded balances, e-money | CASS / EMI / crypto custody (jurisdiction-specific) |
| **REG-T03** | Autonomous spending with real funds (Apply-on + rails) | Authorised activity + Consumer Duty outcomes |
| **REG-T04** | Consumer-facing advice / arranging / promotions via agents | PERG / COBS / MCOB / CONC / financial promotions |
| **REG-T05** | Uncontrolled or shadow tool execution bypassing gateway/E2 | Accountability break; perimeter opacity |
| **REG-T06** | Agent wallets or agent-titled balances | Ownership / liability / safeguarding mischaracterisation |
| **REG-T07** | Marketing Aether as regulated advice, payments, or “FCA compliant agent bank” | Misrepresentation; perimeter expectations |
| **REG-T08** | Production fiat/crypto movement | Existing treasury **C3** — counsel before movement |
| **REG-T09** | Cross-border consumer agent products | Local licensing + Mills-style perimeter pressure |

**Non-triggers (alone):** connecting a model provider **behind** Aether governance for lab inference; simulated tools; read-only treasury observatory; decision-only E2/gateway.

---

## 5. Mills alignment (directional)

| Mills foundation | Aether posture | Assessment |
|------------------|----------------|------------|
| Agent identity | Runtime registry + sessions + L3 placeholder | Directionally aligned (lab) |
| Authorisation / delegation | Caps + tool registry + E2 + fail-closed | Directionally aligned |
| Audit / monitoring / revocation | Audit events + reconstruct + freeze/disable | Directionally aligned |
| Payments infrastructure | Explicitly absent | Correct avoidance |
| Liability / redress | Not a consumer product yet | Deferred — required before retail |
| Consumer digital ID linkage | Not built | Expected gap; largely ecosystem |

Autonomy spectrum: Aether remains **assistive / constrained lab** (human/operator accountability; simulated execution). Approver/Observer modes are **out of scope** until REG triggers are cleared.

---

## 6. Explicit freezes (Phase 31.5)

Until a later phase **explicitly** lifts each item:

| Freeze | Status |
|--------|--------|
| Apply enablement | **FROZEN OFF** |
| Real tool execution | **FROZEN** |
| Custody / payment rails | **FROZEN** |
| Treasury mutation expansion (beyond existing authorised write façade scope) | **FROZEN** |
| Autonomous financial decisions with real funds | **FROZEN** |
| External model adapters in production | **FROZEN** (lab adapters only under [PROVIDER_INTEGRATION_SECURITY_GATE.md](PROVIDER_INTEGRATION_SECURITY_GATE.md)) |

---

## 7. Safe claims vs unsafe claims

| Safe | Unsafe |
|------|--------|
| “Enterprise governance for AI agent actions” | “FCA-authorised agentic bank” |
| “Fail-closed enforcement before spend-shaped tools” | “Consumer Duty compliant” (without firm + evidence) |
| “Aligned with Mills’ identity/authority/audit themes” | “Mills-certified / FCA-approved AI agent” |
| “Lab simulated budgets only” | “Agents hold £1 of customer money” |

---

## 8. Document control

| Item | Rule |
|------|------|
| Positioning changes | Require Phase review + update of this doc |
| Crossing REG-T01…T09 | Legal review mandatory; engineering alone insufficient |
| Relationship to product GTM | GTM must cite this boundary or obtain superseding counsel memo |

**STOP — positioning review only.**
