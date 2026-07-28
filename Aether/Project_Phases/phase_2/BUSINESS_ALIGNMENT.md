# BUSINESS_ALIGNMENT.md — Protocol Value Mapping

## Status

**Analysis only — not a product specification**

Purpose: determine where Aether's protocol primitives create **unique value** relative to existing APIs and payment platforms, before choosing the first networked prototype.

The Control Plane, marketplace UI, and enterprise dashboards are **application layers** — out of scope.

---

## 1. What Phase 1 Enables Technically

| Primitive | Technical capability |
|-----------|---------------------|
| PROTO-0 | Machine identity with delegable, revocable, expiring permissions |
| PROTO-1 | Bilateral verifiable shared state between two agents |
| PROTO-2 | Conditional settlement from signed terms + receipt + timeout |

**Unique combination:** portable cryptographic authority + bilateral state agreement + rule-based economic release — designed for headless agents, not human accounts.

---

## 2. User Segment Analysis

### 2.1 Enterprise AI operators

**Profile:** Companies deploying autonomous agents internally or for customers. Need spend control, audit, and policy compliance.

| Need | Current solutions | Aether protocol value | Gap |
|------|-------------------|----------------------|-----|
| Agent permissions | IAM roles, API scopes, OAuth | Delegable capability envelopes with narrowing + expiry | **High** — IAM is human-centric, not machine-portable |
| Spending controls | Corporate cards, cloud billing alerts, custom limits | Capability `max_spend` + escrow terms + fee budget | **Medium** — exists in custom form; not interoperable |
| Audit trails | Cloud logs, SIEM | Canonical signed CBOR + dual-signed state history | **Medium** — stronger portability than platform logs |
| Cross-vendor agents | Locked to each vendor's API key model | Portable `AgentId` + capabilities | **High** — no standard today |
| Settlement | Internal ledger / Stripe | Settlement-agnostic binding (future) | **Low today** — PROTO-2 is simulated |

**Unique value proposition:**

> Policy-bound autonomous keys that work across agent runtimes, with auditable signed history — without giving each vendor root access to a human account.

**Best first wedge:** Enterprise agent spend control with capability-gated escrow.

**First prototype if this wedge:** PROTO-4 (settlement backend spike) + enterprise ledger adapter.

**Buyer:** Platform engineering / AI infrastructure lead.  
**Pain severity:** High and growing as agent deployment scales.  
**Willingness to adopt protocol vs product:** Enterprises buy **products**; protocol value is in avoiding lock-in. Control Plane (later) is the product; protocol is the moat.

---

### 2.2 Agent marketplaces

**Profile:** Platforms matching task posters with agent providers (compute, data, API work).

| Need | Current solutions | Aether protocol value | Gap |
|------|-------------------|----------------------|-----|
| Identity | Platform accounts, KYC | Portable `AgentId` | **High** |
| Trust | Platform ratings (opaque) | Evidence-backed reputation from protocol events | **High** — if PROTO-3 delivers |
| Settlement | Platform escrow (Upwork, Fiverr model) | Rule-based escrow + provider-signed receipt | **Medium** — similar concept, different trust root |
| Dispute | Platform support team | Deterministic local rule (limited) | **Low today** — platform dispute is richer |
| Discovery | Platform search | `AgentDirectoryV0` (future) | **Low today** — not built |

**Unique value proposition:**

> Agents that can prove identity, authority, and completion history across marketplaces — without each platform owning the trust graph.

**Best first wedge:** Interoperable agent identity + escrow between two marketplace participants.

**First prototype if this wedge:** PROTO-NET-0 (transport) + networked PROTO-2.

**Buyer:** Marketplace operator or protocol-first marketplace startup.  
**Pain severity:** Medium — platforms prefer lock-in.  
**Risk:** Incumbents add "agent API keys" faster than protocol adoption.

---

### 2.3 Autonomous software ecosystems

**Profile:** Multi-agent systems where agents coordinate without central orchestrator (research, open networks, agent swarms).

| Need | Current solutions | Aether protocol value | Gap |
|------|-------------------|----------------------|-----|
| Coordination | Central orchestrator (LangGraph, custom) | Bilateral channels + escrow primitives | **Medium** |
| Trust between agents | Orchestrator vouches | Capability + reputation | **High** — decentralised trust |
| Micro-payments | Not viable | Channels (future) + escrow | **High** — if settlement works |
| Discovery | Hard-coded agent lists | Directory / DHT (future) | **High** — not built |

