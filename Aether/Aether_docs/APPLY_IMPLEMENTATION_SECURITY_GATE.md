# Apply Implementation Security Gate

**Document:** `APPLY_IMPLEMENTATION_SECURITY_GATE.md`  
**Phase:** 9 — End-to-End Security & Architecture Gate  
**Scope:** Control Plane Apply pipeline (Phases 1–8)  
**Date:** 2026-07-30  
**Review mode:** Architecture, authority, and security review only — no protocol mutations implemented or enabled.

---

## Executive Summary

The Apply implementation through Phase 8 is a **fail-closed, non-mutating orchestration stack**. Protocol state cannot change through Apply today:

- `apply_enabled()` is hard-coded `false`
- The sole production call site of `protocol::proto0_write::execute` finalises as `APPLY_EXECUTION_DISABLED`
- Even if `apply_enabled()` were flipped, the adapter still returns `ADAPTER_NOT_IMPLEMENTED` (no writes)
- No Apply HTTP execute route exists
- `aether-core` is untouched by Apply

**Gate decision: PASS WITH CONDITIONS**

Justification:

- **PASS** for the current boundary: Apply cannot accidentally or maliciously mutate PROTO-0 state.
- **CONDITIONS** must be resolved before enabling real PROTO-0 mutations or `apply_enabled=true`.
- The next authorised implementation phase may add a real `proto0_write` mutation body **still behind** `apply_enabled=false`, provided Mandatory Fixes below remain open until enablement checklist completion.

---

## Architecture Review

### Subsystems inspected

| Subsystem | Path | Responsibility |
|-----------|------|----------------|
| Canonical hash / mapping | `apply/canonical_json.rs`, `policy_mapping.rs`, `execution/hash.rs` | Deterministic binding |
| Attestation | `apply/attestation/` | Durable dry-run evidence |
| Approval | `apply/approval/` | Authorisation binding + SoD |
| Signature | `apply/signature/` | Apply-purpose intent authentication |
| Validation | `apply/validation/` | G1–G14 fail-closed gates |
| Re-simulation | `apply/resimulation/` | Pre-execution freshness evidence |
| Replay | `apply/replay/` | One-shot operation guard |
| Execution | `apply/execution/` | Orchestration to PROTO-0 boundary |
| Adapter | `protocol/proto0_write.rs` | Sole future mutation boundary |

### Verified invariants

| ID | Invariant | Status |
|----|-----------|--------|
| A1 | Module DAG is acyclic (lower layers do not import `execution`) | Verified |
| A2 | Responsibilities are separated (evidence vs authz vs crypto vs orchestration) | Verified |
| A3 | No hidden PROTO-0 write paths outside `proto0_write::execute` | Verified |
| A4 | Protocol read APIs (`proto0`) are observation-only | Verified |
| A5 | Execution ordering is deterministic inside `execute_apply` | Verified |
| A6 | Pre-reserve failures create no replay row and do not consume approvals | Verified |

### Architecture finding — ordering vs frozen docs

**Code order (Phase 8):**

```
validate → re-simulate → reserve (+ consume) → begin → proto0_write → finalise
```

**Frozen docs (`APPLY_PROTOCOL_SPECIFICATION`, sequence diagrams, blueprint):**

```
gates → reserve → re-simulate → execute → finalise
```

Current code is **safer for retries** (failed re-sim does not consume approval) but **diverges from normative docs**. This is a conformance condition, not a live mutation hole.

**Condition C1:** Reconcile ordering before enablement — either align code to reserve→resim (and accept post-reserve sim-failure consumes approval per failure model), or amend frozen docs + failure model to adopt resim→reserve with explicit rationale.

---

## Authority Review

| Claim | Verdict | Evidence |
|-------|---------|----------|
| PROTO-0 remains sole protocol authority | Hold | Adapter is only mutation boundary; currently non-mutating |
| Control Plane never grants protocol authority | Hold | Approvals/signatures are CP records only |
| Approvals never activate policies | Hold | Approval routes bind dry-run evidence; policy lifecycle separate |
| Signatures authenticate only | Hold | Phase 5 INV-S1; no capability grant in signer |
| Replay never authorises execution | Hold | Replay is one-shot guard, not permission |
| Execution never bypasses `proto0_write` | Hold | Sole call site in `execution/steps.rs` |
| `apply_enabled=false` prevents execution | Hold | Hard-coded false + adapter reject + pipeline hard-block |

