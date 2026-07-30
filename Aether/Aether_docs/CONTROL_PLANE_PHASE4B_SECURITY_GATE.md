# Control Plane Phase 4B Security Gate

**Document type:** Architecture & Security Review (pre-mutation gate)  
**Date:** 2026-07-29  
**Scope:** Milestone 2 through Phase 4A (dry-run). **No Apply / PROTO-0 mutation authorised.**  
**Related:** [CONTROL_PLANE_AUTHORITY_MODEL.md](CONTROL_PLANE_AUTHORITY_MODEL.md), [CONTROL_PLANE_WRITE_SECURITY.md](CONTROL_PLANE_WRITE_SECURITY.md), [CONTROL_PLANE_POLICY_THREAT_MODEL.md](CONTROL_PLANE_POLICY_THREAT_MODEL.md), [../Project_Phases/phase_3/CONTROL_PLANE_MILESTONE_2_PLAN.md](../Project_Phases/phase_3/CONTROL_PLANE_MILESTONE_2_PLAN.md), [../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md](../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md)

---

## Executive Summary

The Control Plane governance pipeline is complete up to **dry-run**. Authority isolation holds: the Control Plane does not mutate protocol state, does not call PROTO-0 grant/revoke, and treats approved policies as inactive CP records. The signer authenticates operations; it is not the authority.

This review verified architecture invariants, mutation-path security, policy state machine, audit correlation, protocol adapter boundaries, configuration defaults, threats, and tests.

**Gate decision: PASS WITH CONDITIONS**

Phase 4B (live Apply) must not begin until the mandatory actions in § Required Fixes are completed or explicitly accepted with compensating controls. Several High/Critical issues were either remediated during this review or remain as production/4B blockers.

### Remediation status (Apply Safety Design phase)

| Item | Status |
|------|--------|
| M1 Production config gate | **Remediated** — `production_config_check()`; `CP_ENV=production` / `CP_PRODUCTION=1` fail closed |
| M2 Apply design (binding, no dry-run sig reuse, confirm) | **Documented / frozen** — [CONTROL_PLANE_APPLY_SECURITY_MODEL.md](CONTROL_PLANE_APPLY_SECURITY_MODEL.md) |
| M3 Signed-operation replay protection | **Documented / frozen** — [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md) (store not implemented) |
| M4 Content-addressed execution binding | **Remediated (dry-run)** — `execution_hash` on dry-run report; count fingerprint retained only as non-mutation check |
| M5 Concurrency + Apply security tests | **Test plan created** — [CONTROL_PLANE_APPLY_TEST_PLAN.md](CONTROL_PLANE_APPLY_TEST_PLAN.md); Apply-path tests deferred until Apply coding |
| M6 Cookie `Secure` | **Remediated** — `CP_SECURE_COOKIES`; required true in production gate |
| M7 Dry-run in `mutation_audit` | **Recommended** — see Apply security model § Audit; not yet implemented |

Hardening applied in the original review (no Apply, no aether-core changes):

| Fix | Status |
|-----|--------|
| Concurrent update TOCTOU (`UPDATE` without status guard) | **Remediated** — `WHERE status IN ('draft','rejected')` |
| Reject SoD asymmetry (submitter could reject) | **Remediated** — reject mirrors approve SoD |
| `CorsLayer::permissive()` | **Remediated** — allowlist / localhost-only CORS |
| Dry-run audit missing `operation_id` / version | **Remediated** — included in `DRY_RUN_COMPLETED` |

---

## Architecture Review

### Pipeline (as implemented)

```text
Operator
  → Authentication (JWT)
  → RBAC
  → Origin + CSRF (mutating policy routes)
  → Policy lifecycle (CP DB only)
  → Signer (EnterpriseSigner behind trait)
  → Dry-run simulation (read-only)
  ✗  Apply / PROTO-0  ← NOT IMPLEMENTED
```

### Components

| Layer | Role | Protocol effect |
|-------|------|-----------------|
| Observatory (M1) | Read adapters | None |
| Policy templates (P2) | CP DB lifecycle | None |
| Signer (P3) | Attest `GovernanceOperation` | None |
| Dry-run (P4A) | Pre-flight validation + sign | None |
| Apply (P4B) | Future only | PROTO-0 decides |

### Findings

- No Apply route, no `grant_capability` / revoke callers in Control Plane source.
- `ExecutionMode::Live` exists as a reserved enum variant only; executor always uses `DryRun`.
- Protocol adapters under `protocol/` are observation-only (`&ProtocolState`).
- `AppState.signing` is used by dry-run (and startup wiring), not by auth or policy CRUD.

---

## Authority Review

### Verified invariants

