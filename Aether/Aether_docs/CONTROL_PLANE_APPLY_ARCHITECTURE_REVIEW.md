# Control Plane Apply Architecture Review — Final Authorisation Gate

**Document type:** Architecture review (pre-implementation authorisation gate)  
**Date:** 2026-07-30  
**Reviewer scope:** Phase 4B frozen design + current non-mutating runtime  
**Related:** [CONTROL_PLANE_PHASE4B_SECURITY_GATE.md](CONTROL_PLANE_PHASE4B_SECURITY_GATE.md), [CONTROL_PLANE_APPLY_SECURITY_MODEL.md](CONTROL_PLANE_APPLY_SECURITY_MODEL.md), [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md), [CONTROL_PLANE_APPLY_TEST_PLAN.md](CONTROL_PLANE_APPLY_TEST_PLAN.md), [CONTROL_PLANE_MILESTONE_2_PLAN.md](../Project_Phases/phase_3/CONTROL_PLANE_MILESTONE_2_PLAN.md), [CONTROL_PLANE_SECURITY_MODEL.md](CONTROL_PLANE_SECURITY_MODEL.md)

---

## Executive Summary

This review evaluates whether Aether is architecturally ready to cross from **governance planning** into **governance execution** (PROTO-0 mutations via a controlled Apply path).

**Current runtime state (verified):**

- Control Plane is **completely non-mutating**
- No Apply routes exist (`POST /api/policies/:id/dry-run` only)
- No PROTO-0 grant/revoke callers in Control Plane source
- `ExecutionMode::Live` is reserved; executor always uses `DryRun`
- Protocol adapters are read-only (`&ProtocolState`)
- `production_config_check()` wired at startup; `execution_hash` emitted on dry-run reports

**Design state:**

- Apply pipeline, authority invariants, replay model, execution binding, concurrency rules, and test plan are **frozen in documentation**
- Replay store, Apply approval binding store, Apply routes, and PROTO-0 write adapters are **not implemented**

**Conclusion:** The architecture is **sound enough to authorise Apply *implementation planning*** subject to mandatory pre-ship conditions. Aether is **not** authorised to execute protocol mutations today.

---

## Gate Decision

# **PASS WITH CONDITIONS**

### Justification

| Criterion | Result |
|-----------|--------|
| Authority model coherent; PROTO-0 sole protocol authority | **Pass** |
| Control Plane non-mutating today | **Pass** (code verified) |
| Apply pipeline specified with fail-closed stage contracts | **Pass** (design) |
| Execution hash replaces count fingerprint for binding | **Pass** (implemented on dry-run) |
| Replay model specifies one-shot execution | **Pass** (design only) |
| TOCTOU scenarios addressed in design | **Pass with gaps** (see §4) |
| Audit reconstructability | **Conditional** (M7 open; Apply fields not yet written) |
| Test plan exists | **Pass** (Apply-path tests not implemented) |
| Ready to mutate protocol now | **Fail** (by design — implementation blocked) |

**PASS WITH CONDITIONS** means: architecture review approves proceeding to **Apply implementation** only after listed mandatory actions close and a **separate explicit implementation authorisation** is recorded. It does **not** permit mutations until those conditions are met.

---

## 1. Authority Review

### Documented and verified invariants

| ID | Invariant | Evidence | Status |
|----|-----------|----------|--------|
| **AUTH-1** | PROTO-0 is the sole protocol authority | No CP write adapters; authority model I1–I2; Apply model § Authority | **Verified (runtime)** |
| **AUTH-2** | Control Plane never owns protocol authority | CP requests/signs; does not grant capabilities itself | **Verified (design + runtime)** |
| **AUTH-3** | Signer authenticates; does not authorise | `SigningGateway` signs/verifies only; signer success ≠ protocol success | **Verified** |
| **AUTH-4** | Raw signing keys confined to signer module | `EnterpriseKeyMaterial` private to `enterprise_signer.rs` | **Verified** |
| **AUTH-5** | Policy template approval ≠ protocol activation | `protocol_active: false` on approve; API notes | **Verified** |
| **AUTH-6** | Policy template approval ≠ Apply authorisation | Separate frozen stage “Human Approval” binds `dry_run_id` + `execution_hash` | **Design only** |
| **AUTH-7** | Dry-run never mutates protocol state | Count fingerprint before/after; integration tests | **Verified** |
| **AUTH-8** | Dry-run signatures are not Apply evidence | Apply model + replay model; dry-run `action` suffix `.dry_run` | **Design** (enforcement pending Apply) |
| **AUTH-9** | `execution_hash` binds approval to one intended mutation | SHA-256 over policy id/version/content, target, intent, params | **Verified (dry-run emission)** |
| **AUTH-10** | Replay model: signed Apply op executes at most once | `signed_operation_replay` design; unique `operation_id` | **Design only** |
| **AUTH-11** | Browser never holds PROTO-0 keys | Security model non-negotiables | **Verified** |
| **AUTH-12** | Escrow/settlement/reputation out of CP write scope | Authority model §4.3; no adapters | **Verified** |

