# Control Plane MVP Architecture

## Status

**Implementation approved — architecture frozen before coding begins**

Date: 2026-07-29

Approval: Phase 3 Enterprise Agent Governance Console — Implementation Approval

Related: [PHASE_3_WEDGE_DECISION.md](PHASE_3_WEDGE_DECISION.md), [CONTROL_PLANE_ARCHITECTURE.md](CONTROL_PLANE_ARCHITECTURE.md), [CONTROL_PLANE_SECURITY_MODEL.md](CONTROL_PLANE_SECURITY_MODEL.md)

---

## 1. Overview

The MVP Control Plane is a single-tenant enterprise governance interface. It connects to the Aether protocol stores as a read-only observer for all protocol state except capability management, where it is an authorised writer via PROTO-0.

```text
┌─────────────────────────────────────────────────────────────┐
│                    Enterprise Operator                       │
│                      (browser, JWT)                          │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Frontend (React SPA)                       │
│  Agent Inventory │ Policy Console │ Economic │ Reputation    │
│  Audit Timeline  │ Settlement     │ Capability View          │
└────────────────────────┬────────────────────────────────────┘
                         │  REST + JSON
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  Backend API Server (Axum)                   │
│                                                              │
│  ┌─────────────────┐   ┌──────────────────────────────┐     │
│  │  Auth Middleware │   │  Audit Logger (append-only)  │     │
│  └─────────────────┘   └──────────────────────────────┘     │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │               Protocol Integration Layer              │   │
│  │  ┌───────────┐ ┌────────────┐ ┌──────────────────┐  │   │
│  │  │ PROTO-0   │ │ PROTO-2/4  │ │ PROTO-3          │  │   │
│  │  │ Reader +  │ │ Reader     │ │ Reader            │  │   │
│  │  │ Writer    │ │ (read only)│ │ (read only)       │  │   │
│  │  └───────────┘ └────────────┘ └──────────────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │               Control Plane Database (SQLite)         │   │
│  │  policy_templates │ audit_log │ operator_accounts    │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                         │
          ┌──────────────┴───────────────┐
          │                              │
          ▼ read                         ▼ write (PROTO-0 only)
┌──────────────────────┐    ┌───────────────────────────────┐
│  Protocol Stores     │    │  PROTO-0 Capability API       │
│  (append-only)       │    │  grant_capability()           │
│  PROTO-0 CapStore    │    │  revoke_capability()          │
│  PROTO-2 EscrowStore │    │  freeze_identity()            │
│  PROTO-4 SettleStore │    │  revoke_identity()            │
│  PROTO-3 ReputStore  │    └───────────────────────────────┘
└──────────────────────┘
```

---

## 2. Frontend Architecture

### 2.1 Technology Selection

| Choice | Selection | Rationale |
|--------|-----------|-----------|
| Framework | React 18 (TypeScript) | Mature ecosystem; strong enterprise UI tooling |
| State management | Zustand | Lightweight; avoids Redux overhead for MVP scale |
| Data fetching | TanStack Query | Cache management, background refresh, loading states |
| UI components | shadcn/ui + Tailwind CSS | Composable, unstyled-by-default; no vendor lock-in |
| Routing | React Router v6 | Standard SPA routing |
| Charts | Recharts | React-native charting; sufficient for MVP metrics |

### 2.2 Route Map

```text
/                          → redirect to /agents
/agents                    → Agent Inventory (list)
/agents/:id                → Agent Detail
  /agents/:id/capabilities → Capability list for agent
  /agents/:id/escrows      → Escrow history for agent
  /agents/:id/settlements  → Settlement history for agent
  /agents/:id/reputation   → Reputation metrics for agent
  /agents/:id/events       → Audit timeline for agent
/policies                  → Policy template list
/policies/new              → Create policy template
/policies/:id              → Policy detail + apply
/capabilities              → All active capability grants
/capabilities/:id          → Capability detail + revocation
/economic                  → Economic activity overview
/economic/escrows          → Escrow list + filter
/economic/settlements      → Settlement list + filter
/reputation                → Reputation overview
/reputation/:agent_id      → Agent metrics + evidence
/audit                     → System audit log
/settings                  → Operator account settings
```