| ID | Invariant | Evidence | Status |
|----|-----------|----------|--------|
| A1 | Control Plane is not protocol authority | No protocol store writes; approve returns `protocol_active: false` | **Verified** |
| A2 | PROTO-0 remains final authority for capabilities | No grant/revoke integration; dry-run only simulates | **Verified** |
| A3 | Signer authenticates; does not authorise or mutate | `SigningGateway` signs/verifies; no PROTO-0 calls | **Verified** |
| A4 | Business logic cannot access raw signing keys | `SigningKey` confined to `enterprise_signer.rs` private `EnterpriseKeyMaterial` | **Verified** |
| A5 | Policy approval never activates a policy | Status → `approved` in CP DB only; explicit API notes | **Verified** |
| A6 | Dry-run never mutates protocol state | Fingerprint before/after; tests assert index unchanged | **Verified** |
| A7 | Only future Apply may reach write adapters | No write adapter exists; Apply not shipped | **Verified** |
| A8 | Template ≠ apply | Phase 2 + 4A docs and API contract | **Verified** |

### Residual authority risks (for Phase 4B design)

- Dry-run protocol fingerprint is **count-based**, not content-hash — insufficient as a binding for Apply.
- Signed dry-run payloads return `signature_hex` to clients; Apply must not reuse dry-run signatures without nonce / purpose binding.

---

## Security Review

### Mutation path matrix (current)

| Route | JWT | RBAC | Origin | CSRF | Audit |
|-------|-----|------|--------|------|-------|
| `POST /auth/login\|refresh\|logout` | Cookie/body | N/A | No* | Issues CSRF | Yes |
| `GET /api/csrf-token` | Yes | Any | No | N/A (issue) | No |
| `POST/PUT /api/policies*` (CRUD lifecycle) | Yes | Yes | Yes | Yes | Yes (`audit_log` + `mutation_audit`) |
| `POST .../dry-run` | Yes | operator+ | Yes | Yes (not consumed) | Yes (`audit_log` + `signer_audit`) |

\* Auth cookies use `SameSite=Strict`; Origin allowlist applies to `/api` mutations via `guard_mutation`.

### Enforcement assumptions

1. JWT middleware on all `/api/*`.
2. Mutating policy routes call `guard_mutation` (Origin + CSRF) then service RBAC.
3. Approve/reject/archive consume CSRF; create/update/submit/dry-run do not (replay until TTL).
4. Request ID middleware is global; dry-run/policy handlers propagate into audits.
5. Signer required on dry-run path; not required for CP-local policy CRUD (by design).

### Missing / deferred enforcement (before Apply)

| Gap | Severity | Notes |
|-----|----------|-------|
| Apply confirmation (`confirm: true`) | High | Required by write-security model; N/A until Apply |
| Single-use CSRF on Apply | High | Must consume on Apply |
| Signed-operation replay store | High | Required for Apply |
| Strong protocol binding (hash, not counts) | High | Required for Apply |
| `mutation_audit` row for dry-run | Medium | Correlation gap; recommend before 4B |
| `Secure` cookie flag | Medium | Required for HTTPS production |
| Require `CP_ALLOWED_ORIGINS` in production | Medium | Empty allowlist is localhost-oriented |

---

## State Machine Review

### Allowed transitions

```text
Draft ──submit──► PendingReview ──approve──► Approved ──archive──► Archived
                       │
                       └──reject──► Rejected ──archive──► Archived
                                      │
Draft ◄──update (new version)─────────┘
```

### Rejected (enforced)

| Transition / action | Enforcement |
|---------------------|-------------|
| Approved → Draft | Update blocked (service + SQL status guard) |
| Archived → edit/submit | Update blocked |
| PendingReview → edit | Update blocked |
| Viewer create/approve | RBAC |
| Operator approve | `require_admin` |
| Self-approve / self-reject (creator or submitter) | SoD checks |

### Hardening in this review

- SQL update now requires `status IN ('draft','rejected')` (closes concurrent submit→update downgrade).
- Reject SoD now also blocks `submitted_by == actor`.

---

## Audit Review

### Stores

| Table | Use |
|-------|-----|
| `audit_log` | Auth, POLICY_*, DRY_RUN_*, SIGN_REQUEST_* mirrors |
| `mutation_audit` | Policy lifecycle writes (`request_id`, role, payload_hash, `protocol_result=not_applicable`) |
| `signer_audit` | Sign attempts (`request_id`, `operation_id`, `policy_id`, outcome) |

### Reconstructability

| Link | Lifecycle | Dry-run |
|------|-----------|---------|
| `request_id` | Yes | Yes |
| `policy_id` / version | Yes (metadata / target) | Version + `operation_id` on COMPLETED (hardened) |
| `operation_id` | N/A (no sign) | `signer_audit` + dry-run audit metadata |
| Signer identity | Optional until sign | Yes |

