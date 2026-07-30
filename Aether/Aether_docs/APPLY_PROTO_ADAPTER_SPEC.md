# Apply PROTO-0 Adapter Specification

**Document type:** Frozen adapter & canonicalization specification (final pre-implementation design)  
**Status:** Design freeze — **no implementation authorised**  
**Version:** `aether.cp.proto_adapter.v1`  
**Date:** 2026-07-30  
**Resolves:** Blueprint OQ-1, OQ-5  
**Normative inputs:** [APPLY_PROTOCOL_SPECIFICATION.md](APPLY_PROTOCOL_SPECIFICATION.md), [APPLY_IMPLEMENTATION_BLUEPRINT.md](APPLY_IMPLEMENTATION_BLUEPRINT.md), [APPLY_STATE_MACHINE.md](APPLY_STATE_MACHINE.md), [APPLY_FAILURE_MODEL.md](APPLY_FAILURE_MODEL.md)

**Constraints:** Does not modify `aether-core`. All mutations go through existing `aether_core` public APIs only.

---

## 1. Purpose

This document resolves the **final protocol ambiguities** blocking Apply implementation:

1. Exact mapping from CP policy templates → PROTO-0 operations  
2. Deterministic `execution_hash` across platforms  
3. Frozen `proto0_write` adapter contract  
4. PROTO-0 error → Apply error code mapping  
5. Cross-platform hash test vectors  

After this document, **no design decisions remain** for MVP Apply (grant, revoke, freeze).

---

## 2. Supported policy types

### 2.1 Intent resolution (normative)

`capability_intent` is resolved by `predict_protocol_operation(policy)` using **exactly** these rules (first match wins):

| Order | Condition | `capability_intent` |
|-------|-----------|---------------------|
| 1 | `policy_type` lowercase contains `"revoke"` | `CapabilityRevoke` |
| 2 | `policy_data.action == "revoke"` AND `policy_data.capability_id` present | `CapabilityRevoke` |
| 3 | `policy_type` lowercase contains `"freeze"` | `FreezeIdentity` |
| 4 | `policy_type` lowercase contains `"grant"` OR `"capability"` | `CapabilityGrant` |
| 5 | `policy_type == "capability_constraints"` | `CapabilityGrant` |
| 6 | `policy_type == "policy_apply"` | `CapabilityGrant` |
| 7 | (default) | `PolicyApply` → **treated as `CapabilityGrant` at adapter** |

**Unsupported intents at MVP:** `Unknown` → reject before PROTO-0 with `APPLY_UNSUPPORTED_OPERATION`.

### 2.2 Template → PROTO-0 operation map

| CP `policy_type` (examples) | `capability_intent` | Adapter operation | PROTO-0 API |
|-----------------------------|---------------------|-------------------|-------------|
| `capability_grant`, `capability_constraints`, `policy_apply` | `CapabilityGrant` | `GrantCapability` | `grant_capability()` |
| `capability_revoke` | `CapabilityRevoke` | `RevokeCapability` | `CapabilityStore::revoke()` |
| `identity_freeze` | `FreezeIdentity` | `FreezeIdentityRequest` | `IdentityRegistry::freeze()` |

---

## 3. Policy mapping — GrantCapability

### 3.1 CP template → `CapabilityV0` + `CapabilityGrant`

```text
PolicyTemplate (approved)
        ↓ map_grant_policy()
CapabilityV0 (unsigned semantic object)
        ↓ CapabilityGrant::sign(enterprise_signing_key)
CapabilityGrant (SignedMessage)
        ↓ grant_capability()
CapabilityStore insert (PROTO-0)
```

### 3.2 Required fields

| Field | Location | Type | Validation |
|-------|----------|------|------------|
| `target_agent_id` | Policy column | string | Non-empty; must exist in registry; `status == active` |
| `policy_data.actions` | JSON | string[] | Length ≥ 1; each action 1–128 chars, `[a-z0-9._-]+` |
| `policy_type` | Policy column | string | Resolves to `CapabilityGrant` |

### 3.3 Optional fields (`policy_data`)

