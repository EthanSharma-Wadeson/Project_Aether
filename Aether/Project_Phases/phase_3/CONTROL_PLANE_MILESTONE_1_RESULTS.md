# Control Plane Milestone 1 Results — Read-Only Observatory

## Status

**Milestone 1 complete — awaiting security review gate before Milestone 2**

Date: 2026-07-29

Related: [CONTROL_PLANE_MVP_ARCHITECTURE.md](CONTROL_PLANE_MVP_ARCHITECTURE.md), [CONTROL_PLANE_SECURITY_MODEL.md](CONTROL_PLANE_SECURITY_MODEL.md)

---

## 1. Implemented Features

### Backend (`Aether/control_plane/`)

| Component | Status |
|-----------|--------|
| Axum HTTP server | Complete |
| JWT authentication (login, refresh, logout) | Complete |
| bcrypt password hashing (cost 12) | Complete |
| SQLite schema (`operators`, `refresh_tokens`, `audit_log`) | Complete |
| Read-only protocol adapters (PROTO-0, PROTO-2, PROTO-3, PROTO-4) | Complete |
| Observation index (bootstrap from enterprise demo happy path) | Complete |
| Append-only audit log for dashboard access | Complete |

### API Routes (all require JWT)

| Route | Source |
|-------|--------|
| `GET /api/agents` | PROTO-0 + index |
| `GET /api/agents/:id` | PROTO-0 |
| `GET /api/agents/:id/capabilities` | PROTO-0 |
| `GET /api/agents/:id/escrows` | PROTO-2 |
| `GET /api/agents/:id/settlements` | PROTO-4 |
| `GET /api/agents/:id/reputation` | PROTO-3 |
| `GET /api/agents/:id/events` | Unified timeline |
| `GET /api/capabilities` | PROTO-0 |
| `GET /api/escrows` | PROTO-2 |
| `GET /api/settlements` | PROTO-4 |
| `GET /api/reputation/:agent_id` | PROTO-3 |
| `GET /api/audit` | CP audit log |
| `GET /api/system` | Health + index counts |

### Frontend (`Aether/control_plane/frontend/`)

| View | Status |
|------|--------|
| Login | Complete |
| Agent Inventory | Complete |
| Agent Detail (capabilities, escrows, settlements, reputation, timeline) | Complete |
| Capability View | Complete |
| Economic Activity | Complete |
| Settlement View (soft/hard finality displayed) | Complete |
| Reputation View (metrics + chart, no rankings) | Complete |
| Audit Timeline | Complete |

---

## 2. Architecture Compliance

| Requirement | Status |
|-------------|--------|
| Control Plane is observational only | **Verified** — no write routes implemented |
| PROTO-0 write operations excluded | **Verified** — no grant/revoke/freeze endpoints |
| PROTO-2/3/4 read-only | **Verified** — adapters expose getters only |
| Policy engine excluded | **Verified** |
| Operator signing key excluded | **Verified** |
| Protocol code unchanged | **Verified** — no modifications to `aether-core` |
| Authority invariant preserved | **Verified** |

---

## 3. Tests

### Control Plane (`cargo test -p aether-control-plane`)

**18/18 tests pass** (security remediation expanded suite)

| Test | Result |
|------|--------|
| Valid login succeeds | Pass |
| Invalid password fails (generic error) | Pass |
| Unknown user returns generic error | Pass |
| Expired token rejected | Pass |
| Protected routes require auth | Pass |
| Agents endpoint returns observatory data (3 agents) | Pass |
| Dashboard escrow state matches protocol | Pass |
| No write routes exposed | Pass |
| Viewer role stored correctly | Pass |
| Repeated failures trigger rate limit (429) | Pass |
| Successful login resets rate limit | Pass |
| Refresh token rotation works | Pass |
| Old refresh tokens rejected (reuse) | Pass |
| Access token cannot be used as refresh | Pass |
| Refresh token cannot access API routes | Pass |
| JWT issuer validation | Pass |
| JWT audience validation | Pass |
| Security headers present | Pass |

### Protocol Regression (`cargo test -p aether-core`)

**311 tests pass** (unchanged)

### Linting

- `cargo fmt --check` — pass (control_plane)
- `cargo clippy --all-targets -- -D warnings` — pass (control_plane)

---

## 4. Milestone 1 Security Checklist

| Control | Status |
|---------|--------|
| bcrypt password hashing (cost ≥ 12) | Implemented |
| JWT verification on all `/api/*` routes | Implemented |
| JWT issuer / audience / type claims | Implemented |
| Role claim in JWT (`admin` / `viewer`) | Implemented |
| Short-lived access tokens (15 min) | Implemented |
| HttpOnly + SameSite=Strict refresh cookie | Implemented |
| Refresh token rotation + reuse detection | Implemented |
| Login rate limiting (IP + username, progressive backoff) | Implemented |
| Browser security headers | Implemented |
| Secrets from environment only | Implemented |
| Parameterised SQL queries (sqlx) | Implemented |
| Response sanitisation (no stack traces in errors) | Implemented |
| Read timestamps on protocol data (`read_at`) | Implemented |
| Audit log for login, refresh, and observation actions | Implemented |
| CSRF tokens | N/A for Milestone 1 (read-only) |
| TLS | **Not required for localhost MVP** — documented |

---

## 5. Limitations

1. **Observation index** — Entity IDs are indexed at bootstrap from the enterprise demo happy path. Full protocol store enumeration requires read-only iterator APIs in `aether-core` (deferred).
2. **Single demo dataset** — Bootstrap loads Phase 2 enterprise demonstrator scenario in-process, not from external store files.
3. **Default credentials** — `admin`/`admin` and `viewer`/`viewer` seeded on first run (override via `CP_ADMIN_PASSWORD` / `CP_VIEWER_PASSWORD`).
4. **In-memory rate limiter** — Suitable for single-process MVP; Redis-backed `LoginRateLimitStore` is the documented swap point for multi-instance deploy.
5. **Enterprise registration workflow** — Deferred to Milestone 2.
6. **Policy templates / capability revocation UI** — Explicitly excluded from Milestone 1.

---

## 6. Running Locally

### Backend

```bash
cd Aether/control_plane
export CP_JWT_SECRET=dev-secret-change-me
cargo run -p aether-control-plane
```

Server listens on `http://127.0.0.1:3001`.

### Frontend (development)

```bash
cd Aether/control_plane/frontend
npm install
npm run dev
```

Vite dev server proxies API calls to `:3001`.

### Frontend (production bundle served by backend)

```bash
cd Aether/control_plane/frontend
npm run build
cd ..
CP_JWT_SECRET=dev-secret-change-me \
CP_FRONTEND_DIR=./frontend/dist \
cargo run -p aether-control-plane
```

---

## 7. Milestone 2 Gate

Milestone 2 (Policy Management) is **blocked** until:

1. Milestone 1 security review approves the checklist above
2. CSRF protection is designed for write routes
3. Operator signing-key handling is designed and reviewed

---

## Freeze Statement

> Milestone 1 delivers a read-only Enterprise Agent Governance Console over the Phase 2 demonstrator dataset. No protocol authority boundaries were changed. Milestone 2 requires explicit security gate approval.
