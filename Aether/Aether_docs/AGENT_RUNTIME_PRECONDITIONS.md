# Agent Runtime Preconditions — Phase 27.5 Closure

| Field | Value |
|-------|--------|
| **Document** | `AGENT_RUNTIME_PRECONDITIONS.md` |
| **Phase** | 27.5 — Agent Runtime Preconditions Closure |
| **Status** | **DESIGN CLOSED** — Phase 28 lab implementation still requires separate approval |
| **Date** | 2026-08-01 |
| **Inputs** | [AGENT_RUNTIME_SECURITY_GATE.md](AGENT_RUNTIME_SECURITY_GATE.md) AR-C1…C10, [AGENT_RUNTIME_ARCHITECTURE.md](AGENT_RUNTIME_ARCHITECTURE.md), [AGENT_IDENTITY_MODEL.md](AGENT_IDENTITY_MODEL.md), [AGENT_TOOL_PERMISSION_MODEL.md](AGENT_TOOL_PERMISSION_MODEL.md) |
| **Non-goals** | Provider API connections, external tool execution, Apply, PROTO-0 mutation, treasury writes, agent wallets, code |

---

## Executive Summary

Phase 27 left **PASS WITH CONDITIONS**. This phase **freezes the precondition specifications** that an Agent Runtime Adapter must satisfy before lab integration (Phase 28).

| Class | Status after 27.5 |
|-------|-------------------|
| Design specs (boundary, L3 creds, tool registry, sessions, sandbox, redaction, forbids) | **Closed** |
| Empirical E2 proof + injection suite in CI | **Remain as Phase 28 entry tests** (specified here; not executed in 27.5) |
| Production agent execution / Apply | **Still forbidden** |

**Gate posture for entering Phase 28:** **PASS WITH CONDITIONS** — implement only a **lab** adapter that meets these preconditions; no production, no rails, no Apply.

---

## 1. Runtime adapter boundary

```text
Agent Runtime Adapter
        │
        ▼
E2 Enforcement (evaluate)
        │
        ▼
Decision (ALLOW | DENY | REQUIRES_REVIEW)
        │
        ▼
Tool gateway (future; lab simulated only)  — only if ALLOW / post-REVIEW approve
```

### 1.1 May

| Permission | Detail |
|------------|--------|
| Receive agent requests | Typed intents from session / provider tool-calls |
| Provide model metadata | `provider`, `model_id` for audit (non-authoritative) |
| Create runtime sessions | `session_id`, bind L3 credential → `agent_id` + org |
| Call E2 evaluate | Mandatory for every governed tool |
| Map tools → actions | Via tool registry only |
| Emit audit events | Session start/end, tool attempt, enforcement correlation |

### 1.2 Must not (hard)

| Forbidden | Rationale |
|-----------|-----------|
| Call `proto0_write` | INV-E03 / AR-C9 |
| Change capabilities | E4 forbidden; Apply-only (future) |
| Alter allocations / treasury writes | Financial plane separation |
| Bypass E2 | AR-C6 |
| Execute Apply | Apply enablement separate |
| Treat provider API keys as agent identity | §2 |
| Create agent wallets | Treasury ownership rules |

### 1.3 Dependency rule (AR-C9)

Future adapter crate/module **must not** depend on Apply execution or `proto0_write` APIs. Allowed: read-only proto0 observation, E2 service, tool registry, session store, audit append.

### 1.4 AR-C1 closure

Lab-only implementation approval is a **separate Phase 28 kickoff**. This document does **not** authorise shipping production adapters.

---

## 2. L3 runtime credentials (AR-C2)

### 2.1 Purpose

Authenticate the **workload** that acts as a registered agent — distinct from:

- Operator Console JWT (L6)  
- Provider inference API keys (secrets only)  
- Prompt-asserted identity  

### 2.2 Requirements

