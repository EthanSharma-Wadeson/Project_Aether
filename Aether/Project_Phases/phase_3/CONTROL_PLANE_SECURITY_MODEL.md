# Control Plane Security Model

## Status

**Security review — required before Milestone 2 implementation**

Date: 2026-07-29

Related: [CONTROL_PLANE_MVP_ARCHITECTURE.md](CONTROL_PLANE_MVP_ARCHITECTURE.md), [CONTROL_PLANE_THREAT_MODEL.md](CONTROL_PLANE_THREAT_MODEL.md)

---

## 1. Scope

This document classifies security threats specific to the Control Plane MVP. It extends the protocol-level threat models (PROTO-0 through PROTO-3) with application-layer threats introduced by the dashboard, API server, and operator authentication boundary.

**Critical invariant:** The Control Plane is an application layer above the protocol stack. A compromised Control Plane is an application security incident. It cannot compromise protocol cryptographic guarantees (escrow validity, capability chain integrity, settlement proofs) because those guarantees are enforced by protocol logic, not by the Control Plane.

---

## 2. Threat Classification

### 2.1 Compromised Admin Account

**Threat:** An attacker gains access to a dashboard operator account with `admin` role.

**Attack vectors:**
- Credential theft (phishing, password reuse, keylogger)
- Session token theft (XSS, network interception)
- Brute force against login endpoint
- Insider threat

