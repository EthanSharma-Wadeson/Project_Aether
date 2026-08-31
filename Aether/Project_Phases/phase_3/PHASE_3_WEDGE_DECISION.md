# Phase 3 Wedge Decision — First Control Plane Product

## Status

**Product decision gate — no implementation authorised**

Date: 2026-07-29

Prerequisites met:

| Gate | Status |
|------|--------|
| PROTO-0/1/2 | Implemented (183 tests) |
| PROTO-NET-0 | Implemented + security APPROVE |
| PROTO-4 | Implemented + security APPROVE WITH DOCUMENTED LIMITATIONS |
| PROTO-3 | Implemented + security APPROVE WITH DOCUMENTED LIMITATIONS (34 tests) |
| Enterprise Spend Control demonstrator | Complete (4/4 scenarios) |
| Control Plane architecture | Complete (4 documents) |

Total: 311 protocol tests pass.

Related: [CONTROL_PLANE_ARCHITECTURE.md](CONTROL_PLANE_ARCHITECTURE.md), [CONTROL_PLANE_BUSINESS_CASE.md](CONTROL_PLANE_BUSINESS_CASE.md), [PHASE_2_WEDGE_DECISION.md](../phase_2/PHASE_2_WEDGE_DECISION.md)

---

## 1. Decision Question

> What is the smallest Control Plane workflow that proves Aether's protocol stack solves a real problem better than existing tools?

---

## 2. Option A — Enterprise Agent Governance Console

### User Pain

| Dimension | Assessment |
|-----------|-----------|
| **Who** | Enterprise AI/platform engineering leads, security teams, FinOps |
| **Frequency** | Daily — every agent deployment requires authority configuration |
| **Pain intensity** | High — agents are blocked or unconstrained; no middle ground |
| **Existing solutions** | IAM (coarse-grained, no economic limits), API key management (no delegation hierarchy), internal dashboards (no cryptographic evidence) |

Enterprises are deploying AI agents today. The governance gap is immediate: IAM systems control *access* but not *economic authority*. A corporate API key grants all-or-nothing access to a service — there is no way to say "this agent can spend up to £500 on compute, with a dispute window, and revocable by its principal."

PROTO-0 capabilities solve this exactly. The Phase 2 demonstrator proved it in CLI form. The Control Plane makes it usable.

### Willingness to Pay

| Factor | Score (1–10) | Rationale |
|--------|-------------|-----------|
| Enterprise budget availability | **9** | Security/compliance budgets are large and growing with AI adoption |
| Urgency | **8** | Agent deployments are happening now; governance is the blocker |
| Adoption friction | **4** | Requires integrating with existing identity/IAM; new mental model for capability-based auth |
| Sales complexity | **5** | Enterprise sales cycle; need champion in security or platform team |
| **Overall** | **7** | |

### Protocol Fit

| Score | 8/10 |
|-------|------|

PROTO-0 capabilities directly map to enterprise governance primitives. The entire Phase 2 stack (escrow, settlement, reputation) provides the economic evidence trail enterprises need for compliance. The Phase 2 demonstrator already proved the happy path.

### Missing Infrastructure

| Component | Status | Effort |
|-----------|--------|--------|
| REST API server | Not built | Medium |
| Protocol store readers (read-only adapters) | Not built | Low — stores have public read APIs |
| Policy template engine | Not built | Low — template → capability translation |
| Web dashboard | Not built | Medium |
| Authentication (CP-level) | Not built | Low — standard JWT |
| PROTO-0 key integration for write ops | Not built | Medium — HSM/key management design needed |

### Time to First Demo

**2–3 weeks** for a functional prototype that:
- Registers an enterprise with a root capability
- Displays agents and their capabilities
- Creates a spend policy and issues capabilities
- Shows escrow history and settlement evidence
- Displays reputation metrics

### Strategic Value

| Path | Fit |
|------|-----|
| Enterprise adoption | **Direct** — this IS the enterprise product |
| Autonomous agent economy | **Strong** — governance is prerequisite for enterprise trust in agent autonomy |
| Marketplace | **Moderate** — enterprise agents need governance before they can participate in open markets |
| Research ecosystem | **Weak** — researchers need observation, not governance |

---

## 3. Option B — AI Research Observatory

### User Pain

