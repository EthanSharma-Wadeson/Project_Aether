# Aether Protocol v1.0 — Release Certification

| Field | Value |
|-------|--------|
| **Document** | `V1_RELEASE_CERTIFICATION.md` |
| **Phase** | 13 — Production Readiness & Release Certification |
| **Nature** | Engineering review (no implementation) |
| **Date** | 2026-08-01 |
| **Protocol label under review** | Aether Protocol v1.0 (certification label) |
| **Crate versions** | `aether-core` 0.1.0 · `aether-control-plane` 0.1.0 · `aether-enterprise-demo` 0.1.0 |
| **Apply status** | `apply_enabled() == false` (hard-coded; production env cannot enable) |
| **Reviewer role** | Final engineering certification |

---

## Executive Summary

Aether ships a layered protocol reference stack (PROTO-0…4 + NET-0) plus an Enterprise Control Plane with a complete Apply **governance** pipeline (dry-run → attestation → approval → signature → validation → re-simulation → replay → HTTP surface → PROTO-0 adapter). Live protocol mutation via Apply is **intentionally impossible** in production.

**Release recommendation: READY WITH CONDITIONS**

| Product surface | Recommendation |
|-----------------|----------------|
| Protocol + Control Plane with **Apply disabled** (observatory, policy lifecycle, dry-run, approvals, prepare, disabled execute) | **READY WITH CONDITIONS** for constrained enterprise-local deployment |
| **Apply-enabled** live mutation | **NOT READY** — requires final enablement gate |
| Unqualified claim of “complete production agent economy / live settlement / live network” | **NOT READY** — simulators and OPEN threat findings remain |

Calling this “v1.0” is a **certification milestone**, not a claim already asserted by crate versions (still `0.1.0` / “v0” in core). Conditions below must be accepted before production use.

---

## Architecture Certification

### Layering (VALIDATED)

| Layer | Location | Boundary |
|-------|----------|----------|
| PROTO-0 Identity & Authority | `core` identity / capability / permission / verifier | Authoritative local state |
| PROTO-1 Channels | `core/channel` | Simulator; no live transport |
| PROTO-2 Escrow | `core/escrow` | Simulator; no real money |
| PROTO-NET-0 | `core/network` | Local simulation; no TCP/UDP |
| PROTO-4 Settlement | `core/settlement` | Mock `enterprise.ledger.v0` |
| PROTO-3 Reputation | `core/reputation` | Read-only indexer vs authority layers |
| Control Plane | `control_plane` | Depends on `aether-core`; governance & observation |
| Apply engine | `control_plane/src/apply/*` | Gates, replay, HTTP; mutations gated |

### Mutation & authority boundaries (VALIDATED)

- **Single CP mutation entry:** `protocol/proto0_write::{execute, execute_shared}`
- Supported ops: CapabilityGrant, CapabilityRevoke, FreezeIdentity; unknown intents rejected (`APPLY_UNSUPPORTED_OPERATION`)
- Pipeline uses `execute_shared` while disabled → no `&mut` mutation path in production HTTP
- Signer gateway does **not** call PROTO-0
- Reputation must not mutate authority layers (PROTO-3 design)

### Dependency graph (VALIDATED with notes)

```
aether-control-plane → aether-core
                     → aether-enterprise-demo → aether-core
```

- No circular crate dependency found
- Protocol state in CP is **process-memory** (demo bootstrap), not a durable protocol database — **ACCEPTED** for Apply-off observatory; **OPEN** for multi-instance HA

### Determinism, replay, recovery (VALIDATED for Apply-off)

| Property | Status | Evidence |
|----------|--------|----------|
| Deterministic mapping / hashing | VALIDATED | Canonical JSON, execution_hash, adapter determinism tests |
| One-shot execution (`operation_id`) | VALIDATED | Replay store + atomic reserve/consume |
| Crash survival of replay rows | VALIDATED | Persistence / restart tests |
| Stuck detection + admin abort | VALIDATED | Startup scan, sweeper, reconcile HTTP |
| Blind PROTO-0 retry | VALIDATED absent | Reconcile rules forbid auto-retry |

### Architecture findings

1. **VALIDATED:** Clear proto layering and sole Apply mutation boundary.
2. **ACCEPTED:** NET-0 / escrow / settlement remain local simulators by design at v1.0.
3. **OPEN:** Durable multi-node protocol state and live networking are out of scope for this certification.
4. **OPEN (doc):** Frozen Apply specs still describe reserve→resim; code is resim→reserve (safer). Must be waived or docs amended before Apply enablement.

---

## Security Certification