### Recommendations

1. Record dry-run in `mutation_audit` with `protocol_result=not_applicable` for one timeline.
2. Persist a durable `dry_run_id` / operation binding for Apply to reference.
3. Pre-production: tamper-evident hash chain (SEC-CP-03).

---

## Protocol Boundary Review

| Adapter | Mode | Mutation paths |
|---------|------|----------------|
| `proto0` | Read | None |
| `proto2` / `proto3` / `proto4` | Read | None |
| `execution/simulate.rs` | Read simulation | None |
| Future Apply adapters | Not present | **Only intended write boundary** |

`ProtocolState` fields are public but held in `Arc` without interior mutability after bootstrap — dry-run and routes take shared references only.

---

## Configuration Review

| Variable | Default | Production safety |
|----------|---------|-------------------|
| `CP_JWT_SECRET` | **Required** | Good |
| `CP_ALLOWED_ORIGINS` | Empty → localhost Origin rules + localhost CORS | Must set in production |
| `CP_SIGNER_MODE` / `IDENTITY` | `enterprise` / `enterprise-default` | OK |
| `CP_SIGNER_SEED_HEX` | Optional; else **identity-derived deterministic key** | **Unsafe** for production without explicit seed/KMS |
| `CP_ADMIN_PASSWORD` etc. | Defaults `admin`/`viewer`/`operator` | **Unsafe** — change or refuse boot in prod |
| CORS | Allowlist / localhost (hardened) | Good after fix |
| Cookie `Secure` | Not set | Required behind HTTPS |
| `CP_BOOTSTRAP_DEMO` | Parsed; demo world always bootstrapped today | Clarify / gate for prod |

---

## Threat Review

| Threat | Likelihood | Impact | Existing mitigation | Recommended |
|--------|------------|--------|---------------------|-------------|
| Privilege escalation (viewer→write) | Low | High | RBAC on services | Keep; add role regression tests |
| CSRF bypass | Low–Med | High | Synchronizer + Origin; SameSite refresh | Consume CSRF on Apply; prod allowlist |
| Origin spoofing | Low | Med | Exact allowlist when set; missing Origin allowed for non-browser | Require Origin on browser mutating routes in prod |
| Replay (CSRF token) | Med | Med | TTL 2h; consume on approve | Consume on Apply; optional consume on create |
| Replay (signed ops) | High (post-Apply) | High | Dry-run purpose string only | Nonce / jti store; purpose=`apply` |
| Confused deputy | Med | High | SoD; CP not protocol authority | Apply must re-validate policy hash + version |
| Audit tampering | Med | High | Append-only SQLite | Hash chain / external sink |
| Signer misuse / key prediction | High if default identity key | High | Trait boundary | Mandatory seed or KMS; refuse identity-derived in prod |
| Race: concurrent approve | Low | Low | Conditional SQL | Keep |
| Race: update vs submit | Med | Med | **Fixed** status-guarded UPDATE | Add concurrency test |
| TOCTOU dry-run → Apply | High (post-Apply) | High | No Apply yet | Bind Apply to policy version+hash + fresh dry-run / attestation |
| CORS reflection | High (pre-fix) | High | **Fixed** restrictive CORS | Keep; integrate with `CP_ALLOWED_ORIGINS` |

---

## Testing Review

### Present coverage

- M1: auth, JWT claims, refresh rotation/reuse, rate limit, headers, observatory reads, no protocol write routes
- P1: RBAC helpers, CSRF, Origin, request ID, mutation_audit foundation
- P2: lifecycle, SoD self-approve, CSRF/Origin, viewer list, versioning
- P3: sign/verify/audit, no protocol coupling
- P4A: dry-run success/block, viewer, CSRF, Origin, signer fail, simulation reject, no mutation

### Missing before Phase 4B

| Test | Priority |
|------|----------|
| Concurrent update vs submit | High |
| Concurrent dual-approve (one success) | Medium |
| CSRF replay on non-consume routes | Medium |
| Duplicate dry-run / idempotency expectations | Medium |
| Expired JWT on dry-run | Low (covered generically) |
| Stale approved policy (version bump after dry-run) | High for Apply |
| Replayed signed operation rejected | High for Apply |
| Request cancellation / client disconnect | Low |
| Production config refuse (default passwords / missing origins) | High |

---

## Known Risks

