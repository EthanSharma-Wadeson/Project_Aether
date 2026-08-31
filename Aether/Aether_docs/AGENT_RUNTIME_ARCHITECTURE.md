# Agent Runtime Architecture — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `AGENT_RUNTIME_ARCHITECTURE.md` |
| **Phase** | 27 — Agent Runtime Integration Gate |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Inputs** | [ENFORCEMENT_RUNTIME_MODEL.md](ENFORCEMENT_RUNTIME_MODEL.md), [APPLY_TREASURY_ENFORCEMENT_MODEL.md](APPLY_TREASURY_ENFORCEMENT_MODEL.md), [TREASURY_ARCHITECTURE.md](TREASURY_ARCHITECTURE.md), [AGENT_IDENTITY_MODEL.md](AGENT_IDENTITY_MODEL.md), [AGENT_TOOL_PERMISSION_MODEL.md](AGENT_TOOL_PERMISSION_MODEL.md) |
| **Apply / PROTO-0 / Treasury writes** | **None** in this phase |
| **Production agent execution** | **Forbidden** in this phase |

---

## Executive Summary

External AI systems (Claude, Gemini, OpenAI, local models, enterprise agents) are **workloads that act under Aether-governed agent principals**. The model provider is replaceable; **identity, capability, treasury allocation, policy, and E2 enforcement are not**.

Aether never lets a provider SDK, API key, or prompt reach Treasury or PROTO-0 directly.

```text
Model provider (Claude | Gemini | OpenAI | local | enterprise)
        │  inference only
        ▼
Agent Runtime Adapter (future)     ← provider-agnostic tool loop
        │  typed action intent + agent authn
        ▼
Control Plane
        │
        ├─► E2 Enforcement evaluate  → ALLOW | DENY | REQUIRES_REVIEW
        │
        ├─► Tool gateway (future)    → execute only if ALLOW (+ post-checks)
        │
        ├─► E1 human review (future) → if REQUIRES_REVIEW
        │
        └─► Apply (future only)      → PROTO-0 mutation path; never auto from agent
```

**STOP:** No adapters, no provider API keys in-repo, no production loops, no Apply enablement in Phase 27.

---

## 1. Design goals

| Goal | Meaning |
|------|---------|
| Provider-agnostic governance | Same agent_id + caps + allocation regardless of LLM vendor |
| Fail-closed spend & tools | No ALLOW without E2 (and policy) for governed actions |
| No agent wallets | Organisation owns funds; agents receive allocations only |
| Human accountability | Capability/treasury authority changes via governance + Apply (future) |
| Auditable loop | Every tool attempt correlates to `request_id` + enforcement decision |

---

## 2. Runtime components (logical)

| Component | Responsibility | Must not |
|-----------|----------------|----------|
| **Model provider client** | Tokens in / tokens out | Hold treasury credentials; call CP as admin |
| **Agent Runtime Adapter** | Map provider tool-calls → Aether action intents; attach agent auth | Bypass E2; invent capabilities |
| **Agent credential** | Authenticate the **registered agent principal** (not the LLM vendor) | Be confused with org operator JWT |
| **E2 Enforcement** | Decide ALLOW/DENY/REVIEW (Phase 26 exists) | Mutate PROTO-0 / treasury |
| **Tool gateway** | Execute allowlisted tools after ALLOW | Execute on DENY; widen permissions |
| **Session / loop governor** | Rate, budget, recursion limits | Unlimited tool loops |
| **Audit** | Persist intents, decisions, tool results | Drop correlation ids |

---

## 3. End-to-end action flow

```text
1. User / orchestrator starts session bound to agent_id + organisation_id
2. Model proposes tool call (e.g. send_email, purchase, allocate, http_request)
3. Adapter normalises → EnforcementRequest (action, asset, amount, …)
4. POST enforcement/evaluate (or in-process E2) — fail-closed
5. Branch:
     DENY           → return structured deny to model; no tool side effects
     REQUIRES_REVIEW→ queue E1 case; tool waits or aborts per policy
     ALLOW          → tool gateway executes; optional post-action attest
6. Audit ENFORCEMENT_* + TOOL_ATTEMPT / TOOL_RESULT
7. Model continues with tool result (never with elevated secret material)
```