| Field | Type | Default | Validation |
|-------|------|---------|--------------|
| `max_spend` | u64 | `null` (unlimited) | If present, > 0 |
| `asset` | string | `"AETHER_TEST"` in demo; **required in prod** if `max_spend` set | 1–32 chars |
| `counterparties` | string[] | `null` | Each valid agent_id |
| `rate_limit.max_ops` | u64 | `100` | > 0 |
| `rate_limit.window_seconds` | u64 | `60` | > 0 |
| `valid_after` | u64 | `0` | Unix seconds |
| `valid_before` | u64 | `null` | If set, > `valid_after` |
| `delegation_depth` | u32 | `0` | 0 = direct grant; >0 requires `parent_capability_id` |
| `parent_capability_id` | string | `null` | 64 hex chars (32 bytes); parent must exist, not revoked |
| `issuer_agent_id` | string | Enterprise root agent | Must exist; must have root authority for actions |

### 3.4 `CapabilityV0` construction rules

```text
protocol_version  = 0
schema_version    = 0
issuer            = issuer_agent_id (resolved)
subject           = SubjectRef::AgentId(target_agent_id)
actions           = policy_data.actions (order preserved)
constraints       = mapped from policy_data optional fields
delegation_depth  = policy_data.delegation_depth ?? 0
parent_capability_id = decode_hex(policy_data.parent_capability_id) if delegation_depth > 0 else None
```

**Signing:** `CapabilityGrant::sign(&enterprise_signing_key, &capability)` using server-held enterprise key (same key material as PROTO-0 issuer operational key).

**Expected issuer public key:** Resolved from `issuer_agent_id` entry in `IdentityRegistry`.

### 3.5 Grant validation (pre-PROTO, shared by simulate + adapter)

| Check | Failure code |
|-------|--------------|
| Missing `target_agent_id` | `APPLY_INVALID_POLICY` |
| Empty `actions` | `APPLY_INVALID_POLICY` |
| Target not found | `APPLY_AGENT_NOT_FOUND` |
| Target frozen | `APPLY_IDENTITY_FROZEN` |
| Target revoked | `APPLY_IDENTITY_REVOKED` |
| Target not active | `APPLY_AGENT_NOT_ACTIVE` |
| Invalid `parent_capability_id` | `APPLY_INVALID_POLICY` |
| Parent revoked | `APPLY_CAPABILITY_REVOKED` |

### 3.6 `execution_hash` inputs (grant)

Included in hash document (see §6):

- `capability_intent` = `"CapabilityGrant"`
- `policy_data` = **canonicalized** (§5)
- `target_agent` = `target_agent_id`
- `execution_parameters.policy_data` = same canonical `policy_data`

---

## 4. Policy mapping — RevokeCapability

### 4.1 CP template → revocation

```text
PolicyTemplate
        ↓ map_revoke_policy()
RevokeCapabilityRequest { capability_id, revoked_at }
        ↓ CapabilityStore::revoke()
revoked map entry (PROTO-0)
```

### 4.2 Required fields

| Field | Location | Type | Validation |
|-------|----------|------|------------|
| `policy_data.capability_id` | JSON | string | 64 hex chars → 32 bytes |
| `policy_type` | Policy column | string | Contains `revoke` OR intent rule #2 |

### 4.3 Optional fields

| Field | Type | Default |
|-------|------|---------|
| `target_agent_id` | string | Used for simulation only; **not required** if `capability_id` set |
| `revoked_at` | u64 | Current Unix time at `execute()` |

### 4.4 Validation

| Check | Failure code |
|-------|--------------|
| Missing/invalid `capability_id` hex | `APPLY_INVALID_POLICY` |
| Capability not in store | `APPLY_CAPABILITY_NOT_FOUND` |
| Already revoked | `APPLY_CAPABILITY_REVOKED` |

### 4.5 Idempotency note

PROTO-0 `revoke()` is idempotent on store (sets revoked map). Apply replay store still ensures **at most one Apply operation** per `operation_id`; duplicate grant of same capability is separate concern.

### 4.6 `execution_hash` inputs (revoke)

- `capability_intent` = `"CapabilityRevoke"`
- `policy_data` must include canonical `capability_id`

---

## 5. Policy mapping — FreezeIdentity

### 5.1 CP template → freeze

```text
PolicyTemplate
        ↓ map_freeze_policy()
FreezeIdentityRequest { agent_id }
        ↓ IdentityRegistry::freeze()
AgentStatus::Frozen
```

### 5.2 Required fields

| Field | Location | Type | Validation |
|-------|----------|------|------------|
| `target_agent_id` | Policy column | string | Non-empty |
| `policy_type` | Policy column | string | Contains `freeze` |

