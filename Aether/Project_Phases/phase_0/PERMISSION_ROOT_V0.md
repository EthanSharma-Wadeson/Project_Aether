# PERMISSION_ROOT_V0.md — Phase 0

## Status

**Provisional design note — sufficient for PROTO-0 identity/capability testing only**

This note defines a minimal v0 permission-root commitment for Phase 0 and Phase 1 test planning. It is intentionally narrow. It does **not** attempt to design the final Phase 3 permission system, a production Merkle optimisation strategy, or a distributed revocation network.

This document does **not** authorise the start of PROTO-0.

---

## 1. Definition

### What `permission_root` represents

`permission_root` is the current cryptographic commitment to an agent's **top-level authority state**.

For v0, it represents:

- which `AgentId` the authority belongs to
- which root-version of authority is active
- which top-level capability definition all delegated authority must descend from

### What authority state it commits to

For v0, `permission_root` commits to:

- the identity being governed
- the active root-authority definition
- the current root version used to invalidate stale capability chains after an authorised root change

It does **not** commit to every active descendant capability in a production-grade distributed data structure.

### Commitment model

For v0, `permission_root` is a commitment to a **root-authority capability descriptor**, not a fully optimised production Merkle set.

That means:

- dynamic descendant capabilities are proven by a signed parent chain
- the root commits to the top of that chain
- stale chains fail when the active root version changes

This is sufficient for PROTO-0 identity/capability testing and avoids over-engineering the Phase 0 model.

---

## 2. v0 Structure

### Committed object

```text
PermissionRootV0 {
  protocol_version: u32,
  schema_version: u32,
  agent_id: AgentId,
  root_version: u64,
  root_authority_capability_id: bytes32
}
```

### Canonical encoding rules

- authoritative wire encoding: schema-locked CBOR
- field order is protocol-critical
- canonical field order:
  1. `protocol_version`
  2. `schema_version`
  3. `agent_id`
  4. `root_version`
  5. `root_authority_capability_id`

`agent_id` is encoded as UTF-8 text. `root_authority_capability_id` is encoded as a CBOR byte string.

### Commitment construction

```text
permission_root = SHA-256(canonical_cbor(PermissionRootV0))
```

### Relation to `AgentIdentityV0`

`AgentIdentityV0` carries the current `permission_root` value:

```text
AgentIdentityV0 {
  protocol_version,
  schema_version,
  operational_public_key,
  permission_root,
  metadata_commitment
}
```

The identity does **not** embed the full root-authority descriptor directly; it embeds the commitment.

The full `PermissionRootV0` object must be available to verifiers through the canonical identity/registry state for the current root version.

---

## 3. Capability Binding

### Root-authority definition

For v0, define a canonical top-level authority descriptor:

```text
RootAuthorityCapabilityV0 {
  protocol_version: u32,
  schema_version: u32,
  issuer: AgentId,
  subject: AgentId,                  // self-authority anchor
  actions: [ActionSelector],
  constraints: {
    max_spend: optional u64,
    asset: optional string,
    counterparties: optional [AgentId],
    rate_limit: optional RateLimit,
    valid_after: optional u64,
    valid_before: optional u64
  },
  max_delegation_depth: u32
}
```

Its canonical identifier is:

```text
root_authority_capability_id =
  SHA-256(canonical_cbor(RootAuthorityCapabilityV0))
```

`PermissionRootV0.root_authority_capability_id` commits to this object.

### How a capability proves authorisation

Every `CapabilityV0` used in v0 must be evaluated against:

1. the current `AgentIdentityV0.permission_root`
2. the current `PermissionRootV0`
3. the committed `RootAuthorityCapabilityV0`
4. its parent chain, if any

### Parent-link model

`CapabilityV0` keeps:

- `issuer`
- `subject`
- `actions`
- `constraints`
- `delegation_depth`
- `parent_capability_id`

Rules:

- **direct capability:** `delegation_depth = 0`, `parent_capability_id = null`
- **delegated child:** `delegation_depth > 0`, `parent_capability_id` is required

### Direct capability authorisation

A direct capability is authorised when all are true:

- its `issuer == AgentId`
- it is signed by the current authorised operational key of that identity
- its action set is a subset of the root-authority action set
- its constraints are equal to or narrower than the root-authority constraints
- its delegation rules do not exceed `max_delegation_depth`
- it is evaluated against the current `permission_root`

### Child capability authorisation

A child capability is authorised when all are true:

- `parent_capability_id` resolves to a valid parent capability
- the parent capability was itself valid under the same active root version
- the child's `delegation_depth == parent.delegation_depth + 1`
- the child's action set is a subset of the parent's action set
- the child's constraints are equal to or narrower than the parent's constraints
- the child's expiry window does not exceed the parent's expiry window
- the child's delegation depth does not exceed the root-authority `max_delegation_depth`

### Child-boundedness rule

Children may **reduce** authority, but may never widen it.

This satisfies invariants 2 and 12.

---

## 4. Authority Constraints

The v0 model supports these constraint surfaces:

### Actions

- actions are explicit selectors
- no capability implies authority for unspecified actions

### Spend / resource limits

- `max_spend` is an upper bound, not a suggestion
- if an asset is specified, the child may not widen the asset scope

### Counterparty restrictions

