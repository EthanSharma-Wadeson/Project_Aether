# Apply Implementation Blueprint

**Document type:** Implementation blueprint (design freeze — authoritative for coding)  
**Status:** Blueprint frozen — **Apply implementation not authorised**  
**Version:** `aether.cp.apply.blueprint.v1`  
**Date:** 2026-07-30  
**Normative inputs:** [APPLY_PROTOCOL_SPECIFICATION.md](APPLY_PROTOCOL_SPECIFICATION.md), [APPLY_STATE_MACHINE.md](APPLY_STATE_MACHINE.md), [APPLY_SEQUENCE_DIAGRAMS.md](APPLY_SEQUENCE_DIAGRAMS.md), [APPLY_FAILURE_MODEL.md](APPLY_FAILURE_MODEL.md), [CONTROL_PLANE_APPLY_SECURITY_MODEL.md](CONTROL_PLANE_APPLY_SECURITY_MODEL.md), [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md), [CONTROL_PLANE_PHASE4B_SECURITY_GATE.md](CONTROL_PLANE_PHASE4B_SECURITY_GATE.md), [CONTROL_PLANE_APPLY_ARCHITECTURE_REVIEW.md](CONTROL_PLANE_APPLY_ARCHITECTURE_REVIEW.md), [CONTROL_PLANE_MVP_ARCHITECTURE.md](../Project_Phases/phase_3/CONTROL_PLANE_MVP_ARCHITECTURE.md)

**Current runtime:** `apply_enabled = false`. No Rust Apply code. No protocol mutations.

---

## 0. Blueprint purpose

This document maps every frozen Apply specification onto **concrete source files, HTTP APIs, SQLite schemas, execution stages, and tests** so that multiple engineers can implement Apply with identical behaviour.

Nothing in this blueprint contradicts the normative protocol documents. Where this blueprint adds detail (file paths, SQL DDL), it is **binding for implementation**.

---

## 1. Module layout

### 1.1 Crate tree (new + extended)

```text
Aether/control_plane/src/
├── apply/                          # NEW — Apply domain (all mutation orchestration)
│   ├── mod.rs                      # Public exports; apply_enabled gate
│   ├── config.rs                   # Frozen TTL constants; feature flag
│   ├── errors.rs                   # ApplyError + HTTP mapping
│   ├── models.rs                   # Request/response DTOs; row types
│   ├── validation.rs               # Gates G1–G14 orchestration
│   ├── approval.rs                 # Apply approval grant/cancel/expiry
│   ├── attestation.rs              # dry_run_attestations persistence
│   ├── replay.rs                   # reserve / execute / finalise / abort
│   ├── execution.rs                # Apply pipeline coordinator
│   ├── prepare.rs                  # Canonical payload + sign-body builder
│   ├── signature.rs                # purpose=apply verify; rejects dry-run sigs
│   ├── reconcile.rs                # stuck recovery; admin actions
│   ├── audit.rs                    # Apply-specific audit helpers
│   └── routes.rs                   # HTTP handlers (gated by apply_enabled)
│
├── db/
│   ├── apply.rs                    # NEW — SQL for approvals, attestations, replay
│   └── mod.rs                      # EXTEND — migrations for Apply tables
│
├── protocol/
│   ├── proto0.rs                   # EXTEND — read adapters (existing)
│   ├── proto0_write.rs             # NEW — sole PROTO-0 mutation boundary
│   └── state.rs                    # EXTEND — Arc<RwLock<ProtocolState>> handle
│
├── execution/
│   ├── executor.rs                 # EXTEND — persist dry_run_attestations on complete
│   ├── hash.rs                     # EXISTING — execution_hash (no logic change)
│   └── simulate.rs                 # EXISTING — re-simulation (shared with Apply)
│
├── routes/
│   ├── mod.rs                      # EXTEND — mount apply routes when enabled
│   └── policies.rs                 # EXTEND — wire attestation on dry-run
│
├── audit/
│   └── mutation.rs                 # EXTEND — Apply metadata fields
│
├── config/
│   └── mod.rs                      # EXTEND — CP_APPLY_ENABLED env
│
└── lib.rs                          # EXTEND — protocol RwLock; apply module
```

### 1.2 Module responsibilities

| Module | Responsibility | Must NOT |
|--------|----------------|----------|
| `apply/mod.rs` | Feature gate `apply_enabled`; re-exports | Call PROTO-0 directly |
| `apply/config.rs` | `APPLY_APPROVAL_TTL`, etc.; read `CP_APPLY_ENABLED` | Override frozen TTLs silently |
| `apply/errors.rs` | Stable error codes → HTTP status | Leak signer/key material |
| `apply/models.rs` | Serde types for API + DB rows | Contain business logic |
| `apply/validation.rs` | Gates G1–G14 in normative order | Skip gates when `apply_enabled` |
| `apply/approval.rs` | CRUD `apply_approvals`; SoD; TTL; consume on reserve | Grant without admin |
| `apply/attestation.rs` | Persist/load `dry_run_attestations` | Accept non-executable dry-runs for bind |
| `apply/replay.rs` | Replay store lifecycle; idempotent reads | Auto-retry PROTO-0 |
| `apply/execution.rs` | End-to-end pipeline: validate→reserve→sim→proto→finalise | Bypass replay |
| `apply/prepare.rs` | Build `APPLY_PAYLOAD_SCHEMA` + `payload_hash` | Sign (signer does) |
| `apply/signature.rs` | Verify Apply signatures; reject `purpose≠apply` | Accept dry-run signatures |
| `apply/reconcile.rs` | Admin stuck recovery per failure model | Blind PROTO-0 retry |
| `apply/audit.rs` | Normalised Apply audit records | Mutate audit rows |
| `apply/routes.rs` | Axum handlers; CSRF/Origin guards | Register routes if `apply_enabled=false` |
| `db/apply.rs` | SQLx queries; transactions for reserve+consume | PROTO-0 calls |
| `protocol/proto0_write.rs` | **Only** file calling `aether_core` mutation APIs | Escrow/settlement/reputation writes |
| `execution/executor.rs` | On `DRY_RUN_COMPLETED` + executable: insert attestation | Change hash algorithm |

