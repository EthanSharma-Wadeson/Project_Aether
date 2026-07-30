# Apply Protocol Specification

**Document type:** Frozen protocol specification (implementation-authoritative)  
**Status:** Specification freeze — **no Apply implementation authorised**  
**Version:** `aether.cp.apply.v1`  
**Date:** 2026-07-30  
**Related:** [APPLY_STATE_MACHINE.md](APPLY_STATE_MACHINE.md), [APPLY_SEQUENCE_DIAGRAMS.md](APPLY_SEQUENCE_DIAGRAMS.md), [APPLY_FAILURE_MODEL.md](APPLY_FAILURE_MODEL.md), [CONTROL_PLANE_APPLY_SECURITY_MODEL.md](CONTROL_PLANE_APPLY_SECURITY_MODEL.md), [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md), [CONTROL_PLANE_APPLY_ARCHITECTURE_REVIEW.md](CONTROL_PLANE_APPLY_ARCHITECTURE_REVIEW.md)

---

## 1. Purpose

This document freezes the **exact Apply protocol** for the Aether Control Plane. Multiple independent implementations that conform to this specification **must** produce identical observable behaviour for all inputs.

This is a protocol specification, not an implementation plan.

**Current runtime:** `apply_enabled = false`. No protocol mutations exist.

---

## 2. Terminology

| Term | Definition |
|------|------------|
| **Template approval** | CP policy lifecycle transition to `approved`. Does **not** authorise protocol mutation. |
| **Apply approval** | Durable record binding `dry_run_id` + `execution_hash` + approver. Authorises **one** Apply attempt. |
| **Dry-run attestation** | Durable record of a successful dry-run (`executable = true`) including `dry_run_id` and `execution_hash`. |
| **Apply request** | HTTP/API request to execute a bound Apply approval. |
| **Apply operation** | One logical mutation attempt identified by `operation_id`. |
| **Replay reservation** | Insert into replay store with `status = reserved` before PROTO-0 call. |
| **PROTO-0** | Sole protocol authority for capability grant/revoke. |

---

## 3. Frozen constants

| Constant | Value | Notes |
|----------|-------|-------|
| `EXECUTION_HASH_SCHEMA` | `aether.cp.execution_hash.v1` | Immutable once published |
| `APPLY_PAYLOAD_SCHEMA` | `aether.cp.apply_payload.v1` | Apply signing payload |
| `APPLY_SIGN_BODY_SCHEMA` | `aether.cp.apply_sign_body.v1` | Bytes signed by signer |
| `APPLY_APPROVAL_TTL` | **60 minutes** | From `apply_approval.granted_at` |
| `APPLY_SIGNATURE_TTL` | **15 minutes** | From `signed_at` on Apply signature |
| `REPLAY_RESERVE_TTL` | **5 minutes** | `reserved` without `executing` → `stuck` |
| `DRY_RUN_BINDING_MAX_AGE` | **60 minutes** | Dry-run `completed_at` must be within this at Apply time |
| `APPLY_CONFIRM_REQUIRED` | `true` | Request body field `confirm` must be literal `true` |

All times are UTC. Comparisons use `now >= expires_at` as expired (fail closed).

---

## 4. Preconditions (hard gates)

An Apply request is **invalid** unless **all** are true at validation time:

| # | Gate | Check |
|---|------|-------|
| G1 | JWT | Valid access token; operator identity resolved |
| G2 | RBAC | Role ∈ `{operator, admin}` for Apply submit; `admin` for Apply approval grant |
| G3 | CSRF | Valid synchronizer token; **consumed** on Apply submit |
| G4 | Origin | Origin in `CP_ALLOWED_ORIGINS` (or localhost rules in dev) |
| G5 | Template | Policy `status = approved` and not `archived` |
| G6 | Dry-run | Durable attestation exists; `executable = true` |
| G7 | Binding | `execution_hash` recomputed from live policy matches attestation |
| G8 | Apply approval | Active approval record matches `dry_run_id` + `execution_hash` |
| G9 | Expiry | Apply approval not expired; dry-run not stale; signature not expired |
| G10 | Confirm | `confirm: true` in Apply request body |
| G11 | Replay | `reserve(operation_id)` succeeds (unique) |
| G12 | Re-simulation | Read-only PROTO-0 simulation passes immediately before PROTO-0 call |
| G13 | Signature | Apply-purpose signature verifies against current signer identity |
| G14 | Purpose | Signature payload `purpose = apply` (dry-run signatures rejected) |