- if a parent restricts counterparties, the child may only keep the same set or a narrower subset

### Rate limits

- if a parent specifies a rate limit, the child may only keep the same window with an equal or lower maximum rate

### Expiry

- `valid_before` of a child must be less than or equal to the parent's `valid_before`
- `valid_after` may not create authority before the parent's valid window

### Delegation limits

- delegation must not exceed `max_delegation_depth`
- once the effective depth limit is reached, further delegation fails closed

---

## 5. Mutation and Identity Continuity

### Does changing the permission root change `AgentId`?

**No.**

`AgentId` remains stable across authorised permission-root changes.

This preserves historical attribution and aligns with `DEC-005`.

### How updates are authenticated

Permission-root updates are authenticated by the current authorised identity key under the current identity state.

For v0:

- a root change must be represented as an authenticated identity update event
- the new `permission_root` replaces the previous active root
- the old root remains historically attributable via prior registry state

### Historical attribution

Previous permission roots remain historically attributable because:

- the identity stays the same
- root versions are monotonic
- old capability chains can be evaluated against the root version they were issued under

### How stale capabilities fail closed

Each capability evaluation must be bound to the active root version.

A capability fails closed if:

- it was issued under an older root version than the currently active root version, and
- the implementation is evaluating it against the current authority state without an explicit historical-validation path

For v0, normal operational validation uses the **current active root only**.

That means authorised root rotation invalidates stale capability chains for new actions.

---

## 6. Revocation Interaction

### What expiry handles automatically

Expiry handles:

- routine time-bounded authority
- short-lived delegated permissions
- automatic invalidation after `valid_before`

### What `CapabilityRevoke` handles

`CapabilityRevoke` handles:

- explicit invalidation of a direct or child capability before expiry
- compromise of a delegated key or subject
- revocation of a specific capability chain node

If a parent capability is revoked, all children depending on that parent link fail closed.

### What identity freeze / revoke handles

Identity `frozen` or `revoked` handles:

- emergency stop of all active authority under that identity
- failure containment when root-level trust is no longer valid

### What is deferred beyond v0

Deferred beyond v0:

- distributed revocation propagation
- production revocation accumulators / CRLs
- network-wide low-latency revocation sync
- advanced multi-party revocation semantics

---

## 7. PROTO-0 Test Requirements

This design directly enables security testing for invariants 1, 3, 11, and 12.

### Invariant mapping

| Invariant | Enabled by this design |
|-----------|------------------------|
| 1. Capability limits cannot be exceeded | subset-of-parent and subset-of-root checks |
| 3. Expired or revoked capabilities cannot authorise actions | expiry, explicit revoke, and root-version checks |
| 11. Capability checks occur before economic authorisation | verifier sequence requires capability validation before action approval |
| 12. Child delegations remain bounded by parents | parent links + depth + narrowing rules |

### Required adversarial cases

#### Forged parent link

Test:

- child references a `parent_capability_id` that does not exist or hashes to a different parent

Expected:

- reject before action authorisation

#### Capability escalation

Test:

- child widens actions, counterparties, spend limit, or rate limit beyond parent/root authority

Expected:

- reject as out-of-bounds delegation

#### Excessive delegation depth

Test:

- child depth exceeds parent depth + 1 or exceeds root `max_delegation_depth`

Expected:

- reject

#### Expired capability

Test:

- capability with `valid_before < now`

Expected:

- reject

#### Revoked capability

Test:

- capability ID appears in revoke set

Expected:

- reject

#### Stale permission root

Test:

- capability chain issued under prior `root_version` after an authorised root update

Expected:

- reject for current-state validation

#### Mismatched identity

Test:

- capability issuer or root binding does not match the `AgentId` referenced by the active permission root

Expected:

- reject

#### Altered capability constraints

Test:

- post-signature change to action list, spend limit, counterparties, or expiry

Expected:

- signature or canonical-ID mismatch; reject

---

## 8. Explicit Non-Goals

This note explicitly defers:

- final production Merkle-tree optimisation
- distributed revocation propagation
- multi-party permission roots
- advanced threshold authority
- Phase 3 capability markets

---

## 9. Decision Status

**Status:** Provisional  
**Confidence:** Medium

### Assumptions

- `AgentId` remains stable across authorised permission-root updates
- one operational key is sufficient for v0 identity/capability testing
- root-authority commitment + signed parent chain is sufficient for PROTO-0
- current-state validation is the default path for v0

### Risks

- root-authority descriptor may need refinement once real capability storage is implemented
- current-state-only validation may be too blunt for later historical/dispute use cases
- direct-capability semantics may need tightening once real transport envelopes exist
- monotonic root-version handling depends on identity update semantics not yet fully implemented

### Review triggers

Reassess when:

- PROTO-0 adversarial tests reveal ambiguity in parent-chain validation
- the root-authority descriptor proves too weak for realistic capability narrowing tests
- historical validation requirements emerge before Phase 2
- identity rotation or freeze semantics force a different root-update model

---

## Sufficiency for PROTO-0

This structure is deliberately minimal, but it is sufficient for:

- identity registration carrying a real `permission_root`
- direct and delegated capability validation
- stale-root invalidation tests
- revocation and expiry tests
- H2-style adversarial capability enforcement tests

It is **not** intended as the final production permission architecture.
