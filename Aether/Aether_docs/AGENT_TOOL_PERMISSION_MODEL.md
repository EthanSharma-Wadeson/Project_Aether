# Agent Tool Permission Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `AGENT_TOOL_PERMISSION_MODEL.md` |
| **Phase** | 27 — Agent Runtime Integration Gate |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Companion** | [AGENT_RUNTIME_ARCHITECTURE.md](AGENT_RUNTIME_ARCHITECTURE.md), [ENFORCEMENT_RUNTIME_MODEL.md](ENFORCEMENT_RUNTIME_MODEL.md) |

---

## 1. Tool execution model

```text
Agent (model + runtime)
    → requests action (typed tool call)
    → Aether E2 enforcement
    → ALLOW / DENY / REQUIRES_REVIEW
    → tool execution (only on ALLOW, or after human approve on REVIEW)
```

**Fail-closed:** If enforcement is unavailable → **DENY** (no tool side effects).

---

## 2. Agent wallet principle (normative)

**Agents NEVER own money.**

```text
Organisation treasury
        ↓
Agent allocation          (non-owning spend authorisation)
        ↓
Capability permissions    (PROTO-0 — what actions exist)
        ↓
Runtime enforcement E2    (may this happen right now?)
        ↓
Approved action / tool
```

| Forbidden | Required |
|-----------|----------|
| Agent wallets / balances titled to agent | Org-owned treasuries |
| Direct provider → treasury SDK | E2 before spend tools |
| Model “decides” it has funds | `remaining_minor` + caps decide |

---

## 3. Permission stack (intersection)

An action proceeds only if **all** applicable layers pass:

| Layer | Question |
|-------|----------|
| Identity | Is `agent_id` active (not frozen/revoked)? |
| Capability | Is the action granted and within `max_spend` / asset? |
| Allocation | Is allocation active, unexpired, sufficient `remaining`? |
| Policy | Does CP policy allow / not block? |
| Risk | Velocity, allowlists, session ceilings |
| Tool allowlist | Is this tool registered for this agent? |
| Host / data allowlist | For `http.request`, is destination permitted? |

Effective spend: `min(capability.max_spend, allocation.remaining, session_ceiling, policy.ceiling)`.

---

## 4. Tool classes

| Class | Examples | Spend-scoped? | Default posture |
|-------|----------|---------------|-----------------|
| **Research / read** | web.fetch (allowlisted), search | No | Capability + host allowlist |
| **Comms** | email.send | Usually no | Capability + recipient policy |
| **Analysis / report** | generate_report | No | Capability |
| **External API** | http.request | Maybe | Strict allowlist; deny SSRF patterns |
| **Purchase / spend** | purchase.*, subscription.* | **Yes** | Cap ∧ allocation ∧ E2 |
| **Treasury mutation** | allocate, fund, transfer_external | **Yes / ops** | **DENY for agents** — operator/treasury façade only |
| **Capability mutation** | grant/revoke self | N/A | **DENY** — Apply only (future) |

### 4.1 Example decisions (sandbox narrative)

| Request | Outcome | Why |
|---------|---------|-----|
| “Purchase approved API credits” (£1 budget, purchase cap) | **ALLOW** | Cap + allocation + allowlisted merchant SKU |
| “Transfer funds externally” | **DENY** | Unsupported / `UNSUPPORTED_ACTION` or policy block; no external rails |
| “Increase monthly budget” | **REQUIRES_REVIEW** | Authority change → E1; not agent self-serve |

---

## 5. Mapping provider tools → Aether actions

| Provider tool call (illustrative) | Aether `action` | Notes |
|-----------------------------------|-----------------|-------|
| `send_email` | `comms.email.send` | Non-spend unless billed add-on |
| `buy_subscription` | `purchase.subscription` | Requires amount + asset |
| `allocate_treasury` | *(rejected mapping)* | Agents cannot allocate; return DENY to model |
| `http_request` | `http.request` | URL allowlist mandatory |

Unknown tools → **DENY** (`UNSUPPORTED_ACTION`).

---

## 6. REVIEW path

`REQUIRES_REVIEW` means **no silent execution**:

1. Tool gateway holds or aborts  
2. E1 case created with evidence (future)  
3. Human approves/denies  
4. On approve: either re-evaluate E2 and execute, or run operator-only treasury/Apply path  
5. Agent never auto-escalates its own capabilities  

---

## 7. Testing model (future sandbox)

Design-only now; future lab (no real money, Apply off):

### Agent A — Claude API

| Field | Value |
|-------|--------|
| Budget | £1 allocation |
| Capabilities | research, API calls (allowlisted), small purchases |
| Provider | Anthropic (inference only) |

### Agent B — Gemini API

| Field | Value |
|-------|--------|
| Budget | £1 allocation |
| Capabilities | data analysis, reporting |
| Provider | Google (inference only) |

### Simulated tasks

| Task | Expected |
|------|----------|
| Purchase approved API credits | **ALLOW** (A) |
| Transfer funds externally | **DENY** (A/B) |
| Increase monthly budget | **REVIEW** |
| B attempts purchase | **DENY** (capability missing) |
| A exhausts £1 then purchases | **DENY** (`ALLOCATION_EXCEEDED`) |

Sandbox tools return **simulated** receipts; journal may record lab posts only if a separate lab treasury mode is approved — **not** authorised by this doc alone.

---

## 8. Explicit forbids (tool layer)

- Tool handlers that call Treasury write APIs without operator dual-control  
- Tools that call Apply / `proto0_write`  
- Tools that mint capabilities because the model “asked”  
- Passing provider API keys into tool environment as agent identity  
- Side-channel execution that skips E2 (“helpful” local shell)  

---

## 9. Freeze statement

Tools are allowlisted capabilities gated by E2; spend tools require allocation; agents never own wallets; budget increases are REVIEW/operator paths. Implementation is a future phase.

**This document does not authorise implementation.**