Classification key: **VALIDATED** (controls evidenced) · **OPEN** (residual risk) · **ACCEPTED** (known limit, explicitly accepted for Apply-off v1.0)

### Domain matrix

| Domain | Classification | Notes |
|--------|----------------|-------|
| Identity (PROTO-0) | VALIDATED / OPEN | Local identity/capability strong; not a global PKI |
| Capabilities | VALIDATED | Grant/revoke/freeze gated; escalation checks in core |
| Escrow | ACCEPTED | Simulator; not legal escrow finality |
| Settlement | ACCEPTED / OPEN | Mock ledger; not production rails |
| Reputation | ACCEPTED / OPEN | Indexer limitations (Sybil, honesty) documented in PROTO-3 review |
| Governance (policies) | VALIDATED | Draft→submit→approve SoD, CSRF/origin on writes |
| Execution (dry-run) | VALIDATED | No mutation; attestation binding |
| Apply pipeline | VALIDATED (disabled) | Full gate chain; mutations blocked |
| HTTP API (Phase 12) | VALIDATED (disabled) | JWT, RBAC, Origin, CSRF, confirm; returns disabled outcome |
| Replay | VALIDATED | Atomic prepare; duplicates fail closed |
| Recovery | VALIDATED | Scan / stuck / admin abort; no assumed success |
| Audit | VALIDATED / OPEN | Reconstructable request trails; hash-chained / external ship OPEN |
| Keys / signer | ACCEPTED / OPEN | `CP_SIGNER_SEED_HEX` required in prod; HSM OPEN |

### Threat classification

| Threat | Classification | Mitigation / residual |
|--------|----------------|------------------------|
| Replay | VALIDATED | Replay store one-shot + approval consume |
| TOCTOU (TTL mid-flight) | VALIDATED | Pre-PROTO-0 recheck (approval, signature, attestation, policy) |
| Confused deputy | VALIDATED / OPEN | Purpose-bound signatures, SoD; residual CP indexing trust |
| Privilege escalation | VALIDATED | RBAC Admin/Operator/Viewer; admin-only execute/reconcile |
| Signer misuse | VALIDATED / OPEN | Purpose=`apply`; seed custody OPEN (no HSM) |
| Audit tampering | OPEN | Append-only SQLite; no hash-chain / external attest |
| Stale state | VALIDATED | Attestation/approval/signature TTLs + boundary recheck |
| Race conditions | VALIDATED | BEGIN IMMEDIATE prepare; concurrent tests |
| Crash recovery | VALIDATED / ACCEPTED | Replay durable; protocol memory state not durable |
| Accidental Apply enablement | VALIDATED | Hard-false + production config reject `CP_APPLY_ENABLED` |

### Apply enablement gate (C1–C10) — summary

| ID | Code status | Remaining before `apply_enabled=true` |
|----|-------------|----------------------------------------|
| C1 | Closed (resim→reserve) | Amend frozen docs |
| C2 | Closed (atomic txn) | — |
| C3 | Closed (freshness) | — |
| C4 | Closed (consistency + audit fields) | Optional `mutation_audit` wiring |
| C5 | Closed | — |
| C6 | Closed | Operator runbook |
| C7 | Closed | — |
| C8 | Closed | Dual-control flip of runtime switch |
| C9 | Closed at HTTP (confirm + CSRF) | Re-verify under enablement gate |
| C10 | Closed for disabled path | Live-mutation test suite required |

### Security findings

1. **VALIDATED:** With Apply disabled, live mutation via CP Apply path is not achievable under reviewed controls.
2. **OPEN:** Transport confidentiality, partition, and host MITM for any future live NET layer.
3. **OPEN:** Sybil / indexer honesty (PROTO-3), settlement finality (PROTO-4).
4. **OPEN:** HSM/KMS, dual approval for high-risk ops, hash-chained audit.
5. **ACCEPTED for v1.0 Apply-off:** Simulator economics and non-durable protocol memory.

---

## Operations Review

| Area | Finding | Class |
|------|---------|--------|
| Startup | Migrate → seed → `startup_scan` (non-fatal) → serve | VALIDATED |
| Shutdown | Process exit; no coordinated protocol flush (memory state) | ACCEPTED |
| Restart | CP DB durable; protocol re-bootstrap from demo | ACCEPTED / OPEN |
| Configuration | `production_config_check` fail-closed | VALIDATED |
| Apply config | Cannot start production with Apply requested | VALIDATED |
| Database | SQLite WAL for CP metadata | ACCEPTED |
| Migrations | Inline `Db::migrate()` | ACCEPTED / medium-term debt |
| Key management | Env seed hex; prod forbids ephemeral | ACCEPTED / OPEN |
| Logging | tracing + request id | VALIDATED |
| Monitoring / alerting | Minimal; no formal SLO/alert pack | OPEN |
| Backups | Operator responsibility for SQLite file | OPEN (runbook needed) |
| Deployment | Single-process CP assumed | ACCEPTED |

