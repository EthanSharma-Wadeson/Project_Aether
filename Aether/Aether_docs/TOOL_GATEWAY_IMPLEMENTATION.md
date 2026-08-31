# Tool Gateway Implementation — Phase 30

| Field | Value |
|-------|--------|
| **Document** | `TOOL_GATEWAY_IMPLEMENTATION.md` |
| **Phase** | 30 — Tool Registry & Decision Gateway Implementation |
| **Status** | **IMPLEMENTED (decision only)** |
| **Gate** | **PASS WITH CONDITIONS** |
| **Date** | 2026-08-01 |
| **Design** | [TOOL_REGISTRY_MODEL.md](TOOL_REGISTRY_MODEL.md), [TOOL_GATEWAY_SECURITY_GATE.md](TOOL_GATEWAY_SECURITY_GATE.md) |

---

## Success criteria

Aether answers **“Can this AI agent use this tool?”** with a deterministic **ALLOW / DENY / REQUIRES_REVIEW** — **without executing anything**.

---

## Architecture

```text
POST /api/tools/evaluate
  ToolRequest (agent_id, session_id, tool_id, parameters)
        ↓
  Agent identity validation (runtime registry)
        ↓
  Runtime session validation
        ↓
  Tool registry lookup (org-scoped)
        ↓
  Capability requirement → E2 evaluate()
        ↓
  Gateway decision (+ audit + replay record)
        ↓
  tools_executed = false  (always)
```

---

## Module

`control_plane/src/tools/runtime/`

| File | Role |
|------|------|
| `models.rs` | Tool, ToolRequest, GatewayDecision, risk/status |
| `registry.rs` | Org-scoped tool CRUD |
| `policy.rs` | Risk/RESTRICTED → REVIEW composition |
| `gateway.rs` | Decision pipeline |
| `audit.rs` | TOOL_* events |
| `errors.rs` | Typed errors |

### Tool model

`tool_id`, `organisation_id`, `name`, `description`, `risk_level`, `required_capability`, `status`  
Status: `ACTIVE` \| `DISABLED` \| `RESTRICTED`

### HTTP

| Method | Path | Notes |
|--------|------|-------|
| POST/GET | `/api/tools` | Register / list (lab) |
| POST | `/api/tools/:tool_id/disable` | Disable |
| POST | `/api/tools/evaluate` | **Decision only** |

---

## Security

Rejects: unknown tool, disabled tool, frozen agent, expired session, cross-org tool, missing capability (via E2), replayed `request_id`.

Flags always false: `tools_executed`, `apply_invoked`, `proto0_mutated`, `treasury_mutated`.

---

## Tests

`control_plane/tests/phase30_tool_gateway_tests.rs` — registry + gateway + security (all green).

---

## Gate: PASS WITH CONDITIONS

| Condition | Next |
|-----------|------|
| No tool **execution** handlers | Separate exec phase |
| Runtime agent often linked to PROTO-0 id for ALLOW demos | Explicit linking phase |
| No provider integrations | Later |
| RESTRICTED/CRITICAL → REVIEW without human case store yet | E1 later |

**Forbidden held:** execution layer, model providers, Apply, treasury mutation, wallets.

**STOP.**