Failure at any gate → **no PROTO-0 mutation** (fail closed).

---

## 5. Execution hash (`execution_hash`)

### 5.1 Generation

`execution_hash` = lowercase hex(SHA-256(canonical_json_bytes)) where `canonical_json` is:

```json
{
  "schema": "aether.cp.execution_hash.v1",
  "policy_id": "<string>",
  "policy_version": <int64>,
  "policy_content_hash": "<string>",
  "policy_type": "<string>",
  "policy_data": <object>,
  "target_agent": "<string|null>",
  "capability_intent": "<CapabilityGrant|CapabilityRevoke|PolicyApply|Unknown>",
  "execution_parameters": {
    "policy_type": "<string>",
    "name": "<string>",
    "description": "<string>",
    "status": "<string>",
    "policy_data": <object>
  }
}
```

**Field order is normative.** Implementations must emit keys in the order above. `policy_data` and nested objects use JSON serialization with stable key ordering (UTF-8 byte sort of keys at each object level) **or** the fixed `json!` insertion order as above — conforming implementations must use the **fixed insertion order** specified.

### 5.2 Immutability

For a fixed policy version and content, `execution_hash` is **immutable**. Any change to bound fields produces a different hash.

### 5.3 Verification at Apply

At Apply validation:

1. Load live policy by `policy_id`.
2. Compute `predicted_intent` = `predict_protocol_operation(policy)` (same rules as dry-run).
3. Recompute `execution_hash'` from live policy + `predicted_intent`.
4. **Reject** if `execution_hash' ≠ execution_hash` in request/approval/attestation.

### 5.4 Rejection conditions

| Condition | Result |
|-----------|--------|
| Policy version changed | Reject (`APPLY_STALE_POLICY_VERSION`) |
| Policy content hash changed | Reject (`APPLY_STALE_EXECUTION_HASH`) |
| Target agent changed | Reject |
| Capability intent changed | Reject |
| `policy_data` changed | Reject |
| Policy not `approved` | Reject |
| Policy `archived` | Reject |

---

## 6. Dry-run attestation

### 6.1 Record (`dry_run_attestations`)

| Field | Type | Required |
|-------|------|----------|
| `dry_run_id` | UUID string | PK |
| `policy_id` | string | yes |
| `policy_version` | int64 | yes |
| `execution_hash` | string (64 hex) | yes |
| `operation_id` | UUID string | yes (dry-run sign op) |
| `request_id` | string | yes |
| `operator_id` | string | yes |
| `executable` | bool | yes |
| `predicted_protocol_operation` | enum string | yes |
| `completed_at` | RFC3339 | yes |
| `signer_identity` | string | yes if executable |
| `dry_run_signature_hex` | string | yes if executable; **not valid for Apply** |

Only rows with `executable = true` may be referenced by Apply approval.

### 6.2 `dry_run_id` lifecycle

- Minted at dry-run start (UUID v4).
- Immutable once persisted.
- One attestation row per successful dry-run completion.
- Failed dry-runs may be logged but **must not** be bindable to Apply approval.

---

## 7. Apply approval binding

### 7.1 Record (`apply_approvals`)

| Field | Type | Required |
|-------|------|----------|
| `apply_approval_id` | UUID string | PK |
| `policy_id` | string | yes |
| `policy_version` | int64 | yes |
| `dry_run_id` | UUID string | FK → attestation |
| `execution_hash` | string | yes |
| `request_id` | string | yes (approval request) |
| `approver_id` | string | yes |
| `approver_role` | string | yes |
| `granted_at` | RFC3339 | yes |
| `expires_at` | RFC3339 | `granted_at + APPLY_APPROVAL_TTL` |
| `status` | enum | `active` \| `expired` \| `cancelled` \| `consumed` |
| `consumed_by_operation_id` | UUID \| null | set on successful reserve |

### 7.2 Grant rules

- Only `admin` may grant Apply approval.
- Approver **must not** be the dry-run `operator_id` (SoD).
- Approver **must not** be policy `created_by` if same as submitter SoD rules for template approve apply analogously.
- Binding fields are **immutable** after grant.
- At most **one** `active` approval per `(policy_id, dry_run_id, execution_hash)` triple.

### 7.3 Expiry

