# PHASE_2_WEDGE_DECISION.md — First Real-World Use Case Gate

## Status

**Strategic architecture gate — decision record; no implementation authorised**

Date: 2026-07-28  
Owner: Project Lead  

Prerequisites met:

| Gate | Status |
|------|--------|
| PROTO-0 / 1 / 2 | Complete (183 tests) |
| PROTO-NET-0 | Complete + security **APPROVE** |
| Architecture / threat / business alignment docs | Complete |

This document chooses the **smallest valuable workflow** Aether should optimise toward next. It does not implement code, open PROTO-4, or start reputation / marketplace work.

Related: [BUSINESS_ALIGNMENT.md](BUSINESS_ALIGNMENT.md), [PROTO_NET_0_SECURITY_REVIEW.md](PROTO_NET_0_SECURITY_REVIEW.md), [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) DEC-P2-006.

---

## 1. Decision Question

> What is the smallest real-world workflow where Aether’s **existing** primitives create a clear advantage over IAM, API keys, and platform escrow — and what is the **one next experiment** that tests that advantage?

Current stack (validated locally):

```text
Application        [not built]
Coordination       [not built]
Transport          PROTO-NET-0 ✓
Economic           PROTO-2 ✓ (simulated)
State              PROTO-1 ✓ (local)
Authority          PROTO-0 ✓
```

---

## 2. Option A — Enterprise Agent Spend Control

### Problem

Organisations deploying AI agents need to let software act without giving it a human’s unrestricted account. They need:

- controlled spending and action limits  
- delegated, revocable authority  
- audit trails for compliance  
- policy enforcement without a human click per action  

Pain is **immediate**: agent fleets are already being piloted; security/compliance teams block or constrain autonomy because tools are human-centric (OAuth, corporate cards, vendor API keys).

### Customer

| Role | Motivation |
|------|------------|
| AI / platform engineering lead | Ship agents without unbounded risk |
| Security / GRC | Prove who authorised what, within what limits |
| Finance / FinOps | Cap spend; reconcile agent-driven costs |

**Buyer type:** Enterprise (buys products; protocol is the differentiating substrate).  
**Design partner shape:** One company with 5–50 internal agents and a clear spend policy.

### Aether mapping

```text
Human / organisation (principal)
        │
        ▼
PROTO-0  CapabilityGrant (max_spend, actions, expiry, revoke)
        │
        ▼
AI Agent (operational key)
        │
        ▼
PROTO-2  Escrow / receipt / fee budget  (authorised economic action)
        │
        ▼
PROTO-4  Settlement binding → real ledger / payment adapter  [MISSING]
        │
        ▼
Audit trail (signed CBOR history) for compliance
```

Optional later: PROTO-NET-0 for multi-host agent fleets; PROTO-1 for high-frequency internal state.

### Why existing tools are insufficient

| Tool | Gap vs Aether |
|------|----------------|
| IAM / OAuth | Human sessions; scopes not portable machine capabilities with spend + narrowing |
| Vendor API keys | Platform-owned; no cross-vendor portable `AgentId` |
| Corporate cards / Stripe | Payment without cryptographic policy envelope; weak agent-native audit |
| Custom internal limits | Rebuilt per system; not interoperable or dual-party verifiable |

**Advantage only if** agents must act autonomously **and** authority must be delegable/auditable **and** (eventually) verifiable outside one vendor’s logs.

### Minimum required features (wedge MVP)

1. PROTO-0 grants with spend/action limits (exists)  
2. PROTO-2 escrow or spend reservation with receipts (exists, simulated)  
3. **SettlementBinding + one external adapter** (missing → PROTO-4)  
4. Exportable audit of signed events (mostly exists; packaging needed)  
5. Revoke / freeze that stops further economic ops (exists locally)

### Missing pieces

| Missing | Priority for this wedge |
|---------|-------------------------|
| PROTO-4 settlement adapter (enterprise ledger or payment API) | **P0** |
| SettlementBindingV0 / FinalityStageV0 | **P0** |
| Operator-facing Control Plane UI | Defer (demo CLI/script OK) |
| Multi-host NET | Nice-to-have after ledger proof |
| Reputation | Not required for single-tenant spend control |

### Business viability

- **Pain:** High and growing  
- **Willingness to pay:** Highest of candidates (security + FinOps budgets)  
- **Path to revenue:** Product (Control Plane) on protocol moat — later  
- **Risk:** Enterprises buy products, not protocols; need a thin demo surface soon after PROTO-4  

---

## 3. Option B — Agent-to-Agent Economy

### Problem

Autonomous agents may need to discover peers, negotiate work, hire specialists, exchange value, and accumulate trust — without a central marketplace owning identity and escrow.