### 1.3 `AppState` changes (blueprint only)

```text
AppState {
    ...
    protocol: Arc<RwLock<ProtocolState>>,   // was Arc<ProtocolState>
    apply_enabled: bool,                     // from config; default false
}
```

All PROTO-0 writes acquire `protocol.write()` inside `proto0_write` only, after replay `reserve`.

---

## 2. API design

**Base path:** `/api` (JWT middleware on all routes below).

**Global gate:** If `apply_enabled = false`, all Apply endpoints return `403` with `APPLY_DISABLED`.

### 2.1 Dry-run (existing — extend behaviour)

#### `POST /api/policies/:policy_id/dry-run`

| | |
|---|---|
| **Auth** | JWT + CSRF (not consumed) + Origin |
| **RBAC** | `operator` \| `admin` |
| **Request** | `{}` or `{ "confirm": true }` (optional; dry-run does not require confirm) |
| **Response 200** | `{ "protocol_mutated": false, "apply_enabled": <bool>, "report": { "dry_run_id", "execution_hash", "executable", ... } }` |
| **Blueprint change** | On `executable=true`, **persist** `dry_run_attestations` row |
| **Audit** | `DRY_RUN_*` (existing) + `mutation_audit` row (`protocol_result=not_applicable`, M7) |
| **Failures** | Existing execution errors; `403` RBAC; `403` CSRF; `404` policy |

---

### 2.2 Apply approval

#### `POST /api/policies/:policy_id/apply-approvals`

Grant Apply approval (bind dry-run to approver).

| | |
|---|---|
| **Auth** | JWT + CSRF (**consumed**) + Origin |
| **RBAC** | `admin` only |
| **Request** | `{ "dry_run_id": "<uuid>", "execution_hash": "<64hex>", "confirm": true }` |
| **Validation** | G5–G7; attestation exists; hash match; SoD (approver ≠ dry-run operator); dry-run age ≤ 60m |
| **Response 201** | `{ "apply_approval_id": "<uuid>", "policy_id", "policy_version", "dry_run_id", "execution_hash", "granted_at", "expires_at", "status": "active" }` |
| **Failures** | See error catalogue §8 |

#### `GET /api/policies/:policy_id/apply-approvals`

List approvals for policy (admin).

| | |
|---|---|
| **Auth** | JWT |
| **RBAC** | `admin` |
| **Query** | `?status=active` (optional) |
| **Response 200** | `{ "items": [ ApplyApprovalView, ... ] }` |

#### `DELETE /api/policies/:policy_id/apply-approvals/:apply_approval_id`

Cancel active approval.

| | |
|---|---|
| **Auth** | JWT + CSRF (consumed) + Origin |
| **RBAC** | `admin` |
| **Request** | `{ "confirm": true }` |
| **Response 204** | — |
| **Failures** | `404` not found; `409` not active |

---

### 2.3 Apply prepare (signing helper)

#### `POST /api/policies/:policy_id/apply/prepare`

Build canonical signing payload **without** reserving or mutating. Operator signs client-side or server calls signer in execute.

| | |
|---|---|
| **Auth** | JWT + CSRF (not consumed) + Origin |
| **RBAC** | `operator` \| `admin` |
| **Request** | `{ "apply_approval_id": "<uuid>", "operation_id": "<uuid>", "confirm": true }` |
| **Validation** | G5–G10 (except G11 replay, G12–G14) |
| **Response 200** | `{ "operation_id", "apply_payload": { ... APPLY_PAYLOAD_SCHEMA ... }, "payload_hash", "sign_body_template": { "schema", "purpose": "apply", ... }, "expires_at": "issued_at+15m" }` |
| **Note** | Does not call Signer; returns bytes to sign. Server-side signing happens in `/apply` if client omits signature (optional MVP: server signs after validation). |
| **Failures** | `APPLY_APPROVAL_EXPIRED`, `EXECUTION_HASH_MISMATCH`, etc. |

---

### 2.4 Apply execute

#### `POST /api/policies/:policy_id/apply`

**Normative mutation entry point** (frozen spec §12.2).

| | |
|---|---|
| **Auth** | JWT + CSRF (**consumed**) + Origin |
| **RBAC** | `operator` \| `admin` |
| **Request** | `{ "confirm": true, "apply_approval_id": "<uuid>", "operation_id": "<uuid>", "dry_run_id": "<uuid>", "execution_hash": "<hex>", "issued_at": "<RFC3339>", "expires_at": "<RFC3339>", "sign_body": { ... APPLY_SIGN_BODY_SCHEMA ... }, "signature": "<hex>" }` |
| **Pipeline** | validation G1–G14 → `replay.reserve` → re-simulate → `replay.execute` (PROTO-0) → `replay.finalise` → audit |
| **Response 200** | `{ "operation_id", "replay_status": "executed", "protocol_result": "success", "protocol_detail": { ... } }` |
| **Response 409** | Cached replay / in progress / stale hash |
| **Response 422** | `PROTO_REJECTED`, `SIMULATION_FAILED` |
| **Response 503** | `REPLAY_STUCK` / uncertain |