- `expires_at = granted_at + 60 minutes`.
- Expired approvals transition to `status = expired` (lazy on read or background sweep).
- Apply with expired approval → reject (`APPLY_APPROVAL_EXPIRED`).

### 7.4 Cancellation

- `admin` may cancel `active` approval → `status = cancelled`.
- Cancelled approval cannot be used for Apply.

### 7.5 Consumption

- On successful `reserve()`, approval → `status = consumed`, `consumed_by_operation_id` set.
- Consumed approval cannot be reused.

---

## 8. Replay protocol

See [APPLY_PROTOCOL_SPECIFICATION.md §8](APPLY_PROTOCOL_SPECIFICATION.md) and [APPLY_FAILURE_MODEL.md](APPLY_FAILURE_MODEL.md) for `reserve` / `execute` / `finalise` / `abort`.

**Invariant:** A signed Apply operation executes PROTO-0 **at most once**.

### 8.1 Record (`signed_operation_replay`)

| Field | Type | Notes |
|-------|------|-------|
| `operation_id` | UUID | PK |
| `apply_approval_id` | UUID | FK |
| `dry_run_id` | UUID | |
| `execution_hash` | string | |
| `policy_id` | string | |
| `policy_version` | int64 | |
| `payload_hash` | string | SHA-256 of apply payload |
| `signer_identity` | string | |
| `request_id` | string | Apply submit request |
| `operator_id` | string | Apply submitter |
| `issued_at` | RFC3339 | |
| `expires_at` | RFC3339 | signature expiry |
| `reserved_at` | RFC3339 | |
| `status` | enum | see state machine |
| `protocol_result` | enum \| null | `success` \| `rejected` |
| `failure_reason` | string \| null | |
| `finalised_at` | RFC3339 \| null | |

### 8.2 `reserve(operation_id, ...)`

**Pre:** All gates G1–G13 pass.  
**Effect:** Insert row `status = reserved` with unique `operation_id`. Set approval `consumed`.  
**Post:** Returns `reserved` or error `REPLAY_DUPLICATE` / `REPLAY_CONFLICT`.  
**Timeout:** If `status = reserved` for > `REPLAY_RESERVE_TTL` without transition to `executing`, move to `stuck`.

### 8.3 `execute(operation_id)`

**Pre:** `status = reserved`. Re-simulation passes.  
**Effect:** `status = executing`. Invoke PROTO-0 mutation adapter **once**.  
**Post:** Returns protocol outcome; caller must `finalise`.

### 8.4 `finalise(operation_id, outcome)`

**Pre:** `status = executing`.  
**Effect:** `status = executed` or `rejected`; set `protocol_result`, `failure_reason`, `finalised_at`.  
**Post:** Terminal. No further mutation.

### 8.5 `abort(operation_id, reason)`

**Pre:** `status ∈ {reserved, executing}` and authorised (system timeout or admin reconcile).  
**Effect:** `status = aborted`; no PROTO-0 call if still `reserved`; if `executing`, document outcome from reconcile policy (see failure model).  
**Post:** Terminal for automatic retry; manual reconcile may create new `operation_id`.

### 8.6 Duplicate requests

| Case | Behaviour |
|------|-----------|
| Same `operation_id` retry | If `executed` → return cached outcome (HTTP 200 with prior result). If `rejected` → return prior rejection. If `reserved`/`executing`/`stuck` → reject `REPLAY_IN_PROGRESS`. |
| New `operation_id`, same approval (already consumed) | Reject `APPLY_APPROVAL_CONSUMED` |
| New `operation_id`, new approval, same binding | Allowed if gates pass; PROTO-0 may reject duplicate grant |

---

## 9. Signatures (`purpose = apply`)

### 9.1 Apply payload (hashed, not signed directly)

Canonical JSON (`APPLY_PAYLOAD_SCHEMA`):

```json
{
  "schema": "aether.cp.apply_payload.v1",
  "purpose": "apply",
  "operation_id": "<uuid>",
  "request_id": "<string>",
  "apply_approval_id": "<uuid>",
  "dry_run_id": "<uuid>",
  "policy_id": "<string>",
  "policy_version": <int64>,
  "execution_hash": "<64 hex>",
  "capability_intent": "<enum string>",
  "target_agent": "<string|null>",
  "confirm": true,
  "issued_at": "<RFC3339>",
  "expires_at": "<RFC3339>"
}
```