### 2.3 View Responsibilities

| View | Read | Write |
|------|------|-------|
| Agent Inventory | PROTO-0, PROTO-3 summary | None |
| Agent Detail | PROTO-0, PROTO-2, PROTO-4, PROTO-3 | None |
| Capability View | PROTO-0 capability store | Revoke (PROTO-0) |
| Policy Console | CP policy templates | Create/update template; apply → PROTO-0 |
| Economic Activity | PROTO-2 escrow store | None |
| Settlement View | PROTO-4 settlement store | None |
| Reputation View | PROTO-3 metrics + events | None |
| Audit Timeline | CP audit log + protocol events | None |

### 2.4 Auth Flow

```text
Operator → Login page → POST /auth/login → JWT (15min access + 7d refresh)
         → All subsequent requests carry Bearer token
         → Expired token → transparent refresh or redirect to login
```

---

## 3. Backend Architecture

### 3.1 Technology Selection

| Choice | Selection | Rationale |
|--------|-----------|-----------|
| Language | Rust | Consistent with protocol codebase; shared types |
| HTTP framework | Axum 0.7 | Async-first; tower middleware compatibility |
| Serialisation | serde + serde_json | Standard in Aether codebase |
| Database ORM | sqlx (async) | Compile-time query validation; SQLite for MVP |
| Authentication | JWT (jsonwebtoken crate) | Standard; stateless |
| Logging | tracing + tracing-subscriber | Structured logs; consistent with protocol layer |

### 3.2 Crate Structure

```text
control_plane/
├── Cargo.toml
├── src/
│   ├── main.rs                  # server bootstrap
│   ├── config.rs                # config loading (env + file)
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── middleware.rs        # JWT extraction + validation
│   │   └── handlers.rs          # login, refresh, logout
│   ├── routes/
│   │   ├── mod.rs               # route registration
│   │   ├── agents.rs
│   │   ├── capabilities.rs
│   │   ├── policies.rs
│   │   ├── economic.rs
│   │   ├── settlements.rs
│   │   ├── reputation.rs
│   │   ├── audit.rs
│   │   └── system.rs
│   ├── protocol/
│   │   ├── mod.rs
│   │   ├── identity.rs          # PROTO-0 read adapter
│   │   ├── capability.rs        # PROTO-0 read + write adapter
│   │   ├── escrow.rs            # PROTO-2 read adapter
│   │   ├── settlement.rs        # PROTO-4 read adapter
│   │   └── reputation.rs        # PROTO-3 read adapter
│   ├── policy/
│   │   ├── mod.rs
│   │   ├── engine.rs            # template → capability translation
│   │   └── validator.rs         # policy constraint validation
│   ├── audit/
│   │   ├── mod.rs
│   │   └── logger.rs            # append-only audit log writer
│   └── db/
│       ├── mod.rs
│       ├── migrations/          # sqlx migration files
│       └── models.rs            # CP-internal data models
```

### 3.3 API Endpoint Map

All routes require `Authorization: Bearer <jwt>` unless noted.

**Auth (no JWT required)**
```
POST /auth/login          → { username, password } → { access_token, refresh_token }
POST /auth/refresh        → { refresh_token } → { access_token }
DELETE /auth/logout       → invalidates refresh token
```

**Agents (read-only)**
```
GET /api/agents                     → list agents with status + metrics summary
GET /api/agents/:id                 → agent identity detail
GET /api/agents/:id/capabilities    → active capabilities (from PROTO-0)
GET /api/agents/:id/escrows         → escrow history (from PROTO-2)
GET /api/agents/:id/settlements     → settlement history (from PROTO-4)
GET /api/agents/:id/reputation      → AgentMetricsV0 + events (from PROTO-3)
GET /api/agents/:id/events          → unified protocol event timeline
```

