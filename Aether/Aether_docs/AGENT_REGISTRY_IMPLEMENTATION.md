# Agent Registry Implementation — Phase 28

| Field | Value |
|-------|--------|
| **Document** | `AGENT_REGISTRY_IMPLEMENTATION.md` |
| **Phase** | 28 — Agent Registry & Runtime Identity Foundation |
| **Status** | **IMPLEMENTED (lab identity only)** |
| **Gate** | **PASS WITH CONDITIONS** |
| **Date** | 2026-08-01 |
| **Preconditions** | [AGENT_RUNTIME_PRECONDITIONS.md](AGENT_RUNTIME_PRECONDITIONS.md) |

---

## Scope

Lab-only **Control Plane** agent registration and runtime sessions.

| Included | Excluded |
|----------|----------|
| Org-scoped agent registry | Claude/Gemini/OpenAI API calls |
| Runtime sessions + placeholder L3 creds | Tool execution |
| Identity → E2 evaluate boundary | Apply / PROTO-0 mutation |
| Audit events | Treasury writes / agent wallets |

**Success:** Aether can represent *“Claude Research Agent owned by Organisation X is ACTIVE and has a valid runtime session”* without executing anything.

---

## Architecture

```text
Organisation (CP treasury org + runtime_organisations status)
      │
Agent Identity (runtime_agents — CP-local ids `rt-agent-…`)
      │
Runtime Session + L3 placeholder credential
      │
Agent Request (POST /api/runtime/evaluate)
      │
Identity validation
      │
E2 evaluate()   ← no tools
      │
ALLOW / DENY / REVIEW
```

**Note:** Runtime `agent_id` is **not** a PROTO-0 write. E2 may DENY `AGENT_NOT_FOUND` until a future linking phase; identity validity is independent.

---

## Module

`control_plane/src/agents/runtime/`

| File | Role |
|------|------|
| `models.rs` | Agent/session/L3/provider enums |
| `registry.rs` | Org + agent CRUD / status |
| `sessions.rs` | Session lifecycle + credential validation |
| `service.rs` | Orchestration + E2 boundary |
| `audit.rs` | AGENT_* / SESSION_* events |
| `errors.rs` | Typed errors → CP HTTP |

### HTTP

| Method | Path |
|--------|------|
| POST/GET | `/api/runtime/agents` |
| GET | `/api/runtime/agents/:agent_id` |
| POST | `/api/runtime/agents/:agent_id/freeze` |
| POST | `/api/runtime/agents/:agent_id/revoke` |
| POST | `/api/runtime/sessions` |
| POST | `/api/runtime/sessions/:id/revoke` |
| POST | `/api/runtime/sessions/:id/expire-check` |
| POST | `/api/runtime/evaluate` |

Operator + CSRF on mutations. Org = `config.treasury_organisation_id` (no cross-org list/get).

---

## Models

**Agent status:** `ACTIVE` \| `FROZEN` \| `DISABLED` \| `REVOKED`  
**Session status:** `CREATED` \| `ACTIVE` \| `EXPIRED` \| `REVOKED`  
**Provider (metadata only):** `ANTHROPIC` \| `GOOGLE` \| `OPENAI` \| `LOCAL` \| `ENTERPRISE`

**L3 placeholder** binds: `credential_id`, `agent_id`, `organisation_id`, `session_id`, `expires_at`, `jti` (replay-protected).

Provider API keys are **not** accepted as identity.

---

## Audit

`AGENT_CREATED`, `AGENT_FROZEN`, `AGENT_REVOKED`, `SESSION_CREATED`, `SESSION_EXPIRED`, `SESSION_REVOKED` — with `agent_id`, `organisation_id`, `actor`, `request_id`.

---

## Tests

`control_plane/tests/phase28_agent_registry_tests.rs` — registry, sessions, security (invalid cred, org mismatch, replay, deterministic lookup), identity→E2 boundary flags.

---

## Gate: PASS WITH CONDITIONS

| Condition | For |
|-----------|-----|
| No PROTO-0 link / dual-id mapping yet | Later phase before real ALLOW on lab tools |
| L3 is placeholder (not mTLS/JWT issuer prod) | Phase 28+ hardening |
| No provider adapters | Phase 29+ |
| E2 ALLOW for `rt-agent-*` needs capability/allocation alignment | Sandbox wiring later |

**Forbidden remain:** model APIs, tools, wallets, Apply, treasury mutation.

**STOP.**