`payload_hash` = lowercase hex(SHA-256(UTF-8 bytes of canonical JSON)).

`expires_at` = `issued_at + APPLY_SIGNATURE_TTL` (15 minutes).

### 9.2 Signed body (`APPLY_SIGN_BODY_SCHEMA`)

Bytes signed by Ed25519:

```json
{
  "schema": "aether.cp.apply_sign_body.v1",
  "purpose": "apply",
  "operation_id": "<uuid>",
  "request_id": "<string>",
  "payload_hash": "<64 hex>",
  "signer_identity": "<string>",
  "signed_at": "<RFC3339>"
}
```

**Field order normative** as listed.

`signature` = Ed25519_sign(private_key, UTF-8(canonical JSON of sign body)).

### 9.3 Verification order

1. Parse Apply request; extract `operation_id`, signature, sign body fields.
2. Reject if `purpose ≠ apply`.
3. Reject if `now >= expires_at` (payload).
4. Recompute `payload_hash` from live canonical payload; reject on mismatch.
5. Verify Ed25519 signature over sign body with **current** signer public key for `signer_identity`.
6. Reject if `signer_identity ≠` configured active signer (key rotation: old signatures invalid).
7. Reject if dry-run signature presented instead of Apply signature.
8. Proceed to replay `reserve`.

### 9.4 Dry-run signature rejection

Any signature whose payload or sign body has `purpose = dry_run` **or** action suffix `.dry_run` **must be rejected** at Apply with `APPLY_INVALID_SIGNATURE_PURPOSE`.

### 9.5 Key rotation

- Signer identity change or key rotation invalidates all pending Apply signatures.
- Operators must re-sign with new key after rotation.
- `signer_identity` recorded in replay row and audit.

---

## 10. Mandatory re-simulation

Immediately before `execute()` (after `reserve`, before PROTO-0):

1. Run same read-only simulation as dry-run (`simulate_policy_apply` or revoke/grant variant).
2. **Reject** if `executable = false` → `finalise(rejected)`, reason from simulation.
3. Count-based protocol fingerprint check optional; **must not** be used as binding.

Protocol state may change between dry-run and Apply; re-simulation is the authoritative pre-mutation check.

---

## 11. PROTO-0 adapter

### 11.1 Allowed mutations (MVP)

| `capability_intent` | PROTO-0 API | CP role |
|---------------------|-------------|---------|
| `CapabilityGrant` | `grant_capability` | Build request from `policy_data`; submit |
| `CapabilityRevoke` | `revoke_capability` | Build request from `policy_data`; submit |

### 11.2 Authority

- PROTO-0 validates and accepts or rejects.
- CP **never** writes protocol stores directly.
- CP records `protocol_result` from PROTO-0 outcome only.

### 11.3 Out of scope (forbidden)

Escrow, settlement, reputation, channel mutation via Apply.

---

## 12. Apply API (normative behaviour, not implementation)

### 12.1 Grant Apply approval

`POST /api/policies/{policy_id}/apply-approvals`

- Auth: JWT + CSRF (consume) + Origin + `admin`
- Body: `{ "dry_run_id": "<uuid>", "execution_hash": "<hex>", "confirm": true }`
- Validates attestation + hash + SoD
- Creates `apply_approvals` row `status = active`

### 12.2 Submit Apply

`POST /api/policies/{policy_id}/apply`

- Auth: JWT + CSRF (consume) + Origin + `operator|admin`
- Body:

```json
{
  "confirm": true,
  "apply_approval_id": "<uuid>",
  "operation_id": "<uuid>",
  "dry_run_id": "<uuid>",
  "execution_hash": "<hex>",
  "issued_at": "<RFC3339>",
  "expires_at": "<RFC3339>",
  "signature": "<hex>",
  "sign_body": { ... APPLY_SIGN_BODY_SCHEMA ... }
}
```

- Executes gate chain → reserve → re-simulate → execute → finalise → audit

**Note:** Routes are specified for behavioural conformance; **must not be enabled** until implementation authorisation.

### 12.3 Cancel Apply approval

`DELETE /api/policies/{policy_id}/apply-approvals/{apply_approval_id}`

- Auth: `admin`
- Sets `status = cancelled` if `active`

---

## 13. Audit requirements

Every Apply attempt (success or failure) **must** append immutable records containing:

| Field | Source |
|-------|--------|
| `request_id` | Apply submit or approval request |
| `operation_id` | Client-supplied UUID |
| `dry_run_id` | Binding |
| `execution_hash` | Binding |
| `apply_approval_id` | Binding |
| `signer_identity` | Sign body |
| `policy_id` | Policy |
| `policy_version` | Policy |
| `operator_id` | JWT sub (submit) or approver (approval) |
| `timestamps` | `requested_at`, `reserved_at`, `finalised_at` |
| `protocol_result` | `success` \| `rejected` \| `not_applicable` |
| `replay_status` | Terminal replay state |
| `failure_reason` | If failed |

**Stores (append-only):**

- `audit_log` — event stream (`APPLY_*` actions)
- `mutation_audit` — one row per Apply attempt
- `signer_audit` — sign/verify outcomes

See [APPLY_FAILURE_MODEL.md](APPLY_FAILURE_MODEL.md) for per-stage audit codes.

---

## 14. Security invariants (normative)

| ID | Invariant |
|----|-----------|
| INV-1 | Apply never bypasses PROTO-0 |
| INV-2 | Signer never authorises; only attests |
| INV-3 | Replay executes PROTO-0 at most once per `operation_id` |
| INV-4 | `execution_hash` immutable for fixed policy binding |
| INV-5 | Dry-run signature invalid for Apply |
| INV-6 | No mutation without `confirm: true` |
| INV-7 | No mutation without CSRF (consumed) |
| INV-8 | No mutation without valid JWT |
| INV-9 | No mutation without RBAC |
| INV-10 | No mutation without active Apply approval |
| INV-11 | No mutation without replay reservation |
| INV-12 | No mutation after approval expiry |
| INV-13 | No mutation after signature expiry |
| INV-14 | No mutation without mandatory re-simulation pass |
| INV-15 | Template approval alone does not permit Apply |
| INV-16 | `aether-core` authority rules unchanged by CP |
| INV-17 | Approval consumption is single-use |
| INV-18 | Failed paths leave protocol state unchanged |

---

## 15. Error codes (stable)

| Code | Meaning |
|------|---------|
| `APPLY_DISABLED` | `apply_enabled = false` |
| `APPLY_CONFIRM_REQUIRED` | `confirm ≠ true` |
| `APPLY_APPROVAL_MISSING` | No active approval |
| `APPLY_APPROVAL_EXPIRED` | Approval TTL exceeded |
| `APPLY_APPROVAL_CANCELLED` | Approval cancelled |
| `APPLY_APPROVAL_CONSUMED` | Approval already used |
| `APPLY_STALE_EXECUTION_HASH` | Hash mismatch |
| `APPLY_STALE_POLICY_VERSION` | Version mismatch |
| `APPLY_DRY_RUN_NOT_EXECUTABLE` | Attestation not executable |
| `APPLY_DRY_RUN_STALE` | Dry-run older than `DRY_RUN_BINDING_MAX_AGE` |
| `APPLY_INVALID_SIGNATURE` | Crypto verify failed |
| `APPLY_INVALID_SIGNATURE_PURPOSE` | Dry-run sig or wrong purpose |
| `APPLY_SIGNATURE_EXPIRED` | Signature TTL exceeded |
| `APPLY_SIMULATION_FAILED` | Re-simulation blocked |
| `APPLY_PROTO_REJECTED` | PROTO-0 rejected |
| `REPLAY_DUPLICATE` | `operation_id` exists |
| `REPLAY_IN_PROGRESS` | Op not terminal |
| `REPLAY_STUCK` | Reserved too long; manual reconcile |

---

## 16. Success criteria (specification completeness)

Implementation is mechanically derivable when:

1. All state transitions match [APPLY_STATE_MACHINE.md](APPLY_STATE_MACHINE.md).
2. All message flows match [APPLY_SEQUENCE_DIAGRAMS.md](APPLY_SEQUENCE_DIAGRAMS.md).
3. All failure paths match [APPLY_FAILURE_MODEL.md](APPLY_FAILURE_MODEL.md).
4. Every protocol mutation has **one** authority source: **PROTO-0 accept**.
5. Every failure path terminates deterministically with audit + no mutation (except successful PROTO-0 accept).

---

## 17. Freeze statement

> **Apply Protocol Specification v1 is frozen.** This document is authoritative for Apply implementation. `apply_enabled` remains `false`. No protocol mutations are authorised until separate implementation authorisation and conformance verification.