---

### 2.5 Apply operation queries

#### `GET /api/apply/operations/:operation_id`

| | |
|---|---|
| **Auth** | JWT |
| **RBAC** | `operator` \| `admin` (own ops); `admin` (all) |
| **Response 200** | `ApplyOperationView` (replay row + summary) |

#### `GET /api/apply/operations/:operation_id/reconcile-status`

| | |
|---|---|
| **Auth** | JWT |
| **RBAC** | `admin` |
| **Response 200** | `{ "operation_id", "replay_status", "protocol_observation": { ... read-only ... }, "recommended_action": "finalise_executed" \| "finalise_rejected" \| "abort" \| "pending" }` |

#### `POST /api/apply/operations/:operation_id/reconcile`

| | |
|---|---|
| **Auth** | JWT + CSRF (consumed) + Origin |
| **RBAC** | `admin` |
| **Request** | `{ "action": "finalise_executed" \| "finalise_rejected" \| "abort", "confirm": true, "reason": "<string>", "protocol_evidence": { ... } }` |
| **Response 200** | Updated `ApplyOperationView` |
| **Side effect** | `reconcile_jobs` row; replay terminal transition; audit |

#### `GET /api/apply/pending`

| | |
|---|---|
| **Auth** | JWT |
| **RBAC** | `admin` |
| **Query** | `?status=stuck,reserved,executing` |
| **Response 200** | `{ "items": [ ApplyOperationView, ... ] }` |

---

### 2.6 Error response shape (all Apply endpoints)

```json
{
  "error": "<human message>",
  "code": "<STABLE_CODE>",
  "retryable": false,
  "operation_id": "<uuid|null>",
  "request_id": "<string>"
}
```

---

## 3. Persistence

### 3.1 `dry_run_attestations`

| Column | Type | Constraints |
|--------|------|-------------|
| `dry_run_id` | TEXT | PRIMARY KEY |
| `policy_id` | TEXT | NOT NULL, FK → `policy_templates(id)` |
| `policy_version` | INTEGER | NOT NULL |
| `execution_hash` | TEXT | NOT NULL, CHECK(length=64) |
| `operation_id` | TEXT | NOT NULL (dry-run sign op) |
| `request_id` | TEXT | NOT NULL |
| `operator_id` | TEXT | NOT NULL, FK → `operators(id)` |
| `executable` | INTEGER | NOT NULL (0/1) |
| `predicted_protocol_operation` | TEXT | NOT NULL |
| `completed_at` | TEXT | NOT NULL (RFC3339) |
| `signer_identity` | TEXT | NULL |
| `dry_run_signature_hex` | TEXT | NULL |
| `report_json` | TEXT | NOT NULL (full DryRunReport snapshot) |

**Indexes:**

- `idx_dry_run_policy` ON `(policy_id, completed_at DESC)`
- `idx_dry_run_hash` ON `(policy_id, execution_hash)`

**Uniqueness:** PK `dry_run_id`.

**Retention:** Append-only; archive rows older than 90 days to cold storage (operational policy; no DELETE in app).

---

### 3.2 `apply_approvals`

| Column | Type | Constraints |
|--------|------|-------------|
| `apply_approval_id` | TEXT | PRIMARY KEY |
| `policy_id` | TEXT | NOT NULL, FK → `policy_templates(id)` |
| `policy_version` | INTEGER | NOT NULL |
| `dry_run_id` | TEXT | NOT NULL, FK → `dry_run_attestations(dry_run_id)` |
| `execution_hash` | TEXT | NOT NULL |
| `request_id` | TEXT | NOT NULL |
| `approver_id` | TEXT | NOT NULL, FK → `operators(id)` |
| `approver_role` | TEXT | NOT NULL |
| `granted_at` | TEXT | NOT NULL |
| `expires_at` | TEXT | NOT NULL |
| `status` | TEXT | NOT NULL CHECK IN (`active`,`expired`,`cancelled`,`consumed`) |
| `consumed_by_operation_id` | TEXT | NULL, FK → `signed_operation_replay(operation_id)` |
| `cancelled_at` | TEXT | NULL |
| `cancelled_by` | TEXT | NULL |

**Indexes:**

- `idx_apply_approval_policy_status` ON `(policy_id, status)`
- `idx_apply_approval_active` ON `(policy_id, dry_run_id, execution_hash)` WHERE `status='active'`

**Uniqueness:** At most one `active` per `(policy_id, dry_run_id, execution_hash)` — enforce in transaction.

**Retention:** Append-only status transitions; no DELETE.

---

### 3.3 `signed_operation_replay` (replay store)

| Column | Type | Constraints |
|--------|------|-------------|
| `operation_id` | TEXT | PRIMARY KEY |
| `apply_approval_id` | TEXT | NOT NULL, FK → `apply_approvals` |
| `dry_run_id` | TEXT | NOT NULL |
| `execution_hash` | TEXT | NOT NULL |
| `policy_id` | TEXT | NOT NULL |
| `policy_version` | INTEGER | NOT NULL |
| `payload_hash` | TEXT | NOT NULL |
| `signer_identity` | TEXT | NOT NULL |
| `request_id` | TEXT | NOT NULL |
| `operator_id` | TEXT | NOT NULL |
| `issued_at` | TEXT | NOT NULL |
| `expires_at` | TEXT | NOT NULL |
| `reserved_at` | TEXT | NOT NULL |
| `status` | TEXT | NOT NULL CHECK IN (`reserved`,`executing`,`executed`,`rejected`,`stuck`,`aborted`) |
| `protocol_result` | TEXT | NULL CHECK IN (`success`,`rejected`) |
| `failure_code` | TEXT | NULL |
| `failure_reason` | TEXT | NULL |
| `protocol_detail_json` | TEXT | NULL |
| `finalised_at` | TEXT | NULL |