### Customer / user

| Actor | Motivation |
|-------|------------|
| Marketplace / agent-network builders | Interoperable trust + settlement |
| Research / open multi-agent systems | Coordination without orchestrator as trust root |
| Specialist agent operators | Get paid for bounded work |

**Buyer type:** Early (protocol-native startups, labs). Near-term willingness to pay is **lower** than enterprise.

### Aether mapping

```text
Research Agent
        │
PROTO-NET-0  Discover + session
        │
Negotiate (missing: NegotiationTermsV0)
        │
Specialist Agent
        │
PROTO-2  Escrow + SettlementReceipt
        │
Settlement (missing: real backend)
        │
Reputation (missing: PROTO-3)
```

### Market timing

- Vision fits Aether’s long-term thesis strongly  
- Open agent-to-agent markets are **early**; orchestrators (LangGraph, etc.) still absorb coordination  
- Incumbent marketplaces prefer lock-in; may not adopt portable trust soon  

### Technical requirements beyond today

| Need | Status |
|------|--------|
| Authenticated messaging | PROTO-NET-0 ✓ (local) |
| Networked escrow / channels | Missing (NET + PROTO-1/2 integration) |
| Real settlement | Missing (PROTO-4) |
| Negotiation schema | Missing |
| Reputation / Sybil resistance | Missing (PROTO-3 + bonds) |
| Distributed discovery | Missing (directory is local only) |

### Adoption barriers

- Cold-start: who are the first paying counterparties?  
- Trust without reputation is weak for open hire  
- Legal/tax treatment of agent-to-agent payments unclear  
- Competitors can ship “agent API + Stripe Connect” faster as closed platforms  

### Business viability

- **Strategic value:** Highest long-term  
- **Near-term revenue:** Low  
- **Speed to credible customer demo:** Slower — needs NET + escrow + preferably reputation  

---

## 4. Evaluation Framework

Scores 1–10 (higher = better for choosing as **next** wedge). “Missing infrastructure” inverted: higher = less missing.

| Category | Option A — Enterprise spend | Option B — Agent-to-agent |
|----------|----------------------------:|--------------------------:|
| Immediate customer pain | **9** | 5 |
| Willingness to pay | **8** | 4 |
| Fit with current architecture | **8** | 7 |
| Amount of missing infrastructure (↑ = less missing) | **7** | 3 |
| Speed to first prototype | **8** | 5 |
| Competitive advantage | **7** | **8** |
| Long-term strategic value | 7 | **9** |
| **Total** | **54** | **41** |

**Reading:** Option A wins on pain, pay, and speed. Option B wins on long-term strategic purity and differentiation — but needs more missing layers before a customer-facing experiment is honest.

PROTO-NET-0 being done does **not** flip the score: enterprise buyers still need **real ledger binding** more than peer TCP. Agent-economy buyers need NET + settlement + trust stack.

---

## 5. Minimum Viable Protocol Extension

### Option A — smallest next prototype

```text
PROTO-4 — Settlement binding + one external payment / ledger adapter
```

**Scope (design intent only):**

- `SettlementBindingV0`: link `AgentId` → backend account  
- `FinalityStageV0`: expose soft vs settlement-final stages  
- One adapter: **enterprise/internal ledger stub or payment-API sandbox** (not multi-chain)  
- Wire PROTO-2 terminal outcomes to adapter (fund/release/refund → ledger entries)  
- Preserve: capability gate before economic ops; `hard_settlement_placeholder` becomes meaningful only on adapter confirm  

**Explicitly not in PROTO-4:**

- Tokens / L1  
- Reputation  
- Marketplace UI  
- DHT discovery  
- Full Control Plane product  

### Option B — smallest next prototype

```text
Networked PROTO-2 (over PROTO-NET-0) — two processes, hire → receipt → release
```

**Scope:**

- Carry signed escrow messages in `MessageEnvelopeV0`  
- Two independent `SessionStore`s + `EscrowStore`s  
- Still simulated balances **unless** PROTO-4 done first  

Honest assessment: without settlement, this proves **distributed coordination**, not economic reality. For Option B’s story, both NET integration **and** settlement eventually matter; doing NET-escrow first is the purest “agent economy” spike but weaker for paying customers.

---

## 6. Success Criteria (Recommended Path)

Assuming **Option A** (see §8).

### Technical success

1. Agent with bounded `CapabilityGrant` can only trigger settlement within limits  
2. Over-limit / revoked / frozen identity fails closed before adapter debit  
3. Escrow release/refund maps to deterministic ledger entries (adapter)  
4. Audit export: signed capability + escrow + settlement references for one workflow  
5. Existing PROTO-0/1/2/NET regression remains green  