1. Deterministic signer key from identity string if no seed (dev convenience).
2. Default operator passwords if env unset.
3. Missing-Origin allowed for API mutations (non-browser tooling).
4. Dry-run CSRF not consumed (intentional repeatability; audit noise).
5. Count-based protocol fingerprint unsuitable for Apply binding.
6. Three audit tables without a formal correlation schema.
7. WRITE_SECURITY.md header still says “design only” (doc drift).

---

## Required Fixes

### Completed in this review

1. Status-guarded policy content updates (TOCTOU).
2. Reject SoD aligned with approve.
3. Restrictive CORS (no permissive `*`).
4. Dry-run audit includes `operation_id` + `policy_version`.

### Mandatory before Phase 4B coding starts

| # | Action | Owner | Remediation status |
|---|--------|-------|--------------------|
| M1 | Production config gate: refuse default passwords; require `CP_ALLOWED_ORIGINS`; require `CP_SIGNER_SEED_HEX` or KMS backend (no identity-derived keys) | Engineering | **Done** — `config::production_config_check` |
| M2 | Apply design: single-use CSRF, `confirm: true`, policy version+hash binding, fresh attestation (no dry-run signature reuse) | Architecture | **Design frozen** — Apply security model (implementation still blocked) |
| M3 | Signed-operation replay protection (nonce / purpose / TTL) | Engineering | **Design frozen** — replay model doc (store not built) |
| M4 | Content-addressed protocol observation binding (replace count fingerprint) | Engineering | **Done for binding** — `execution_hash`; counts kept as dry-run mutation detector only |
| M5 | Add concurrency + CSRF-replay + stale-version tests | Engineering | **Plan done** — Apply tests deferred to Apply PR |
| M6 | Cookie `Secure` when TLS / `CP_SECURE_COOKIES=1` | Engineering | **Done** — config + cookie helpers + prod gate |
| M7 | Record dry-run in `mutation_audit` (or unified governance audit view) | Engineering | **Open** — recommendation documented; implement before Apply coding |

### Remaining blockers before Apply implementation

| Blocker | Status |
|---------|--------|
| Separate Apply design approval | **Required** |
| Replay store implementation | Open (design frozen) |
| Apply routes + PROTO-0 write adapters | Explicitly blocked |
| M7 dry-run `mutation_audit` row | Open |
| S*/C* Apply security & concurrency tests | Open (plan exists) |

**Apply remains unauthorised.** Closing design docs and M1/M4/M6 does not enable mutations.

### Recommended (non-blocking for starting 4B design)

- Update WRITE_SECURITY.md status to reflect implemented controls.
- Tamper-evident audit (SEC-CP-03).
- MFA for admin (SEC-CP-01).

---

## Recommendation

Proceed to **Apply design approval** using the frozen specs:

- [CONTROL_PLANE_APPLY_SECURITY_MODEL.md](CONTROL_PLANE_APPLY_SECURITY_MODEL.md)
- [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md)
- [CONTROL_PLANE_APPLY_TEST_PLAN.md](CONTROL_PLANE_APPLY_TEST_PLAN.md)

Do **not** enable Apply in any environment until M7 and Apply-path tests (S*/C*) are implemented and this gate is re-run after a separate Apply design approval.

Dry-run may continue in development as a pre-flight tool. Production exposure of the Control Plane should already satisfy M1 (origins, passwords, signer seed, secure cookies) even without Apply.

---

## Gate Decision

# **PASS WITH CONDITIONS**

### Justification

| Criterion | Result |
|-----------|--------|
| Control Plane cannot bypass protocol today | **Pass** — no write path to PROTO-0 |
| Authority invariants A1–A8 | **Pass** |
| Security middleware on CP mutations | **Pass** (with documented CSRF-consume nuances) |
| Critical CORS misconfiguration | **Pass after remediation** |
| Concurrent policy update race | **Pass after remediation** |
| Production-safe defaults for keys/passwords/origins | **Pass after M1 remediation** (when `CP_ENV=production`) |
| Apply-ready replay / TOCTOU / binding design | **Pass (design frozen)** — M2–M4 documented; Apply not built |
| Test readiness for Apply | **Conditional** — config/hash tests done; Apply S*/C* pending |

**Therefore:** the system is **approved to remain in dry-run-only governance mode**. Apply architecture is frozen for review. **Not approved to ship Apply / PROTO-0 mutations** until remaining blockers close and Apply receives separate approval.

---

## Freeze Statement

> Phase 4B Security Gate: **PASS WITH CONDITIONS**. Mandatory production gate (M1), Secure cookies (M6), and content-addressed `execution_hash` (M4) are remediated. Apply security / replay models are frozen. No protocol mutations are authorised. Apply remains blocked until remaining blockers (M7, Apply tests, separate design approval) are closed and this gate is re-verified. The signer authenticates; PROTO-0 decides — only when Apply is explicitly gated open.