### Authority escalation search

| Candidate path | Result |
|----------------|--------|
| HTTP `/apply-approvals` | Authz binding only; no PROTO-0 |
| Dry-run `/policies/:id/dry-run` | Read-only simulation + attestation |
| Direct `execute_apply` library call | Still blocked at adapter / `APPLY_EXECUTION_DISABLED` |
| `apply_enabled` env/config override | None — hard-coded `false` |
| Defaulting unknown intent to `CapabilityGrant` | **Condition C5** — fail-closed gap before enablement |

No live escalation to protocol mutation was found.

---

## Security Review

### Gate enforcement matrix

| Control | Enforced where | Gap |
|---------|----------------|-----|
| JWT / auth | Validation G1–G2 (request preconditions from middleware) | Execute HTTP not mounted yet |
| RBAC | G3 `require_operator`; approval admin-only | — |
| CSRF | G4 precondition; write guard on approval routes | Execute CSRF consume not wired (no execute route) |
| Origin | Approval/policy write guards | Same as CSRF for future execute |
| Replay | Reserve uniqueness + IMMEDIATE txn | Consume not in same txn (C2) |
| Approval expiry | G7 / lazy expire on get | Mid-pipeline re-check incomplete (C3) |
| Signature expiry | G12 | Not re-checked after validate (C3) |
| `execution_hash` | G9 + resimulation compare | — |
| Policy version/hash | G13 + resimulation | — |
| Re-simulation | Required before reserve in pipeline | Doc order divergence (C1) |
| Audit | Validation + execution + signature + approval events | Correlation gaps (C4) |
| Apply enabled | Adapter + pipeline hard-block | G5 intentionally passes while disabled |

### Missing enforcement (pre-enablement)

1. Early `guard_apply_enabled` at top of `execute_apply` (do not rely only on hard-coded boundary outcome).
2. `confirm: true` as an explicit validation gate (today embedded in signed payload only).
3. CSRF consume-on-execute (when execute route exists).
4. Unknown `operation_intent` must not default to `CapabilityGrant`.

---

## Replay Review

| Property | Status |
|----------|--------|
| Reserve before PROTO-0 call | Verified (pipeline) |
| One execution slot per `operation_id` | Verified (PK + IMMEDIATE) |
| Concurrent reserve / concurrent execute | Verified by tests |
| Crash after reserve persistence | Verified |
| Terminal immutability | Verified |
| Reserved TTL → Stuck | Verified (sweeper) |
| Admin reconcile finalise/abort | **Missing** (list helpers only) |
| Startup scan of non-terminal rows | **Missing** |

### Race notes

- `BEGIN IMMEDIATE` serialises replay INSERT — good.
- Approval consume is a **second** statement after reserve commit — crash window: reserved + approval still active (C2).
- Mitigated today by aborting reserved slot if consume fails; still not normative atomicity.

---

## TOCTOU Review

```
Validation → Re-simulation → Reserve → Begin → PROTO-0 boundary
```

| Drift class | Detected? | Residual |
|-------------|-----------|----------|
| Policy version/hash | G13 + resim | Low while resim mandatory |
| Protocol state change | Resim `simulate_policy_apply` | Low |
| `execution_hash` | G9 + resim | Low |
| Approval expiry/cancel between G7 and consume | Partial (consume path) | Medium — C3 |
| Signature TTL between G12 and PROTO-0 | Not re-checked | Medium — C3 |
| Attestation expiry after G8 | Not re-checked at resim (`get` not `get_valid`) | Medium — C3 |
| Dry-run stale vs current protocol | Resim | Low |

**Recommendation:** Immediately before PROTO-0 (and ideally inside the reserve/consume transaction boundary), re-validate:

- approval still `active` and unexpired
- signature purpose/TTL/crypto still valid
- attestation still `executable` and unexpired
- `execution_hash` still matches current policy binding

