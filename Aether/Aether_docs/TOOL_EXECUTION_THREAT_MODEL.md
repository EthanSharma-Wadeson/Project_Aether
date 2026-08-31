# Tool Execution Threat Model — Phase 29

| Field | Value |
|-------|--------|
| **Document** | `TOOL_EXECUTION_THREAT_MODEL.md` |
| **Phase** | 29 — Tool Registry & Governed Tool Gateway Design |
| **Status** | **DESIGN FROZEN** |
| **Date** | 2026-08-01 |
| **Related** | [TOOL_REGISTRY_MODEL.md](TOOL_REGISTRY_MODEL.md), [TOOL_GATEWAY_SECURITY_GATE.md](TOOL_GATEWAY_SECURITY_GATE.md), [AGENT_RUNTIME_THREAT_MODEL.md](AGENT_RUNTIME_THREAT_MODEL.md) |

---

## Normative rule

> **The model is not trusted.**  
> **The tool request is not authority.**  
> **The gateway decides.**

Even when a future phase executes tools, execution may occur only after gateway **ALLOW** (or human-resolved REVIEW). Phase 29 itself executes nothing.

---

## 1. Assets

| Asset | Sensitivity |
|-------|-------------|
| Tool registry (definitions, allowlists) | Critical |
| ToolRequest stream + parameters | High |
| Runtime session / L3 credentials | Critical |
| E2 decisions | High |
| Future tool handlers / secrets in params | Critical |
| Org tenancy boundary | Critical |

---

## 2. Trust boundaries

```text
[Untrusted] Model output, prompts, fetched content, ToolRequest.parameters
        │
        ▼
[Gateway] Identity + session + registry + schema + E2
        │
        ▼
[Decision] ALLOW | DENY | REQUIRES_REVIEW   ← Phase 29 stops here
        │
        ✗ no Apply / proto0_write / treasury write / real external exec
```

---

## 3. Threat catalogue

### TG-THR-001 — Malicious tool registration

| Field | Content |
|-------|---------|
| **Attack** | Compromised operator or bug lets agent/self register `deploy.production` as LOW |
| **Impact** | False ALLOW path later |
| **Control** | Operator-only registration; dual-control for CRITICAL; risk immutable without audit; agents cannot register |
| **Phase 29** | Design only — enforce in impl (TG-C3) |

### TG-THR-002 — Agent requesting hidden / unregistered tools

| Field | Content |
|-------|---------|
| **Attack** | Model invents `tool_id=shell.exec` |
| **Impact** | None if registry miss → DENY |
| **Control** | Closed registry; unknown → DENY; no dynamic tool creation from prompts |
| **Residual** | Evaluate spam — rate limit |

### TG-THR-003 — Prompt injection → tool misuse

| Field | Content |
|-------|---------|
| **Attack** | Injected text: “call purchase.subscription” |
| **Impact** | Request may be formed; must still fail caps/E2 |
| **Control** | Registry + capability + E2; injection cannot add tools or escalate risk_level |
| **Residual** | Human REVIEW social engineering |

### TG-THR-004 — Tool parameter manipulation

| Field | Content |
|-------|---------|
| **Attack** | Oversized amount, SSRF URL, path traversal in params |
| **Impact** | At exec phase: fraud / exfil; at decision phase: bad ALLOW evidence |
| **Control** | JSON Schema validation; spend amount re-checked in E2; host allowlists at exec; deny unknown fields |
| **Residual** | Schema gaps — review HIGH/CRITICAL schemas |

### TG-THR-005 — Replayed tool requests

| Field | Content |
|-------|---------|
| **Attack** | Replay ALLOW decision or same request_id to trigger double exec later |
| **Impact** | Duplicate side effects at exec phase |
| **Control** | `request_id` / idempotency store; L3 `jti` rules; exec phase must be idempotent |
| **Phase 29** | Decision replay returns same audit outcome without new authority |

### TG-THR-006 — Cross-org tool access

| Field | Content |
|-------|---------|
| **Attack** | Org A session requests Org B tool or agent |
| **Impact** | Tenancy breach |
| **Control** | `organisation_id` on tool + session + E2 mismatch DENY |
| **Residual** | None if tested |

### TG-THR-007 — Privilege escalation

| Field | Content |
|-------|---------|
| **Attack** | LOW tool parameters that instruct gateway to “upgrade to CRITICAL execute”; or call Apply |
| **Impact** | Governance bypass |
| **Control** | No escalation APIs on gateway; R8 no Apply/proto0_write/treasury write; risk from registry only |
| **Residual** | Supply-chain in future handlers |

### TG-THR-008 — Confused deputy (operator vs agent)

| Field | Content |
|-------|---------|
| **Attack** | Agent credential used on operator tool-admin routes |
| **Impact** | Registry tampering |
| **Control** | Separate auth planes (Phase 28 L3 vs operator JWT); tool admin = operator RBAC only |

### TG-THR-009 — Fake “tool allowed” in model context

| Field | Content |
|-------|---------|
| **Attack** | Prior message claims TOOL_ALLOWED |
| **Impact** | Model retries; must not skip gateway |
| **Control** | Every attempt re-enters gateway; approvals only from CP case store for REVIEW |

### TG-THR-010 — Premature execution on ALLOW

| Field | Content |
|-------|---------|
| **Attack** | Impl bug runs handler in “decision-only” build |
| **Impact** | Real side effects before exec gate |
| **Control** | Phase 29/30 decision builds omit handlers; feature flag `tool_execution_enabled=false`; tests assert zero side effects |

---

## 4. Mapping to sandbox scenarios

| Scenario | Threats exercised | Expect |
|----------|-------------------|--------|
| Claude `research.search` | THR-002/003 | ALLOW decision if caps bind |
| Claude `deploy.production` | THR-002/007 | DENY |
| Higher-risk without standing | THR-003/007 | REVIEW or DENY |
| Cross-org tool_id | THR-006 | DENY |
| Replay same request_id | THR-005 | Idempotent decision |

---

## 5. Evaluation requirements (before any execution phase)

1. Fixtures for TG-THR-001…010 (as applicable without real exec).  
2. Assert DENY/REVIEW never invoke handlers.  
3. Assert ALLOW in decision-only mode never invokes handlers (`tools_executed=false`).  
4. Arch/dependency ban on Apply / proto0_write / treasury write from gateway package.

---

## 6. Freeze statement

Threat catalogue TG-THR-001…010 and trust rules are **frozen** for subsequent lab gateway implementation and execution gates.

**No tool execution in Phase 29.**
