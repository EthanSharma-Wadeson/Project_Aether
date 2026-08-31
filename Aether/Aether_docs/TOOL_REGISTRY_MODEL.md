# Tool Registry Model — Phase 29 Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TOOL_REGISTRY_MODEL.md` |
| **Phase** | 29 — Tool Registry & Governed Tool Gateway Design |
| **Status** | **DESIGN FROZEN** — no implementation authorised in this phase |
| **Date** | 2026-08-01 |
| **Inputs** | [AGENT_TOOL_PERMISSION_MODEL.md](AGENT_TOOL_PERMISSION_MODEL.md), [AGENT_RUNTIME_PRECONDITIONS.md](AGENT_RUNTIME_PRECONDITIONS.md), [AGENT_REGISTRY_IMPLEMENTATION.md](AGENT_REGISTRY_IMPLEMENTATION.md), [ENFORCEMENT_RUNTIME_MODEL.md](ENFORCEMENT_RUNTIME_MODEL.md) |
| **Lab scope** | Registry + gateway **decision** design only |
| **Forbidden** | Provider APIs, real tool execution, Apply, PROTO-0 mutation, treasury writes, agent wallets |

---

## Executive Summary

AI agents may **request** actions only through a **governed tool layer**: registered tools, typed `ToolRequest`s, identity/session checks, then **E2 enforcement**. Phase 29 freezes that control surface.

**This phase does not execute tools after ALLOW** — decision and audit only (future lab simulation is a separate implementation phase).

```text
Agent → Runtime session → ToolRequest → Gateway (identity + registry + E2) → ALLOW|DENY|REQUIRES_REVIEW
                                                                              ↓
                                                                    (no execution yet)
```

---

## 1. Tool registry record

A **tool** is a first-class Control Plane object (org-scoped or platform-catalog with org enablement).

| Field | Type | Meaning |
|-------|------|---------|
| `tool_id` | string | Stable id (e.g. `research.search`) |
| `name` | string | Human label |
| `description` | string | Operator-facing purpose |
| `owner` | string | Team / system accountable for handler |
| `risk_level` | enum | `LOW` \| `MEDIUM` \| `HIGH` \| `CRITICAL` |
| `required_capability` | string \| list | Action selector(s) E2/capability must grant |
| `required_policy` | object \| null | e.g. must have approved policy type; blocked actions |
| `allowed_agents` | list \| `*` | Explicit allowlist of runtime `agent_id`s, or all active agents in org when `*` **and** capability matches |
| `status` | enum | `ACTIVE` \| `DISABLED` \| `RETIRED` |
| `spend_scoped` | bool | If true → E2 requires allocation + amount |
| `organisation_id` | string | Tenancy (no cross-org tool use) |
| `sandbox_simulation_only` | bool | Lab: handlers must be simulated |
| `parameter_schema` | JSON Schema | Validates `ToolRequest.parameters` |
| `created_at` / `created_by` | audit | Registration provenance |

### 1.1 Risk levels

| Level | Meaning | Default gateway posture (after identity + registry hit) |
|-------|---------|--------------------------------------------------------|
| **LOW** | Read / research, low blast radius | E2 → often **ALLOW** if caps/policy pass |
| **MEDIUM** | External I/O, bounded side effects | E2 → ALLOW or REVIEW per policy |
| **HIGH** | Spend / irreversible commercial | Spend-scoped E2; prefer REVIEW if ambiguous |
| **CRITICAL** | Production deploy / authority change | Default **REQUIRES_REVIEW**; often DISABLED in lab |

### 1.2 Examples

| tool_id | risk | required_capability | Notes |
|---------|------|---------------------|-------|
| `research.search` | **LOW** | `research.search` | Allowlisted sources later |
| `api.call.external` | **MEDIUM** | `http.request` / `api.call.external` | Host allowlist mandatory at exec phase |
| `purchase.subscription` | **HIGH** | `purchase.subscription` | Spend-scoped |
| `deploy.production` | **CRITICAL** | `deploy.production` | Lab: DISABLED or REVIEW-only |
| `code.analysis` | **LOW** | `code.analysis` | Gemini sandbox tool |
| `treasury.allocate` | **CRITICAL** | — | **Not agent-callable** — registry may exist only to DENY |

