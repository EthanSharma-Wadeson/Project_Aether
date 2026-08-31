# Provider Adapter Implementation — Phase 32

| Field | Value |
|-------|--------|
| **Document** | `PROVIDER_ADAPTER_IMPLEMENTATION.md` |
| **Phase** | 32 — Provider Adapter Security Implementation (Lab Only) |
| **Status** | **IMPLEMENTED (mock providers only)** |
| **Gate** | **PASS WITH CONDITIONS** |
| **Date** | 2026-08-01 |
| **Design** | [PROVIDER_INTEGRATION_SECURITY_GATE.md](PROVIDER_INTEGRATION_SECURITY_GATE.md), [AGENT_RUNTIME_HARDENING_REVIEW.md](AGENT_RUNTIME_HARDENING_REVIEW.md) |

---

## Success criteria

Aether can ask an **external-model-shaped** component for reasoning while proving the model has **zero authority**.

Every governed request ends as **ALLOW / DENY / REQUIRES_REVIEW** with:

- `tools_executed = false`
- `external_execution = false`
- `apply_invoked = false`
- `proto0_mutated = false`
- `treasury_mutated = false`
- `provider_has_authority = false`

---

## Architecture

```text
POST /api/providers/lab/reason
        │
        ▼
ProviderAdapterService
        │
        ├─ trusted agent_id + session_id (HTTP body — operator plane)
        ├─ session governor precheck (TTL / max actions / cancel / breaker)
        │
        ▼
Lab mock Provider (Research | Finance | Hostile)
        │  proposes intents only
        ▼
Mapper (allowlist + injection reject)
        │  strips claimed identity / authority
        ▼
Tool Gateway → E2
        │
        ▼
Sandbox executor (simulate SUCCESS / STOPPED / REVIEW_PENDING)
        │
        ▼
Audit: PROVIDER_REASONED | PROVIDER_PROPOSAL_REJECTED | PROVIDER_GOVERNED_DECISION
```

### Module

`control_plane/src/providers/`

| File | Role |
|------|------|
| `traits.rs` | `Provider` — reason → proposals |
| `models.rs` | Metadata, request/response, `ProposedIntent`, governed result |
| `secrets.rs` | `SecretRef` (`secret://…` only); `redact_for_audit` |
| `governors.rs` | Session TTL / max actions / cancel / circuit breaker |
| `mapper.rs` | Untrusted intent → `ToolEvaluateHttpRequest` |
| `mocks/` | `ResearchAgentModel`, `FinanceSimulationModel`, `HostileInjectionModel` |
| `service.rs` | Orchestration |
| `audit.rs` | Provider audit events |
| `errors.rs` | Typed errors |

### HTTP

| Method | Path |
|--------|------|
| POST | `/api/providers/lab/reason` |
| POST | `/api/providers/lab/sessions/:session_id/cancel` |

---

## Security boundary

| Rule | Enforcement |
|------|-------------|
| Provider ≠ identity | Agent/session taken from trusted request; model claims ignored/rejected |
| API keys are secrets only | `SecretRef` rejects raw `sk-` / long opaque tokens; never logged in plaintext |
| Provider cannot grant authority | `claimed_capability` → DENY |
| Provider cannot call tools | No tool handlers; sandbox simulate only after gateway |
| Models cannot bypass E2 | `bypass_e2` / `claimed_decision` → DENY before gateway |
| Models cannot access treasury/protocol | Flags + forbidden tool prefixes → DENY |
| All actions require Aether governance | Path always mapper → gateway → E2 → sandbox |

Live Claude / Gemini / OpenAI HTTP clients are **not** implemented. Interfaces exist via `Provider` + `ProviderKind` metadata for a future phase.

---

## Session governors

Table: `provider_session_governors`

| Hook | Behaviour |
|------|-----------|
| TTL | Uses session `expires_at`; expired → governor DENY |
| Max actions | Default 20; then DENY |
| Cancellation | `POST …/cancel` sets `cancelled`; further reason calls fail closed |
| Frozen / unusable session | Precheck DENY |
| Circuit breaker | Opens after consecutive DENY storm (default 5) |

---

## Mock providers

| Mock | Model id | Proposes |
|------|----------|----------|
| **ResearchAgentModel** | `mock-research-agent-v1` | `research.search` |
| **FinanceSimulationModel** | `mock-finance-simulation-v1` | `purchase.subscription` |
| **HostileInjectionModel** | `mock-hostile-injection-v1` | Injection-shaped intents for negative tests |

---

## Threat mitigations (Phase 32)

| Threat | Mitigation |
|--------|------------|
| Prompt injection → false authority | Mapper ignores claimed agent/session; rejects capability / bypass / decision claims |
| Unrestricted tools | Allowlist + forbidden tool list |
| Key leakage in logs | Redaction + SecretRef handles only |
| Runaway loops | Max actions + circuit breaker + cancel |
| Confused deputy | Operator CSRF routes; provider never becomes admin |
| Shadow execution | No real tool/network side effects |

---

## Tests

`phase32_provider_adapter_tests.rs`:

- Research mock → governed decision, zero authority flags
- Finance mock → non-ALLOW typical path, no execution
- Hostile: bypass E2, unrestricted tool, capability claim, treasury/protocol
- Governor cancel blocks further reason
- SecretRef + redaction unit tests
- Mapper never adopts model identity

---

## Remaining conditions (from Phase 31.5)

| ID | Status after Phase 32 |
|----|------------------------|
| PI-C1 lab phase approval | **Closed** for *mock* adapters only |
| PI-C2 session governors | **Partially closed** — hooks + persistence implemented; production L3 still open |
| PI-C3 secret handling | **Partially closed** — SecretRef + redaction; no live key materialisation |
| PI-C4 tool allowlist mapper | **Closed** for lab mocks |
| PI-C5 correlation + redaction | **Partially closed** — correlation_id + provider_call_id + redact; retention policy still open |
| PI-C6 injection suite | **Closed** for lab hostile mock coverage |
| PI-C7 arch no Apply/proto0/treasury write | **Held** by design (adapter does not import write paths) |
| PI-C8 simulated only | **Held** |
| PI-C9 regulatory freezes | **Held** |
| PI-C10 comms | **Held** — `provider_has_authority: false` |

**Still open before live provider HTTP:** real L3 credentials, prompt retention policy, live API key vault integration, production circuit-breaker tuning, CI arch crate import lint.

---

## Explicit freezes (unchanged)

- No Apply enablement  
- No real tool execution  
- No custody / payment rails  
- No treasury mutation from provider path  
- No autonomous financial decisions  
- No production Claude/Gemini/OpenAI clients  

---

## Gate: PASS WITH CONDITIONS

Lab mock provider foundation is implemented and proves **model has zero authority**. Live external provider network adapters remain **not authorised**.

**STOP.**