### 3.1 Example intents

| Model says | Normalised action (illustrative) | Expected E2 posture (sandbox) |
|------------|----------------------------------|-------------------------------|
| “Send email” | `comms.email.send` | Capability + policy; usually non-spend |
| “Buy software subscription” | `purchase.subscription` | Spend-scoped: cap ∧ allocation ∧ policy |
| “Allocate £1 from treasury” | `treasury.allocate` / spend | **DENY** for agent self-service allocation increase unless operator path |
| “Call external API” | `http.request` | Allowlist host/method; capability-bound |

---

## 4. Model provider abstraction

```text
                ┌─────────────────────────────┐
                │   ProviderAdapter trait     │
                │  - complete(messages)       │
                │  - stream (optional)        │
                │  - parse_tool_calls()       │
                └─────────────▲───────────────┘
                              │
     ┌────────────┬───────────┼───────────┬────────────┐
     │            │           │           │            │
 Anthropic     Google      OpenAI     Local/HF    Enterprise
  Claude       Gemini      API        runtime      internal
```

| Provider | Role in Aether | Secrets |
|----------|----------------|---------|
| Anthropic Claude API | Inference + native tools | Org-held API key in secret store — **not** agent wallet |
| Google Gemini API | Same | Same |
| OpenAI API | Same | Same |
| Local / self-hosted | Same interface | No vendor key; still agent principal auth to CP |
| Internal enterprise agents | May skip LLM; still emit intents | Workload identity / mTLS |

**Invariant:** Switching Claude → Gemini must not change `agent_id`, allocation, or capabilities. Only the inference backend changes.

---

## 5. Relationship to existing systems (normative)

```text
Agent request
    ↓
Enforcement E2          ← Phase 26 (evaluate only today)
    ↓
Treasury allocation check (read)
    ↓
Policy check
    ↓
(Proposal)              ← E3 future; non-executing
    ↓
Human approval          ← E1 future
    ↓
Apply (future only)     ← PROTO-0 mutation; never agent-triggered auto-execute
```

| System | Runtime may | Runtime must not |
|--------|-------------|------------------|
| E2 | Call evaluate on every governed tool | Treat evaluate as execution |
| Treasury | Read allocation remaining | Reserve/settle/fund from agent path without separate gated ops |
| PROTO-0 | Read identity/caps | Mutate |
| Apply | Never (until separately enabled) | Auto-execute from tool loop |
| Operator Console | Observe sessions / denies | Be replaceable by the LLM |

---

## 6. Session & loop model (design)

| Control | Purpose |
|---------|---------|
| `session_id` | Bind transcript + tool attempts |
| `max_tool_calls` / `max_depth` | Stop runaway loops |
| `session_spend_ceiling` | Cap cumulative spend per session ≤ allocation remaining |
| `idle / wall-clock TTL` | Expire sessions |
| `tool result redaction` | Strip secrets from model-visible output |

---

## 7. Sandbox vs production

| Mode | Allowed | Forbidden |
|------|---------|-----------|
| **Design (this phase)** | Documents only | All runtime code / keys |
| **Future lab sandbox** | Simulated tools, £1 budgets, mock purchases | Real rails, Apply on, external money |
| **Production** | After separate gates | Agent wallets; provider→treasury; auto Apply |

See testing model in [AGENT_RUNTIME_SECURITY_GATE.md](AGENT_RUNTIME_SECURITY_GATE.md) and identity/tool docs.

---

## 8. Explicit prohibitions

- Agents having wallets or legal title to funds  
- Agents bypassing Aether (direct vendor→bank/treasury)  
- Direct Claude/Gemini/OpenAI → treasury access  
- Automatic capability escalation from prompts or tool results  
- Automatic Apply execution from the agent loop  
- Production agent execution under this phase  

---

## 9. Freeze statement

Provider-agnostic adapter + mandatory E2-before-tool + no agent wallets are **frozen**. Implementation requires a separate phase after the security gate conditions are closed.

**This document does not authorise implementation.**