**Indexes:**

- `idx_replay_status_reserved_at` ON `(status, reserved_at)` — stuck sweeper
- `idx_replay_policy` ON `(policy_id, finalised_at DESC)`
- `idx_replay_approval` ON `(apply_approval_id)`

**Uniqueness:** PK `operation_id`.

**Retention:** Never DELETE; terminal rows permanent.

---

### 3.4 `mutation_audit` (extend existing)

Add nullable columns via migration (or JSON `metadata` column if preferred — blueprint mandates explicit columns):

| New column | Type |
|------------|------|
| `operation_id` | TEXT NULL |
| `dry_run_id` | TEXT NULL |
| `execution_hash` | TEXT NULL |
| `apply_approval_id` | TEXT NULL |
| `replay_status` | TEXT NULL |
| `failure_code` | TEXT NULL |

Existing columns unchanged. Every Apply attempt: exactly **one** row with `action = 'apply.execute'`.

Dry-run (M7): `action = 'policy.dry_run'`, `protocol_result = not_applicable`, populate `dry_run_id` + `execution_hash`.

---

### 3.5 `reconcile_jobs`

| Column | Type | Constraints |
|--------|------|-------------|
| `reconcile_job_id` | TEXT | PRIMARY KEY |
| `operation_id` | TEXT | NOT NULL, FK → `signed_operation_replay` |
| `admin_id` | TEXT | NOT NULL |
| `request_id` | TEXT | NOT NULL |
| `action` | TEXT | NOT NULL CHECK IN (`finalise_executed`,`finalise_rejected`,`abort`) |
| `reason` | TEXT | NOT NULL |
| `protocol_evidence_json` | TEXT | NOT NULL |
| `prior_replay_status` | TEXT | NOT NULL |
| `new_replay_status` | TEXT | NOT NULL |
| `created_at` | TEXT | NOT NULL |

**Indexes:** `idx_reconcile_operation` ON `(operation_id, created_at DESC)`.

**Retention:** Append-only audit of manual interventions.

---

### 3.6 Transaction boundaries

| Operation | Tables in one transaction |
|-----------|---------------------------|
| `reserve` | `signed_operation_replay` INSERT + `apply_approvals` UPDATE consumed |
| `finalise` | `signed_operation_replay` UPDATE + `mutation_audit` INSERT |
| `reconcile` | `signed_operation_replay` UPDATE + `reconcile_jobs` INSERT + audit |

Use SQLite `BEGIN IMMEDIATE` for `reserve` to prevent double-consume.

---

## 4. Execution pipeline

Normative runtime flow inside `apply/execution.rs::execute_apply()`:

| Step | Function | Inputs | Outputs | Fail codes | Audit event |
|------|----------|--------|---------|------------|-------------|
| 1 | `guard_apply_enabled` | config | — | `APPLY_DISABLED` | — |
| 2 | `validate_jwt` | headers | `AuthContext` | `AUTH_INVALID` | `APPLY_FAILED` |
| 3 | `validate_rbac` | role | — | `RBAC_DENIED` | `APPLY_FAILED` |
| 4 | `validate_csrf` | token, consume=true | — | `CSRF_INVALID` | `APPLY_FAILED` |
| 5 | `validate_origin` | Origin header | — | `ORIGIN_INVALID` | `APPLY_FAILED` |
| 6 | `validate_confirm` | body.confirm | — | `APPLY_CONFIRM_REQUIRED` | `APPLY_FAILED` |
| 7 | `load_policy` | policy_id | `PolicyTemplate` | `POLICY_NOT_FOUND` | `APPLY_FAILED` |
| 8 | `validate_policy_status` | status=approved | — | `POLICY_NOT_APPROVED` | `APPLY_FAILED` |
| 9 | `load_attestation` | dry_run_id | row | `DRY_RUN_NOT_FOUND` | `APPLY_FAILED` |
| 10 | `validate_dry_run_executable` | executable=true | — | `DRY_RUN_NOT_EXECUTABLE` | `APPLY_FAILED` |
| 11 | `validate_dry_run_age` | completed_at | — | `DRY_RUN_STALE` | `APPLY_FAILED` |
| 12 | `verify_execution_hash` | policy, hash | — | `EXECUTION_HASH_MISMATCH` | `APPLY_FAILED` |
| 13 | `load_apply_approval` | approval_id | row | `APPLY_APPROVAL_MISSING` | `APPLY_FAILED` |
| 14 | `validate_approval_active` | status, expiry | — | `APPLY_APPROVAL_EXPIRED` | `APPLY_FAILED` |
| 15 | `validate_approval_binding` | ids/hash match | — | `APPLY_APPROVAL_BINDING_INVALID` | `APPLY_FAILED` |
| 16 | `verify_apply_signature` | sign_body, sig | — | `SIGNATURE_INVALID`, `SIGNATURE_EXPIRED`, `SIGNATURE_PURPOSE_INVALID` | `SIGN_VERIFY_FAILED` |
| 17 | `audit_requested` | context | — | — | `APPLY_REQUESTED` |
| 18 | `replay.reserve` | operation_id, ... | row | `REPLAY_DUPLICATE`, `REPLAY_IN_PROGRESS` | `APPLY_RESERVED` |
| 19 | `resimulate` | protocol, policy | outcome | `SIMULATION_FAILED` → finalise rejected | `APPLY_SIMULATION_REJECTED` |
| 20 | `replay.mark_executing` | operation_id | — | — | — |
| 21 | `proto0_write.execute` | intent, policy | ProtoOutcome | `PROTO_REJECTED`, `PROTO_TIMEOUT` | — |
| 22 | `replay.finalise` | outcome | terminal | — | `APPLY_PROTO_SUCCESS` / `APPLY_PROTO_REJECTED` |
| 23 | `audit_completed` | full context | — | — | `APPLY_COMPLETED` / `APPLY_FAILED` |
| 24 | `return_response` | — | HTTP body | — | — |