### 5.3 Optional fields

None for MVP.

### 5.4 RBAC

Apply execute requires **`admin`** role when `capability_intent == FreezeIdentity` (stricter than grant/revoke).

### 5.5 Validation

| Check | Failure code |
|-------|--------------|
| Agent not found | `APPLY_AGENT_NOT_FOUND` |
| Already frozen | `APPLY_IDENTITY_FROZEN` (no-op reject) |
| Already revoked | `APPLY_IDENTITY_REVOKED` |

### 5.6 Unfreeze

**Not in MVP.** `unfreeze_identity()` adapter method returns `ADAPTER_NOT_IMPLEMENTED` without mutation (`aether-core` has no `unfreeze` API).

### 5.7 `execution_hash` inputs (freeze)

- `capability_intent` = `"FreezeIdentity"` — **Note:** enum string in hash is `FreezeIdentity` (not in `ProtocolOperationKind` today; extend enum at implementation with same string)

**Implementation note:** Add `ProtocolOperationKind::FreezeIdentity` with `as_str() -> "FreezeIdentity"` before Apply coding. Simulation and hash must use it for freeze policies.

---

## 6. Canonical serialization (`execution_hash`)

### 6.1 Goal

Identical `execution_hash` on Windows, macOS, and Linux for identical policy binding inputs.

### 6.2 Top-level hash document (fixed field order)

The object hashed **must** be built with this **exact key order**:

1. `schema`
2. `policy_id`
3. `policy_version`
4. `policy_content_hash`
5. `policy_type`
6. `policy_data`
7. `target_agent`
8. `capability_intent`
9. `execution_parameters`

`execution_parameters` object key order:

1. `policy_type`
2. `name`
3. `description`
4. `status`
5. `policy_data`

### 6.3 `canonicalize_json(value)` (nested objects)

Apply to every `policy_data` value **before** insertion into hash document:

| Rule | Specification |
|------|---------------|
| **Objects** | Keys sorted lexicographically by UTF-8 byte order; recurse into values |
| **Arrays** | Preserve element order; canonicalize each element |
| **Strings** | UTF-8; no normalization (NFC not applied) |
| **Numbers** | Integers only in `policy_data`; reject floats at policy validation. Serialize without leading zeros, no exponent. |
| **Booleans** | `true` / `false` lowercase |
| **Null** | JSON `null` |
| **Whitespace** | None in output |
| **Unicode** | `ensure_ascii=false`; encode UTF-8 in JSON string escapes only when required by JSON |

**Serialization:** `serde_json::to_vec` on a `serde_json::Value` built programmatically with sorted `Map` keys, **or** equivalent. **Do not** use `HashMap` iteration order.

### 6.4 Hash algorithm

```text
execution_hash = lowercase_hex(SHA-256(UTF-8(canonical_json_bytes)))
```

Where `canonical_json_bytes` = compact JSON of §6.2 document with §6.3 applied to both `policy_data` fields.

### 6.5 `policy_content_hash` (separate from execution_hash)

Computed at policy save time by `compute_policy_hash()`:

```json
{
  "name": "...",
  "description": "...",
  "target_agent_id": "...|null",
  "policy_type": "...",
  "policy_data": <canonicalized>,
  "version": <int>
}
```

Key order fixed as above. **Same `canonicalize_json` for `policy_data`.**

### 6.6 Relationship to current dry-run code

Implementation **must update** `execution/hash.rs` to call `canonicalize_json()` on `policy_data` before hashing. Current insertion-order-only behaviour is **superseded** by this spec for cross-platform conformance.

---

## 7. Hash conformance test vectors

Implementations **must** pass all vectors (platform-independent).

### TV-1 — Grant (`capability_constraints`)

**Input:**

```json
{
  "schema": "aether.cp.execution_hash.v1",
  "policy_id": "pol-tv1",
  "policy_version": 1,
  "policy_content_hash": "abc123",
  "policy_type": "capability_constraints",
  "policy_data": {"actions": ["transfer"], "max_spend": 100},
  "target_agent": "agent-demo-1",
  "capability_intent": "CapabilityGrant",
  "execution_parameters": {
    "policy_type": "capability_constraints",
    "name": "grant-demo",
    "description": "test vector 1",
    "status": "approved",
    "policy_data": {"actions": ["transfer"], "max_spend": 100}
  }
}
```