### Critical distinction (must not be conflated in implementation)

Two separate approval concepts exist:

| Concept | What it approves | Current state |
|---------|------------------|---------------|
| **Template approval** | CP policy lifecycle (`draft` → `approved`) | **Implemented** |
| **Apply approval** | Execution of a specific `dry_run_id` + `execution_hash` binding | **Not implemented** |

Implementation must treat template approval as a **prerequisite** only. Apply approval is the binding that authorises a single mutation attempt.

### Authority findings

- **Strength:** Clear separation between CP governance records and PROTO-0 authority is consistent across all Phase 4B documents and matches runtime behaviour.
- **Gap:** Apply approval binding has no durable store or API yet — only specification.
- **Gap:** `GovernanceOperation` uses `action` strings (e.g. `protocol.capability.grant.dry_run`) rather than an explicit `purpose` enum (`dry_run` \| `apply`). Implementation must enforce purpose separation at verification time.

---

## 2. Apply Pipeline Review

### Expected flow (review target)

```text
Policy
  ↓
Dry Run
  ↓
Execution Hash
  ↓
Approval
  ↓
Signer
  ↓
Apply
  ↓
PROTO-0
  ↓
Mutation
  ↓
Audit
```

### Frozen design flow (CONTROL_PLANE_APPLY_SECURITY_MODEL.md)

```text
Policy Version → Dry Run → Execution Hash → Human Approval
  → Signed Operation → Protocol Adapter → PROTO-0 Authority Check → Mutation
```

### Reconciliation

The flows are **equivalent** when mapped as:

| Review stage | Frozen stage | Notes |
|--------------|--------------|-------|
| Policy | Policy Version | Must be `approved` template with stable `policy_version` + content `hash` |
| Dry Run | Dry Run | Produces `dry_run_id`, validation report, dry-run signature |
| Execution Hash | Execution Hash | Logical artifact; **computed during dry-run** in current implementation |
| Approval | Human Approval | **Apply-specific** binding — not template approve |
| Signer | Signed Operation (part 1) | Mint Apply-purpose signature after binding validated |
| Apply | Signed Operation (part 2) + Protocol Adapter | HTTP/route + adapter invocation |
| PROTO-0 | PROTO-0 Authority Check | Protocol validation decides |
| Mutation | Mutation | Only if PROTO-0 accepts |
| Audit | (cross-cutting) | Must record every stage outcome |

### Legal successor graph

Each stage has exactly one legal successor when successful; any validation failure terminates in **reject + audit** (no mutation).

```text
Policy Version ──(status≠approved)──► REJECT
       │
       ▼ approved
Dry Run ──(simulation/signer fail)──► REJECT
       │
       ▼ executable
Execution Hash ──(recompute mismatch)──► REJECT
       │
       ▼ bound
Human Approval ──(binding invalid/expired/cancelled)──► REJECT
       │
       ▼ bound approval record
Signed Operation ──(verify/replay/expiry fail)──► REJECT
       │
       ▼ reserved in replay store
Protocol Adapter ──(adapter error)──► REJECT (no PROTO-0 call)
       │
       ▼
PROTO-0 Authority Check ──(protocol reject)──► REJECT (audited; no state change)
       │
       ▼ accept
Mutation ──► Audit (terminal success)
```

### Ambiguities to resolve at implementation (not blockers to design pass)

| # | Ambiguity | Required resolution |
|---|-----------|---------------------|
| P1 | “Approval” overloaded with template approve | Name Apply binding `apply_approval` or `execution_approval` in schema/API |
| P2 | `confirm: true` required by WRITE_SECURITY W5 | Apply request body must include explicit confirmation |
| P3 | Re-simulation at Apply time | Design implies PROTO-0 check at Apply; **must re-run read-only simulation immediately before reserve→execute** to close protocol-state TOCTOU |
| P4 | Approval TTL | Frozen docs say “expired approval rejected” but do not fix TTL; implementation must define (recommend ≤ dry-run age, e.g. 15–60 min) |