**Ordering invariant (INV-3):** Steps 18 → 19 → 20 → 21 → 22 are strict. No step 21 before 18.

---

## 5. Replay store

### 5.1 Schema

See §3.3 `signed_operation_replay`.

### 5.2 State transitions

```text
(none) --reserve()--> reserved
reserved --execute()--> executing
executing --finalise(success)--> executed
executing --finalise(fail)--> rejected
reserved --timeout 5m--> stuck
executing --timeout/uncertain--> stuck
reserved|executing|stuck --abort(admin)--> aborted
```

### 5.3 Locking strategy

- **DB:** `INSERT` with PK `operation_id` provides uniqueness.
- **Reserve transaction:** `BEGIN IMMEDIATE`; update approval `active→consumed` only if still `active`.
- **In-process:** Optional `Mutex` on `operation_id` for duplicate concurrent HTTP — DB is source of truth.
- **Protocol lock:** `RwLock::write` on `ProtocolState` held only during step 21.

### 5.4 Crash recovery

| Crash point | Row state | Recovery |
|-------------|-----------|----------|
| After reserve, before executing | `reserved` | Timer → `stuck`; admin reconcile |
| During PROTO-0 | `executing` | Admin reads protocol; reconcile |
| After PROTO-0, before finalise | `executing` | Reconcile finalise from PROTO-0 read |
| Before audit | terminal in replay | Backfill audit idempotent on `operation_id` |

**Background task:** `apply/reconcile.rs::sweep_stuck_reserved()` every 60s: `reserved` where `now - reserved_at > 5m` → `stuck`.

### 5.5 Cleanup

- No DELETE on replay rows.
- Optional archival job (ops): export terminal rows > 1 year.

### 5.6 Duplicate handling

Implement `replay::reserve` idempotent read path per APPLY_PROTOCOL_SPECIFICATION §8.6.

### 5.7 Manual reconciliation

`reconcile::apply_action(admin, operation_id, action, evidence)` — writes `reconcile_jobs`, transitions replay, appends audit. Requires read-only PROTO-0 evidence JSON in request.

---

## 6. Protocol adapter contract

**File:** `protocol/proto0_write.rs`  
**Rule:** This is the **only** module that mutates `ProtocolState` / calls `aether_core` write APIs. **Do not modify `aether-core`.**

### 6.1 Trait (blueprint)

```text
trait Proto0WriteAdapter {
    fn grant_capability(ctx: &ApplyContext, policy: &PolicyTemplate) -> Proto0Outcome;
    fn revoke_capability(ctx: &ApplyContext, policy: &PolicyTemplate) -> Proto0Outcome;
    fn freeze_identity(ctx: &ApplyContext, policy: &PolicyTemplate) -> Proto0Outcome;
    fn unfreeze_identity(ctx: &ApplyContext, policy: &PolicyTemplate) -> Proto0Outcome;
}
```

`ApplyContext` carries: `operation_id`, `request_id`, `execution_hash`, `operator_id`, `protocol: &mut ProtocolState`.

### 6.2 `grant_capability`

| | |
|---|---|
| **Inputs** | `policy.target_agent_id`, `policy.policy_data` → build `CapabilityGrant` per [APPLY_PROTO_ADAPTER_SPEC] (see §12 OQ-1) |
| **Core call** | `aether_core::capability::grant::grant_capability(&grant, &mut registry, &mut caps, expected_issuer_pk)` |
| **Success output** | `Proto0Outcome { result: Success, capability_id: hex, detail: {...} }` |
| **Error mapping** | Core `Error` → `PROTO_REJECTED` + stable subcode in `protocol_detail_json` |
| **Audit metadata** | `capability_id`, `target_agent`, `policy_id`, `operation_id` |

### 6.3 `revoke_capability`

| | |
|---|---|
| **Inputs** | `policy.policy_data.capability_id` (32-byte hex), `revoked_at` timestamp |
| **Core call** | `CapabilityStore::revoke(cap_id, revoked_at)` after existence check |
| **Success** | `Proto0Outcome { result: Success }` |
| **Errors** | Not found → `PROTO_REJECTED`; already revoked → `PROTO_REJECTED` |

### 6.4 `freeze_identity`

| | |
|---|---|
| **Inputs** | `policy.target_agent_id` |
| **Core call** | `IdentityRegistry::freeze(agent_id)` |
| **RBAC** | Apply path requires `admin` + `confirm` (policy_type contains `freeze`) |
| **Success** | `Proto0Outcome { result: Success }` |

### 6.5 `unfreeze_identity`