**Expected `execution_hash`:**

```text
400857ca8a241ca6d884abb573d76dbe51a7406697ea3434f1ff83e8b95281ed
```

### TV-2 — Revoke (nested key sort)

`policy_data` input unordered: `{"capability_id": "<64 zeros>", "zeta": 1, "alpha": "β"}`  
Canonical order: `alpha`, `capability_id`, `zeta`.

**Expected `execution_hash`:**

```text
ecf5f5aa7c2fadb287c09c175345ded8cd293689f2b40b9d41d9b742b6747730
```

(Full document in implementation test file `tests/apply_hash_vectors.rs`.)

### TV-3 — Null target

Same as TV-1 but `"target_agent": null`, `policy_id": "pol-tv3"`.

**Expected `execution_hash`:**

```text
c851c28ada15bc3a580bac558fbfbc9a44905cee3009b8ade1ff513b1fbeac4b
```

### TV-4 — Freeze intent

`policy_type": "identity_freeze"`, `capability_intent": "FreezeIdentity"`, `policy_data": {}`, `target_agent": "agent-demo-1"`.

**Expected `execution_hash`:**

```text
9d92be2f6206d8609b8f23b36316bbf0c135fc11816f636f60da212a9c31782b
```

---

## 8. Adapter contract (`protocol/proto0_write.rs`)

### 8.1 Types

```text
ApplyContext {
    operation_id: String,
    request_id: String,
    execution_hash: String,
    operator_id: String,
    capability_intent: ProtocolOperationKind,
}

Proto0WriteRequest {
    ctx: ApplyContext,
    policy: PolicyTemplate,
    enterprise_issuer: EnterpriseIssuerRef,  // signing key + agent_id
    revoked_at: u64,                          // revoke only
}

Proto0Outcome {
    result: Proto0Result,           // Success | Rejected | Timeout
    apply_error_code: Option<String>,
    core_error: Option<String>,       // Debug; not exposed to client in prod
    protocol_detail: serde_json::Value,
}

Proto0ResultDetail (on Success):
  Grant  → { "capability_id": "<64 hex>", "issuer": "<agent_id>", "subject": "<agent_id>" }
  Revoke → { "capability_id": "<64 hex>", "revoked_at": <u64> }
  Freeze → { "agent_id": "<string>", "status": "frozen" }
```

### 8.2 Entry point

```text
proto0_write::execute(
    protocol: &mut ProtocolState,
    request: Proto0WriteRequest,
) -> Proto0Outcome
```

**Dispatch:**

| `capability_intent` | Function |
|---------------------|----------|
| `CapabilityGrant`, `PolicyApply` | `grant_capability()` |
| `CapabilityRevoke` | `revoke_capability()` |
| `FreezeIdentity` | `freeze_identity()` |
| Other | `Proto0Outcome::Rejected(APPLY_UNSUPPORTED_OPERATION)` |

### 8.3 Inputs

- `policy` — approved template; already validated by Apply pipeline  
- `protocol` — exclusive `&mut` for duration of call  
- `enterprise_issuer` — configured enterprise agent + `SigningKey` (never exposed to handlers)

### 8.4 Outputs

- `Success` — mutation applied; detail populated  
- `Rejected` — no mutation; mapped error code  
- `Timeout` — uncertain; caller marks replay `stuck`

### 8.5 Idempotency

- **Grant:** NOT idempotent — second grant creates new capability_id (different semantic object unless identical bytes). Replay store prevents duplicate Apply ops.  
- **Revoke:** Setting revoked twice is safe at store level; Apply still one-shot per `operation_id`.  
- **Freeze:** Second freeze on frozen agent → `APPLY_IDENTITY_FROZEN` without state change.

### 8.6 Audit metadata (required in `protocol_detail_json`)

```json
{
  "adapter_version": "aether.cp.proto_adapter.v1",
  "operation": "GrantCapability|RevokeCapability|FreezeIdentityRequest",
  "operation_id": "<uuid>",
  "execution_hash": "<hex>",
  "policy_id": "<string>",
  "policy_version": 1,
  "capability_intent": "<string>",
  "core_api": "grant_capability|CapabilityStore::revoke|IdentityRegistry::freeze"
}
```

---

## 9. Error mapping (PROTO-0 → Apply)