| Requirement | Spec |
|-------------|------|
| **Scoped to `agent_id`** | Claim `sub` / `agent_id` must equal registered principal; enforcement uses this id only |
| **Organisation bound** | Claim `organisation_id` must match CP treasury org / tenancy; mismatch → DENY |
| **Expiry** | Default TTL **≤ 1 hour**; max TTL **≤ 24 hours**; reject expired |
| **Revocation** | Deny-list / version epoch (`cred_version`); revoke-all on compromise |
| **Replay protection** | `jti` unique; optional nonce store TTL ≥ credential life; reject reuse of one-shot tokens if used |
| **Audit identity** | Every E2/tool audit row includes `agent_id`, `organisation_id`, `credential_id`/`jti`, `session_id` |

### 2.3 Recommended format (design)

**Short-lived signed Agent Runtime JWT** (lab default):

```text
header: alg = EdDSA or RS256 (org runtime signing key — not PROTO-0 grant key)
claims:
  iss = "aether-cp-runtime"
  aud = "aether-agent-runtime"
  sub = <agent_id>
  organisation_id = <org>
  sid = <session_id>          // optional bind
  jti = <unique>
  iat, exp
  cred_version = <u64>
  permissions_hint = []       // NEVER authoritative; E2 re-reads live caps
```

**Enterprise preferred (later):** workload identity / mTLS with SPIFFE-like `agent_id` SAN — same logical claims.

### 2.4 Issuance & rotation

| Step | Actor |
|------|-------|
| Issue | Operator (admin) or automated issuer after registration — **not** the model |
| Rotate | On schedule or compromise; bump `cred_version`; old JWTs fail |
| Suspend | CP marks registration suspended → all L3 rejects even if JWT unexpired |

### 2.5 Provider API keys are secrets, not identity

| Artifact | Is identity? | Use |
|----------|--------------|-----|
| Anthropic / Gemini / OpenAI API key | **No** | Org secret manager → inference client only |
| L3 Agent Runtime JWT / mTLS | **Yes (runtime)** | CP + E2 + tool gateway |
| PROTO-0 `agent_id` | **Yes (protocol)** | Capability / freeze truth |

Accepting a vendor API key as `Authorization` for E2/tools is a **specification violation**.

---

## 3. Tool registry model (AR-C3)

### 3.1 Record schema

| Field | Type | Meaning |
|-------|------|---------|
| `tool_id` | string | Stable id (e.g. `research.search`) |
| `owner` | string | Team / system owning the handler |
| `risk_level` | `low` \| `medium` \| `high` \| `critical` | Drives default E2/REVIEW posture |
| `required_capability` | string / list | Action selector(s) that must be granted |
| `policy_requirements` | object | e.g. approved policy types, blocked without policy |
| `spend_scoped` | bool | If true → allocation + amount required |
| `provider_mappings` | map | Provider tool name → `tool_id` |
| `sandbox_simulation_only` | bool | Lab: no real external side effects |
| `enabled` | bool | Kill switch |

### 3.2 Examples

| tool_id | risk | required_capability | Notes |
|---------|------|---------------------|-------|
| `research.search` | **low** | `research.search` | Allowlisted sources |
| `http.request` | **medium** | `http.request` | Host allowlist mandatory |
| `purchase.subscription` | **high** | `purchase.subscription` | Spend-scoped |
| `deploy.production` | **critical** | `deploy.production` | Default **REQUIRES_REVIEW**; lab disabled |
| `treasury.allocate` | **critical** | — | **Not agent-callable**; registry entry exists only to DENY map |

### 3.3 Risk → decision defaults

| Risk | Default if caps/alloc/policy pass |
|------|-----------------------------------|
| low | ALLOW (still E2) |
| medium | ALLOW or REVIEW per policy |
| high | ALLOW only under spend limits; else REVIEW |
| critical | **REQUIRES_REVIEW** (no silent execute) |

Unknown `tool_id` / unmapped provider tool → **DENY** (`UNSUPPORTED_ACTION`).

---