**Unique value proposition:**

> Machine-native coordination substrate so agents can transact and delegate without a central platform as trust root.

**Best first wedge:** Two-agent hire→work→settle over network with portable identity.

**First prototype if this wedge:** PROTO-NET-0 → networked PROTO-1/2.

**Buyer:** Research lab or open-agent-network builder.  
**Pain severity:** High for vision; low near-term revenue.  
**Risk:** Ecosystems may tolerate central orchestrator longer than expected.

---

## 3. Competitive Landscape (Protocol Level)

| Capability | Stripe / PayPal | OAuth / IAM | Smart contract escrow | Aether (Phase 1) |
|------------|-----------------|-------------|----------------------|------------------|
| Machine identity | ❌ | Partial (API keys) | Address only | ✅ PROTO-0 |
| Delegable permissions | ❌ | Partial (scopes) | Allowlist only | ✅ PROTO-0 |
| Bilateral state | ❌ | ❌ | Limited (channels) | ✅ PROTO-1 local |
| Conditional settlement | ✅ (platform) | ❌ | ✅ (on-chain) | ✅ PROTO-2 simulated |
| Portable across platforms | ❌ | ❌ | Partial | Designed |
| Headless-native | Partial | Partial | Partial | ✅ |
| No platform trust root | ❌ | ❌ | Partial (chain) | Designed |

**Aether wins on composition for autonomous agents.**  
**Aether loses on deployment readiness, UX, legal rails, and liquidity.**

---

## 4. Credible Problem Assessment

### Is there a real problem?

**Yes.** The mismatch between human-centric identity/payment rails and autonomous software actors is structural. Phase 0 problem statement remains valid.

### Is Aether's Phase 1 approach credible?

**Yes, as a foundation.** 183 tests demonstrate rule engines work. That is necessary but not sufficient.

### Is there a customer worth continuing toward?

| Segment | Credible? | Near-term revenue? | Protocol fit |
|---------|-----------|---------------------|--------------|
| Enterprise AI operators | **Yes** | **Highest** | Strong — spend + policy |
| Agent marketplaces | Maybe | Medium | Strong — if they want openness |
| Open agent ecosystems | Yes (vision) | Low | Strongest protocol fit |

**Recommendation:** Optimise Phase 2 for **enterprise agent spend control** as the first design partner wedge, while keeping protocol settlement-agnostic for marketplace/ecosystem expansion.

---

## 5. Unique Value Summary

Aether creates unique value **only where all three apply**:

1. Agents must act **autonomously** (no human in the loop per action)
2. Authority must be **delegable, revocable, and auditable**
3. Counterparties must **verify trust without a shared platform operator**

If any condition is false, existing APIs and payment platforms are good enough.

---

## 6. Implications for Phase 2 Prototype Selection

| Wedge | First prototype | Rationale |
|-------|-----------------|-----------|
| Enterprise spend control | PROTO-4 → settlement binding | Buyers need real ledger, not TCP |
| Marketplace interoperability | PROTO-NET-0 → networked PROTO-2 | Agents must talk before they settle |
| Open agent ecosystem | PROTO-NET-0 → networked PROTO-1 | Coordination before economics |

**Decision gate:** DEC-P2-006 resolves after design partner conversations.

---

## 7. What Not to Build Yet

| Item | Why defer |
|------|-----------|
| Control Plane product | Needs wedge + networked proto first |
| Marketplace UI | Application layer |
| Token / native asset | No protocol need evidenced |
| Public blockchain deployment | Settlement-agnostic; spike first |
| Global decentralised discovery | Enterprise curated directory sufficient for pilot |

---

## 8. Success Criteria (Business Alignment)

Phase 2 business alignment succeeds when:

1. At least one user segment has a **named pain** mapped to specific protocol primitives
2. Unique value vs incumbents is articulable in one sentence
3. First prototype choice (DEC-P2-006) follows from segment choice
4. No product features are designed before wedge is chosen

---

## Freeze Statement

> This document maps protocol value to user segments. It does not define a product, pricing, or go-to-market plan.