### 9.1 `aether_core::error::Error` mapping

| Core `Error` | Apply code | HTTP (via pipeline) |
|--------------|------------|---------------------|
| `IdentityNotFound` | `APPLY_AGENT_NOT_FOUND` | 422 |
| `IdentityNotActive` | `APPLY_AGENT_NOT_ACTIVE` | 422 |
| `IdentityAlreadyRegistered` | `APPLY_PROTO_CONFLICT` | 422 |
| `InvalidSignature` | `APPLY_INVALID_GRANT_SIGNATURE` | 422 |
| `SigningContextMismatch` | `APPLY_INVALID_GRANT_SIGNATURE` | 422 |
| `MalformedCbor` | `APPLY_INVALID_POLICY` | 422 |
| `MalformedObject(_)` | `APPLY_INVALID_POLICY` | 422 |
| `AgentIdMismatch` | `APPLY_INVALID_POLICY` | 422 |
| `PermissionRootMismatch` | `APPLY_PERMISSION_DENIED` | 422 |
| `InvalidRootAuthority` | `APPLY_PERMISSION_DENIED` | 422 |
| `StaleRootVersion` | `APPLY_PERMISSION_DENIED` | 422 |
| `NonMonotonicRootVersion` | `APPLY_PERMISSION_DENIED` | 422 |
| `CapabilityNotFound` | `APPLY_CAPABILITY_NOT_FOUND` | 422 |
| `CapabilityRevoked` | `APPLY_CAPABILITY_REVOKED` | 422 |
| `ParentMissing` | `APPLY_INVALID_POLICY` | 422 |
| `Escalation(_)` | `APPLY_CAPABILITY_DENIED` | 422 |
| `ExcessiveDelegationDepth` | `APPLY_CAPABILITY_DENIED` | 422 |
| `InvalidDelegationDepth` | `APPLY_INVALID_POLICY` | 422 |
| `UnexpectedAuthorisation` | `APPLY_PROTO_INTERNAL` | 500 |
| `Rejected(reason)` | See §9.2 | 422 |
| `CapabilityDenied` | `APPLY_CAPABILITY_DENIED` | 422 |
| (other PROTO-1+) | `APPLY_PROTO_UNSUPPORTED` | 422 |

### 9.2 `RejectReason` mapping

| `RejectReason` | Apply code |
|----------------|----------|
| `IdentityFrozen` | `APPLY_IDENTITY_FROZEN` |
| `IdentityRevoked` | `APPLY_IDENTITY_REVOKED` |
| `CapabilityNotFound` | `APPLY_CAPABILITY_NOT_FOUND` |
| `CapabilityRevoked` | `APPLY_CAPABILITY_REVOKED` |
| `CapabilityExpired` | `APPLY_CAPABILITY_EXPIRED` |
| `CapabilityNotYetValid` | `APPLY_CAPABILITY_NOT_YET_VALID` |
| `Escalation` | `APPLY_CAPABILITY_DENIED` |
| `ExcessiveDelegationDepth` | `APPLY_CAPABILITY_DENIED` |
| `InvalidDelegationDepth` | `APPLY_INVALID_POLICY` |
| `ActionNotPermitted` | `APPLY_CAPABILITY_DENIED` |
| `SpendExceeded` | `APPLY_CONSTRAINT_VIOLATION` |
| `AssetNotPermitted` | `APPLY_CONSTRAINT_VIOLATION` |
| `CounterpartyNotPermitted` | `APPLY_CONSTRAINT_VIOLATION` |
| `RateLimitExceeded` | `APPLY_CONSTRAINT_VIOLATION` |
| `MissingCapability` | `APPLY_CAPABILITY_DENIED` |
| `InvalidSignature` | `APPLY_INVALID_GRANT_SIGNATURE` |
| `SigningContextMismatch` | `APPLY_INVALID_GRANT_SIGNATURE` |
| `MalformedObject` | `APPLY_INVALID_POLICY` |
| (default) | `APPLY_PROTO_REJECTED` |

### 9.3 Adapter-local errors (pre-PROTO)

| Condition | Apply code |
|-----------|------------|
| Invalid policy mapping | `APPLY_INVALID_POLICY` |
| Unsupported operation | `APPLY_UNSUPPORTED_OPERATION` |
| Unfreeze requested | `ADAPTER_NOT_IMPLEMENTED` |
| Issuer not configured | `APPLY_SIGNER_NOT_CONFIGURED` |