---

## Threat Review

| Threat | Likelihood | Impact | Mitigation | Residual |
|--------|------------|--------|------------|----------|
| Privilege escalation | L | H | RBAC + SoD + viewer reject | Route wiring must preserve gates |
| Replay / double mutate | M | H | Unique reserve + IMMEDIATE | Reconcile for stuck ops (C6) |
| Confused deputy | L | H | Sole `proto0_write` path | Guard against future bypasses |
| TOCTOU | M | H | Hash gates + resim | Mid-pipeline TTL (C3) |
| Signature replay / wrong purpose | M | H | Purpose=`apply`, unique op id | Mid-pipeline TTL |
| Approval replay | M | H | Single-use consume | Atomic reserve+consume (C2) |
| Stale dry-run | M | H | Attestation TTL + resim | C3 |
| Stale policy | M | H | G13 + resim | Low |
| Audit tampering | M | H | Append-only audit_log | No hash-chain; reconstruction tests missing (C4/C7) |
| Race conditions | M | M | IMMEDIATE + status SQL | Concurrent validation create gaps |
| Partial failures | M | H | Fail-closed finalise | Real PROTO-0 timeout untested |
| Crash after reserve | M | H | Persist + stuck sweeper | No admin reconcile (C6) |
| DoS | M | M | Login rate limit only | No Apply-specific limits |
| Signer compromise | M | H | Short TTL + identity bind | No KMS/HSM (accepted residual) |

---

## State Machine Review

### Approval

`Active → {Expired, Consumed, Cancelled}` — terminal immutable; concurrent consume one-wins. **Hold.**

### Replay

`Reserved → Executing → {Executed, Rejected, Aborted}`; `Reserved → {Stuck, Aborted}`. Invalid transitions rejected. **Hold.**

### Execution orchestration

`Created → Validated → ResimulationApproved → ReplayReserved → Executing → {Executed, Rejected, Aborted, Stuck}`. Invalid transitions rejected in unit tests. **Hold.**

### Ambiguity

None found in coded transition matrices. Operational ambiguity remains for **stuck after crash** without admin reconcile (C6).

---

## Testing Review

### Present (strong)

- Concurrent replay reserve; concurrent execution attempts
- Crash after reserve persistence; stuck sweeper
- Validation/resim failure does not reserve
- Hash/policy/signature/approval failure paths
- Terminal replay immutability
- Fingerprint unchanged after blocked execution

### Missing / partial (before enablement)

| Test | Status |
|------|--------|
| Concurrent approval **create** | Missing (consume/cancel covered) |
| Concurrent validation | Missing |
| Approval expiry during execution (post-G7) | Missing |
| Signature expiry during execution (post-G12) | Missing |
| Audit reconstruction by `request_id` | Missing |
| Crash after **rejected** persistence | Missing |
| Replay reconciliation (admin finalise/abort) | Missing |
| Replay after restart **reconcile policy** | Partial (persistence only) |
| `confirm: true` gate | Missing |
| CSRF consume on execute | N/A until route exists |

---

## Known Risks

### Accepted while Apply disabled

1. Hard-disabled mutation stack (intentional).
2. G5 passes while disabled to allow auth-chain testing.
3. Signer without KMS/HSM (enterprise residual; TTLs mitigate).
4. Login-only rate limiting.
5. Audit not yet hash-chained / externally shipped.
6. Doc/code ordering divergence (tracked as C1; does not enable mutation).

### Not accepted for `apply_enabled=true`

All Mandatory Fixes below.

---

## Mandatory Fixes

Before **real PROTO-0 mutation enablement** or `apply_enabled=true`:

| ID | Fix | Priority |
|----|-----|----------|
| **C1** | Resolve reserve↔resim ordering vs frozen docs | High |
| **C2** | Atomic `reserve` + approval `consume` in one `BEGIN IMMEDIATE` | High |
| **C3** | Re-check approval, signature, attestation TTL/status immediately before PROTO-0 | High — **partial (Phase 10):** approval + signature re-checked in `step_proto0_boundary`; attestation TTL still deferred |
| **C4** | Propagate `audit_correlation_id` (and full id set) on all execution/validation audits | Medium |
| **C5** | Reject unknown `operation_intent` (no default to `CapabilityGrant`) | High — **remediated (Phase 10):** `parse_intent` → `Unknown`; adapter rejects Unknown/PolicyApply with `APPLY_UNSUPPORTED_OPERATION` |
| **C6** | Admin reconcile for `stuck`/`executing` + startup non-terminal scan | High |
| **C7** | Audit reconstruction test by `request_id`; wire Apply attempts into mutation audit as required by blueprint | Medium |
| **C8** | Early `guard_apply_enabled` on `execute_apply`; stop treating disabled as hard-coded success path once adapter can mutate | High — **partial (Phase 10):** adapter refuses mutation unless `apply_enabled()==true`; production flag remains `false`; early pipeline short-circuit still deferred until enablement design |
| **C9** | Explicit `confirm: true` + CSRF consume on future execute route | High |
| **C10** | Add missing tests listed in Testing Review | High |

---

## Recommendation

1. **Do not enable Apply.** Keep `apply_enabled() == false`.
2. **Do not expose Apply execute HTTP routes** until C8–C10 are designed and tested.
3. **Next implementation phase may** implement real `proto0_write` mutation bodies **still rejected unless** `apply_enabled()` is true — i.e. code present, path unreachable in production.
4. Treat Mandatory Fixes as the enablement checklist entrance criteria.
5. Re-run this gate (or a delta gate) after mutation body lands and again before flipping the flag.

---

## Gate Decision

# **PASS WITH CONDITIONS**

### Why not PASS

Open enablement blockers (C1–C10), especially atomic reserve/consume, mid-pipeline TTL re-checks, reconcile/recovery, and doc/code ordering reconciliation.

### Why not FAIL

No path currently mutates protocol state. Authority boundary is intact. Replay, validation, signature purpose binding, and re-simulation provide a coherent fail-closed skeleton. Verification suite is green with Apply disabled and `aether-core` unchanged.

### Authorised next step

Implement the real `proto0_write` mutation boundary **behind** `apply_enabled=false`, without Apply execute routes and without enabling production Apply.

---

## Phase 10 delta (mutation adapter)

**Status:** Real PROTO-0 mutation bodies land in `control_plane/src/protocol/proto0_write.rs`, reachable only when `apply_enabled()==true` (test override only). Production remains disabled. No Apply execute HTTP routes.

**Gate conditions addressed in this layer:**

| ID | Phase 10 status |
|----|-----------------|
| C3 | Approval + signature expiry re-checked immediately before adapter; attestation TTL re-check still open |
| C5 | Unknown / PolicyApply intents rejected; never default to Grant |
| C8 | Adapter hard-stops on `apply_enabled()==false` before any mutation |

**Still open before enablement (pre-Phase 11):** C1, C2, C3 (attestation), C4, C6, C7, C8 (early pipeline guard), C9, C10.

**Phase 11:** See `APPLY_ENABLEMENT_READINESS_REVIEW.md` — READY FOR FINAL GATE. C1–C8/C10 closed in code; C9 HTTP route residual; `apply_enabled()` remains false.

---

## Verification (Phase 9)

Executed:

```bash
cargo fmt --check
cargo test -p aether-control-plane
cargo test -p aether-core
cargo clippy -p aether-control-plane --all-targets -- -D warnings
```

**Result:** All passed. `apply_enabled()` remains `false`. No `aether-core` changes.

---

## Appendix — Verified security invariants (summary)

| ID | Statement |
|----|-----------|
| INV-S | Signer authenticates; does not grant capability |
| INV-V | Validation does not mutate protocol / consume / reserve |
| INV-R | Re-simulation does not mutate protocol / consume / reserve |
| INV-E | Execution reaches PROTO-0 only via `proto0_write::execute` |
| INV-D | `APPLY_EXECUTION_DISABLED` / `APPLY_DISABLED` leave protocol unchanged |
| INV-H | No Apply execute HTTP surface |
| INV-F | `apply_enabled()` hard-false until separate authorisation |
