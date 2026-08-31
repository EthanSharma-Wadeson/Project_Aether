# Control Plane Write Security — Milestone 2

## Status

**Design only — no write endpoints authorised**

Date: 2026-07-29

Related: [CONTROL_PLANE_POLICY_THREAT_MODEL.md](CONTROL_PLANE_POLICY_THREAT_MODEL.md), [CONTROL_PLANE_AUTHORITY_MODEL.md](CONTROL_PLANE_AUTHORITY_MODEL.md), [../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md](../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md)

---

## 1. Purpose

Define **required controls before any write endpoint exists**. Milestone 1 auth (JWT, refresh rotation, rate limiting, security headers) is necessary but not sufficient for browser-originated mutations.

---

## 2. Required Controls (Gate)

Before the first `POST`/`PUT`/`DELETE` under `/api` ships:

| # | Control | Requirement |
|---|---------|-------------|
| W1 | CSRF tokens | Synchronizer token or double-submit; required on all state-changing requests |
| W2 | Origin / Referer validation | Reject cross-site origins; allow only configured CP origin(s) |
| W3 | SameSite cookie policy | Refresh cookie remains `SameSite=Strict`; path `/auth` |
| W4 | Request replay protection | CSRF token single-use or short TTL; optional idempotency key on apply/revoke |
| W5 | Mutation confirmation | Destructive actions require explicit `confirm: true` (or two-step apply) |
| W6 | Audit logging | Append-only record per attempt (success and failure) |
| W7 | RBAC enforcement | Role from verified JWT; middleware-level checks |
| W8 | PROTO-0 validation | Protocol-affecting writes call PROTO-0 APIs only; never raw store mutation |

No write route may merge without W1–W8.

---

## 3. CSRF Protection Strategy

### 3.1 Recommended pattern

**Synchronizer token** (preferred over double-submit alone):

```text
Login / session establish
        │
        ▼
CP issues CSRF token (bound to operator session)
        │
        ▼
SPA stores token in memory (not localStorage)
        │
        ▼
Each mutating request sends:
  Authorization: Bearer <access_jwt>
  X-CSRF-Token: <csrf>
```

### 3.2 Validation rules

| Check | Fail closed if |
|-------|----------------|
| CSRF header present | Missing / empty |
| Token matches server session store | Mismatch |
| Token not expired | TTL exceeded (e.g. 1–2 hours, refreshable) |
| Token not already consumed (if single-use for high-risk ops) | Replay |
| `Origin` matches allowlist | Missing or foreign origin |

### 3.3 Interaction with SPA

- Vite dev: allow `http://127.0.0.1:5173` and API origin in allowlist.
- Production: single origin (API serves SPA) — simplest CSRF posture.
- Access JWT remains in memory/localStorage as today; CSRF is **additional** for writes.

### 3.4 Why SameSite alone is insufficient

`SameSite=Strict` protects the refresh cookie. Access tokens sent as `Authorization` headers are not cookies; CSRF-like abuse via malicious pages is lower risk for pure Bearer APIs, but:

- Future cookie-based session modes
- Browser extensions / XSS
- Confused-deputy form posts

require explicit CSRF + Origin for defence in depth before writes.

---

## 4. Replay Protection

| Mechanism | Use |
|-----------|-----|
| CSRF single-use on `apply` / `revoke` / `freeze` | Primary |
| Optional `Idempotency-Key` header | Safe client retries without double-grant |
| Short access-token TTL (15 min) | Limits stolen-token window (M1) |
| Audit of duplicate attempts | Detection |

---

## 5. Mutation Confirmation

| Action | Confirmation rule |
|--------|-------------------|
| Create / update policy template | None beyond CSRF (local only) |
| Apply policy | Explicit apply step separate from template save; UI confirmation |
| Revoke capability | Body must include `confirm: true` |
| Freeze identity | Body must include `confirm: true`; `admin` only |

---

## 6. Future API Boundary (Design Only — Do Not Implement)

All routes require:

- Valid access JWT (`typ=access`)
- CSRF + Origin (writes)
- Audit append
- Role check

### 6.1 `POST /api/policies`

Create policy template (CP DB only).

| Requirement | Detail |
|-------------|--------|
| Authentication | Bearer access JWT |
| Authorization | `operator` or `admin` |
| Audit | `policy.create` + payload hash |
| Protocol validation | None (no PROTO-0 call) |

### 6.2 `PUT /api/policies/:id`

Update archived-or-active template.

| Requirement | Detail |
|-------------|--------|
| Authentication | Bearer access JWT |
| Authorization | `operator` or `admin` |
| Audit | `policy.update` |
| Protocol validation | None |

### 6.3 `POST /api/policies/:id/apply`

Translate template → PROTO-0 `grant_capability`.

| Requirement | Detail |
|-------------|--------|
| Authentication | Bearer access JWT |
| Authorization | `operator` or `admin` |
| Audit | `policy.apply` + `proto0_ref` on success |
| Protocol validation | PROTO-0 grant path (signature, chain, narrowing, root) |
| Confirmation | Separate from template create; UI confirm |

### 6.4 `POST /api/policies/:id/approve`

Optional staging hook (if draft→approved workflow enabled).

| Requirement | Detail |
|-------------|--------|
| Authentication | Bearer access JWT |
| Authorization | `admin` (recommended) or dual-control later |
| Audit | `policy.approve` |
| Protocol validation | None until apply |

**Note:** Approve does not issue capabilities. Apply does.

### 6.5 `POST /api/capabilities/:id/revoke`

Revoke via PROTO-0.

| Requirement | Detail |
|-------------|--------|
| Authentication | Bearer access JWT |
| Authorization | `operator` or `admin` |
| Audit | `capability.revoke` + confirm flag |
| Protocol validation | PROTO-0 revoke flow |
| Confirmation | `confirm: true` required |

### 6.6 Explicitly not designed for Milestone 2

```text
POST /api/escrows/*
POST /api/settlements/*
POST /api/reputation/*
```

---

## 7. Signing Key Handling (Design — No Implementation)

| Rule | Requirement |
|------|-------------|
| Storage | File path via env (`CP_KEY_PATH`); never DB; never frontend |
| Use | Server-side only for PROTO-0 grant/revoke/freeze |
| Logging | Never log key material or raw signatures beyond audit refs |
| Rotation | Documented restart-based rotation for M2; HSM later |
| Scope | Enterprise delegated capability ceiling enforced by PROTO-0 |

---

## Freeze Statement

> No write routes ship until CSRF, Origin validation, confirmation semantics, RBAC, audit, and PROTO-0-only mutation path are implemented and tested. This document is the security gate for those controls — not an implementation licence.