| | |
|---|---|
| **Status** | **MVP: NOT AVAILABLE** — `aether-core` `IdentityRegistry` has `freeze()` but no `unfreeze()` |
| **Behaviour** | Return `PROTO_REJECTED` / `ADAPTER_NOT_IMPLEMENTED` until core exposes API |
| **Blueprint** | Trait method exists; implementation returns error without mutation |

### 6.6 `Proto0Outcome`

```text
Proto0Outcome {
    result: Success | Rejected | Timeout,
    failure_code: Option<String>,
    failure_reason: Option<String>,
    protocol_detail_json: serde_json::Value,
}
```

### 6.7 Dispatch

`proto0_write::execute(ctx, policy)` matches `predict_protocol_operation(policy)`:

| Intent | Method |
|--------|--------|
| `CapabilityGrant` | `grant_capability` |
| `CapabilityRevoke` | `revoke_capability` |
| `PolicyApply` | `grant_capability` (default) |
| `freeze` policy_type | `freeze_identity` |
| Other | `Rejected` without mutation |

---

## 7. Audit contract

### 7.1 Immutable schema (`audit_log.metadata` JSON)

Every Apply-related event **must** include:

```json
{
  "schema": "aether.cp.apply_audit.v1",
  "request_id": "string",
  "operation_id": "uuid|null",
  "dry_run_id": "uuid|null",
  "execution_hash": "hex|null",
  "apply_approval_id": "uuid|null",
  "policy_id": "string",
  "policy_version": 0,
  "actor_id": "string",
  "actor_role": "string",
  "approver_id": "string|null",
  "signer_identity": "string|null",
  "event": "APPLY_REQUESTED|...",
  "failure_code": "string|null",
  "failure_reason": "string|null",
  "replay_status": "string|null",
  "protocol_result": "success|rejected|not_applicable|null",
  "timestamp": "RFC3339"
}
```

### 7.2 Event catalogue

| Event | When | protocol_result |
|-------|------|-----------------|
| `DRY_RUN_REQUESTED` | Dry-run start | not_applicable |
| `DRY_RUN_COMPLETED` | Dry-run success | not_applicable |
| `DRY_RUN_FAILED` | Dry-run fail | not_applicable |
| `APPLY_APPROVAL_GRANTED` | Approval created | not_applicable |
| `APPLY_APPROVAL_CANCELLED` | Approval cancelled | not_applicable |
| `APPLY_REQUESTED` | Execute received | not_applicable |
| `APPLY_RESERVED` | Replay reserved | not_applicable |
| `APPLY_SIMULATION_REJECTED` | Re-sim fail | rejected |
| `APPLY_PROTO_SUCCESS` | PROTO-0 accept | success |
| `APPLY_PROTO_REJECTED` | PROTO-0 reject | rejected |
| `APPLY_COMPLETED` | Terminal success | success |
| `APPLY_FAILED` | Any gate failure | rejected / not_applicable |
| `APPLY_REPLAY_STUCK` | Timer | not_applicable |
| `APPLY_REPLAY_ABORTED` | Admin abort | not_applicable |
| `APPLY_RECONCILED` | Admin reconcile | per action |
| `SIGN_VERIFY_FAILED` | Bad signature | not_applicable |

### 7.3 Cross-store writes per Apply execute

1. `audit_log` — ≥2 rows (`APPLY_REQUESTED`, terminal event)
2. `mutation_audit` — exactly 1 row
3. `signer_audit` — 1 row on verify attempt
4. `signed_operation_replay` — 1 row lifecycle

---

## 8. Error catalogue

| Code | Meaning | HTTP | Retryable | Audit code |
|------|---------|------|-----------|------------|
| `APPLY_DISABLED` | Feature flag off | 403 | No | `APPLY_FAILED` |
| `AUTH_INVALID` | JWT bad/missing | 401 | No* | `APPLY_FAILED` |
| `RBAC_DENIED` | Wrong role | 403 | No | `APPLY_FAILED` |
| `CSRF_INVALID` | CSRF fail | 403 | No* | `APPLY_FAILED` |
| `ORIGIN_INVALID` | Origin fail | 403 | No | `APPLY_FAILED` |
| `APPLY_CONFIRM_REQUIRED` | confirm≠true | 400 | No | `APPLY_FAILED` |
| `POLICY_NOT_FOUND` | Unknown policy | 404 | No | `APPLY_FAILED` |
| `POLICY_NOT_APPROVED` | Template not approved | 409 | No | `APPLY_FAILED` |
| `POLICY_ARCHIVED` | Archived | 409 | No | `APPLY_FAILED` |
| `POLICY_STALE` | Version changed | 409 | No | `APPLY_STALE_POLICY_VERSION` |
| `DRY_RUN_NOT_FOUND` | No attestation | 404 | No | `APPLY_FAILED` |
| `DRY_RUN_NOT_EXECUTABLE` | executable=false | 409 | No | `APPLY_FAILED` |
| `DRY_RUN_STALE` | >60m old | 409 | No | `APPLY_DRY_RUN_STALE` |
| `EXECUTION_HASH_MISMATCH` | Hash recompute ≠ submitted | 409 | No | `APPLY_STALE_EXECUTION_HASH` |
| `APPLY_APPROVAL_MISSING` | No approval row | 403 | No | `APPLY_APPROVAL_MISSING` |
| `APPLY_APPROVAL_EXPIRED` | TTL exceeded | 403 | No | `APPLY_APPROVAL_EXPIRED` |
| `APPLY_APPROVAL_CANCELLED` | Cancelled | 403 | No | `APPLY_APPROVAL_CANCELLED` |
| `APPLY_APPROVAL_CONSUMED` | Already used | 409 | No | `APPLY_APPROVAL_CONSUMED` |
| `APPLY_APPROVAL_BINDING_INVALID` | Field mismatch | 409 | No | `APPLY_FAILED` |
| `APPLY_SOD_VIOLATION` | Self-approve apply | 403 | No | `APPLY_FAILED` |
| `SIGNATURE_INVALID` | Crypto fail | 403 | No | `SIGN_VERIFY_FAILED` |
| `SIGNATURE_EXPIRED` | expires_at past | 403 | No | `APPLY_SIGNATURE_EXPIRED` |
| `SIGNATURE_PURPOSE_INVALID` | dry-run sig | 403 | No | `APPLY_INVALID_SIGNATURE_PURPOSE` |
| `SIGNER_UNAVAILABLE` | Signer down | 503 | Yes | `APPLY_FAILED` |
| `REPLAY_DUPLICATE` | op exists terminal | 200/409 | No | `APPLY_REPLAY_CACHED` |
| `REPLAY_IN_PROGRESS` | reserved/executing | 409 | Yes (poll) | `APPLY_REPLAY_REJECTED` |
| `REPLAY_STUCK` | Needs reconcile | 503 | No | `APPLY_REPLAY_STUCK` |
| `SIMULATION_FAILED` | Re-sim blocked | 422 | No | `APPLY_SIMULATION_REJECTED` |
| `PROTO_REJECTED` | PROTO-0 no | 422 | No | `APPLY_PROTO_REJECTED` |
| `PROTO_TIMEOUT` | Network/timeout | 503 | No | `APPLY_REPLAY_STUCK` |
| `ADAPTER_NOT_IMPLEMENTED` | e.g. unfreeze | 501 | No | `APPLY_FAILED` |