**Capabilities**
```
GET  /api/capabilities                  → list all active capability grants
GET  /api/capabilities/:id              → capability detail + delegation chain
POST /api/capabilities/revoke           → { capability_id } → PROTO-0 revoke
```

**Policies (CP-internal + PROTO-0 write on apply)**
```
GET    /api/policies                    → list policy templates
POST   /api/policies                    → create template
GET    /api/policies/:id                → template detail
PUT    /api/policies/:id                → update template
DELETE /api/policies/:id                → archive template
POST   /api/policies/:id/apply          → { target_agent_id } → issue capabilities
```

**Economic (read-only, PROTO-2)**
```
GET /api/escrows                    → list escrows (filters: status, agent, asset, date)
GET /api/escrows/:id                → escrow detail + finality view
GET /api/economic/summary           → aggregate metrics
```

**Settlements (read-only, PROTO-4)**
```
GET /api/settlements                → list settlement bindings
GET /api/settlements/:id            → binding detail + finality state
```

**Reputation (read-only, PROTO-3)**
```
GET /api/reputation/:agent_id       → AgentMetricsV0
GET /api/reputation/:agent_id/events → ReputationEventV0 list
GET /api/reputation/:agent_id/evidence → EvidenceRefV0 list
```

**Audit (read-only, CP-internal)**
```
GET /api/audit                      → audit log (filters: operator, action, date)
GET /api/audit/:id                  → event detail
```

**System**
```
GET /api/system/health              → CP health + protocol store connectivity
GET /api/system/indexes/status      → index freshness
```

---

## 4. Protocol Integration Layer

### 4.1 Design Principle

The Protocol Integration Layer (PIL) is a read-only bridge from CP backend to protocol stores, plus a write bridge to PROTO-0. It never holds protocol authority — it calls protocol APIs with operator-delegated keys.

### 4.2 Protocol Adapters

Each adapter wraps a protocol store with a CP-facing interface:

```rust
// Example: capability adapter (read + write)
pub struct CapabilityAdapter<'a> {
    store: &'a CapabilityStore,
    signing_key: &'a SigningKey,   // operator's delegated PROTO-0 key
}

impl<'a> CapabilityAdapter<'a> {
    pub fn list_for_agent(&self, agent_id: &AgentId) -> Vec<CapabilityGrant> { ... }
    pub fn get(&self, capability_id: &CapabilityId) -> Option<CapabilityGrant> { ... }
    pub fn revoke(&self, capability_id: &CapabilityId) -> Result<(), CapabilityError> { ... }
    // grant() is called only via policy engine, not directly from routes
}

// Example: escrow adapter (read-only)
pub struct EscrowAdapter<'a> {
    store: &'a EscrowStore,
}

impl<'a> EscrowAdapter<'a> {
    pub fn list_for_agent(&self, agent_id: &AgentId) -> Vec<EscrowRecord> { ... }
    pub fn get(&self, escrow_id: &EscrowId) -> Option<EscrowRecord> { ... }
    pub fn finality_view(&self, escrow_id: &EscrowId) -> EconomicFinalityViewV0 { ... }
}
```

### 4.3 Key Management (MVP)

For MVP, operator signing keys are loaded at server startup from a local key file (Ed25519 keypair, PEM format). The key must be a delegated capability from the enterprise root principal.

**Key constraints:**
- Key file path set via `CONTROL_PLANE_KEY_PATH` environment variable
- Keys are never stored in the CP database
- Keys are never transmitted to the frontend
- Key rotation requires server restart (MVP limitation — documented)

Production key management (HSM, secret manager) is explicitly out of scope for MVP.

### 4.4 Protocol Store Access

For MVP, protocol stores are accessed as in-process Rust types (the CP backend is co-located with protocol store files). This avoids network complexity for the first milestone.