## 4. Session controls (AR-C4)

| Control | Default (lab) | Behaviour |
|---------|---------------|-----------|
| **Session lifetime** | 30 min idle / 2 h wall | Expire → new session + new L3 bind |
| **Max actions / session** | 50 tool attempts | Further tools → DENY / circuit break |
| **Max tool depth** | 8 | Nested/loop guard |
| **Maximum spend context** | `min(£1 sandbox allocation remaining, session_spend_ceiling)` | Track cumulative simulated spend |
| **Cancellation** | Operator or agent runtime `session.cancel` | In-flight tools abort; no partial spend commit without E2 |
| **Frozen agent** | PROTO-0 status `frozen` | All tools DENY (`AGENT_FROZEN`); session marked blocked |
| **Suspended registration** | CP flag | Reject L3; no new sessions |

Session must store: `session_id`, `agent_id`, `organisation_id`, `provider` metadata, counters, spend accumulator, status.

---

## 5. E2 proof requirements (AR-C6)

**Before any agent tool execution path is merged**, the following must pass in CI (see [AGENT_RUNTIME_TEST_PLAN.md](AGENT_RUNTIME_TEST_PLAN.md)). Phase 26 already covers evaluate; Phase 28 must prove **adapter → E2** wiring.

| Case | Expected |
|------|----------|
| ALLOW | Valid identity + capability + allocation + policy |
| DENY | Missing capability |
| DENY | Expired allocation |
| DENY | Frozen agent |
| DENY | Cross-organisation request |
| REVIEW | High-risk / critical tool (e.g. `force_review` or `deploy.production`) |

Invariant: **zero** tool side effects on DENY; **zero** Apply/treasury writes from adapter.

---

## 6. Secret redaction (AR-C8)

Tool results returned to the model **must** redact:

- API keys, JWTs, cookies  
- Allocation/journal internal ids if policy says so (optional)  
- Operator PII beyond need  
- Raw CSRF / signing material  

Redaction runs in the tool gateway **before** appending to model context. Failure to redact → treat as incident; do not “best effort leave secrets in”.

---

## 7. Sandbox specification (AR-C5) — summary

Full tasks in test plan. Normative:

| Agent | Provider (future) | Simulated allocation | Caps |
|-------|-------------------|----------------------|------|
| **A** | Claude | £1 | research, allowlisted API, small purchase |
| **B** | Gemini | £1 | analysis, reporting |

| Task | Expected |
|------|----------|
| Research API call | ALLOW (A) |
| Unauthorised purchase | DENY |
| Increase budget | REVIEW |

No external money movement; no agent wallets; label all UI/docs “simulated”.

---

## 8. Comms (AR-C10)

Forbidden phrases in product copy: “agent wallet”, “Claude’s money”, “Gemini balance”.  
Required: “organisation allocation”, “simulated lab budget”, “Aether decides”.

---

## 9. AR-C closure matrix

| ID | Topic | 27.5 status |
|----|-------|-------------|
| AR-C1 | Lab-only impl approval | **Spec closed** — Phase 28 kickoff still required |
| AR-C2 | L3 credentials | **Spec closed** |
| AR-C3 | Tool registry | **Spec closed** |
| AR-C4 | Session controls | **Spec closed** |
| AR-C5 | Sandbox simulated only | **Spec closed** |
| AR-C6 | E2 proof tests | **Plan closed** — execute in Phase 28 |
| AR-C7 | Injection suite | **Threat + plan closed** — execute in Phase 28 |
| AR-C8 | Redaction | **Spec closed** |
| AR-C9 | No Apply/proto0_write in adapter | **Spec closed** |
| AR-C10 | Comms | **Spec closed** |

---

## 10. Freeze statement

Adapter boundary, L3 credentials, tool registry, session controls, and sandbox rules are **frozen** as preconditions for Phase 28 lab work.

**This document does not authorise Phase 28 code, provider connections, or Apply enablement.**