| Dimension | Assessment |
|-----------|-----------|
| **Who** | AI researchers, academic labs, agent framework developers |
| **Frequency** | Per-experiment — not daily operational pain |
| **Pain intensity** | Moderate — researchers currently use ad-hoc logging and custom scripts |
| **Existing solutions** | Weights & Biases (ML-focused, not agent economics), custom dashboards, Jupyter notebooks |

Researchers studying multi-agent economics lack standard tools. But the pain is episodic (per experiment) rather than continuous, and researchers are accustomed to building their own tooling.

### Willingness to Pay

| Factor | Score (1–10) | Rationale |
|--------|-------------|-----------|
| Budget availability | **3** | Grant-funded; prefer free/open-source tools |
| Urgency | **4** | Important but not blocking; workarounds exist |
| Adoption friction | **3** | Researchers are early adopters; lower friction |
| Sales complexity | **2** | Community adoption, not enterprise sales |
| **Overall** | **3** | |

### Protocol Fit

| Score | 7/10 |
|-------|------|

PROTO-3 reputation metrics and deterministic event logs are exactly what researchers need. But the research value depends on having enough agent activity to study — which requires enterprise adoption first.

### Missing Infrastructure

| Component | Effort |
|-----------|--------|
| Read-only API server | Low |
| Event query interface | Medium |
| Dataset export | Low |
| Visualisation | Medium–High |

### Time to First Demo

**1–2 weeks** for a read-only explorer over test data.

### Strategic Value

| Path | Fit |
|------|-----|
| Enterprise adoption | **Weak** — researchers don't become enterprise customers |
| Research ecosystem | **Direct** — builds academic community |
| Marketplace | **Weak** — does not advance marketplace infrastructure |
| Agent economy | **Moderate** — published research validates the economic model |

---

## 4. Option C — Agent Developer Debugging Platform

### User Pain

| Dimension | Assessment |
|-----------|-----------|
| **Who** | Developers building agents on Aether |
| **Frequency** | Per-bug — high during development, drops after deployment |
| **Pain intensity** | Moderate — currently requires reading raw store dumps and test output |
| **Existing solutions** | `cargo test` output, log files, custom debugging scripts |

Developer debugging pain is real but inherently temporary per project. And the developer audience only exists once Aether has adoption — a chicken-and-egg problem.

### Willingness to Pay

| Factor | Score (1–10) | Rationale |
|--------|-------------|-----------|
| Budget availability | **4** | Developer tools market is competitive; expect free tier |
| Urgency | **3** | Only urgent once developers are building on Aether |
| Adoption friction | **2** | Developers are the easiest adopters |
| Sales complexity | **2** | Self-serve or open-source |
| **Overall** | **3** | |

### Protocol Fit

| Score | 6/10 |
|-------|------|

Protocol traceability is useful for debugging, but the primary debugging tool is already `cargo test` with 311 passing tests. A visual debugger adds convenience, not capability.

### Missing Infrastructure

Same as Option B, plus interactive protocol event inspection.

### Time to First Demo

**1–2 weeks** for a protocol event viewer over test fixtures.

### Strategic Value

| Path | Fit |
|------|-----|
| Developer adoption | **Direct** — attracts developers |
| Enterprise adoption | **Weak** — developers don't sign enterprise contracts |
| Marketplace | **Weak** |
| Agent economy | **Weak** |

---

## 5. Comparison Matrix

| Criterion | A: Enterprise Governance | B: Research Observatory | C: Developer Debugging |
|-----------|-------------------------|------------------------|----------------------|
| Pain intensity | **High** | Moderate | Moderate |
| Pain frequency | **Daily** | Per-experiment | Per-bug |
| Willingness to pay | **7/10** | 3/10 | 3/10 |
| Protocol fit | **8/10** | 7/10 | 6/10 |
| Time to demo | 2–3 weeks | 1–2 weeks | 1–2 weeks |
| Strategic value | **Enterprise → economy** | Research community | Developer community |
| Revenue potential | **High** | Low | Low–Medium |
| Existing validation | **Phase 2 demonstrator** | None | 311 tests |

---

## 6. Recommendation

### Selected: Option A — Enterprise Agent Governance Console

**Why now:**
- Enterprise AI agent deployment is happening today
- Governance is the #1 blocker to enterprise adoption of autonomous agents
- The Phase 2 demonstrator already validated the protocol primitives in CLI form
- Converting CLI → dashboard is the natural next step