```text
CP Backend process
    ├── loads PROTO-0 stores from configured path
    ├── loads PROTO-2 stores from configured path
    ├── loads PROTO-4 stores from configured path
    └── loads PROTO-3 stores from configured path
```

The store paths are set via environment variables. No store modifications are made except through PROTO-0 write APIs with valid signing keys.

---

## 5. Storage Strategy

### 5.1 Protocol Stores (source of truth)

| Store | Access | Location |
|-------|--------|----------|
| PROTO-0 IdentityRegistry | Read + PROTO-0 write API | Protocol data directory |
| PROTO-0 CapabilityStore | Read + PROTO-0 write API | Protocol data directory |
| PROTO-2 EscrowStore | Read only | Protocol data directory |
| PROTO-4 SettlementStore | Read only | Protocol data directory |
| PROTO-3 ReputationStore | Read only | Protocol data directory |

### 5.2 Control Plane Database (SQLite — MVP)

| Table | Purpose | Key columns |
|-------|---------|-------------|
| `operators` | Dashboard user accounts | id, username, password_hash, role, created_at |
| `policy_templates` | Reusable capability configurations | id, name, constraints_json, created_by, created_at, archived |
| `audit_log` | Append-only record of all write operations | id, operator_id, action, target, result, timestamp |
| `refresh_tokens` | JWT refresh token store | id, operator_id, token_hash, expires_at, revoked |

SQLite is acceptable for single-tenant MVP. Migration path to Postgres is straightforward via sqlx.

### 5.3 Audit Log

The audit log records every write operation attempted via the CP:

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | |
| `operator_id` | UUID | Foreign key to operators |
| `action` | TEXT | e.g. `capability.grant`, `capability.revoke`, `policy.apply` |
| `target` | TEXT | Agent ID, capability ID, etc. |
| `payload_hash` | TEXT | SHA-256 of operation payload |
| `proto0_tx_id` | TEXT | PROTO-0 transaction reference (if write succeeded) |
| `result` | TEXT | `success` / `failure` |
| `error` | TEXT | Error message (if failure) |
| `timestamp` | DATETIME | UTC |

The audit log is append-only. No delete or update operations are permitted.

---

## 6. Authentication Model

### 6.1 Operator Authentication

MVP uses username + password with JWT tokens:

```text
Operator credentials → bcrypt hash comparison → access token (15 min) + refresh token (7 days)
```

| Token type | Lifetime | Storage | Rotation |
|-----------|---------|---------|----------|
| Access token | 15 minutes | Browser memory (not localStorage) | Per-expiry |
| Refresh token | 7 days | HttpOnly cookie | Per-use rotation |

### 6.2 Role Model (MVP)

Two roles for MVP:

| Role | Permissions |
|------|------------|
| `admin` | All read + all write (policy apply, capability revoke, freeze) |
| `viewer` | All read, no write |

Full RBAC (per-agent, per-policy permissions) is a post-MVP milestone.

### 6.3 PROTO-0 Key Authorization

The operator's PROTO-0 signing key is separate from dashboard authentication. The CP holds one delegated key at the enterprise level. All PROTO-0 write operations (grant, revoke, freeze) are signed with this key.

This key is not user-specific in MVP — all `admin` operators share the enterprise delegation. Per-operator key delegation is post-MVP.

### 6.4 Session Security

- CSRF protection: SameSite=Strict cookie + CSRF token header for state-changing requests
- Rate limiting: `POST /auth/login` — 5 attempts per minute per IP
- Failed auth attempts logged to audit log
- No plaintext passwords stored or logged

---

## 7. Deployment Model (MVP)

### 7.1 Target Environment

Single-tenant local deployment for MVP. No cloud infrastructure, no containers required.

