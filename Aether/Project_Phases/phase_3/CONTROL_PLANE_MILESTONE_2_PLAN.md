# Control Plane Milestone 2 Plan — Policy Management

## Status

**Phase 4A complete — Dry-run execution pipeline; Phase 4B Security Gate: PASS WITH CONDITIONS**  
**Phase 4B Apply Safety Design & Gate Remediation — complete (design freeze; no Apply)**  
**Phase 4B Apply Specification Freeze — complete (protocol spec frozen; no implementation)**  
**Phase 4B Apply Implementation Blueprint — complete (design freeze; no Rust)**

Date: 2026-07-30

Prerequisite: Milestone 1 complete. Milestone 2 Phases 1–3 complete. Phase 4A dry-run complete.

**Security gate:** [CONTROL_PLANE_PHASE4B_SECURITY_GATE.md](../../Aether_docs/CONTROL_PLANE_PHASE4B_SECURITY_GATE.md) — **PASS WITH CONDITIONS**. Do not enable live Apply until remaining blockers in that document are closed and Apply receives separate implementation authorisation.

**Apply architecture (frozen):** [CONTROL_PLANE_APPLY_SECURITY_MODEL.md](../../Aether_docs/CONTROL_PLANE_APPLY_SECURITY_MODEL.md), [SIGNED_OPERATION_REPLAY_MODEL.md](../../Aether_docs/SIGNED_OPERATION_REPLAY_MODEL.md), [CONTROL_PLANE_APPLY_TEST_PLAN.md](../../Aether_docs/CONTROL_PLANE_APPLY_TEST_PLAN.md), [CONTROL_PLANE_APPLY_ARCHITECTURE_REVIEW.md](../../Aether_docs/CONTROL_PLANE_APPLY_ARCHITECTURE_REVIEW.md)

**Apply protocol specification (frozen — implementation-authoritative):**

- [APPLY_PROTOCOL_SPECIFICATION.md](../../Aether_docs/APPLY_PROTOCOL_SPECIFICATION.md)
- [APPLY_STATE_MACHINE.md](../../Aether_docs/APPLY_STATE_MACHINE.md)
- [APPLY_SEQUENCE_DIAGRAMS.md](../../Aether_docs/APPLY_SEQUENCE_DIAGRAMS.md)
- [APPLY_FAILURE_MODEL.md](../../Aether_docs/APPLY_FAILURE_MODEL.md)

**Apply implementation blueprint (frozen — maps spec to code; no implementation):**

- [APPLY_IMPLEMENTATION_BLUEPRINT.md](../../Aether_docs/APPLY_IMPLEMENTATION_BLUEPRINT.md)
- [APPLY_PROTO_ADAPTER_SPEC.md](../../Aether_docs/APPLY_PROTO_ADAPTER_SPEC.md) — **final pre-implementation design**

**Explicit:** No protocol mutations exist. `apply_enabled = false`. Dry-run does **not** call PROTO-0 mutation APIs. Approved policies remain inactive until a future controlled Apply implementation is explicitly authorised and the blueprint §11 readiness checklist is complete.

Related:

- [CONTROL_PLANE_POLICY_THREAT_MODEL.md](../../Aether_docs/CONTROL_PLANE_POLICY_THREAT_MODEL.md)
- [CONTROL_PLANE_AUTHORITY_MODEL.md](../../Aether_docs/CONTROL_PLANE_AUTHORITY_MODEL.md)
- [CONTROL_PLANE_WRITE_SECURITY.md](../../Aether_docs/CONTROL_PLANE_WRITE_SECURITY.md)
- [CONTROL_PLANE_SECURITY_MODEL.md](../../Aether_docs/CONTROL_PLANE_SECURITY_MODEL.md)
- [CONTROL_PLANE_MILESTONE_1_RESULTS.md](CONTROL_PLANE_MILESTONE_1_RESULTS.md)
- [CONTROL_PLANE_MVP_ARCHITECTURE.md](CONTROL_PLANE_MVP_ARCHITECTURE.md)