Unknown / DISABLED / not in `allowed_agents` → **DENY** before E2 (or after identity with reason `TOOL_NOT_PERMITTED`).

---

## 2. Tool request model

```text
Agent
  ↓
Runtime session (+ L3 credential)
  ↓
Tool request
```

### 2.1 `ToolRequest`

| Field | Meaning |
|-------|---------|
| `request_id` | Unique id (client or gateway-minted); correlates audit |
| `agent_id` | Runtime registry agent |
| `session_id` | Active runtime session |
| `organisation_id` | Must match session + CP org |
| `tool_id` | Registry key |
| `parameters` | JSON object conforming to tool schema |
| `timestamp` | Client/gateway time (server also stamps) |
| `credential` | L3 binding (Phase 28 placeholder shape) |
| `amount_minor` / `asset_id` | Required when `spend_scoped` |
| `idempotency_key` | Optional; replay of same key returns prior decision |

**Normative:** A `ToolRequest` is a **claim to attempt**, not authority.

---

## 3. Gateway decision flow (Phase 29 — no execution)

```text
Tool request
    ↓
Validate agent identity (registry ACTIVE; org not frozen)
    ↓
Validate session (ACTIVE/CREATED; not expired/revoked; bound to agent)
    ↓
Validate L3 credential (agent + org + expiry + credential_id; replay rules)
    ↓
Check tool registry (exists, ACTIVE, org, allowed_agents, schema)
    ↓
Map tool → EnforcementRequest (action = required_capability / tool_id)
    ↓
Call E2 evaluate()
    ↓
Compose gateway decision:
    ALLOW | DENY | REQUIRES_REVIEW
    ↓
Audit TOOL_*
    ↓
STOP — do not execute tool handlers in Phase 29
```

### 3.1 Decision composition

| Step fails | Gateway outcome |
|------------|-----------------|
| Identity / session / L3 | **DENY** (`IDENTITY_*` / session reasons) |
| Tool missing / disabled / not allowed | **DENY** (`TOOL_NOT_REGISTERED` / `TOOL_DISABLED` / `TOOL_NOT_PERMITTED`) |
| Schema invalid | **DENY** (`TOOL_PARAMETER_INVALID`) |
| E2 DENY | **DENY** (propagate E2 deny taxonomy) |
| E2 REQUIRES_REVIEW **or** risk CRITICAL default | **REQUIRES_REVIEW** |
| E2 ALLOW and risk not forced to review | **ALLOW** (decision only — **no side effects**) |

Fail-closed: registry or E2 unavailable → **DENY**.

---

## 4. Tool permissions mapping

| Plane | Role for tools |
|-------|----------------|
| **Capability** | `required_capability` must be granted (PROTO-0 / future linked caps); tool_id alone is insufficient |
| **Allocation** | If `spend_scoped`, remaining ≥ amount; org treasury ownership unchanged |
| **Policy** | `required_policy` + E2 policy checks; blocked actions → DENY |
| **Risk level** | Escalates ALLOW → REVIEW for CRITICAL (and HIGH when policy says) |
| **allowed_agents`** | Narrows which runtime agents may even ask |

### 4.1 Example

| Agent | Capability | `research.search` | `deploy.production` |
|-------|------------|-------------------|---------------------|
| Claude Research | `research.search` | **Allowed to request** → E2 may ALLOW | **Denied** (no cap + not in allowed_agents / CRITICAL) |
| Gemini Analysis | `code.analysis` | **Denied** | **Denied** |

---

## 5. Future sandbox preparation

| Agent | Simulated allocation | Tools |
|-------|----------------------|-------|
| **Claude** | £1 | `research.search` |
| **Gemini** | £1 | `code.analysis` |

| Test | Expect |
|------|--------|
| Claude → `research.search` | **ALLOW** (decision) |
| Claude → `deploy.production` | **DENY** |
| Agent → higher-risk tool without standing | **REQUIRES_REVIEW** or **DENY** |

Execution of simulated handlers is **out of scope** for Phase 29 docs; reserved for a later lab implementation gate.

---

## 6. Freeze statement

Tool record shape, `ToolRequest`, gateway decision flow (identity → registry → E2 → ALLOW/DENY/REVIEW), and permission mapping are **frozen**.

**No tool execution, provider integrations, Apply, or treasury changes in this phase.**