```text
┌─────────────────────────────┐
│  Enterprise operator machine│
│  or internal server          │
│                             │
│  ┌────────────────────────┐ │
│  │ CP Backend (binary)    │ │
│  │ Axum HTTP on :3001     │ │
│  └────────────────────────┘ │
│                             │
│  ┌────────────────────────┐ │
│  │ CP Frontend (static)   │ │
│  │ Served by backend      │ │
│  └────────────────────────┘ │
│                             │
│  ┌────────────────────────┐ │
│  │ Protocol stores        │ │
│  │ (flat files / RocksDB) │ │
│  └────────────────────────┘ │
│                             │
│  ┌────────────────────────┐ │
│  │ CP SQLite DB           │ │
│  │ ./control_plane.db     │ │
│  └────────────────────────┘ │
└─────────────────────────────┘
```

### 7.2 Configuration

All configuration via environment variables:

| Variable | Purpose | Default |
|----------|---------|---------|
| `CP_HOST` | Bind address | `127.0.0.1` |
| `CP_PORT` | HTTP port | `3001` |
| `CP_DB_PATH` | SQLite database path | `./control_plane.db` |
| `CP_KEY_PATH` | Operator PROTO-0 signing key (PEM) | required |
| `PROTO0_STORE_PATH` | PROTO-0 store directory | required |
| `PROTO2_STORE_PATH` | PROTO-2 store directory | required |
| `PROTO4_STORE_PATH` | PROTO-4 store directory | required |
| `PROTO3_STORE_PATH` | PROTO-3 store directory | required |
| `CP_JWT_SECRET` | JWT signing secret | required |
| `CP_LOG_LEVEL` | Log verbosity | `info` |

### 7.3 Build and Run

```bash
# Build
cargo build -p control_plane --release

# Run
CP_KEY_PATH=./keys/operator.pem \
PROTO0_STORE_PATH=./data/proto0 \
PROTO2_STORE_PATH=./data/proto2 \
PROTO4_STORE_PATH=./data/proto4 \
PROTO3_STORE_PATH=./data/proto3 \
CP_JWT_SECRET=<secret> \
./target/release/control_plane
```

---

## 8. Implementation Milestones

### Milestone 1 — Read-Only Observatory

**Scope:** all 7 read-only workflows. No write operations. Zero PROTO-0 mutations.

Deliverables:
- Backend: Axum server + auth + all read-only routes
- Protocol adapters: PROTO-0 (read), PROTO-2, PROTO-4, PROTO-3
- Frontend: Agent Inventory, Agent Detail, Economic Activity, Settlement View, Reputation View, Audit Timeline (read-only)
- Security review on read paths before Milestone 2

**Exit criterion:** The Phase 2 enterprise demonstrator scenario is fully observable through the dashboard with no writes.

### Milestone 2 — Policy Management

**Scope:** write workflows (capability issuance, revocation, policy application). Gated on Milestone 1 security review.

Deliverables:
- Policy engine: template → PROTO-0 capability translation
- Backend: `/api/policies` CRUD + apply; `/api/capabilities/revoke`
- Frontend: Policy Console, Capability revocation
- Audit log integration for all writes
- End-to-end demo: create policy → apply → observe agent capability → revoke

**Exit criterion:** Phase 2 enterprise scenario (spend governance, revocation) fully replicable through dashboard.

---

## 9. Non-Goals (Frozen)

The following are explicitly excluded from the MVP and must not be added without a new approval gate:

- Multi-tenant isolation
- SSO/SAML/OIDC integration
- Per-operator PROTO-0 key delegation
- Real-time streaming (WebSocket/SSE)
- Alerting and notifications
- Dataset export
- Marketplace integration
- AETH payments
- Public agent directory
- Governance DAO
- Token incentives
- Production cloud deployment
- Container orchestration
- Horizontal scaling

---

## Freeze Statement

> This architecture document defines the implementation boundary for the Control Plane MVP. Changes to protocol stores, new economic mechanisms, AETH functionality, or marketplace features require a new design and approval gate. Implementation proceeds milestone-by-milestone: Milestone 1 (read-only) first, Milestone 2 (policy management) only after Milestone 1 security validation.
