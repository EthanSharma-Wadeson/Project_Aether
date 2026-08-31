# Apply ↔ Treasury Enforcement Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `APPLY_TREASURY_ENFORCEMENT_MODEL.md` |
| **Phase** | 25 — Capability Treasury Enforcement Design Gate |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Inputs** | [APPLY_TREASURY_SYNC_ARCHITECTURE.md](APPLY_TREASURY_SYNC_ARCHITECTURE.md), [APPLY_TREASURY_SYNC_SECURITY_GATE.md](APPLY_TREASURY_SYNC_SECURITY_GATE.md), [APPLY_TREASURY_SYNC_IMPLEMENTATION.md](APPLY_TREASURY_SYNC_IMPLEMENTATION.md) (Phase 24 observation), INV-S01…S08 |
| **Protocol impact** | **None** in this phase |
| **Apply impact** | **None** — `apply_enabled` remains false |
| **Treasury write impact** | **None** |

---

## Executive Summary

Phase 24 detects **authority drift** between Treasury allocations (financial authority) and PROTO-0 capabilities (protocol authority). Phase 25 defines what happens **after** detection — without implementing enforcement.

**Recommended model: Layered E1 + E2 + E3 (proposal-only)**

| Layer | Role |
|-------|------|
| **E2 — Runtime denial** | Continuous fail-closed spend gate (no PROTO-0 mutation) |
| **E1 — Human approval workflow** | Governance decision on remediation path |
| **E3 — Capability reduction proposal** | Creates Apply-shaped intents only; never silent execute |
| **E4 — Automatic capability mutation** | **Rejected** |

Drift observation (Phase 24) remains non-mutating. Enforcement must never silently change permissions, move money, or bypass PROTO-0 / Apply gates.

**STOP:** No code, routes, Apply flip, protocol edits, or treasury write changes in Phase 25.

---

## 1. Authority Planes (unchanged)

| Plane | Authority | Must not |
|-------|-----------|----------|
| **Treasury** | Financial allocation, freeze, remaining, journal | Grant/revoke PROTO-0 capabilities |
| **Control Plane** | Governance, policy, approvals, observation, workflow | Be source of protocol truth; invent journal lines |
| **Apply** | Sole mutation executor into PROTO-0 (when enabled) | Post treasury books; auto-heal without approval |
| **PROTO-0** | Protocol identity / capability / freeze truth | Be written by Treasury or CP sync jobs |

Drift means planes disagree. Enforcement chooses **which plane binds now** (runtime) and **how humans rematerialise agreement** (governance + Apply).

---

## 2. What Phase 24 Already Emits

| Sync status | Typical meaning | Enforcement posture (future) |
|-------------|-----------------|------------------------------|
| `SYNC_OK` | Planes aligned for scanned scope | No action |
| `DRIFT_DETECTED` | Missing allocation or missing capability | Runtime DENY spend; open review case |
| `REQUIRES_REVIEW` | Overscoped / expired / inactive funding with live grant | Runtime DENY; escalate severity; dual-control remediation |

Observation findings map into **severity levels** (Section 4) for workflow SLAs — not into automatic mutation.

---

## 3. Enforcement Options — Evaluation

### 3.1 Option E1 — Human approval workflow

**Behaviour:** Drift creates a **case** (or uses existing mutation/approval patterns). Operators choose: fund allocation, decrease allocation, propose CapabilityRevoke/reduce, freeze agent/treasury, or accept lab waiver (audited, non-prod).

| Dimension | Assessment |
|-----------|------------|
| **Security** | Strong — no silent change; dual-control possible; aligns INV-S07 |
| **Operational** | Latency; needs UX, ownership, SLA by severity |
| **Compliance** | Best audit story; human accountable for permission change |
| **Verdict** | **Adopt** as mandatory path for any remediation that changes authority |

### 3.2 Option E2 — Runtime denial without mutation

**Behaviour:** On spend / economic action, intersect capability ∧ allocation ∧ policy ∧ freeze (Hybrid D). On drift or insufficient remaining → **DENY** (or policy **ESCALATE**). No capability write; no journal invent.

| Dimension | Assessment |
|-----------|------------|
| **Security** | Strong fail-closed; stops overspend without touching PROTO-0 |
| **Operational** | Immediate protection; agents may “look” entitled while blocked — needs operator clarity |
| **Compliance** | Safe: denies action, does not alter recorded grants until Apply |
| **Verdict** | **Adopt** as continuous enforcement layer (independent of Apply on/off for product spend paths) |

### 3.3 Option E3 — Capability reduction proposal

**Behaviour:** System drafts a **proposed** CapabilityRevoke / lower-`max_spend` Apply intent bound to drift evidence (`allocation_id`, finding ids, `request_id`). Proposal sits in CP until approved and executed via Apply gates.