### Business success

1. Named persona: e.g. “Platform eng lead at company deploying internal research/support agents”  
2. Pain measurable: “cannot give agents spend without sharing root cloud/billing credentials”  
3. Demo in &lt;15 minutes: grant → agent spend → receipt → ledger line → revoke stops next spend  
4. Design partner feedback: would they trial vs building ad hoc limits?  

---

## 7. Risks

### Technical

| Risk | Mitigation |
|------|------------|
| Adapter choice biases protocol | Keep settlement-agnostic; one spike, not lock-in |
| Soft vs hard finality confusion | Explicit `FinalityStageV0` in PROTO-4 |
| Skipping NET then needing it for multi-host fleets | Revisit after PROTO-4; NET-0 already exists |

### Adoption

| Risk | Mitigation |
|------|------------|
| Enterprises want UI/product not library | Thin demo harness; Control Plane later |
| “Just use vendor IAM” objection | Demo portable grant + revoke + signed audit across two runtimes |

### Competitive

| Risk | Mitigation |
|------|------------|
| Cloud vendors ship agent spend controls | Differentiate on portable crypto authority + dual-party evidence, not dashboards |
| Closed platforms ignore protocol | Start with internal enterprise design partner, not marketplace cold-start |

### Regulatory

| Risk | Mitigation |
|------|------------|
| Payment / money-transmitter rules | Prefer internal ledger or licensed partner sandbox for PROTO-4; no token issuance |
| Audit/compliance claims overstated | Demo “cryptographic audit trail,” not legal certification |

### Dependency

| Risk | Mitigation |
|------|------------|
| Design partner unavailable | PROTO-4 can still use synthetic ledger; business success criteria wait on partner |
| PROTO-4 blocked on DEC-P2-003 | Shortlist two adapters; spike one |

---

## 8. Proven vs Assumed

### Proven (tests / reviews)

- PROTO-0 fail-closed identity + capability authority  
- PROTO-1 dual-signed state + dispute chain rules (local)  
- PROTO-2 escrow/receipt/fee conservation (simulated)  
- PROTO-NET-0 authenticated session + envelope (local); security **APPROVE**  
- Composition works in one process  

### Assumed (needs real-world validation)

- Enterprises will adopt protocol-shaped spend control vs vendor-native features  
- One settlement adapter is enough to prove value  
- Signed audit trails matter to GRC buyers beyond existing SIEM logs  
- Agent-to-agent open markets will pay for portable trust (deferred hypothesis)  
- Logical-time and local-revocation models survive multi-host deployment  

---

## 9. Final Recommendation

### Recommended Path

**Enterprise Agent Spend Control**

### Why this is the best next move

1. **Highest near-term pain and willingness to pay** among evaluated wedges  
2. **Least missing infrastructure** — PROTO-0 + PROTO-2 already encode the policy/economic core; the gap is **real settlement binding**  
3. **Fastest honest demo** — does not require reputation, open discovery, or a marketplace cold-start  
4. **Preserves long-term optionality** — settlement-agnostic PROTO-4 still serves Option B later; PROTO-NET-0 remains ready for multi-host fleets  
5. **Matches unique value rule** — autonomous action + delegable/auditable authority; third condition (no shared platform) can start as single-tenant verifiable audit and expand  

Option B remains the **strategic north star**, not the **next experiment**. Building open agent economy first optimises for vision while delaying contact with a paying problem.

### What should be built next

After explicit acceptance of this gate:

1. Freeze PROTO-4 design + acceptance tests (settlement binding + **one** adapter)  
2. Implement PROTO-4 spike  
3. Demo: capability-bounded agent spend → escrow/receipt → ledger finality stage → revoke  

### What should explicitly NOT be built yet

- Reputation (PROTO-3)  
- Marketplace / Control Plane product UI  
- Tokens, public L1, multi-backend production settlement  
- DHT / open discovery  
- Full networked PROTO-1 mesh  
- Negotiation / reputation-gated hire flows  

---

## 10. Decision Lock

| Field | Value |
|-------|-------|
| Chosen wedge | **Enterprise Agent Spend Control** |
| Next prototype | **PROTO-4** (SettlementBinding + one adapter) |
| Deferred wedge | Agent-to-Agent Economy (after settlement evidence) |
| DEC-P2-006 update | Prefer **B (PROTO-4)** as next build; NET-0 already done |

**Implementation starts only after** this document is explicitly accepted.

---

## Freeze Statement

> This is a strategic architecture gate. It determines what Aether builds next. It does not authorise PROTO-4 coding, reputation, marketplace systems, or tokens until acceptance is recorded.
