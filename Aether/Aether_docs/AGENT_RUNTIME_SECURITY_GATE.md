# Agent Runtime Security Gate — Phase 27 Verdict

| Field | Value |
|-------|--------|
| **Document** | `AGENT_RUNTIME_SECURITY_GATE.md` |
| **Phase** | 27 — Agent Runtime Integration Gate |
| **Date** | 2026-08-01 |
| **Scope** | Design how external AI agents interact with CP + E2 |
| **Non-goals** | Production agent execution, code, provider API integrations, Apply enablement, PROTO-0 mutation, treasury writes |
| **Pack** | [AGENT_RUNTIME_ARCHITECTURE.md](AGENT_RUNTIME_ARCHITECTURE.md), [AGENT_IDENTITY_MODEL.md](AGENT_IDENTITY_MODEL.md), [AGENT_TOOL_PERMISSION_MODEL.md](AGENT_TOOL_PERMISSION_MODEL.md) |

---

## Executive Summary

External models are **inference backends**. Aether governs **registered agent principals** via identity, capabilities, treasury allocations, policy, and **E2 enforcement before any tool side effect**. Agents never own money; providers never touch treasury; Apply never auto-runs from the agent loop.

### Gate decision

# **PASS WITH CONDITIONS**

| Option | Result |
|--------|--------|
| PASS | — |
| **PASS WITH CONDITIONS** | **Selected** |
| FAIL | — |

**Meaning:** Architecture, identity, tool permission, and security requirements are coherent with Phases 23–26 and treasury ownership rules. **Implementation is not authorised** until conditions below are closed. **Production agent execution and Apply enablement are not authorised.**

---

## 1. Resolved design questions (summary)

| # | Question | Resolution |
|---|----------|------------|
| 1 | Agent identity | Registered PROTO-0 `agent_id` + org tenancy + future runtime credential; **not** vendor API keys or prompts |
| 2 | Provider abstraction | ProviderAdapter over Claude / Gemini / OpenAI / local / enterprise; governance unchanged across providers |
| 3 | Tool model | Intent → E2 → ALLOW/DENY/REVIEW → tool execute only if allowed |
| 4 | Wallet principle | **Agents never own money**; org treasury → allocation → caps → E2 → action |
| 5 | Testing | Future £1 sandbox agents A (Claude) / B (Gemini) with simulated ALLOW/DENY/REVIEW tasks |
| 6 | Security | Controls for injection, malicious tools, key compromise, impersonation, loops, spend, escalation |
| 7 | Existing systems | Preserve Agent → E2 → allocation → policy → proposal → human → Apply(future) |
| 8 | Forbids | No wallets, no bypass, no provider→treasury, no auto capability escalation, no auto Apply |

---

## 2. Security requirements

| ID | Threat | Required control (design) |
|----|--------|---------------------------|
| **AR-S01** | Prompt injection → false authority | Identity from runtime credential only; prompts never set `agent_id`/caps |
| **AR-S02** | Malicious / unexpected tool calls | Strict tool allowlist; unknown → DENY; schema validation |
| **AR-S03** | Compromised provider API keys | Keys ≠ agent auth; rotate vendor keys; no CP privilege |
| **AR-S04** | Agent impersonation | L3 mTLS/JWT bound to `agent_id`; short TTL; revoke + PROTO-0 freeze |
| **AR-S05** | Runaway loops | `max_tool_calls`, depth, wall-clock TTL, circuit breaker |
| **AR-S06** | Excessive spending | E2 + remaining + session spend ceiling; fail closed |
| **AR-S07** | Privilege escalation | No self-grant tools; no Apply from loop; E4 forbidden |
| **AR-S08** | SSRF / data exfil via http tools | Host allowlist, block link-local/metadata, redact secrets in tool results |
| **AR-S09** | Confused deputy (operator vs agent) | Separate credential planes; agent JWT cannot call admin treasury/Apply |
| **AR-S10** | Bypass Aether | Contract: all governed tools only via gateway; no shadow shells in production design |

Inherited: **INV-E01…E08**, **INV-S01…S08**, treasury **no agent title**.

---

## 3. Relationship preservation check

| Stage | Preserved? |
|-------|------------|
| Agent request | **Yes** |
| Enforcement E2 | **Yes** (mandatory before tool) |
| Treasury allocation check | **Yes** (read) |
| Policy check | **Yes** |
| Proposal | **Yes** (future E3; non-executing) |
| Human approval | **Yes** (future E1) |
| Apply (future only) | **Yes** — never auto from agent |

---

## 4. Implementation blockers (conditions)

| ID | Condition | Severity |
|----|-----------|----------|
| **AR-C1** | Separate **implementation approval** for lab Agent Runtime Adapter (no production) | Blocking |
| **AR-C2** | Specify L3 agent credential format + issuance/rotation (≠ operator JWT, ≠ vendor key) | Blocking |
| **AR-C3** | Tool registry schema + mapping table from provider tools → Aether actions | Blocking |
| **AR-C4** | Session governor limits (tools, depth, spend, TTL) as config invariants | Blocking |
| **AR-C5** | Sandbox mode: simulated purchases only; ban external money movement | Blocking for lab |
| **AR-C6** | Prove every spend tool path calls E2 (integration tests) | Blocking |
| **AR-C7** | Prompt-injection eval suite against authority confusion | Blocking before any external tool enablement |
| **AR-C8** | Secret redaction policy for tool results returned to models | Blocking |
| **AR-C9** | No Apply / proto0_write imports in runtime adapter crate | Blocking |
| **AR-C10** | Comms: never market “agent wallet” or “Claude has £1” as title | Comms |

---

## 5. Sequencing

```text
Phase 27 design (this gate) ──PASS WITH CONDITIONS──►
    Future: lab runtime adapter + simulated tools (£1) ──►
        Hardening (AR-C*) ──►
            Optional: limited production tools (non-rail) after separate gate
                Apply enablement remains a separate gate (spend grants after sync/E2 proven)
```

| Question | Answer |
|----------|--------|
| Implement runtime before Apply enablement? | **Yes** for lab + E2-gated tools |
| Enable Apply because agents exist? | **No** |
| Production agents in this phase? | **No** |

---

## 6. Conditions checklist

| Criterion | Met? |
|-----------|------|
| Identity model defined | **Yes** |
| Provider abstraction defined | **Yes** |
| Tool → E2 → execute model defined | **Yes** |
| No agent wallets documented | **Yes** |
| Sandbox testing model sketched | **Yes** |
| Threat controls AR-S01…S10 | **Yes** |
| Pipeline to Apply preserved | **Yes** |
| Explicit forbids listed | **Yes** |
| Code / integrations / Apply / treasury writes in this phase | **No** (correct) |

---

## 7. Residual risks (accepted until impl)

1. Design alone does not stop a misbuilt adapter that skips E2 — **AR-C6** must close in code review/tests.  
2. Prompt injection remains an arms race — deny-by-default tools reduce blast radius.  
3. Multi-tenant credential mix-ups — hard separate agent vs operator auth stores.  
4. Lab simulated spend may be mistaken for real funds — labelling + no rails.

---

## 8. Freeze statement

Agent runtime architecture, identity model, tool permission model, AR-S* controls, and sequencing are **frozen**.

**Gate: PASS WITH CONDITIONS.**

**This gate does not authorise implementation, production agent execution, provider integrations, Apply enablement, PROTO-0 mutation, or treasury writes.**

**STOP.**