---

## 10. Simulation ↔ adapter parity

**Invariant:** `simulate_policy_apply()` and `proto0_write::execute()` **must** use the same:

1. `predict_protocol_operation` / intent resolution  
2. Field validation rules (§3–§5)  
3. `map_grant_policy` / `map_revoke_policy` / `map_freeze_policy` helpers (shared module `apply/policy_mapping.rs`)

Simulation calls validation only; adapter calls validation then PROTO-0.

---

## 11. Enterprise issuer resolution

| Config | Source |
|--------|--------|
| `issuer_agent_id` | `CP_ENTERPRISE_ISSUER_AGENT_ID` or first agent in demo bootstrap with root authority |
| Signing key | Enterprise signer seed (`CP_SIGNER_SEED_HEX`) — **must** correspond to issuer's operational key in registry |

Mismatch between signer and issuer agent → `APPLY_SIGNER_ISSUER_MISMATCH` at grant sign time.

---

## 12. Final design review

### 12.1 Ambiguity check

| Topic | Status |
|-------|--------|
| Policy → PROTO-0 mapping | **Resolved** (§3–§5) |
| Canonical JSON / hash | **Resolved** (§6; supersedes insertion-only nested order) |
| Adapter interface | **Resolved** (§8) |
| Error mapping | **Resolved** (§9) |
| Unfreeze | **Resolved** — explicitly out of MVP |
| FreezeIdentity in enum | **Resolved** — add `ProtocolOperationKind::FreezeIdentity` at implementation |
| TV-4 hash | **Pin in implementation PR** (TV-1–3 normative now) |

### 12.2 Implementation decisions remaining

| # | Decision | Status |
|---|----------|--------|
| D1 | Rust module paths | Frozen in blueprint |
| D2 | Policy field mapping | **Frozen in this doc** |
| D3 | Canonical JSON | **Frozen in this doc** |
| D4 | Error codes | **Frozen in this doc** |
| D5 | TV-4 exact hash | **Pinned** — `9d92be2f6206d8609b8f23b36316bbf0c135fc11816f636f60da212a9c31782b` |

### 12.3 Recommendation

# **READY FOR IMPLEMENTATION**

**Conditions:**

1. Explicit **implementation authorisation** recorded (unchanged from architecture review)  
2. First PR adds `apply_hash_vectors.rs` with TV-1–TV-4  
3. `execution/hash.rs` updated to `canonicalize_json` per §6  
4. `ProtocolOperationKind::FreezeIdentity` added  
5. `apply_enabled` remains `false` until blueprint §11 checklist complete  

No further design documents are required for MVP Apply (grant / revoke / freeze).

---

## 13. Freeze statement

> **APPLY_PROTO_ADAPTER_SPEC v1 is frozen.** This is the last design document before Rust implementation. It does not authorise implementation or protocol mutations. All adapters must conform without modifying `aether-core`.

---

## Appendix A — `map_grant_policy` pseudocode

```text
fn map_grant_policy(policy, issuer) -> Result<CapabilityV0>:
    validate grant fields (§3)
    actions = policy.policy_data["actions"]
    constraints = Constraints {
        max_spend: policy.policy_data["max_spend"],
        asset: policy.policy_data["asset"].unwrap_or(DEFAULT_ASSET),
        counterparties: map_counterparties(policy.policy_data),
        rate_limit: map_rate_limit(policy.policy_data),
        valid_after: policy.policy_data["valid_after"].unwrap_or(0),
        valid_before: policy.policy_data["valid_before"],
    }
    return CapabilityV0 {
        protocol_version: 0,
        schema_version: 0,
        issuer: issuer.agent_id,
        subject: AgentId(policy.target_agent_id),
        actions,
        constraints,
        delegation_depth: policy.policy_data["delegation_depth"].unwrap_or(0),
        parent_capability_id: decode_parent(policy.policy_data),
    }
```

## Appendix B — File placement (implementation)

| Concern | Path |
|---------|------|
| Policy mapping | `src/apply/policy_mapping.rs` |
| Canonical JSON | `src/apply/canonical_json.rs` |
| PROTO-0 writes | `src/protocol/proto0_write.rs` |
| Hash vectors tests | `tests/apply_hash_vectors.rs` |