### Pipeline finding

**Pass with conditions.** Pipeline is unambiguous at the architectural level. Implementation must not skip stages or merge Signer+Apply without replay reservation ordering: **reserve → verify → PROTO-0 → finalise**.

---

## 3. Replay & Idempotency Review

### Requirements vs design

| Requirement | Design coverage | Implementation |
|-------------|-----------------|----------------|
| Duplicate `operation_id` impossible | PK on `signed_operation_replay.operation_id` | **Not built** |
| `execution_hash` immutable | Derived from policy content; policy edit changes hash → stale binding rejected | **Hash function built**; binding store **not built** |
| Signed operation executes once | Reserve → execute → finalise; replay rejects | **Design only** |
| Retries cannot mutate twice | Client retry after success → prior result / conflict | **Design only** |
| Duplicate Apply → no-op or reject | Same `operation_id` → reject; different id + same payload → PROTO-0 may reject duplicate grant | **Design only** |

### Crash recovery (`status=reserved`)

Replay model correctly mandates **fail closed** after reserve-without-finalise. Implementation must:

1. Never auto-retry PROTO-0 for `reserved` rows
2. Expose operator-visible “stuck operation” state for manual reconcile
3. Audit the terminal outcome after reconcile

**Additional test required:** reserved-row crash recovery (not in current test plan).

### Dry-run vs Apply signature separation

| Property | Dry-run | Apply |
|----------|---------|-------|
| Purpose | `dry_run` (via action string today) | `apply` (must be explicit at implementation) |
| Replay store | Not inserted | Reserved before PROTO-0 |
| Reusable for Apply | **Forbidden** | N/A |

### Replay finding

**Pass with conditions.** Model is correct; enforcement is entirely future work. Implementation authorisation must not proceed without replay store as a hard dependency.

---

## 4. TOCTOU Review

| Scenario | Expected behaviour | Design support | Gap |
|----------|-------------------|----------------|-----|
| Policy edited after template approve, before Apply | `policy_version` / content `hash` / `execution_hash` mismatch → reject | Yes | Needs Apply approval store comparing live template |
| Policy edited after dry-run | `execution_hash` mismatch → reject | Yes | Verified hash sensitivity in unit tests |
| Dry-run regenerated (new `dry_run_id`) | Old Apply approval invalid unless bound to new dry-run | Yes | Apply approval must reference exact `dry_run_id` |
| Capability revoked before Apply | PROTO-0 rejects; CP records failure | Implied | **Must re-simulate at Apply** — not explicit in frozen docs |
| Protocol state changed (external) | PROTO-0 rejects; no CP bypass | Implied | `execution_hash` does **not** bind protocol snapshot — acceptable if Apply re-simulates |
| Signer changed / rotated | Old Apply signatures fail verification | Implied | Document signer identity in replay row (done) |
| Concurrent Apply (same binding) | One wins via DB unique constraint; other rejects | Yes | |
| Concurrent Apply (different versions) | Only valid version+hash for current template | Yes | |
| Template approve race (update vs submit) | Status-guarded SQL | **Remediated** | C5/C6 tests still open |
| Client disconnect mid-Apply | Reserved row fail-closed; no blind retry | Yes | Needs crash-recovery test |

### TOCTOU finding

**Pass with conditions.** Policy-side TOCTOU is well covered by `execution_hash` + version binding. Protocol-side TOCTOU relies on PROTO-0 + mandatory re-simulation at Apply — **must be added to implementation spec** as a hard gate before PROTO-0 call.

---

## 5. Audit Review

### Required reconstruction fields

| Field | Dry-run today | Future Apply | Gap |
|-------|---------------|--------------|-----|
| `request_id` | `audit_log` + `signer_audit` | Required | OK for dry-run |
| `operation_id` | `DRY_RUN_COMPLETED` metadata | Required | OK for dry-run |
| `dry_run_id` | `DRY_RUN_*` metadata | Required | OK (audit JSON) |
| `execution_hash` | `DRY_RUN_COMPLETED` metadata | Required | OK |
| `policy_version` | Metadata | Required | OK |
| `approver` | N/A (dry-run is operator) | Apply approver identity | **Not implemented** |
| `signer` / signer event | `signer_audit` | Required | OK for dry-run sign |
| `protocol_result` | N/A (`not_applicable`) | `success` / `rejected` | Apply not built |
| `timestamp` | `audit_log.created_at` | Required | OK |

