# Agent Runtime Hardening Review — Phase 31.5

| Field | Value |
|-------|--------|
| **Document** | `AGENT_RUNTIME_HARDENING_REVIEW.md` |
| **Phase** | 31.5 — Regulatory Alignment & Agent Runtime Hardening Review |
| **Status** | **DESIGN & SECURITY REVIEW — FROZEN** |
| **Date** | 2026-08-01 |
| **Scope** | Readiness before external model adapters — no code, providers, Apply, rails |
| **Inputs** | Phases 26–31, [REGULATORY_POSITIONING.md](REGULATORY_POSITIONING.md), AR/TG conditions |
| **Companions** | [PROVIDER_INTEGRATION_SECURITY_GATE.md](PROVIDER_INTEGRATION_SECURITY_GATE.md) |

---

## Executive summary

Phase 31 completed the **simulated** governance loop:

```text
Agent → Session → ToolRequest → Tool Gateway → E2 → Decision → Audit/Sandbox
```

That loop is **necessary but not sufficient** for connecting Claude / Gemini / OpenAI / local models. This review catalogues runtime readiness, residual gaps, and freezes that must hold before any provider adapter phase.

### Gate contribution

This document feeds the overall Phase 31.5 gate in [PROVIDER_INTEGRATION_SECURITY_GATE.md](PROVIDER_INTEGRATION_SECURITY_GATE.md):

# **PASS WITH CONDITIONS**

External model **adapters are not authorised for production**. A **future lab-only** adapter phase may proceed only if all conditions in the provider gate remain satisfied and freezes hold.

---

## 1. Current runtime surface (as built)

| Layer | Status | Primary refs |
|-------|--------|--------------|
| E2 enforcement | Implemented (evaluate only) | `ENFORCEMENT_RUNTIME_MODEL.md` |
| Runtime agent registry | Implemented (CP-local `rt-agent-…`) | `AGENT_REGISTRY_IMPLEMENTATION.md` |
| Sessions + L3 placeholder | Implemented (jti replay) | Phase 28 |
| Tool registry + decision gateway | Implemented (`tools_executed = false`) | `TOOL_GATEWAY_IMPLEMENTATION.md` |
| Sandbox simulator | Implemented (A/B scenarios) | `AGENT_SANDBOX_IMPLEMENTATION.md` |
| Treasury | Governance / observation / lab ledger; sandbox does **not** write treasury | Treasury pack |
| Apply | Disabled | Project invariant |
| External providers | **None** | — |
| Real tool handlers | **None** | — |

---

## 2. Identity model — review

### Strengths

| Item | Assessment |
|------|------------|
| Provider ≠ identity | Enforced in design and registry (`provider` metadata only) |
| Org tenancy | Agents scoped to organisation |
| Status controls | ACTIVE / FROZEN / DISABLED / REVOKED |
| Separation from operator JWT | Distinct credential plane (design + routes) |

### Gaps before external models

| ID | Gap | Severity | Required before providers |
|----|-----|----------|---------------------------|
| **RH-I1** | Runtime `rt-agent-*` **not** durably linked to PROTO-0 subject (demo linking only) | High for production; Medium for lab | Lab: document dual-id; Production: explicit link + freeze sync |
| **RH-I2** | L3 credentials are **placeholders**, not production mTLS / signed JWT | High for production | Lab OK with short TTL + jti; Production: real L3 |
| **RH-I3** | Owner / accountable human mapping incomplete for SM&CR-style evidence | Medium (regulatory packaging) | Before consumer/enterprise GTM |
| **RH-I4** | No consumer digital-ID ↔ agent mandate artefact (Mills) | Medium (future retail) | Not required for enterprise lab adapters |

**Verdict:** Identity is **adequate for lab adapters** if provider keys never authenticate agents. **Not** production-ready as sole agent auth.

---

## 3. Sessions — review

### Strengths

| Item | Assessment |
|------|------------|
| Session lifecycle | CREATED / ACTIVE / EXPIRED / REVOKED |
| Binding | Credential bound to agent + org + session |
| Sandbox DENY on expired session | Proven in Phase 31 tests |

### Gaps before external models