\*Retryable with fresh credentials/token.

---

## 9. Testing blueprint

### 9.1 Replay

| Test ID | Description | Maps to |
|---------|-------------|---------|
| T-R01 | Same `operation_id` twice → no double PROTO-0 | S1, INV-3 |
| T-R02 | `reserve` + sim fail → rejected, no PROTO | INV-18 |
| T-R03 | `reserved` 5m → `stuck` | T-RC1 |
| T-R04 | Admin abort from `stuck` | reconcile |
| T-R05 | Cached success on duplicate | APPLY_PROTOCOL §8.6 |
| T-R06 | Concurrent reserve same op_id | C3 |

### 9.2 Security

| Test ID | Description | Maps to |
|---------|-------------|---------|
| T-S01 | Dry-run sig at Apply → reject | S6, INV-5 |
| T-S02 | Expired approval | S2, INV-12 |
| T-S03 | Expired signature | S7, INV-13 |
| T-S04 | Modified policy → hash mismatch | S3, INV-4 |
| T-S05 | Wrong dry_run_id | S4 |
| T-S06 | confirm=false | INV-6 |
| T-S07 | CSRF missing on execute | INV-7 |
| T-S08 | Viewer cannot execute | INV-9 |
| T-S09 | apply_enabled=false → 403 | — |

### 9.3 Concurrency

| Test ID | Description | Maps to |
|---------|-------------|---------|
| T-C01 | Two approvals, stale rejected at Apply | C1 |
| T-C02 | Dry-run then policy edit → fail | C2 |
| T-C03 | Duplicate operation_id race | C3 |
| T-C04 | Cancelled approval | C4 |
| T-C05 | Update vs submit (policy) | C5 |
| T-C06 | Dual approve | C6 |

### 9.4 Recovery

| Test ID | Description | Maps to |
|---------|-------------|---------|
| T-RC01 | Crash after reserve | T-RC1 |
| T-RC02 | Protocol changed → re-sim fail | T-RC2 |
| T-RC03 | Audit backfill idempotent | T-RC3 |
| T-RC04 | Reconcile finalise_executed | failure model §5 |
| T-RC05 | Reconcile abort | failure model §4 |

### 9.5 Protocol

| Test ID | Description | Maps to |
|---------|-------------|---------|
| T-P01 | Grant success visible in observatory | M2 exit |
| T-P02 | PROTO reject → no state change | INV-18 |
| T-P03 | Revoke path | MVP scope |
| T-P04 | Freeze path admin-only | MVP scope |
| T-P05 | `aether-core` tests still green | regression |

### 9.6 Audit

| Test ID | Description | Maps to |
|---------|-------------|---------|
| T-A01 | Every failure has mutation_audit row | M7 |
| T-A02 | Dry-run writes mutation_audit | M7 |
| T-A03 | Timeline reconstruct by request_id | T-RC3 |
| T-A04 | signer_audit on verify | INV-2 |

### 9.7 Performance (smoke)

| Test ID | Description |
|---------|-------------|
| T-PF01 | 10 sequential Applies — no leak |
| T-PF02 | Reserve contention 50 concurrent — exactly one succeeds per approval |

---

## 10. Implementation order

Safest coding sequence:

| Phase | Work | Gate |
|-------|------|------|
| **0** | `CP_APPLY_ENABLED` config (default `false`); routes not mounted | No behaviour change |
| **1** | DB migrations: all Apply tables; extend `mutation_audit` | Migration tests |
| **2** | `dry_run_attestations` + executor hook on dry-run | H3, M7 partial |
| **3** | `apply/approval.rs` + approval routes (no execute) | Approval tests |
| **4** | `apply/replay.rs` — reserve/finalise/abort (no PROTO) | T-R01–R06 |
| **5** | `apply/signature.rs` + `prepare.rs` | T-S01, S6 |
| **6** | `apply/validation.rs` — full gate chain | T-S02–S08 |
| **7** | `apply/audit.rs` — normalised events | T-A01–A04 |
| **8** | `protocol/proto0_write.rs` — adapter (behind flag, unit tests with test world) | T-P01–P04 |
| **9** | `protocol/state.rs` — `RwLock` migration | Regression M1 tests |
| **10** | `apply/execution.rs` — wire pipeline | Integration tests |
| **11** | `apply/reconcile.rs` + admin routes + sweeper | T-RC01–RC05 |
| **12** | `apply/routes.rs` — execute route (still `apply_enabled=false` in CI) | Full test suite |
| **13** | Integration tests S*/C* | Test plan exit |
| **14** | Security review + gate re-run | A10 checklist |
| **15** | `apply_enabled=true` only in explicit authorisation env | Production gate |

**Do not enable step 15 until §11 checklist complete.**

---

## 11. Final readiness checklist (`apply_enabled = true`)

Every item must be objectively verifiable (test or scripted check).

### Replay & execution

- [ ] `signed_operation_replay` enforces unique `operation_id`
- [ ] `reserve → execute → finalise` ordering enforced in code review + test T-R01
- [ ] No PROTO-0 call without prior `reserve`
- [ ] No auto-retry PROTO-0 for same `operation_id`
- [ ] Stuck sweeper transitions `reserved` → `stuck` after 5m (T-R03)
- [ ] Admin reconcile API functional (T-RC04, T-RC05)

### Binding & approval

- [ ] `execution_hash` matches APPLY_PROTOCOL_SPECIFICATION §5 byte-for-byte
- [ ] Approval TTL 60m enforced (T-S02)
- [ ] Signature TTL 15m enforced (T-S03)
- [ ] Dry-run max age 60m enforced (T-S05)
- [ ] Approval consumed exactly once on reserve
- [ ] Template approve ≠ Apply approval (integration test)
- [ ] SoD: approver ≠ dry-run operator

### Security

- [ ] `apply_enabled` defaults `false`
- [ ] `confirm: true` required (T-S06)
- [ ] CSRF consumed on execute + approval grant (T-S07)
- [ ] JWT + RBAC on all Apply routes (T-S08)
- [ ] Dry-run signature rejected at Apply (T-S01)
- [ ] `production_config_check` passes in prod profile
- [ ] No `aether-core` modifications in Apply PR

### Audit

- [ ] M7: dry-run in `mutation_audit` (T-A02)
- [ ] Every Apply attempt in `mutation_audit` (T-A01)
- [ ] Audit metadata schema `aether.cp.apply_audit.v1` (T-A03)
- [ ] Failed paths audited (T-A01)

### Protocol

- [ ] Only `proto0_write.rs` mutates protocol
- [ ] Re-simulation mandatory before PROTO-0 (T-RC02)
- [ ] PROTO reject does not corrupt state (T-P02)
- [ ] `cargo test -p aether-core` green
- [ ] `cargo test -p aether-control-plane` green
- [ ] `cargo clippy -p aether-control-plane -- -D warnings` green

### Governance

- [ ] Separate **implementation authorisation** recorded
- [ ] CONTROL_PLANE_PHASE4B_SECURITY_GATE re-run → pass
- [ ] No critical/open High findings on Apply PR
- [ ] APPLY_IMPLEMENTATION_BLUEPRINT conformance review signed off

---

## 12. Open questions (implementation unknowns)

| ID | Question | Resolution owner | Blocks coding? |
|----|----------|------------------|----------------|
| OQ-1 | `policy_data` → `CapabilityGrant` field mapping | **Resolved** — [APPLY_PROTO_ADAPTER_SPEC.md](APPLY_PROTO_ADAPTER_SPEC.md) | No |
| OQ-2 | Server-side vs client-side Apply signing for MVP | Product: recommend server signs in `execute` after verify approval (signer never exposed) | No — both fit blueprint |
| OQ-3 | `unfreeze_identity` | Deferred — adapter returns `ADAPTER_NOT_IMPLEMENTED` | No for MVP grant/revoke |
| OQ-4 | `ProtocolState` persistence across CP restart | MVP: in-memory demo world; prod needs store reload strategy (out of Apply v1) | No for first integration |
| OQ-5 | Nested JSON key order in `policy_data` for hash | **Resolved** — `canonicalize_json` §6 of adapter spec | No |

---

## 13. Freeze statement

> **Apply Implementation Blueprint v1 is frozen.** This document maps normative Apply protocol specifications to concrete modules, APIs, persistence, and tests. **Apply implementation may begin only after explicit implementation authorisation** and must not enable `apply_enabled = true` until §11 checklist is complete. No protocol mutations until that gate opens.

---

## Appendix A — File creation checklist (implementation PR)

| Action | Path |
|--------|------|
| CREATE | `src/apply/mod.rs` … `routes.rs` (12 files) |
| CREATE | `src/db/apply.rs` |
| CREATE | `src/protocol/proto0_write.rs` |
| CREATE | `tests/apply_replay_tests.rs` |
| CREATE | `tests/apply_security_tests.rs` |
| CREATE | `tests/apply_integration_tests.rs` |
| EXTEND | `src/db/mod.rs`, `src/routes/mod.rs`, `src/lib.rs`, `src/execution/executor.rs` |
| EXTEND | `src/config/mod.rs` (`CP_APPLY_ENABLED`) |
| DO NOT MODIFY | `Aether/core/**` |