| Dimension | Assessment |
|-----------|------------|
| **Security** | Good if proposal ≠ execution; must not auto-sign or skip G1–G14 |
| **Operational** | Speeds remediation; reduces copy-paste error |
| **Compliance** | Acceptable if proposal is non-authoritative until Apply succeeds |
| **Verdict** | **Adopt as proposal-only** after Apply path exists for revoke/reduce; never auto-execute |

### 3.4 Option E4 — Automatic capability mutation

**Behaviour:** Sync job or treasury event directly revokes/reduces capabilities (or CP writes CapabilityStore).

| Dimension | Assessment |
|-----------|------------|
| **Security** | **Unacceptable** — silent permission change; bypasses Apply; violates INV-S01/S05/S07 |
| **Operational** | Fast but opaque; race-prone; hard to recover intent |
| **Compliance** | Fails “no silent permission changes” and complete human-accountable trail |
| **Verdict** | **Reject** |

### 3.5 Comparative matrix

| Option | Silent perm change? | Money movement? | Bypasses PROTO-0? | Recoverability |
|--------|---------------------|-----------------|-------------------|----------------|
| E1 Human workflow | No | No (unless separate treasury approve) | No | High |
| E2 Runtime deny | No | No | No | High (deny is reversible by fixing funding/grant) |
| E3 Reduction proposal | No (until Apply) | No | No | High |
| E4 Auto mutation | **Yes** | Possible side effects | **Yes** | Low |

---

## 4. Drift Severity Model

### 4.1 Levels

| Level | Meaning | Default response |
|-------|---------|------------------|
| **INFO** | Informational asymmetry; spend not over-permissioned | Log; optional dashboard; no case required |
| **WARNING** | Authority mismatch that can become unsafe; spend must fail closed | Runtime DENY; open review case; SLA hours |
| **CRITICAL** | Active protocol authority exceeds / outlives funding or identity is frozen | Runtime DENY; urgent case; dual-control remediation; page on-call if configured |

Overall case severity = **max** finding severity (same pattern as Phase 24 status escalation).

### 4.2 Example mapping

| Example | Typical sync finding | Severity | Immediate (E2) | Remediation (E1/E3) |
|---------|----------------------|----------|----------------|---------------------|
| **Expired allocation** with active capability | `expired_allocation_active_capability` | **CRITICAL** | DENY spend | Propose revoke/reduce **or** renew allocation then re-observe |
| **Overscoped capability** (`max_spend` > ceiling) | `capability_limit_exceeds_allocation` | **CRITICAL** | DENY above `min()`; prefer DENY all spend until reviewed if policy strict | Propose lower grant **or** increase allocation (treasury dual-control) |
| **Missing funding** (cap, no allocation) | `capability_without_allocation` | **WARNING** → **CRITICAL** if production spend claimed | DENY | Fund + bind **or** revoke grant |
| **Allocation without capability** | `allocation_without_capability` | **INFO** / **WARNING** | DENY spend (no protocol auth) | Optional Apply grant with backing (B); or reclaim unused allocation |
| **Frozen agent / identity** | (identity freeze + any live spend grant) | **CRITICAL** | DENY | Keep freeze; Apply revoke stale grants; treasury freeze optional |
| **Budget exhaustion** (`remaining` = 0, cap still live) | may still be `SYNC_OK` if ceiling match | **WARNING** (ops) / **CRITICAL** if reserves bypass attempted | DENY new reserve/spend | Top-up allocation **or** reduce/revoke capability |
| **Frozen / closed allocation** with active cap | `inactive_allocation_active_capability` | **CRITICAL** | DENY | Propose revoke; unfreeze only via treasury dual-control + re-observe |

**Note:** Exhaustion may not appear as Phase 24 “drift” if ceilings still match. Enforcement must still bind **remaining**, not only ceiling (INV-S04 / Hybrid D).

### 4.3 Relation to Phase 24 status codes

| Phase 24 | Severity band (default) |
|----------|-------------------------|
| `SYNC_OK` | none / INFO exhaustion-only signals from live remaining checks |
| `DRIFT_DETECTED` | WARNING (escalate to CRITICAL under production spend policy) |
| `REQUIRES_REVIEW` | CRITICAL |

---

## 5. Future Lifecycle

```text
Detection          Phase 24 observe / continuous sync job / pre-spend check
    ↓
Decision           Map findings → severity; choose E2 outcome (DENY/ESCALATE)
                   + recommended remediation class (fund | revoke | reduce | freeze | waive-lab)
    ↓
Approval           Human workflow (E1); dual-control for CRITICAL / high-risk treasury
                   Capability changes only as approved Apply intents (E3 → Apply)
    ↓
Execution          Treasury mutations via treasury write façade only
                   PROTO-0 mutations via Apply only (when enabled)
                   Runtime gate continuously re-reads live state
    ↓
Audit              Correlation: request_id, organisation_id, agent_id,
                   allocation_id, capability_id, finding_ids, case_id,
                   approval_id, operation_id, actor, severity, outcome
```