**Why Aether uniquely fits:**
- Cryptographic capability-based authority (not IAM access control)
- Economic evidence trail (escrow → settlement → finality, not just API logs)
- Deterministic reputation metrics (not reviews or ratings)
- Delegation hierarchy with narrowing (not flat API key permissions)

**Why alternatives are delayed:**
- **Option B (Research)** depends on having agent activity to observe — which requires enterprise adoption first
- **Option C (Debugging)** depends on having developers building on Aether — which requires a product to build on

The enterprise governance console creates the adoption foundation that makes B and C viable later.

---

## 7. MVP Scope — Enterprise Governance Console

### Core Workflows

| # | Workflow | Protocol dependency |
|---|---------|-------------------|
| 1 | Register enterprise with root capability | PROTO-0 identity + root authority |
| 2 | View agent inventory (id, status, capabilities) | PROTO-0 registry + capability store (read) |
| 3 | Create spend policy (max_spend, actions, expiry) | Control Plane internal + PROTO-0 capability template |
| 4 | Apply policy → issue capabilities to agents | PROTO-0 `grant_capability` |
| 5 | View escrow history per agent | PROTO-2 escrow store (read) |
| 6 | View settlement evidence per escrow | PROTO-4 settlement store (read) |
| 7 | View reputation metrics per agent | PROTO-3 reputation store (read) |
| 8 | Revoke capability or freeze agent | PROTO-0 revoke / freeze |
| 9 | Audit log of all governance actions | Control Plane internal |

### Not in MVP

- Multi-tenant isolation
- SSO/SAML integration
- Real-time streaming
- Custom alert rules
- Policy simulation
- Dataset export
- Marketplace integration
- AETH payments

### Architecture Dependencies

| Dependency | Source | Ready? |
|-----------|--------|--------|
| PROTO-0 identity + capabilities | `core/src/identity/`, `core/src/capability/` | Yes |
| PROTO-2 escrow lifecycle | `core/src/escrow/` | Yes |
| PROTO-4 settlement binding | `core/src/settlement/` | Yes |
| PROTO-3 reputation metrics | `core/src/reputation/` | Yes |
| REST API framework | Not built | Needs selection (e.g. Axum, Actix) |
| Web dashboard framework | Not built | Needs selection |
| CP authentication | Not built | Standard JWT |
| Key management for PROTO-0 writes | Not built | Design needed |

### Demo Scenario

The MVP demo should replicate the Phase 2 enterprise demonstrator scenario through the Control Plane:

1. Enterprise operator logs in
2. Sees agent fleet (Alice the payer, Bob the provider)
3. Creates spend policy: "Procurement — max £1,000 per task, compute actions only"
4. Applies policy to Alice → capabilities issued
5. Alice creates escrow with Bob, submits receipt, releases payment
6. Operator views escrow history, settlement evidence, reputation metrics
7. Operator revokes Alice's capability
8. Alice's next action is rejected — visible in dashboard

This is the Phase 2 `cargo run` demo, but through a human interface.

---

## 8. Implementation Boundary (Frozen)

When implementation is approved:

**Build:**
- REST API server with protocol store readers
- Policy template → capability issuance engine
- Web dashboard with 5 views (agent list, agent detail, policy console, escrow history, reputation)
- CP authentication + audit log
- Key management integration for PROTO-0 writes

**Do not build:**
- Marketplace
- Public agent directory
- Social reputation
- Governance DAO
- Token incentives
- AETH implementation
- Production cloud infrastructure
- Multi-tenant isolation
- Real-time event streaming

---

## 9. Success Criteria

The MVP is successful if:

1. An enterprise operator can register, view agents, create a policy, issue capabilities, and see economic outcomes — without touching protocol stores directly
2. Every governance action produces an audit trail
3. The Control Plane cannot bypass PROTO-0 capability enforcement
4. The demo replicates Phase 2 scenarios through the dashboard
5. No protocol code is modified

---

## Freeze Statement

> **Enterprise Agent Governance Console** is the Phase 3 product wedge. It converts the Phase 2 CLI demonstrator into a human-facing governance interface. Implementation requires explicit approval. No marketplace, no tokens, no governance DAO.