### Operational findings

1. Production posture gates are real and fail closed.
2. Missing: formal backup/restore runbook, metrics/alerting pack, multi-instance story.
3. Protocol authority state is not in SQLite — restart loses in-memory protocol mutations from tests/adapters; production Apply-off path does not mutate, so **ACCEPTED**.

---

## Performance Review

| Concern | Assessment |
|---------|------------|
| SQLite | Adequate for single-tenant CP metadata at moderate write rates |
| Audit growth | Unbounded append; needs retention policy for long-lived deploys |
| Replay / attestations / approvals | Indexed by id; growth linear with Apply attempts (even disabled execute creates terminal replay rows) |
| Query complexity | Simple CRUD; no heavy analytics layer |
| Scaling limit | Single writer SQLite + single process memory protocol |
| PostgreSQL | Reasonable v1.1+ migration for multi-tenant / HA CP metadata |

**Finding:** Performance is acceptable for constrained enterprise-local CP. Not certified for high-QPS multi-region Apply.

---

## Documentation Review

### Strengths

- Rich Apply / CP / proto security and blueprint corpus under `Aether_docs/` and `Project_Phases/`
- Phase 11 readiness + Phase 12 HTTP API docs exist
- Security reviews per proto phase recorded

### Gaps / contradictions

| Issue | Severity |
|-------|----------|
| Frozen Apply order (reserve→resim) vs code (resim→reserve) | High (must resolve before enablement) |
| Blueprint error envelope `{code, retryable, …}` vs CP `{error}` | Medium |
| Disabled Apply: blueprint 403 vs HTTP 200 + `APPLY_EXECUTION_DISABLED` | Medium |
| Stale phase READMEs (“M1 read-only”, “no write until approved”) | Medium (operator confusion) |
| CP README still Milestone 1 framing | Medium |
| SECURITY_MODEL.md still Draft | Medium |
| No single “v1.0 scope” doc prior to this certification | High (this doc closes that gap) |
| Outdated “no Apply HTTP routes” claims in older gate text | Medium (superseded by Phase 12) |

### Missing docs (recommended)

- Production runbook (backup, reconcile, incident)
- Compatibility / upgrade policy (below section is authoritative until split)
- Explicit simulator vs production-rail boundary for customers

---

## Testing Review

### Coverage summary

| Layer | Evidence |
|-------|----------|
| Core unit/integration | Identity, capability, channel, escrow, network, settlement, reputation, adversarial suites |
| CP unit | Apply validation, resim, replay, signature, execution, adapter, enablement |
| CP integration | M1, write foundation, policies, signer, dry-run, adapter, Phase 12 HTTP |
| Security / recovery / replay | Present in Apply and core adversarial tests |

### Gaps

- Live Apply mutation E2E (correctly absent while disabled)
- Long-running soak (audit growth, sweeper under load)
- Multi-process crash of CP mid-execute under load
- Frontend E2E not treated as protocol evidence
- No chaos / partition tests for future live NET

### Recommended long-running tests (v1.1+)

1. 24h CP soak with continuous dry-run → approve → prepare → disabled execute  
2. Replay sweeper under synthetic stuck load  
3. Audit table growth / vacuum policy validation  
4. Post-enablement: mutation property tests with dual-control flag in staging only  

---

## Version Audit

### Versions

| Component | Version |
|-----------|---------|
| Certification label | **Aether Protocol v1.0** |
| `aether-core` | 0.1.0 (library still described as v0 reference) |
| `aether-control-plane` | 0.1.0 |
| Apply adapter schema | `aether.cp.proto_adapter.v1` |
| AgentId preimage | `aether:v0:agent-id:` (wire compatibility) |

### Compatibility matrix

| Consumer | Compatible with |
|----------|-----------------|
| CP 0.1.0 | core 0.1.0 (path dependency; lockstep) |
| Apply HTTP | Disabled outcome only |
| Demo harness | Bootstraps in-memory protocol for CP |

### Policies (v1.0)

| Policy | Statement |
|--------|-----------|
| **Migration** | CP schema via `Db::migrate()` on startup; no downgrade guarantee |
| **Upgrade** | core + CP upgraded together; no wire protocol stability promise beyond local simulators |
| **Deprecation** | Simulator APIs may be replaced by live rails in later majors without wire compatibility |
| **Apply enablement** | Separate gate; not part of v1.0 default deploy |

