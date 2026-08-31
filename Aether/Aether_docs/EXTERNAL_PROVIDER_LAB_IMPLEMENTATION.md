# External Provider Lab Implementation — Phase 34

| Field | Value |
|-------|--------|
| **Document** | `EXTERNAL_PROVIDER_LAB_IMPLEMENTATION.md` |
| **Phase** | 34 — External Provider Lab Adapter |
| **Status** | **IMPLEMENTED (lab only — Anthropic/Claude)** |
| **Gate** | **PASS WITH CONDITIONS** |
| **Date** | 2026-08-01 |
| **Provider** | Anthropic Messages API (Claude) |
| **Mode** | Isolated lab — **not** production |

---

## Success criteria

Aether can call a **real external model API** for reasoning while the model retains **zero authority**. Every action still ends:

```text
Model output → Intent extraction → Identity → Session governor
  → Tool Gateway → E2 → ALLOW|DENY|REVIEW → Sandbox only
```

---

## Architecture

```text
AnthropicLabProvider (lab)
        │ HTTP Messages API
        ▼
extract_proposed_intents (JSON only)
        │
        ▼
ProviderAdapterService (existing governance)
        │
        ▼
Sandbox simulate (no real tools / money / Apply / PROTO / treasury writes)
```

Module: `control_plane/src/providers/external/`

| File | Role |
|------|------|
| `anthropic.rs` | Claude lab adapter |
| `intent_extract.rs` | Untrusted JSON → `ProposedIntent` |
| `transport.rs` | `ReqwestLabTransport` + `ScriptedLabTransport` |
| `demo.rs` | Agent A research/finance demo |

Profiles: `anthropic` / `claude` / `external` (research allowlist), `anthropic_finance` (finance allowlist).

---

## Provider boundary

| Rule | Enforcement |
|------|-------------|
| Provider ≠ identity | Agent/session from trusted HTTP only |
| API key via SecretRef only | Raw `sk-` in request rejected; resolve from env |
| No secret logging | `LabApiKey` Debug = REDACTED; `redact_for_audit` on bodies |
| Output = proposals only | Intent extract; mapper strips authority claims |
| No tool execution by provider | `tools_executed = false` always |
| Production freeze | `is_production_mode()` → refuse external adapters |

---

## Secrets handling

1. Client passes `secret_ref`: `secret://org/{org}/providers/anthropic/api_key`
2. `SecretRef::resolve_lab_api_key()` reads:
   - `AETHER_LAB_ANTHROPIC_API_KEY` (preferred)
   - `AETHER_LAB_PROVIDER_API_KEY` (fallback)
3. Key used once for `x-api-key` header; never written to audit metadata
4. Optional: `AETHER_LAB_ANTHROPIC_MODEL`, `AETHER_LAB_ANTHROPIC_API_URL`

---

## Demo scenario (Agent A)

| Field | Value |
|-------|--------|
| Agent | Research assistant (`agent_a`) |
| Provider | Anthropic lab |
| Budget | `simulated_budget_minor` £1 metadata only |
| Allowed tools | `research.search` |

| Task | Expected |
|------|----------|
| Research | **ALLOW** → sandbox **SUCCESS** |
| Finance-shaped (`purchase.subscription`) | **DENY** (not on research allowlist) / otherwise blocked |

HTTP: `POST /api/providers/lab/demo/agent-a-research` (requires lab API key env).

Tests use `ScriptedLabTransport` so CI needs no network key.

---

## Prompt injection testing

Scripted Anthropic responses covering:

- malicious system-style / bypass instructions → **DENY**
- elevated capability claims → **DENY**
- treasury access → **DENY**
- protocol access → **DENY**

---

## Limitations & security assumptions

| Limitation | Assumption |
|------------|------------|
| Lab only | `CP_ENV=production` disables adapter |
| Single vendor (Claude) | Gemini/OpenAI can reuse transport + extract later |
| Model may return prose | Empty intents → fail closed (`EmptyProposals` / DENY path) |
| Env-based secrets | Not a production vault; sufficient for isolated lab |
| No computer-use / tool_use API | Lab requests plain JSON proposals only |
| Live demo non-deterministic | Model may vary; governance still fail-closed on unsafe tools |

---

## Explicit freezes

- No real tool execution  
- No real money / custody / rails  
- No Apply enablement  
- No PROTO-0 writes  
- No treasury writes from provider path  
- No production provider mode  

---

## Gate: PASS WITH CONDITIONS

External **lab** Claude adapter is implemented behind the existing interface. Production mode and live unsupervised provider rollout remain **forbidden**.

**STOP.**
