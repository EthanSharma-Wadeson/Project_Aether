# Runtime Spend Enforcement Model — Phase 26

| Field | Value |
|-------|--------|
| **Document** | `ENFORCEMENT_RUNTIME_MODEL.md` |
| **Phase** | 26 — Runtime Spend Enforcement Foundation |
| **Status** | **IMPLEMENTED (E2 decision engine only)** |
| **Date** | 2026-08-01 |
| **Design** | [APPLY_TREASURY_ENFORCEMENT_MODEL.md](APPLY_TREASURY_ENFORCEMENT_MODEL.md), [APPLY_TREASURY_ENFORCEMENT_SECURITY_GATE.md](APPLY_TREASURY_ENFORCEMENT_SECURITY_GATE.md) |

---

## Purpose

Answer, deterministically and with audit:

> **Can this AI agent perform this action right now?**

Outcomes: **ALLOW** | **DENY** | **REQUIRES_REVIEW**.

---

## E2 — Runtime denial model

```text
Agent Intent
    → Identity / agent status
    → Capability evaluation (PROTO-0 observation)
    → Treasury allocation evaluation (read-only)
    → Policy evaluation (CP templates)
    → Risk evaluation (soft escalate)
    → ALLOW / DENY / REQUIRES_REVIEW
    → Audit events
```

| Property | Rule |
|----------|------|
| Fail-closed | Missing/uncertain authority → DENY |
| No execution | Evaluate does not perform the action |
| No mutation | No PROTO-0 write, no treasury write, no Apply |
| Structured denies | Taxonomy codes (never generic errors) |

### Module

`control_plane/src/enforcement/`

| File | Role |
|------|------|
| `models.rs` | Request, decision, deny taxonomy, evidence |
| `rules.rs` | Spend-scope, expiry, asset compatibility |
| `decision.rs` | Pure deterministic engine |
| `evaluator.rs` | Read-only snapshot loader |
| `audit.rs` | ENFORCEMENT_* events |
| `service.rs` | Orchestration |

### API

`POST /api/enforcement/evaluate` — JWT + operator RBAC + CSRF. **No execution endpoint.**

---

## Relationship with Treasury

| Does | Does not |
|------|----------|
| Read allocations, remaining, status, expiry, treasury freeze | Reserve, fund, settle, adjust |
| Ask “is funding authority present?” | Move money |

Effective spend limit when both present: `min(capability.max_spend, allocation.remaining_minor)`.

---

## Relationship with Apply

| Does | Does not |
|------|----------|
| Emit `requires_remediation` / `proposal_candidate` hints | Call `proto0_write` |
| | Execute Apply / grant / revoke |

Apply remains the **only** path that may change PROTO-0 (future, when enabled).

---

## Why automatic mutation (E4) is forbidden

Silent capability revoke/reduce from sync or enforcement would:

- Bypass Apply governance and dual-control
- Violate INV-E01 / INV-S01 / INV-S07
- Destroy recoverability and audit accountability

Phase 26 **never** auto-mutates capabilities.

---

## Future E1 / E3 remediation path

```text
E2 DENY (now)
  → E1 human case / approval (future)
  → E3 capability reduction **proposal** (future)
  → Apply execution when enabled (future)
  → Re-observe + re-evaluate
```

Until E1/E3 land, operators remediate manually via existing treasury write façade and (later) Apply.

---

## DENY taxonomy (selected)

`CAPABILITY_MISSING`, `ALLOCATION_EXCEEDED`, `EXPIRED_ALLOCATION`, `POLICY_BLOCKED`, `AGENT_FROZEN`, `TREASURY_FROZEN`, `UNKNOWN_ASSET`, `UNSUPPORTED_ACTION`, `ORGANISATION_MISMATCH`, …

---

## Audit

| Event | When |
|-------|------|
| `ENFORCEMENT_REQUESTED` | Every evaluate |
| `ENFORCEMENT_ALLOWED` | ALLOW |
| `ENFORCEMENT_DENIED` | DENY |
| `ENFORCEMENT_REVIEW_REQUIRED` | REQUIRES_REVIEW |

Metadata includes agent, organisation, capability, allocation, policy, `decision_code`, `request_id`.

---

## Success criteria

Aether can answer agent action authority with a deterministic, auditable ALLOW/DENY/REQUIRES_REVIEW decision **without** remediation, Apply execution, or money movement.

**STOP — Phase 26 foundation only.**