**Impact:**
- Attacker can issue PROTO-0 capabilities to arbitrary agents (bounded by the enterprise operator key's own delegation scope)
- Attacker can revoke legitimate capabilities (service disruption)
- Attacker can freeze agent identities (denial of service)
- Attacker can read all protocol state (economic activity, agent identities)
- Attacker cannot steal funds — escrow release requires agent-signed receipts that the CP cannot forge
- Attacker cannot forge reputation events — PROTO-3 event_id is cryptographically bound to evidence
- Attacker cannot exceed the enterprise operator key's delegation depth or scope

**Controls:**

| Control | Status |
|---------|--------|
| bcrypt password hashing (cost ≥ 12) | Required — implement at Milestone 1 |
| Login rate limiting (progressive backoff by IP + username) | **Implemented** — Milestone 1 remediation |
| Short-lived access tokens (15 min) | Required — implement at Milestone 1 |
| HttpOnly + SameSite=Strict refresh token cookies | Required — implement at Milestone 1 |
| Refresh token rotation + reuse detection | **Implemented** — Milestone 1 remediation |
| JWT issuer / audience / type separation | **Implemented** — Milestone 1 remediation |
| Browser security headers (CSP, XCTO, XFO, Referrer-Policy) | **Implemented** — Milestone 1 remediation |
| CSRF token on all state-changing requests | Required — implement at Milestone 2 |
| MFA (TOTP) | OPEN — recommended post-MVP |
| Session anomaly detection (IP/UA change) | OPEN — post-MVP |
| Immutable audit log of all write operations | Required — implement at Milestone 1 |

**Classification: VALIDATED** — bcrypt + progressive login rate limiting + short tokens + refresh rotation + audit log provide adequate MVP protection. MFA is a documented gap.

---

## 2.1a Login Abuse Protection

**Mechanism:** `LoginRateLimitStore` trait with in-memory `InMemoryLoginRateLimiter` (Redis-replaceable).

| Rule | Behaviour |
|------|-----------|
| Scope | `POST /auth/login` only |
| Keys | Client IP **and** normalised username (whichever is worse wins) |
| Threshold | 5 failed attempts within a 15-minute window |
| Response | Failed attempts return `401 { "error": "invalid credentials" }` (no username oracle) |
| Cooldown | After threshold: progressive backoff starting at 30s, doubling per excess failure, capped at 300s |
| Locked response | `429 { "error": "too many login attempts" }` |
| Success | Resets IP and username counters — no permanent lockout |
| IP source | `X-Forwarded-For` → `X-Real-IP` → connect info → `"unknown"` |

---

## 2.1b Refresh Token Lifecycle

```text
login  → issue refresh_A (JWT typ=refresh, unique jti) → store hash(A)
refresh(A) → validate JWT + DB → issue refresh_B → revoke A → store hash(B)
reuse(A)   → detect revoked presentation → revoke ALL operator refresh tokens → audit
```

| Property | Enforcement |
|----------|-------------|
| HttpOnly + SameSite=Strict cookie | Path `/auth` only |
| Rotation on every refresh | Old token immediately revoked |
| Reuse detection | Revoked token re-presentation → full operator session kill + `auth.refresh.reuse_detected` audit |
| Type separation | Access JWT (`typ=access`) rejected by refresh endpoint; refresh JWT rejected by API middleware |

---

## 2.1c JWT Trust Boundaries

| Claim | Access token | Refresh token |
|-------|--------------|---------------|
| `iss` | `aether-control-plane` | `aether-control-plane` |
| `aud` | `cp-api` | `cp-api` |
| `typ` | `access` | `refresh` |
| `jti` | omitted | required (UUID) |
| TTL | 15 minutes | 7 days |
| Authorises `/api/*` | Yes | **No** |
| Authorises `/auth/refresh` | **No** | Yes (plus DB hash check) |

Verification rejects wrong issuer, wrong audience, wrong type, and expired `exp`.

---

## 2.1d Browser Security Headers

Applied to all Control Plane responses (including SPA assets):

| Header | Value |
|--------|-------|
| `Content-Security-Policy` | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; …; frame-ancestors 'none'` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |

CSP allows Vite-built hashed assets under `/assets` and inline styles used by the SPA. `frame-ancestors 'none'` complements `X-Frame-Options`.

---

### 2.2 Malicious Operator

**Threat:** A legitimate operator deliberately misuses the dashboard to harm the system or other agents.

**Attack vectors:**
- Fraudulent capability issuance (issuing capabilities beyond legitimate scope)
- Mass revocation (deliberate service disruption)
- Data exfiltration of agent economic activity
- Fabricated audit records

**Impact:**
- Capability fraud: attacker issues capabilities granting agents more authority than business policy allows, but still bounded by the enterprise operator key's delegation ceiling
- Revocation attack: legitimate agents lose their capabilities
- The enterprise key's own delegation scope is enforced by PROTO-0 — a malicious operator cannot issue capabilities beyond it
- Audit records in the CP database are mutable by a database admin (CP-internal limitation)

**Controls:**

| Control | Status |
|---------|--------|
| Role separation: viewer vs admin | Required |
| PROTO-0 delegation ceiling enforcement | VALIDATED (protocol-enforced) |
| Audit log records all PROTO-0 write operations with operator ID | Required |
| Policy templates require explicit apply action (no bulk auto-apply) | Required |
| Revocation requires confirmation + audit entry | Required |
| Segregation of duties (dual approval) for freeze/revoke | OPEN — post-MVP |

**Classification: VALIDATED for capability ceiling; OPEN for insider threat at CP application layer.** The protocol enforces the hard ceiling. CP-level controls (audit, role separation) reduce but do not eliminate insider risk. Dual approval is a post-MVP hardening measure.

---

### 2.3 Leaked Operator Credentials

**Threat:** Dashboard credentials are leaked via code repositories, configuration files, environment variables, or logs.

**Attack vectors:**
- `CP_JWT_SECRET` committed to version control
- Operator passwords hardcoded in config files
- Credentials appearing in structured logs

**Impact:**
- `CP_JWT_SECRET` leak allows arbitrary JWT forgery → full admin access
- Operator password leak → account takeover
- PROTO-0 signing key leak (separate from dashboard credentials) → capability issuance/revocation with operator authority

**Controls:**

| Control | Status |
|---------|--------|
| `CP_JWT_SECRET` loaded from environment variable only; never hardcoded | Required |
| PROTO-0 key loaded from filesystem path (env var); never in DB or logs | Required |
| Secrets never logged (tracing directives exclude credential fields) | Required |
| `.env` files excluded from version control (`.gitignore`) | Required |
| Key rotation path documented (requires server restart in MVP) | Required — document at Milestone 1 |
| Automatic secret scanning in CI | OPEN — recommended post-MVP |

**Classification: VALIDATED** — standard secret hygiene controls are implementable and sufficient for MVP. Automated scanning is a post-MVP hardening measure.

---

### 2.4 Dashboard Privilege Escalation

**Threat:** A `viewer`-role operator escalates to `admin` via API manipulation or JWT tampering.

**Attack vectors:**
- Modified JWT claims (role field altered)
- Insecure direct object reference (IDOR) on write endpoints
- Missing authorization checks on admin-only routes

**Impact:**
- Viewer gains write access to capability management
- Could issue/revoke capabilities without appropriate authority

**Controls:**

| Control | Status |
|---------|--------|
| JWT signature verification on every request (HS256 with `CP_JWT_SECRET`) | Required |
| Role claim extracted from verified JWT — never from request body/params | Required |
| Every write route checks `role == admin` before proceeding | Required |
| Role check middleware applied at router level (not individual handlers) | Required |
| Integration tests: viewer attempting write routes must receive 403 | Required |

**Classification: VALIDATED** — standard JWT verification + middleware-level role enforcement prevents privilege escalation. Tests must cover this path.

---

### 2.5 Fake Protocol Data Injection

**Threat:** Attacker injects false data into CP-internal indexes or API responses to mislead operators.

**Attack vectors:**
- Direct database write to CP SQLite (physical access or SQL injection)
- API response tampering (MITM between frontend and backend)
- Protocol store file replacement with crafted data

**Impact:**
- Operator sees false agent status, capability state, or economic history
- Could lead to incorrect governance decisions (e.g., believing an agent is compliant when it is not)

**Controls:**

| Control | Status |
|---------|--------|
| CP always reads directly from protocol stores (no intermediate cache layer in MVP) | VALIDATED — no stale cache to poison in Milestone 1 |
| Protocol data is read-only from CP's perspective; tampering requires filesystem access | VALIDATED — same threat surface as any file-based store |
| TLS on API server (self-signed acceptable for local MVP; required for any networked deployment) | Required |
| Parameterised queries (sqlx) prevent SQL injection into CP database | Required |
| PROTO-0 capability chain verification remains in protocol code — CP cannot forge valid chains | VALIDATED (protocol-enforced) |

**Classification: VALIDATED for protocol data integrity (cryptographic); OPEN for physical/filesystem access.** Physical security is out of scope for a software MVP but must be noted for enterprise deployment guidance.

---

### 2.6 Stale Indexes / Stale Protocol Reads

**Threat:** The Control Plane displays outdated protocol state, causing operators to make decisions based on stale information.

**Attack vectors:**
- Protocol store file read while a write is in progress (torn read)
- Long-running query reads a snapshot that is several transactions old
- CP process holds stale in-memory state after store update

**Impact:**
- Operator revokes a capability that has already been superseded
- Operator believes an escrow is open when it has been released
- Operator sees reputation metrics from before a recent event

**Controls:**

| Control | Status |
|---------|--------|
| CP reads protocol stores on each request (no persistent in-memory cache in MVP) | Required — simpler and safer for MVP |
| Read timestamps displayed on all protocol data views | Required |
| "Refresh" action available on all views | Required |
| Protocol stores use atomic writes (existing Aether design) | VALIDATED (existing protocol design) |

**Classification: VALIDATED for atomic store writes; OPEN for latency between store update and CP read.** In MVP (co-located, local), staleness window is negligible. For remote deployment the staleness window must be explicitly bounded.

---

### 2.7 Data Leakage

**Threat:** Sensitive protocol data (agent identities, escrow amounts, settlement details, reputation evidence) is exposed to unauthorised parties.

**Attack vectors:**
- Unauthenticated API access (missing or skipped auth middleware)
- Over-broad query responses (returning all agents when only some are authorised)
- Log output containing economic data
- CP database file accessible to other OS users

**Impact:**
- Competitor agents learn the enterprise's economic activity
- Agents' capabilities are disclosed (enabling targeted attacks)
- Privacy violation of agent interaction history

**Controls:**

| Control | Status |
|---------|--------|
| Authentication middleware applied to all `/api/*` routes | Required |
| No protocol data in server logs (log only operation type, agent ID prefix, outcome) | Required |
| CP database file permissions: `600` (owner read/write only) | Required |
| No CORS in MVP (API and frontend served from same origin) | Required |
| Response objects stripped of internal fields before serialisation | Required |
| Error responses never include stack traces or internal identifiers | Required |

**Classification: VALIDATED** — standard web API security controls are implementable and sufficient for single-tenant MVP. Multi-tenant data isolation is explicitly out of scope.

---

## 3. Out of Scope Threats

The following threats are explicitly not addressed in the MVP. Each has a documented reason.

| Threat | Reason out of scope |
|--------|-------------------|
| DDoS against CP API | MVP is local/internal deployment; no public network exposure |
| Supply chain attacks (compromised Rust crates) | Standard crate auditing; out of scope for MVP |
| Hardware key compromise | MVP uses file-based keys; HSM is post-MVP |
| Multi-tenant data isolation failures | MVP is single-tenant by design |
| Protocol-layer cryptographic breaks | Out of scope — the protocol's own security models cover this |
| Social engineering of operators | Human process control; out of scope for software |
| Side-channel attacks on JWT verification | Standard library mitigations; acceptable for MVP |
| Regulatory compliance (GDPR, SOC2) | Post-MVP; requires production deployment decision |

---

## 4. Authority Boundary Summary

The following table summarises what the Control Plane CAN and CANNOT do, and which layer enforces each constraint.

| Action | CP can do? | Enforced by |
|--------|-----------|-------------|
| Read any protocol store | Yes (with filesystem access) | OS permissions |
| Issue capabilities within operator delegation | Yes | PROTO-0 (delegation narrowing) |
| Issue capabilities beyond operator delegation | No | PROTO-0 (cryptographic enforcement) |
| Revoke capabilities in operator's chain | Yes | PROTO-0 |
| Forge escrow state transitions | No | PROTO-2 (pub(crate) store mutations) |
| Forge settlement proofs | No | PROTO-4 (signature verification) |
| Fabricate reputation events | No | PROTO-3 (deterministic event_id + evidence) |
| Override dispute outcomes | No | PROTO-1/2 (protocol logic) |
| Forge agent signatures | No | Ed25519 cryptographic enforcement |
| Modify audit log (own DB) | Yes (admin DB access) | Not enforced by software in MVP |
| Read all economic activity | Yes (authorised operators) | CP authentication |

---

## 5. Milestone Gating

### Milestone 1 (Read-Only Observatory) — Required controls before deployment

- [x] bcrypt password hashing (cost ≥ 12)
- [x] Login rate limiting (progressive backoff by IP + username)
- [x] JWT verification middleware on all `/api/*` routes (iss / aud / typ)
- [x] Refresh token rotation + reuse detection
- [x] Browser security headers (CSP, XCTO, XFO, Referrer-Policy)
- [x] Role-based route protection (viewer vs admin)
- [x] Secrets loaded from environment (never hardcoded, never logged)
- [x] Parameterised queries (sqlx) throughout
- [x] Response sanitisation (no internal fields, no stack traces in errors)
- [x] Read timestamps displayed on protocol data views
- [ ] CP database file permissions: 600 (deployment checklist)
- [ ] TLS (self-signed acceptable for localhost; required for network exposure)
- N/A — PROTO-0 signing key (Milestone 2 only)

### Milestone 2 (Policy Management) — Additional controls before write operations

- [x] Phase 1: CSRF synchronizer store + validation helper (`X-CSRF-Token`; SameSite-compatible cookie helper)
- [x] Phase 1: Origin allowlist validation (`CP_ALLOWED_ORIGINS`)
- [x] Phase 1: Role middleware primitives (`viewer` / `operator` / `admin`) — no write routes wired yet
- [x] Phase 1: Mutation audit framework (append-only; required fields)
- [x] Phase 1: Request identity (`X-Request-ID`) generation + propagation
- [x] Phase 2: Template lifecycle create → review → approve (no PROTO-0)
- [x] Phase 3: Signer abstraction (no raw key in handlers)
- [x] Phase 4A: Dry-run execution pipeline (no protocol mutation)
- [ ] Phase 4B: PROTO-0 grant/revoke via adapters only
- [ ] Phase 4B: Revocation / apply require explicit confirmation
- [ ] Phase 5: Integration tests — unauthorized roles, CSRF, audit, protocol rejects

---

## 5.1 Secure Write Foundation

Phase 1 delivers the security infrastructure future write operations will use. **No POST/PUT/DELETE write routes** were added.

### CSRF model

- Synchronizer token issued via `CsrfStore::issue(operator_id)` (UUID, operator-bound, TTL).
- Client must send `X-CSRF-Token` on future mutating requests.
- `validate_request_csrf` rejects missing / invalid / expired / operator-mismatched tokens.
- Optional consume-on-use for high-risk mutations.
- Delivery cookie helper `csrf_cookie` uses `SameSite=Strict` (aligned with refresh cookie W3); `HttpOnly=false` so the SPA can copy into the header.
- Refresh cookie remains `SameSite=Strict` / path `/auth` (unchanged).

### Origin checks

- Configured via `CP_ALLOWED_ORIGINS` (comma-separated exact Origins).
- Empty allowlist (local/dev): allow missing Origin (non-browser) and localhost Origins only.
- Non-empty allowlist: exact match required when Origin is present; missing Origin still allowed for non-browser clients (curl, tests).

### RBAC enforcement

- Hierarchy: `viewer` < `operator` < `admin`.
- Primitives: `require_role`, `require_operator`, `require_admin`, `reject_viewer_writes`.
- Viewer remains read-only; **no production write handlers call these yet**.

### Mutation audit lifecycle

Every future mutation must record (append-only `mutation_audit` table):

| Field | Purpose |
|-------|---------|
| `request_id` | Correlates with `X-Request-ID` |
| `operator_id` / `role` | Who requested |
| `action` / `target` | What was attempted |
| `requested_at` / `approved_at` | Timing |
| `signer_identity` | Which signer authenticated (Phase 3+) |
| `protocol_result` | PROTO-0 success / rejected / N/A |
| `failure_reason` | Human-readable failure |
| `payload_hash` | SHA-256 of request payload |

Flow for future writes:

```text
request_id → RBAC → CSRF → Origin → (policy) → Signer → PROTO-0 → mutation_audit.record
```

### Request identity

- Middleware generates or propagates `X-Request-ID`.
- Stored in request extensions (`RequestId`) for audit services.
- Echoed on responses; attached to tracing spans.

**Milestone status:** Secure Write Foundation complete — awaiting Phase 2 approval.

---

## 5.2 Signing Architecture

Phase 3 delivers the signing boundary. **The signer authenticates an already-authorized governance operation. The signer is NOT the authority.**

Authority chain:

```text
Operator
    ↓
Authentication
    ↓
RBAC
    ↓
Policy approval
    ↓
Signer
    ↓
(Future) PROTO-0
    ↓
Protocol validation
    ↓
State transition
```

### Components

| Component | Role |
|-----------|------|
| `GovernanceOperation` | Canonical unsigned intent (operation_id, request_id, policy, operator, action, payload_hash, …) |
| `Signer` trait | `sign_operation` / `verify_signature` / `signer_identity` — no key exposure |
| `EnterpriseSigner` | MVP server-held Ed25519 behind the trait (`CP_SIGNER_MODE=enterprise`) |
| `SigningGateway` | Audited call path (`SIGN_REQUEST_CREATED` / `COMPLETED` / `FAILED`) |

### Configuration

| Env | Purpose |
|-----|---------|
| `CP_SIGNER_MODE` | `enterprise` (default) |
| `CP_SIGNER_IDENTITY` | Audit label, e.g. `enterprise-default` |
| `CP_SIGNER_SEED_HEX` | Optional 32-byte hex seed for deterministic keys (tests/dev) |
| `CP_SIGNER_EPHEMERAL` | Optional fresh key per boot (dev only) |

Future backends (Vault, AWS KMS, Azure Key Vault, GCP KMS, HSM, dedicated signing service) replace `EnterpriseSigner` without changing call sites.

### Explicit Phase 3 boundaries

- No apply / grant / revoke routes
- Signer does **not** call PROTO-0
- No protocol mutations
- Private keys never leave the enterprise signer module
- No `protocol_result` on signing audit (signing is CP-local attestation)

**Milestone status:** Signer Abstraction complete — awaiting Phase 4 approval.

---

## 5.3 Dry-Run Execution

Phase 4A exercises the **complete governance pipeline** without modifying protocol state. Dry-run is mandatory before any future Apply operation.

```text
Operator
    ↓
Authentication
    ↓
RBAC
    ↓
CSRF + Origin
    ↓
Policy approval check
    ↓
Signer
    ↓
Protocol simulation (read-only)
    ↓
DryRunReport
```

| Component | Role |
|-----------|------|
| `ExecutionPlanner` | Approved policy → `GovernanceOperation` → `ExecutionPlan` (`DryRun`) |
| `ExecutionExecutor::dry_run` | Runs validation steps; signs; simulates; **never mutates** |
| `simulate_*` helpers | Answer “would PROTO-0 accept?” via observation only |
| `POST /api/policies/:id/dry-run` | Secured dry-run API |

Audit: `DRY_RUN_REQUESTED` / `DRY_RUN_COMPLETED` / `DRY_RUN_FAILED` — no `protocol_result`.

**Explicit:** No Apply button. No capability grant/revoke. Protocol fingerprint must be unchanged after dry-run.

---

## 6. Future Signing Architecture

Full roadmap: [CONTROL_PLANE_SECURITY_MODEL.md (Aether_docs)](../../Aether_docs/CONTROL_PLANE_SECURITY_MODEL.md).

Summary:

| Stage | Architecture |
|-------|--------------|
| MVP | Single enterprise signing identity behind `Signer` abstraction |
| Production | Isolated signer service (key material out of API process) |
| Enterprise | KMS/HSM-backed keys |
| High assurance | Multi-party approval for destructive actions |

The signer authenticates authorized requests; PROTO-0 remains final authority.

---

## 7. Open Items

| ID | Item | Priority | When |
|----|------|----------|------|
| SEC-CP-01 | MFA (TOTP) for admin operators | High | Post-MVP |
| SEC-CP-02 | Per-operator PROTO-0 key delegation | High | Post-MVP |
| SEC-CP-03 | Tamper-evident audit log (hash chain or external append-only store) | High | Pre-production |
| SEC-CP-04 | Dual approval for freeze/revoke-identity operations | Medium | Post-MVP / high assurance |
| SEC-CP-05 | Session anomaly detection (IP/UA change triggers re-auth) | Medium | Post-MVP |
| SEC-CP-06 | Automated secret scanning in CI | Medium | Post-MVP |
| SEC-CP-07 | Physical security guidance for enterprise deployment | Low | Pre-production |
| SEC-CP-08 | Explicit staleness bound documentation for remote deployment | Medium | Pre-production |
| SEC-CP-09 | Isolated signer service (production signing architecture) | High | Production |

---

## Freeze Statement

> The Control Plane MVP introduces application-layer risk but not protocol-layer risk. Every protocol-affecting write goes through PROTO-0 with the same cryptographic enforcement as any other protocol caller. The signer is not the authority. Dry-run execution (Phase 4A) is complete — awaiting Phase 4B Apply approval. No protocol mutations yet. Milestone 2 ships only after Phases 1–5. Open items are explicitly deferred and documented.