| ID | Gap | Severity | Required before providers |
|----|-----|----------|---------------------------|
| **RH-S1** | Session governor limits (`max_tool_calls`, depth, wall-clock, session spend ceiling) not fully enforced as hard runtime invariants across provider loops | High | **Must** close before any multi-turn provider loop |
| **RH-S2** | No circuit breaker for runaway model↔tool propose loops | High | Required for lab multi-turn |
| **RH-S3** | Concurrent session / credential abuse model incomplete | Medium | Rate limits + one-active-session policy recommended |

**Verdict:** Sessions OK for single-shot sandbox. **Hardening required** for chat/tool-loop adapters.

---

## 4. Credentials — review

### Strengths

| Item | Assessment |
|------|------------|
| jti replay protection | Present for L3 placeholder |
| Explicit non-identities | Provider API keys rejected as agent proof |

### Gaps before external models

| ID | Gap | Severity | Required before providers |
|----|-----|----------|---------------------------|
| **RH-C1** | Secret manager pattern for **provider** keys not implemented in runtime | High | Keys in org secret store; never in prompts/DB plaintext |
| **RH-C2** | Rotation / compromise runbook for L3 vs vendor keys not operationalised | Medium | Document + practice before lab with real keys |
| **RH-C3** | Agent credential must not call admin treasury / Apply routes | High | Arch/RBAC proof before adapters |

**Verdict:** Credential **principle** is sound. Operational secret handling is a **condition**.

---

## 5. Audit reconstruction — review

### Strengths

| Item | Assessment |
|------|------------|
| Sandbox reconstruct | Returns agent, session, tool, decision, reason, simulated result |
| Gateway / E2 / runtime audit events | TOOL_*, ENFORCEMENT_*, AGENT_*, SESSION_*, SANDBOX_* |

### Gaps before external models

| ID | Gap | Severity | Required before providers |
|----|-----|----------|---------------------------|
| **RH-A1** | No end-to-end correlation ID from **provider request → model turn → tool intent → gateway → E2 → outcome** | High | Adapter must emit `provider_call_id` / `model_turn_id` |
| **RH-A2** | Prompt / completion retention policy undefined (PII, retention, redaction) | High | Redaction policy before storing model I/O |
| **RH-A3** | Injection / authority-confusion eval suite not CI-gated (AR-C7 / TG-C8) | High | Before any tool-shaped provider output is trusted |
| **RH-A4** | REVIEW path is record-only (no E1 human workflow) | Medium | Acceptable if adapters treat REVIEW as hard stop |

**Verdict:** Reconstruction is good for sandbox. Provider loops need **stronger correlation + redaction**.

---

## 6. Tool permissions — review

### Strengths

| Item | Assessment |
|------|------------|
| Registry + org scope | Implemented |
| Risk → REVIEW composition | Implemented |
| Operator disable | Implemented |
| Unknown / missing capability → DENY | Sandbox-proven |
| No execution after ALLOW | Invariant held |

### Gaps before external models

| ID | Gap | Severity | Required before providers |
|----|-----|----------|---------------------------|
| **RH-T1** | No mapping layer from **provider tool schemas** → Aether `tool_id` | High | Mandatory in adapter (allowlist only) |
| **RH-T2** | Model must not receive raw “execute” capability — only **propose** intents | High | Adapter contract |
| **RH-T3** | Parameter schema validation vs hostile model JSON | High | Fail closed on schema fail |
| **RH-T4** | SSRF / exfil controls for future http tools (AR-S08) | High before any http exec | N/A while exec frozen |

**Verdict:** Decision gateway is ready to sit **in front of** adapters. Adapters must not invent tools.

---

## 7. E2 enforcement — review

### Strengths

| Item | Assessment |
|------|------------|
| Fail-closed | Design + implementation |
| Read-only treasury / PROTO observation | Held |
| Deny taxonomy | Structured |
| Wired into tool gateway | Phase 30 |
| Sandbox path calls gateway → E2 | Phase 31 |

### Gaps before external models

| ID | Gap | Severity | Required before providers |
|----|-----|----------|---------------------------|
| **RH-E1** | Spend ALLOW demos often need fixture allocation / PROTO subject link | Medium (lab ops) | Document fixtures; no silent auto-fund |
| **RH-E2** | Asset mismatch (e.g. capability `AETHER_TEST` vs treasury `GBP`) causes drift / DENY noise | Medium | Lab fixture hygiene |
| **RH-E3** | Proof that **every** future provider tool path calls E2 (AR-C6) | High | Arch test + integration tests in adapter phase |
| **RH-E4** | Models must never call `/enforcement/evaluate` as authority shortcut to skip gateway | Medium | Single entry: tool gateway |