---

## Technical Debt

### Immediate (required before claiming unqualified production / before Apply enablement)

1. Reconcile frozen Apply ordering docs with code (or formal waiver)  
2. Refresh stale phase/README/CP Milestone framing  
3. Production backup + reconcile runbook  
4. Final Apply enablement gate (if mutations desired)  
5. Align error envelope documentation with implementation  

### Medium-term (v1.1)

1. Hash-chained / externally shipped audit  
2. HSM/KMS signer custody  
3. Dedicated migrations tooling; optional PostgreSQL  
4. Durable protocol state story if Apply goes live  
5. Metrics / alerting pack  
6. Structured Apply error JSON (`code`, `retryable`, ids)  

### Future

1. Live NET transport + confidentiality  
2. Real settlement rails  
3. Multi-instance CP  
4. Dual-approval workflows for high-risk intents  
5. Formal language / compliance artefacts beyond engineering cert  

### Separation

| Required before production (Apply-off) | May wait until v1.1 |
|----------------------------------------|---------------------|
| Accept simulator + memory-protocol limits | PostgreSQL |
| Production config + Apply hard-off verified | HSM |
| Operator backup/reconcile basics | Live NET / settlement |
| Doc hygiene for operators (stale READMEs) | Hash-chained audit |

---

## Known Risks

1. Operators confuse “v1.0” with live multi-agent economy / payments.  
2. Disabled execute still writes terminal replay rows — disk growth.  
3. Doc contradictions cause incorrect enablement assumptions.  
4. Seed-hex signer compromise = full governance signing authority.  
5. Single SQLite file loss = loss of governance history (not protocol math).  
6. Enabling Apply without final gate would be a process failure, not a missing if-check (checks exist).

---

## Production Checklist

### Deploy Allow-list (Apply-off)

- [ ] `CP_ENV=production` (or equivalent) with `production_config_check` passing  
- [ ] Strong JWT secret, explicit origins, secure cookies, non-default passwords  
- [ ] `CP_SIGNER_SEED_HEX` set; ephemeral off; demo bootstrap off  
- [ ] Confirm `CP_APPLY_ENABLED` unset/false  
- [ ] Confirm runtime `apply_enabled()` false  
- [ ] SQLite path on durable volume + backup job  
- [ ] Startup logs show reconcile scan  
- [ ] Smoke: login, policy lifecycle, dry-run, approval, prepare, execute → `APPLY_EXECUTION_DISABLED`, fingerprint unchanged  

### Do not check for v1.0 default

- [ ] `apply_enabled=true`  
- [ ] Live payment / settlement rails  
- [ ] Multi-region HA  

---

## Release Recommendation

# READY WITH CONDITIONS

### Justification

**Why not NOT READY:**  
The protocol stack is coherent; CP governance controls are substantial; Apply mutation is hard-disabled with production config reinforcement; replay/recovery/HTTP surfaces are implemented and tested for the disabled path; architecture boundaries are clear.

**Why not READY FOR PRODUCTION (unqualified):**  
Crate/docs still say v0/0.1.0; NET/escrow/settlement are simulators; OPEN threats remain (HSM, audit integrity, Sybil, transport); protocol state is non-durable; documentation contradictions exist; Apply enablement is explicitly incomplete.

**Conditions of release under this certification:**

1. Deploy **only** with Apply disabled (default).  
2. Scope customer expectation to: local/sim protocol + enterprise governance CP.  
3. Accept OPEN items listed above as residual risk.  
4. Complete Immediate debt items #1–#3 before broad production rollout; #4–#5 before any Apply enablement.  
5. Do not market as live settlement or live networked agent economy.

### Apply-enabled production

**NOT READY** until a dedicated enablement gate closes remaining residuals and flips `apply_enabled()` under dual control.

---

## Remaining Work for v1.1

1. Doc reconciliation (ordering, envelopes, READMEs)  
2. Structured Apply error API  
3. Audit integrity (hash-chain / export)  
4. HSM-backed signer  
5. CP metadata PostgreSQL option  
6. Metrics/alerting + soak tests  
7. Durable protocol state design (if Apply live)  
8. Staging-only Apply enablement drill  
9. Live NET / settlement roadmaps under new security reviews  

---

## Certification Sign-off Block

| Item | Value |
|------|--------|
| **Recommendation** | READY WITH CONDITIONS |
| **Version label** | Aether Protocol v1.0 |
| **Apply** | Disabled |
| **Date** | 2026-08-01 |
| **Authority** | Phase 13 engineering certification (this document) |

*No code was modified in this phase. `apply_enabled` was not enabled. `aether-core` behaviour was not changed.*