### Linkage gaps

| Gap | Severity | Action |
|-----|----------|--------|
| **M7:** Dry-run not in `mutation_audit` | Medium | Implement before Apply — unified timeline |
| No durable `dry_run_attestations` table | Medium | Audit JSON is searchable but fragile; recommend indexed table keyed by `dry_run_id` |
| No `apply_approvals` table | High | Required for binding enforcement |
| No formal correlation schema across 3+ tables | Low | Governance timeline view recommended |
| Tamper-evident audit chain (SEC-CP-03) | Medium | Post-MVP hardening |

### Audit finding

**Conditional pass.** Dry-run audit is sufficient for pre-Apply operations. Apply cannot ship without `mutation_audit` rows for Apply attempts and M7 closure.

---

## 6. Threat Review

| Threat | Likelihood | Impact | Mitigation (design + existing) | Residual risk |
|--------|------------|--------|-------------------------------|---------------|
| **Replay (signed Apply op)** | High if shipped without store | Critical | Replay model; unique `operation_id`; reserve-before-mutate | **High until implemented** |
| **Replay (dry-run sig as Apply)** | Med | Critical | Purpose separation; S6 test | Med until Apply verifier ships |
| **Confused deputy (CP bypasses PROTO-0)** | Low today | Critical | No write adapters; PROTO-0 final check | Low if adapters stay thin |
| **Stale approval / TOCTOU** | High at Apply time | High | `execution_hash` + version + `dry_run_id`; re-simulate | Med — re-simulate must be mandatory |
| **Privilege escalation** | Low | High | RBAC; viewer tests | Low |
| **Signer compromise** | Med | Critical | Prod seed required; KMS roadmap; short Apply TTL | High without KMS/HSM |
| **Concurrent Apply** | Med | Med | DB constraints; version checks | Low if implemented as designed |
| **Malicious operator** | Med | High | SoD on template approve; audit; PROTO-0 narrowing | Med — no multi-party approval in MVP |
| **Rollback attack** | Low | Med | PROTO-0 append-only semantics; no CP rollback API | Low |
| **CSRF on Apply** | Med | High | CSRF + Origin; consume on Apply | Med until Apply route ships |
| **Audit tampering** | Med | High | Append-only SQLite; external sink recommended | Med |
| **Default credentials / weak signer** | High in dev | Critical | `production_config_check` | Low in prod when gate enabled |
| **Origin missing (non-browser)** | Med | Med | Allowed today for API tooling | Med for prod automation paths |

---

## 7. Testing Readiness

### Current coverage (adequate for pre-Apply)

- P1–P6 production config tests — **implemented**
- H1–H3 execution hash — **implemented**
- P4A dry-run security — **implemented**
- M1 observatory / auth regression — **implemented**

### Apply test plan assessment

The [CONTROL_PLANE_APPLY_TEST_PLAN.md](CONTROL_PLANE_APPLY_TEST_PLAN.md) covers the **minimum** security and concurrency cases. **Sufficient as a baseline** for implementation gating.

### Recommended additional tests (before Apply enable)

| ID | Test | Rationale |
|----|------|-----------|
| **T-RC1** | Crash after replay `reserved`, before finalise | Validates fail-closed recovery |
| **T-RC2** | Apply re-simulation rejects when protocol changed since dry-run | Closes protocol TOCTOU |
| **T-RC3** | Governance timeline reconstruction (e2e) | Proves audit linkage across tables |
| **T-RC4** | `confirm: false` on Apply → reject | WRITE_SECURITY W5 |
| **T-RC5** | CSRF consumed on Apply | Single-use high-risk mutation |
| **T-RC6** | Signer identity mismatch (rotated key) → reject | Signer TOCTOU |
| **T-RC7** | Apply with archived policy → reject | Lifecycle edge |
| **T-RC8** | PROTO-0 reject still produces full audit trail | Failure-path completeness |
| **T-RC9** | C5/C6 concurrent policy lifecycle races | Listed in plan but not yet implemented |

### Testing finding

**Conditional pass.** Plan is adequate; expanded list above should be merged into the Apply implementation PR checklist.

---

## 8. Architecture Findings (Summary)

| Area | Assessment |
|------|------------|
| Layering (Observatory → Policy → Signer → Dry-run → [Apply]) | Sound |
| Protocol boundary | Clean; read-only today |
| Production hardening | M1/M6 implemented |
| Content binding | `execution_hash` replaces count fingerprint for Apply |
| Frozen documentation set | Complete and internally consistent |
| Implementation | **Zero Apply / PROTO-0 write code** — correct for this gate |