**Verdict:** E2 is the correct choke point. Adapters must use **gateway → E2**, never bypass.

---

## 8. Readiness matrix (before connecting external models)

| Area | Lab adapter (inference only, no real tools) | Lab adapter + simulated tools | Production providers + real tools |
|------|---------------------------------------------|-------------------------------|-----------------------------------|
| Identity | Conditionally ready | Conditionally ready | **Not ready** (RH-I1, RH-I2) |
| Sessions | Needs RH-S1/S2 for multi-turn | Needs RH-S1/S2 | **Not ready** |
| Credentials | Needs RH-C1/C2 | Needs RH-C1–C3 | **Not ready** |
| Audit | Needs RH-A1/A2 | Needs RH-A1–A3 | **Not ready** |
| Tools | Propose-only OK | Simulated only | **Frozen** |
| E2 | Ready as decision engine | Ready | Ready as decision; exec still frozen |
| Apply / rails / custody | Frozen | Frozen | Frozen |

---

## 9. Future sandbox plan (lab test definition)

Normative plan for a later lab phase (still **no** real money / real tools / Apply):

### 9.1 Agents

| Agent | Role | Provider (metadata) | Tools (simulated) | Budget |
|-------|------|---------------------|-------------------|--------|
| **Agent A** | Research assistant | e.g. Anthropic (lab) | `research.search` only | `simulated_budget_minor` metadata only |
| **Agent B** | Finance simulation assistant | e.g. Google (lab) | Restricted set e.g. `purchase.subscription` → expect DENY or REVIEW unless fixtures allow | Simulated only |

### 9.2 Required loop

```text
1. Register org-scoped agents A/B (runtime registry)
2. Open sessions + L3 placeholder (or lab L3)
3. Provider adapter returns model text / proposed tool intents only
4. Adapter maps intents → ToolRequest (allowlisted tool_id)
5. Tool gateway → E2 → ALLOW | DENY | REQUIRES_REVIEW
6. On ALLOW: simulate tool SUCCESS only (no network side effect)
7. On DENY: stop; no retry storm without governor
8. On REVIEW: record only; no auto-approve
9. Reconstruct full chain by request_id (+ provider_call_id)
```

### 9.3 Success criteria

| Criterion | Required |
|-----------|----------|
| Complete audit reconstruction | Agent, session, tool, E2 decision, reason, simulated outcome |
| Provider key never accepted as `agent_id` | Proven by test |
| Model cannot bypass E2 | Proven by test (direct tool call attempt fails) |
| No treasury write from sandbox/adapter | Arch + test |
| No Apply / PROTO-0 mutation | Arch + test |
| Simulated budgets only | No rails |

---

## 10. Explicit freezes (reaffirmed)

| Freeze | Rule |
|--------|------|
| Apply enablement | Off |
| Real tool execution | Forbidden |
| Custody / payment rails | Forbidden |
| Treasury mutation expansion | Forbidden from runtime/adapter/sandbox |
| Autonomous financial decisions | Forbidden (no real spend autonomy) |

---

## 11. Condition register (carry into provider gate)

| ID | Condition | Blocks |
|----|-----------|--------|
| **RH-CND-1** | Session governors + loop circuit breaker | Multi-turn lab adapters |
| **RH-CND-2** | Provider key secret handling + rotation notes | Any live API key use |
| **RH-CND-3** | Provider→tool_id allowlist mapper; no free-form tools | Any tool-shaped model output |
| **RH-CND-4** | Correlation IDs + prompt/result redaction policy | Persisting model I/O |
| **RH-CND-5** | Injection / authority-confusion suite | Trusting model tool proposals |
| **RH-CND-6** | Arch tests: no Apply / proto0_write / treasury write from adapter | Adapter implementation |
| **RH-CND-7** | Dual-id / PROTO link documented for lab fixtures | Spend ALLOW demos |
| **RH-CND-8** | Regulatory freezes + REG-T triggers remain in force | All provider work |

---

## 12. Document control

Review-only. Implementation of adapters requires a **new phase** authorised under [PROVIDER_INTEGRATION_SECURITY_GATE.md](PROVIDER_INTEGRATION_SECURITY_GATE.md).

**STOP.**
