# Provider Integration Security Gate — Phase 31.5 Verdict

| Field | Value |
|-------|--------|
| **Document** | `PROVIDER_INTEGRATION_SECURITY_GATE.md` |
| **Phase** | 31.5 — Regulatory Alignment & Agent Runtime Hardening Review |
| **Date** | 2026-08-01 |
| **Scope** | Security boundary for future Claude / Gemini / OpenAI / local model adapters |
| **Non-goals** | Implementing adapters, real tool execution, Apply, PROTO-0 mutation, treasury writes, custody/rails |
| **Pack** | [REGULATORY_POSITIONING.md](REGULATORY_POSITIONING.md), [AGENT_RUNTIME_HARDENING_REVIEW.md](AGENT_RUNTIME_HARDENING_REVIEW.md) |

---

## Executive summary

External model providers are **inference backends only**. They are not Aether identity, not spending authority, and not a path to tools, treasury, or protocol mutation.

Phase 31 proved a governed **simulated** loop without providers. Connecting providers is a **separate lab risk** (prompt injection, key leakage, runaway loops, confused deputy). That risk is manageable **only** if adapters sit strictly **outside** enforcement and **inside** Aether’s request path.

### Gate decision

# **PASS WITH CONDITIONS**

| Option | Result |
|--------|--------|
| PASS | — |
| **PASS WITH CONDITIONS** | **Selected** |
| FAIL | — |

**Meaning:**

- Aether’s **governance architecture is ready enough** to *design and, in a later approved phase, implement* **lab-only** provider adapters that **propose** actions.
- Aether is **not ready** for production provider integrations, real tool execution, autonomous spend, or any REG-T trigger from [REGULATORY_POSITIONING.md](REGULATORY_POSITIONING.md).
- **This gate does not authorise coding adapters.** A future phase kickoff is required, and all conditions below must remain closed or explicitly tracked.

---

## 1. Normative security boundary

```text
┌─────────────────────────────────────────────────────────────┐
│                     TRUSTED CONTROL PLANE                     │
│  Identity · Session · Tool Registry · Gateway · E2 · Audit   │
│  (optional) Sandbox simulator                                │
└───────────────────────────▲─────────────────────────────────┘
                            │ ToolRequest only
                            │ (allowlisted tool_id + schema)
┌───────────────────────────┴─────────────────────────────────┐
│                    PROVIDER ADAPTER (lab)                     │
│  Maps model output → proposed intents                        │
│  Holds vendor API keys as org secrets                        │
│  Never authenticates as agent via vendor key                 │
└───────────────────────────▲─────────────────────────────────┘
                            │ prompts / completions
┌───────────────────────────┴─────────────────────────────────┐
│         Claude API · Gemini API · OpenAI API · Local LLM      │
│              UNTRUSTED INFERENCE — not authority              │
└─────────────────────────────────────────────────────────────┘
```

### Absolute rules

| Rule ID | Rule |
|---------|------|
| **PI-R1** | **Provider is not identity.** `agent_id` comes only from Aether runtime identity + L3 credential. |
| **PI-R2** | **API keys are secrets only.** Anthropic / Google / OpenAI / local endpoint credentials never prove agent authority. |
| **PI-R3** | **Models cannot bypass E2.** No side-effect path exists that skips tool gateway → E2. |
| **PI-R4** | **Models cannot directly call tools.** Models may emit structured proposals; only the gateway may decide; execution remains frozen. |
| **PI-R5** | **Models cannot access treasury or protocol state.** No treasury API, no PROTO-0 read/write credentials in the model context by default; any future read-only context is redacted and non-authoritative. |
| **PI-R6** | **All actions require Aether governance.** No shadow shells, no vendor “computer use” with finance side effects, no direct purchase APIs from the adapter. |
| **PI-R7** | **Fail closed.** Malformed proposals, unknown tools, expired sessions, frozen agents → DENY / stop. |
| **PI-R8** | **Lab ≠ production.** Live customer traffic, real funds, and real external side effects remain forbidden. |

Inherited: AR-S01…S10, TG R1–R8, INV-E01…E08, Phase 31.5 freezes.

---

## 2. Provider-specific constraints

| Provider | Allowed lab use | Forbidden |
|----------|-----------------|-----------|
| **Claude API** | Chat/completions; optional tool-*proposal* JSON mapped to allowlist | Computer use / browser / bash with real effects; treating Anthropic key as agent auth |
| **Gemini API** | Same as above | Same class of direct side effects; Google key as agent auth |
| **OpenAI API** | Same as above | Assistants/tools that execute outside Aether; OpenAI key as agent auth |
| **Local models** | Same inference role; still untrusted | Binding local process UID to `agent_id` without L3; unconstrained tool plugins |

**Uniformity rule:** Switching provider must **not** change governance outcomes. Only `provider` / `model_id` metadata change.

---

## 3. Adapter may / must not

### May (future lab phase)

| Permission | Detail |
|------------|--------|
| Call vendor/local inference APIs | Using org-stored secrets |
| Attach `provider` + `model_id` to audit | Non-authoritative |
| Translate model output → `ToolRequest` candidates | Allowlisted `tool_id` only |
| Invoke tool gateway evaluate / sandbox simulate | Decision + simulate only |
| Enforce session governors | max turns, max proposals, TTL, circuit breaker |
| Redact secrets from prompts/results | Before persist or re-inject to model |

