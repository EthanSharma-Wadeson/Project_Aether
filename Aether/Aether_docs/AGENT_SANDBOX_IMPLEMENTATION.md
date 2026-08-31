# Agent Sandbox Implementation — Phase 31

| Field | Value |
|-------|--------|
| **Document** | `AGENT_SANDBOX_IMPLEMENTATION.md` |
| **Phase** | 31 — Agent Runtime Sandbox Implementation |
| **Status** | **IMPLEMENTED (simulated lifecycle)** |
| **Gate** | **PASS WITH CONDITIONS** |
| **Date** | 2026-08-01 |

---

## Success criteria

Complete simulated loop:

```text
Agent → Session → Tool Request → Tool Gateway → E2 → Decision → Audit
```

with **zero external execution** (no Claude/Gemini/OpenAI, no real tools, no Apply, no PROTO-0 mutation). Sandbox runtime performs **no treasury writes**; £1 budgets are `simulated_budget_minor` metadata.

---

## Module

`control_plane/src/agents/sandbox/`

| File | Role |
|------|------|
| `models.rs` | `SandboxAgent`, run/review results |
| `planner.rs` | Task → `ToolRequest` |
| `runtime.rs` | Bootstrap scenarios + orchestrate |
| `executor.rs` | ALLOW→simulate SUCCESS; DENY→STOPPED; REVIEW→review event |
| `audit.rs` | `SANDBOX_TASK_COMPLETED` |

### Scenarios

| Agent | Role | Simulated £1 | Tools |
|-------|------|--------------|-------|
| **A** (`agent_a`) | Research assistant | metadata | `research.search` (allowed) |
| **B** (`agent_b`) | Finance assistant | metadata | `purchase.subscription` (restricted) |

Also seeds `deploy.production` (CRITICAL) for REVIEW demos.

### Execution behaviour (Phase 31)

| Decision | Behaviour |
|----------|-----------|
| ALLOW | Simulate success only |
| DENY | Stop immediately |
| REQUIRES_REVIEW | Insert `sandbox_reviews` PENDING event |

### HTTP

| Method | Path |
|--------|------|
| POST | `/api/sandbox/bootstrap` |
| POST | `/api/sandbox/run` |
| GET | `/api/sandbox/reconstruct/:request_id` |

Reconstruction returns: agent, session, tool, decision, reason (+ simulated_result).

---

## Tests

`phase31_sandbox_tests.rs`: ALLOW, missing capability DENY, expired session DENY, frozen agent DENY, high-risk REVIEW, audit reconstruction.

---

## Gate: PASS WITH CONDITIONS

| Condition | Notes |
|-----------|-------|
| No real tool handlers / providers | Held |
| E2 ALLOW for research may need test fixture allocation | Sandbox itself does not write treasury |
| Agent A linked to PROTO-0 subject for capability demo | Explicit dual-id linking still future |
| REVIEW events are records only (no E1 workflow) | Future |

**STOP.**