---

## 1. Objective

Add Policy Management to the Enterprise Agent Governance Console while preserving:

- PROTO-0 as the only protocol write path
- No escrow / settlement / reputation mutation
- Full auditability of operator actions
- Milestone 1 observatory behaviour unchanged for viewers
- No browser-held signing keys
- No Control Plane bypass of protocol validation

---

## 2. Scope (When Implementation Proceeds)

| In scope | Out of scope |
|----------|--------------|
| Secure write foundation (CSRF, Origin, RBAC, audit) | Marketplace / AETH / governance DAO |
| Policy template workflow (create → review → approve) | Per-operator PROTO-0 keys |
| Signer abstraction (no raw key coupling) | Multi-party approval workflows (MVP) |
| PROTO-0 grant / revoke via adapters | Escrow/settlement/reputation writes |
| Optional identity freeze (admin) | Direct protocol DB/store mutation |
| Extended mutation audit fields | HSM/KMS in first ship |

---

## 3. Confirmed Decisions

| Topic | Decision |
|-------|----------|
| Operator authZ | RBAC: `viewer` / `operator` / `admin` |
| Protocol signing | Signer abstraction → MVP enterprise signer (server-held) |
| Multi-party approval | Deferred (production / high-assurance roadmap) |
| CSRF | Synchronizer token + Origin allowlist |
| Template ≠ apply | Create → Review → Approve → Apply request |
| Destructive actions | Require `confirm: true` |
| Protocol authority | PROTO-0 validation is final; CP only requests |

---

## 4. Acceptance Criteria — Design Gate

| Criterion | Status |
|-----------|--------|
| Threat model approved | **Approved** |
| Authority model approved | **Approved** |
| Signing approach approved | **Approved** (abstraction first; enterprise signer MVP) |
| CSRF design approved | **Approved** |
| Audit requirements approved | **Approved** (see authority model § Mutation Audit) |

**Next gate:** Phase 1 complete before any policy-template or PROTO-0 write routes.

---

## 5. Implementation Sequence

### Phase 1 — Secure Write Foundation

**Status: Complete (foundation only — no write routes shipped)**

**No protocol writes. No policy engine. No signing keys.**

Includes:

- CSRF protection implementation (synchronizer token)
- Origin allowlist validation
- Role authorization middleware (`viewer` / `operator` / `admin`)
- Mutation audit framework (append-only schema + writer API)
- Request identity tracking (operator_id, role, request_id, origin)

Exit criteria:

- [x] Missing CSRF → rejected
- [x] Invalid Origin → rejected
- [x] Viewer rejected by operator/admin role primitives
- [x] Audit framework can record a synthetic mutation attempt
- [x] `aether-core` untouched
- [x] Request ID generation + propagation

---


### Phase 2 — Policy Template Workflow

**Status: Implemented**

**Control Plane database only. Template save MUST NOT equal protocol mutation.**

Allowed:

- Create policy template
- Update policy template
- Review policy template
- Approve policy template state (draft → approved)

Flow:

```text
Create Template
      │
      ▼
Review
      │
      ▼
Approve
      │
      ▼
Apply request   ← Phase 4 only; not executed in Phase 2
```

Exit criteria:

- [x] Templates persist in CP DB with lifecycle states
- [x] Approve does **not** call PROTO-0
- [x] Observatory unchanged for agents/capabilities
- [x] Every template mutation audited
- [x] CSRF + Origin + RBAC on mutating policy routes
- [x] Separation of duties (no self-approval)

---

### Phase 3 — Signing Boundary Abstraction

**Status: Implemented**

**Before any real key material is wired to grant/revoke.**

Create abstraction only:

```text
trait / interface Signer {
    sign_operation(...)
    verify_signature(...)
    signer_identity(...)
}
```