### Must not

| Forbidden | Rationale |
|-----------|-----------|
| Accept vendor API key as `agent_id` / L3 | PI-R1, PI-R2 |
| Execute real tools on ALLOW | Execution freeze |
| Call Apply / `proto0_write` | Protocol freeze |
| Write treasury / fund / reserve / settle | Financial freeze |
| Grant capabilities or register tools as the agent | Confused deputy |
| Auto-approve REQUIRES_REVIEW | Human/E1 later |
| Expose raw allocation/capability dumps to the model as “you may spend £X” authority | Injection / overclaim |
| Disable E2 because the model “insists” | PI-R3 |

---

## 4. Conditions (blocking)

| ID | Condition | Severity |
|----|-----------|----------|
| **PI-C1** | Separate **lab phase approval** to implement adapters (this gate alone is insufficient) | Blocking |
| **PI-C2** | Close **RH-CND-1** session governors + circuit breaker for multi-turn loops | Blocking for chat loops |
| **PI-C3** | Close **RH-CND-2** provider secret storage + rotation notes | Blocking for live API keys |
| **PI-C4** | Close **RH-CND-3** provider tool schema → Aether `tool_id` allowlist mapper | Blocking for tool-shaped outputs |
| **PI-C5** | Close **RH-CND-4** correlation IDs + redaction/retention policy | Blocking before persisting model I/O |
| **PI-C6** | Close **RH-CND-5** injection / authority-confusion test suite | Blocking before trusting proposals |
| **PI-C7** | Close **RH-CND-6** arch tests: adapter has no Apply / proto0_write / treasury write imports | Blocking |
| **PI-C8** | Sandbox/lab success path uses **simulated tools and simulated budgets only** | Blocking |
| **PI-C9** | Regulatory freezes + REG-T01…T09 remain in force ([REGULATORY_POSITIONING.md](REGULATORY_POSITIONING.md)) | Blocking |
| **PI-C10** | Comms: never claim provider = wallet, FCA approval, or agent-owned money | Comms |

---

## 5. Future lab test (normative under this gate)

When a lab adapter phase is approved:

| Step | Agent A (research) | Agent B (finance simulation) |
|------|--------------------|------------------------------|
| Provider | One of Claude/Gemini/OpenAI/local | Different provider preferred (proves uniformity) |
| Tools | `research.search` simulated | Restricted finance-shaped tool simulated |
| Budget | Metadata only | Metadata only |
| Expectation | ALLOW → simulate SUCCESS | DENY or REVIEW without rails |
| Audit | Full reconstruct including provider call id | Same |

**Pass/fail of that future lab** is separate from this gate. This gate only defines the security contract those tests must obey.

---

## 6. Explicit freezes (non-negotiable under PASS WITH CONDITIONS)

| Freeze | Status |
|--------|--------|
| Apply enablement | **OFF** |
| Real tool execution | **FORBIDDEN** |
| Custody / payment rails | **FORBIDDEN** |
| Treasury mutation from adapter/runtime/sandbox | **FORBIDDEN** |
| Autonomous financial decisions with real funds | **FORBIDDEN** |
| Production provider rollout | **FORBIDDEN** |

Lifting any freeze requires a **new** security + (where REG-T applies) **legal** gate — not an informal code change.

---

## 7. Checklist

| Criterion | Met for this review? |
|-----------|----------------------|
| Regulatory boundary documented | **Yes** |
| Runtime gaps identified before providers | **Yes** |
| Provider ≠ identity / keys = secrets | **Yes** |
| Models cannot bypass E2 / call tools / touch treasury | **Yes** (as design rules) |
| Future sandbox plan defined | **Yes** |
| Freezes explicit | **Yes** |
| Adapter implementation authorised now | **No** |
| Production providers authorised | **No** |
| Apply / rails / real exec authorised | **No** |

---

## 8. Sequencing

```text
Phase 31 sandbox (done)
        │
        ▼
Phase 31.5 review (this pack) ── PASS WITH CONDITIONS
        │
        ├──► Future: lab provider adapters (propose-only)  [needs PI-C1…C10]
        │         └── simulated tools only
        │
        ├──► Future: hardening closures (governors, injection suite, L3 prod)
        │
        └──► NOT NOW: real execution / Apply / rails / custody / autonomous spend
```

| Question | Answer |
|----------|--------|
| Ready to **design** lab adapters under these rules? | **Yes** |
| Ready to **implement** adapters in this phase? | **No** — STOP |
| Ready for **production** external models? | **No** |
| Ready for **real tool execution**? | **No** |

---

## 9. Final verdict

# **PASS WITH CONDITIONS**

Aether may proceed to a **future, separately approved lab phase** for external model adapters **only** if:

1. Adapters obey PI-R1…R8  
2. Conditions PI-C1…C10 are satisfied or explicitly waived by a later gate  
3. All Phase 31.5 freezes remain in force  

Until then: **no provider integrations, no execution handlers, no Apply changes, no treasury writes, no protocol changes.**

**STOP after review.**