---

## 9. Missing Tests

- All **S1–S7** and **C1–C4** (Apply-path) — blocked until Apply coding
- **C5, C6** (policy concurrency) — planned pre-Apply, not yet implemented
- **T-RC1–T-RC9** (recommended above) — not in plan yet

---

## 10. Remaining Risks

1. **Implementation drift** — Apply ships without replay store or approval binding store.
2. **Approval conflation** — Template approve mistaken for Apply authorisation.
3. **Protocol TOCTOU** — Without mandatory re-simulation at Apply, stale dry-run could reach PROTO-0 with changed world state (PROTO-0 should still reject, but CP must not skip checks).
4. **`reserved` crash state** — Operational burden if fail-closed is not UX-visible.
5. **Audit fragmentation** — Three tables without M7 / timeline view complicates incident response.
6. **Signer custody** — Server-held keys remain MVP risk until KMS/HSM.
7. **No multi-party approval** — Single admin can bind Apply (accepted MVP scope).

---

## 11. Recommendation

### For governance planning → governance execution transition

| Question | Answer |
|----------|--------|
| Is the Apply **architecture** fit to implement? | **Yes**, subject to conditions below |
| May Aether **execute protocol mutations** now? | **No** |
| May Apply **implementation** begin? | Only after **separate explicit implementation authorisation** and closing mandatory actions |

### Mandatory actions before Apply implementation

| # | Action | Owner |
|---|--------|-------|
| **A1** | Record **separate Apply implementation authorisation** (distinct from this architecture review) | Governance / Architecture |
| **A2** | Implement `signed_operation_replay` store per replay model | Engineering |
| **A3** | Implement Apply approval binding store (`policy_id`, `policy_version`, `dry_run_id`, `execution_hash`, `request_id`, approver, expiry) | Engineering |
| **A4** | Implement durable `dry_run_attestations` lookup (table or indexed store) | Engineering |
| **A5** | Close **M7** — dry-run rows in `mutation_audit` | Engineering |
| **A6** | Implement Apply route with: `confirm: true`, CSRF consume, purpose=`apply`, replay reserve→execute→finalise | Engineering |
| **A7** | Implement PROTO-0 write adapters as **only** mutation boundary; no `aether-core` changes | Engineering |
| **A8** | Mandatory re-simulation immediately before PROTO-0 call | Engineering |
| **A9** | Implement S1–S7, C1–C4, T-RC1–T-RC9; C5–C6 | Engineering |
| **A10** | Re-run security gate after Apply implementation (before any environment enables mutations) | Security |

### Recommended (non-blocking)

- Governance timeline SQL/API view
- Tamper-evident audit chain (SEC-CP-03)
- MFA for admin (SEC-CP-01)
- Update WRITE_SECURITY.md status header (doc drift)

---

## 12. Freeze Statement

> **Apply Architecture Review: PASS WITH CONDITIONS.** Aether is architecturally ready to **plan and implement** a governed Apply path. Aether is **not** authorised to perform protocol mutations. The Control Plane remains dry-run only. PROTO-0 remains sole protocol authority. Apply implementation requires separate explicit authorisation and closure of mandatory actions A1–A10.

---

## Appendix — Reviewed Artifacts

| Document | Version basis | Role in review |
|----------|---------------|----------------|
| CONTROL_PLANE_PHASE4B_SECURITY_GATE.md | 2026-07-29 | Prior gate; remediation status |
| CONTROL_PLANE_APPLY_SECURITY_MODEL.md | 2026-07-29 | Frozen pipeline + invariants |
| SIGNED_OPERATION_REPLAY_MODEL.md | 2026-07-29 | One-shot execution |
| CONTROL_PLANE_APPLY_TEST_PLAN.md | 2026-07-29 | Pre-ship test checklist |
| CONTROL_PLANE_MILESTONE_2_PLAN.md | 2026-07-29 | Sequencing + blockers |
| CONTROL_PLANE_SECURITY_MODEL.md | 2026-07-29 | Signing roadmap + non-negotiables |

### Code verification (2026-07-30)

- `grep`: no `grant_capability` / `revoke_capability` callers in `control_plane/src`
- Routes: `/policies/:id/dry-run` only; `apply_enabled: false`
- `production_config_check` called in `lib.rs::run`
- `execution_hash`, `dry_run_id` on `DryRunReport` and audit metadata
