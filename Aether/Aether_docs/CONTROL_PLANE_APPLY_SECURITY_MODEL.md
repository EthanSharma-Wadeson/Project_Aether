# Control Plane Apply Security Model

**Document type:** Frozen architecture specification (design only)  
**Status:** Frozen for Apply design approval — **Apply is not implemented**  
**Date:** 2026-07-29  
**Related:** [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md), [CONTROL_PLANE_PHASE4B_SECURITY_GATE.md](CONTROL_PLANE_PHASE4B_SECURITY_GATE.md), [CONTROL_PLANE_AUTHORITY_MODEL.md](CONTROL_PLANE_AUTHORITY_MODEL.md), [CONTROL_PLANE_WRITE_SECURITY.md](CONTROL_PLANE_WRITE_SECURITY.md)

---

## Purpose

Define the future mutation pipeline and mandatory invariants before any Apply / PROTO-0 write path is authorised.

This document does **not** authorise implementation of Apply routes, protocol mutations, capability grant/revoke, or production signer usage for live execution.

---

## Frozen Mutation Pipeline

```text
Policy Version
      |
      ↓
Dry Run
      |
      ↓
Execution Hash
      |
      ↓
Human Approval
      |
      ↓
Signed Operation
      |
      ↓
Protocol Adapter
      |
      ↓
PROTO-0 Authority Check
      |
      ↓
Mutation
```

### Stage contracts

| Stage | Produces | Consumes | Must fail closed if |
|-------|----------|----------|---------------------|
| Policy Version | `policy_id`, `policy_version`, content `hash` | Approved CP template | Status ≠ approved |
| Dry Run | `dry_run_id`, validation report, dry-run signature (purpose=`dry_run`) | Approved policy | Simulation blocks; signer missing |
| Execution Hash | `execution_hash` | Policy contents + intent + params | Hash cannot be recomputed / mismatch |
| Human Approval | Approval record bound to hash + dry-run | Executable dry-run | Binding fields diverge |
| Signed Operation | Apply-purpose signature (`purpose=apply`) | Fresh approval + binding | Replay / expiry / missing signer |
| Protocol Adapter | Adapter request | Verified signed op | Adapter unavailable |
| PROTO-0 Authority Check | Accept / reject | Adapter request | Protocol rejects |
| Mutation | Protocol state change | PROTO-0 accept only | Any prior stage invalid |

---

## Authority Invariants

### The Control Plane:

- requests actions
- authenticates operators
- signs authorised requests (when Apply is gated open)

### The Control Plane does NOT:

- grant authority itself
- bypass PROTO-0
- rewrite protocol state
- treat approval as activation
- treat dry-run signatures as Apply authorisation

**PROTO-0 remains the sole protocol authority.** The Control Plane is a governance client.

---

## Approval Binding

Human approval for a future Apply **must** bind to all of:

| Field | Role |
|-------|------|
| `policy_id` | Which template |
| `policy_version` | Exact version approved for execution |
| `dry_run_id` | Specific successful dry-run attestation |
| `execution_hash` | Content-addressed intended mutation |
| `request_id` | Correlation / audit |
| Approver identity | Admin (or designated approver role) |

### Invalidation rules

- Any change to policy contents or version → **approval invalid**
- Any change that alters `execution_hash` → **approval invalid**
- Dry-run for a different policy version → **approval invalid**
- Expired approval → **rejected**
- Approval without matching durable dry-run record → **rejected**

**A changed policy invalidates approval.**

---

## Signature Requirements (Future Apply Operations)

Dry-run signatures authenticate the dry-run pipeline only. Apply must mint a **new** signed operation with purpose `apply`.

Required fields on signed Apply operations:

| Field | Requirement |
|-------|-------------|
| `operation_id` | Unique per operation (UUID) |
| `timestamp` | Issuance time (UTC) |
| `expiry` | Hard TTL; reject after expiry |
| `payload_hash` | Hash of canonical Apply payload |
| `signer_identity` | Non-secret identity label |
| Replay protection reference | `operation_id` (and optional nonce) recorded before mutation |

See [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md).

**Invariant:** Dry-run `signature_hex` **must not** be accepted as an Apply signature.

---

## Execution Hash (Content-Addressed Binding)

Count-based protocol fingerprints are **not** Apply bindings. They remain a dry-run non-mutation sanity check only.

`execution_hash` = SHA-256 over a canonical binding that includes:

- policy contents (`policy_data` + durable content `hash`)
- `policy_version`
- target agent
- capability intent (predicted PROTO-0 operation kind)
- execution parameters (derived from policy type/name/data)

The hash must uniquely identify the intended mutation. Apply recomputes and compares; mismatch → reject.

Implementation (dry-run already emits this field): `execution::hash::compute_execution_hash` / `execution_hash_for_policy`.

---

## Concurrency Model

### Concurrent approvals

**Example:** Two admins approve different versions (or approve after divergent edits).

**Expected:** Only the latest **valid** approved version may proceed to Apply, and only with a dry-run + approval bound to that version’s `execution_hash`. Stale version approvals are rejected at Apply time via version + hash checks.

### Stale dry runs

**Example:** Dry run created → policy changes → Apply attempted.

**Expected:** Reject. `policy_version` / content hash / `execution_hash` no longer match the live template.

### Duplicate requests

**Example:** Client retries the same Apply `operation_id` or identical signed payload.

**Expected:** Idempotent rejection (or idempotent success acknowledgement if already executed once — never a second mutation). See replay model: **a signed operation can execute at most once.**

### Cancelled requests

Cancelled Apply requests must not remain executable. Cancellation marks the approval / operation non-executable; subsequent Apply with the same binding is rejected.

---

## Audit Reconstructability

Future Apply must reconstruct this timeline from durable stores:

```text
request_id
operation_id
dry_run_id
policy_version
execution_hash
approver
signer_event
protocol_result
```

### Recommendation — unified audit timeline

1. **Keep** `audit_log`, `mutation_audit`, and `signer_audit` as append-only sources of truth.
2. **Require** every Apply attempt to write a `mutation_audit` row with:
   - `request_id`, `operation_id` (in action/metadata), `signer_identity`, `protocol_result`, `payload_hash` (= or derived from `execution_hash`), failure reason.
3. **Correlate** via `request_id` as the primary join key; secondary keys: `dry_run_id`, `operation_id`, `policy_id`+`policy_version`.
4. **Before Apply coding:** also record dry-run completion into `mutation_audit` with `protocol_result=not_applicable` and metadata containing `dry_run_id` + `execution_hash` (closes gate M7).
5. **Optional view:** SQL/API “governance timeline” that unions the three tables ordered by timestamp for a single `request_id` / `policy_id`.

---

## Explicit Non-Goals (This Freeze)

- No Apply HTTP routes
- No PROTO-0 mutation calls from Control Plane
- No capability grant/revoke/activate from Control Plane
- No aether-core changes
- No production live execution

---

## Freeze Statement

> Apply architecture is frozen as specified in this document and [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md). Implementation of Apply requires a **separate** design approval after security gate conditions are closed. Until then the Control Plane remains dry-run only and non-mutating.
