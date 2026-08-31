# Tool Gateway Security Gate — Phase 29 Verdict

| Field | Value |
|-------|--------|
| **Document** | `TOOL_GATEWAY_SECURITY_GATE.md` |
| **Phase** | 29 — Tool Registry & Governed Tool Gateway Design |
| **Date** | 2026-08-01 |
| **Scope** | Design of controlled tool layer (lab decision path) |
| **Non-goals** | Real tool execution, provider APIs, Apply, PROTO-0 mutation, treasury writes, wallets, code |
| **Pack** | [TOOL_REGISTRY_MODEL.md](TOOL_REGISTRY_MODEL.md), [TOOL_EXECUTION_THREAT_MODEL.md](TOOL_EXECUTION_THREAT_MODEL.md) |

---

## Executive Summary

Agents request tools only through a **registry-backed gateway** that validates identity/session, checks tool permissions, and **must** call **E2** before any future side effect. The model and the tool request are **not** authority.

Phase 29 freezes design for a **decision-only** gateway. Execution remains forbidden until a later lab phase.

### Gate decision

# **PASS WITH CONDITIONS**

| Option | Result |
|--------|--------|
| PASS | — |
| **PASS WITH CONDITIONS** | **Selected** |
| FAIL | — |

**Meaning:** Tool registry + gateway decision flow are coherent with Phases 26–28 and runtime preconditions. **Implementation is not authorised by this gate alone.** **No tool execution, provider integrations, Apply, or treasury changes.**

---

## 1. Security model (normative rules)

| Rule | Statement |
|------|-----------|
| **R1** | The **model is not trusted**. |
| **R2** | The **tool request is not authority**. |
| **R3** | The **gateway decides** (identity + registry + E2). |
| **R4** | **No execution after ALLOW** in Phase 29 / until a separate exec gate. |
| **R5** | Fail-closed on missing registry, identity, or E2. |
| **R6** | No cross-org tool access. |
| **R7** | Tool registration is **operator-governed**, not agent-self-serve. |
| **R8** | Gateway must not call Apply, `proto0_write`, or treasury writes. |

---

## 2. Audit events (required in any impl)

| Event | When |
|-------|------|
| `TOOL_REQUESTED` | Request accepted for evaluation |
| `TOOL_ALLOWED` | Gateway decision ALLOW |
| `TOOL_DENIED` | Gateway decision DENY |
| `TOOL_REVIEW_REQUIRED` | Gateway decision REQUIRES_REVIEW |

**Metadata (minimum):** `agent_id`, `organisation_id`, `session_id`, `tool_id`, `decision`, `reason`, `request_id`, actor (if operator-triggered), E2 `decision_code` when applicable.

---

## 3. Implementation blockers (conditions)

| ID | Condition | Severity |
|----|-----------|----------|
| **TG-C1** | Separate **lab implementation approval** for registry store + decision gateway (still no real exec) | Blocking |
| **TG-C2** | Persist tool registry with org tenancy + `allowed_agents` + risk + schema | Blocking |
| **TG-C3** | Operator-only tool registration (RBAC + CSRF); agents cannot register tools | Blocking |
| **TG-C4** | Wire ToolRequest → Phase 28 identity/session → E2; prove DENY has zero side effects | Blocking |
| **TG-C5** | Idempotency / replay policy for `request_id` / `idempotency_key` | Blocking |
| **TG-C6** | CRITICAL tools default REVIEW or DISABLED in lab | Blocking |
| **TG-C7** | Parameter schema validation before E2 | Blocking |
| **TG-C8** | Injection + hidden-tool suite from threat model | Blocking before any exec phase |
| **TG-C9** | Arch test: gateway crate has no Apply / proto0_write / treasury write imports | Blocking |
| **TG-C10** | Sandbox agent↔tool bindings (Claude/`research.search`, Gemini/`code.analysis`) as fixtures | Blocking for sandbox |

---

## 4. Sequencing

```text
Phase 29 design (this gate) ──PASS WITH CONDITIONS──►
    Future: lab tool registry + decision gateway (no real exec) ──►
        Future: simulated tool handlers (lab only) ──►
            Provider adapters (separate gate)
                Apply remains separate
```

| Question | Answer |
|----------|--------|
| Implement decision gateway before real execution? | **Yes** |
| Execute tools because ALLOW exists? | **No** (Phase 29 / until exec gate) |
| Enable Apply for tools? | **No** |

---

## 5. Checklist

| Criterion | Met? |
|-----------|------|
| Tool registry fields + risk levels | **Yes** |
| ToolRequest model | **Yes** |
| Gateway flow identity → registry → E2 → decision | **Yes** |
| Permission mapping (caps/alloc/policy/risk) | **Yes** |
| Audit events defined | **Yes** |
| Sandbox prep sketched | **Yes** |
| Threat model companion | **Yes** |
| Code / exec / providers / Apply / treasury in this phase | **No** (correct) |

---

## 6. Residual risks

1. Mis-implemented gateway that skips E2 — must be caught by TG-C4/C9.  
2. Over-broad `allowed_agents: *` — require capability still; prefer explicit lists in lab.  
3. Confusion between runtime `rt-agent-*` and PROTO-0 ids — linking remains a prior condition for real ALLOW on spend tools.

---

## 7. Freeze statement

Gateway security rules R1–R8, audit events, and TG-C* conditions are **frozen**.

**Gate: PASS WITH CONDITIONS.**

**STOP — No tool execution. No provider integrations. No Apply. No treasury changes.**