Audited call path: `SigningGateway` records `SIGN_REQUEST_CREATED` / `COMPLETED` / `FAILED`.

Requirements:

- [x] Control Plane must **not** depend directly on a raw private key type in business logic
- [x] Call sites depend on `Signer` / `SigningGateway`, not file paths or key bytes
- [x] Design for future backends:

| Horizon | Backend |
|---------|---------|
| MVP | Server-held enterprise signer |
| Future | KMS |
| Future | HSM |
| Future | Dedicated signing service |
| High assurance | Multi-party approval |

Exit criteria:

- [x] Enterprise signer compiles into CP behind the trait
- [x] No production keys required (dev seed / identity-derived key)
- [x] Signer usage is auditable through the gateway
- [x] **No protocol mutations** — signer does not call PROTO-0

---

### Phase 4 — PROTO-0 Mutation Integration

Split:

#### Phase 4A — Dry-Run Execution (complete)

**Status: Implemented — simulation only**

- Execution planner: Approved Policy → GovernanceOperation → ExecutionPlan (`execution_mode = DryRun`)
- Dry-run executor: validates policy, RBAC, signer, adapters, target; builds signed operation; **does not** mutate protocol
- Read-only simulation helpers: `simulate_capability_grant` / `simulate_capability_revoke` / `simulate_policy_apply`
- API: `POST /api/policies/:id/dry-run`
- Audit: `DRY_RUN_REQUESTED` / `COMPLETED` / `FAILED` (no `protocol_result`)

#### Phase 4B — Live Apply (not started)

**Only after Apply specification freeze + separate implementation authorisation.**

#### Apply specification freeze (complete — 2026-07-30)

Normative protocol documents. Implementations must conform byte-for-byte on hashing, state transitions, and failure behaviour:

| Document | Contents |
|----------|----------|
| [APPLY_PROTOCOL_SPECIFICATION.md](../../Aether_docs/APPLY_PROTOCOL_SPECIFICATION.md) | Gates, hashes, signatures, replay, audit, error codes |
| [APPLY_STATE_MACHINE.md](../../Aether_docs/APPLY_STATE_MACHINE.md) | All states, timeouts, retries, cancellation |
| [APPLY_SEQUENCE_DIAGRAMS.md](../../Aether_docs/APPLY_SEQUENCE_DIAGRAMS.md) | Normal, replay, expiry, PROTO reject, crash paths |
| [APPLY_FAILURE_MODEL.md](../../Aether_docs/APPLY_FAILURE_MODEL.md) | Fail-closed matrix, reconcile, audit per failure |

Frozen constants: `APPLY_APPROVAL_TTL=60m`, `APPLY_SIGNATURE_TTL=15m`, `REPLAY_RESERVE_TTL=5m`, `DRY_RUN_BINDING_MAX_AGE=60m`.

#### Apply implementation blueprint (complete — 2026-07-30)

Maps frozen spec → modules, APIs, SQLite, pipeline, tests: [APPLY_IMPLEMENTATION_BLUEPRINT.md](../../Aether_docs/APPLY_IMPLEMENTATION_BLUEPRINT.md). **No Rust written.** `CP_APPLY_ENABLED` defaults `false`.

#### Apply readiness requirements (must be true before coding Apply)

| Requirement | Status |
|-------------|--------|
| Production config gate (`production_config_check`) | **Done** |
| Secure cookies configurable + required in prod | **Done** |
| Content-addressed `execution_hash` on dry-run | **Done** |
| Apply security model frozen | **Done** |
| Replay model frozen (`execute at most once`) | **Done** |
| Apply test plan accepted | **Done** (implementation deferred) |
| Apply protocol specification frozen | **Done** |
| Apply implementation blueprint frozen | **Done** |
| Dry-run row in `mutation_audit` (M7) | **Open** |
| Separate Apply **implementation** authorisation | **Open** |
| Replay store + Apply routes | **Blocked** |

Rules:

- No direct database mutation of protocol state
- All writes go through protocol adapters
- Protocol validation decides final outcome
- Every mutation creates audit evidence
- Dry-run remains mandatory pre-flight before Apply
- Approval binds to `policy_id` + `policy_version` + `dry_run_id` + `execution_hash` + `request_id` + approver
- Dry-run signatures must not be reused for Apply
- A signed Apply operation executes at most once

Flow (frozen):

```text
Policy Version → Dry Run → Execution Hash → Human Approval
  → Signed Operation → Protocol Adapter → PROTO-0 Authority Check → Mutation
```

In scope for Phase 4 (after approval):

- Apply approved policy → `grant_capability`
- Revoke capability → `revoke_capability`
- Optional: freeze identity (`admin` + confirm)

Exit criteria:

- Invalid mutations rejected by PROTO-0; CP records failure
- Successful grants/revokes visible in Milestone 1 observatory
- Protocol stores remain authoritative
- Replay / stale dry-run / concurrent approval tests green

---

### Phase 5 — Security Validation

**Before Milestone 2 completion.**

#### Authentication / authorization

- Correct role required for each write class
- Unauthorized roles rejected (403)

#### CSRF / Origin

- Missing token rejected
- Invalid Origin rejected

#### Audit

- Every mutation attempt logged
- Failed attempts logged
- Signer usage logged

#### Protocol

- Invalid mutations rejected
- Protocol state remains authoritative
- No escrow / settlement / reputation writes possible via CP

#### Regression

- `cargo test -p aether-control-plane`
- `cargo test -p aether-core` (unchanged)
- `cargo clippy --all-targets -- -D warnings`

---

## 6. Security Risks to Track During Implementation

| Risk | Mitigation |
|------|------------|
| Shipping writes before Phase 1 | Hard sequence gate |
| Template approve conflated with apply | Phase 2 lifecycle states; apply only in Phase 4 |
| Raw key leakage into handlers | Phase 3 Signer boundary |
| Viewer privilege escalation | Middleware tests in Phase 1 and 5 |
| Audit gaps on failure | Log all attempts; Phase 5 checklist |
| Double-apply flooding | Idempotency + PROTO-0 semantics |

---

## 7. Success Criteria (Post-Implementation)

Milestone 2 is complete only when:

1. Phases 1–5 are done
2. Operators can create → review → approve templates, then apply via PROTO-0, and revoke capabilities
3. Viewers remain read-only
4. Control Plane cannot mutate escrow/settlement/reputation
5. Signer is abstracted; browser never holds keys
6. CSRF + Origin enforced on all writes
7. Mutation audit fields complete (see authority model)
8. Protocol tests remain green
9. Results documented and security review recorded

---

## 8. Remaining Blockers Before Live Apply (Phase 4B)

| Blocker | Status |
|---------|--------|
| Phases 1–3 foundations | **Cleared** |
| Phase 4A dry-run pipeline | **Cleared** |
| Phase 4B security gate (PASS WITH CONDITIONS) | **Cleared for design** |
| Production config + Secure cookies + `execution_hash` | **Cleared** |
| Apply architecture / replay / test plan freeze | **Cleared** |
| Apply protocol specification freeze | **Cleared** |
| M7 dry-run `mutation_audit` | **Open** |
| Separate Apply **implementation** authorisation | **Open** |
| Live PROTO-0 grant/revoke adapters | **Blocked** |
| Apply route + confirmation + replay store | **Blocked** |
| Wire dry-run gate before Apply | **Blocked** |

**Phase 4B Apply Safety Design complete.** Execution remains simulation only. No protocol mutations.

---

## Freeze Statement

> Dry-run execution pipeline complete. Apply protocol v1 and implementation blueprint v1 frozen. `apply_enabled = false`. No protocol mutations exist. Apply coding may begin only after explicit implementation authorisation; enablement requires blueprint §11 checklist. The signer authenticates; the protocol decides.