### 5.1 Decision classes (non-exhaustive)

| Class | Owner plane | Gate |
|-------|-------------|------|
| `runtime_deny` | CP spend gate / agent runtime | Automatic; audited |
| `fund_or_increase_allocation` | Treasury write | Existing dual-control rules |
| `decrease_or_freeze_allocation` | Treasury write | Existing dual-control / freeze rules |
| `propose_capability_revoke_or_reduce` | CP → Apply | Apply approvals + enablement |
| `freeze_identity` | Apply | Apply gates |
| `lab_unfunded_waiver` | CP admin | Audited; **banned in production** |

### 5.2 Recoverability

| Failure | Recovery |
|---------|----------|
| Wrong DENY | Fix funding/grant; next live check ALLOW — no rewind of PROTO-0 needed |
| Wrong Apply revoke | New Apply grant with backing (B); audit prior revoke |
| Wrong treasury decrease | Dual-control increase; never invent journal |
| Auto-mutation (forbidden) | N/A — must not exist; if bug introduces it, freeze Apply + incident |

---

## 6. Security Requirements (normative)

| ID | Requirement |
|----|-------------|
| **INV-E01** | **No silent permission changes** — capability grant/revoke/reduce only via Apply (when enabled), never by sync/enforcement jobs |
| **INV-E02** | **No automatic money movement** — enforcement must not fund, release, settle, or adjust journals as a side effect of drift |
| **INV-E03** | **No bypassing PROTO-0** — CP must not write CapabilityStore/IdentityRegistry; Treasury must not |
| **INV-E04** | **Complete audit trail** — Detection→Decision→Approval→Execution linked by stable correlation ids |
| **INV-E05** | **Recoverability** — Prefer reversible DENY and explicit compensating Apply/treasury actions over in-place silent heal |
| **INV-E06** | **Fail closed** — Unavailable treasury or protocol for spend decision → DENY/ESCALATE, never ALLOW on stale cache alone (INV-S06) |
| **INV-E07** | **Severity honesty** — CRITICAL findings cannot be auto-downgraded without audited policy + dual-control |
| **INV-E08** | **Enforcement never flips `apply_enabled`** |

These sit alongside INV-S01…S08.

---

## 7. Sequencing vs Apply Enablement

| Enforcement piece | Before Apply enablement? | After Apply enablement? |
|-------------------|--------------------------|-------------------------|
| **E2 Runtime denial** | **Yes — preferred next impl** for any product spend path | Required continuously |
| **E1 Human workflow / cases** | **Yes** (cases without Apply execution) | Cases can spawn Apply proposals |
| **E3 Capability reduction proposals** | Can **draft** only; cannot execute | Execute via Apply revoke/reduce |
| **E4 Auto mutation** | Never | Never |
| **Apply-time backing (Hybrid B)** | Design/impl behind flag | Required for production spend grants |

### Normative answer

**Runtime enforcement (E2) should be implemented and proven before enabling production spend CapabilityGrant via Apply.**

**Capability-plane remediation (revoke/reduce) cannot complete until Apply is enabled for those intents** — until then, operators rely on E2 DENY + treasury freeze/decrease to stop funding, with explicit backlog of Apply proposals.

Freeze-only Apply (if separately gated) may proceed in parallel; it does **not** replace E2 for funding drift.

```text
Phase 24 observation (done)
    → Phase 25 design (this doc)
    → Future: E2 spend-gate impl + E1 case workflow   [Apply still off OK]
    → Future: Apply enablement (freeze / then spend intents)
    → Future: E3 proposals execute through Apply
```

---

## 8. Implementation Shape (future — not authorised)

Illustrative only:

| Component | Responsibility |
|-----------|----------------|
| `DriftCase` store | Severity, findings, status, owners, SLA |
| Spend gate library | E2 ALLOW/DENY/ESCALATE using live reads |
| Remediation actions API | Creates treasury mutation requests or Apply proposals — never direct PROTO-0 |
| Console | Case queue; no “Heal now” that skips approval |

**Out of scope forever for sync jobs:** direct `CapabilityStore` mutation, auto-fund, auto-settle.

---

## 9. Compliance Position

| Question | Position |
|----------|----------|
| Does enforcement create custody? | **No** |
| Does DENY create money transmission duties? | **No** |
| Does auto capability mutation create governance failure? | **Yes** — rejected |
| External money movement? | **Forbidden** |

---

## 10. Freeze Statement

Recommended enforcement = **E2 + E1 + E3(proposal-only)**; **E4 rejected**. Severity model INFO/WARNING/CRITICAL and lifecycle Detection→Decision→Approval→Execution→Audit are **frozen** for subsequent implementation phases.

**This document does not authorise implementation.**
