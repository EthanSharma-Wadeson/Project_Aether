# PROTO_0_ACCEPTANCE_TESTS.md — Phase 1

## Status

**Frozen acceptance-test specification — step 1 of Phase 1 sequence**

This document freezes what PROTO-0 must prove before protocol implementation begins.

It does **not** contain implementation.
It does **not** authorise networking, settlement, tokens, wallets, blockchain infrastructure, or production services.

References:

- [PHASE_1_KICKOFF.md](../phase_0/PHASE_1_KICKOFF.md)
- [PERMISSION_ROOT_V0.md](../phase_0/PERMISSION_ROOT_V0.md)
- [V0_WIRE_CRYPTO_GROUP.md](../phase_0/V0_WIRE_CRYPTO_GROUP.md)
- [IMPLEMENTATION_CONSTRAINTS.md](../phase_0/IMPLEMENTATION_CONSTRAINTS.md)
- Phase 0 security invariants in [CONTEXT.md](../phase_0/CONTEXT.md)

---

## 1. Purpose

PROTO-0 validates **identity and capability enforcement**.

It proves that an autonomous agent can:

- possess a cryptographic identity
- prove authority through capabilities
- delegate authority safely
- have actions accepted or rejected deterministically
- fail closed under adversarial conditions

Concretely, PROTO-0 proves that:

- identities can be authenticated
- capabilities can be verified
- delegation cannot increase authority
- invalid authority proofs fail closed
- capability checks occur before any simulated economic authorisation

### Explicit non-goals

PROTO-0 does **NOT** test:

- live networks
- blockchain settlement
- tokens
- payment channels
- escrow
- reputation markets
- AI reasoning systems
- distributed revocation propagation
- production Merkle permission structures
- multi-party authority

Those belong to later prototypes or deferred work.

---

## 2. Minimal State Model

PROTO-0 uses an **in-memory, deterministic** state model only.

### 2.1 Registry view of identity

The verifier operates on a registry entry that combines derived ID, identity material, and status:

```text
AgentRegistryEntryV0 {
  agent_id: AgentId,                         // derived; not freely chosen
  identity: AgentIdentityV0,
  permission_root_material: PermissionRootV0,
  root_authority: RootAuthorityCapabilityV0,
  status: active | frozen | revoked,
  registered_at: u64                         // logical time for PROTO-0
}
```

#### `AgentIdentityV0` (registration material)

```text
AgentIdentityV0 {
  protocol_version: u32,
  schema_version: u32,
  operational_public_key: bytes,             // 32-byte Ed25519
  permission_root: bytes32,                  // SHA-256 commitment
  metadata_commitment: optional bytes
}
```

#### `AgentId` derivation (provisional DEC-005)

```text
id_material = {
  schema_version,
  operational_public_key
}

AgentId = "aether:" + lowercase_hex(
  SHA-256( UTF8("aether:v0:agent-id:") || canonical_cbor(id_material) )
)
```

`permission_root` does **not** affect `AgentId`.

### 2.2 Permission root

```text
PermissionRootV0 {
  protocol_version: u32,
  schema_version: u32,
  agent_id: AgentId,
  root_version: u64,
  root_authority_capability_id: bytes32
}

permission_root =
  SHA-256(canonical_cbor(PermissionRootV0))
```

```text
RootAuthorityCapabilityV0 {
  protocol_version: u32,
  schema_version: u32,
  issuer: AgentId,
  subject: AgentId,
  actions: [ActionSelector],
  constraints: {
    max_spend: optional u64,
    asset: optional string,
    counterparties: optional [AgentId],
    rate_limit: optional RateLimit,   // { max_ops, window_seconds }
    valid_after: optional u64,
    valid_before: optional u64
  },
  max_delegation_depth: u32
}

root_authority_capability_id =
  SHA-256(canonical_cbor(RootAuthorityCapabilityV0))
```

### 2.3 Capability (unsigned semantic object)

`CapabilityV0` is the **unsigned semantic authority object**.

`capability_id` is **derived**, not an authoritative signed field inside the body:

```text
CapabilityV0 {
  protocol_version: u32,
  schema_version: u32,
  issuer: AgentId,
  subject: SubjectRef,                 // AgentId or raw public key bytes
  actions: [ActionSelector],
  constraints: {
    max_spend: optional u64,
    asset: optional string,
    counterparties: optional [AgentId],
    rate_limit: optional RateLimit,
    valid_after: optional u64,
    valid_before: optional u64
  },
  delegation_depth: u32,               // 0 = direct grant
  parent_capability_id: optional bytes // required if depth > 0
}

capability_id =
  SHA-256(canonical_cbor(CapabilityV0))
```

### 2.4 Capability grant (signed envelope)

```text
CapabilityGrant = SignedMessage<CapabilityV0>
```

The authoritative signature belongs to the **signed envelope**, not to a duplicated field inside `CapabilityV0`.

Verifier order (mandatory):

1. Reconstruct canonical CBOR body bytes
2. Reconstruct signing preimage and verify Ed25519 signature under the claimed authorised key
3. Only then validate capability semantics against registry / root / parent chain

### 2.5 Supporting stores

```text
CapabilityStore:      capability_id → { body: CapabilityV0, grant_meta, parent_id }
RevocationStore:      capability_id → revoked_at
EvaluationContext:    { now: u64, active_root_version: u64 }
```

### 2.6 Operations under test

PROTO-0 acceptance tests exercise these operations:

| Operation | Meaning |
|-----------|---------|
| `RegisterIdentity` | Accept or reject identity registration material |
| `UpdatePermissionRoot` | Accept or reject authorised root-version advance |
| `GrantCapability` | Accept or reject a `CapabilityGrant` |
| `RevokeCapability` | Record explicit capability revocation |
| `FreezeIdentity` / `RevokeIdentity` | Change identity status |
| `AuthoriseAction` | Given identity + capability proof + requested action/constraints, accept or reject |

`AuthoriseAction` is a **simulated economic gate**. It must enforce capability checks **before** returning success. No real payment or settlement occurs.

---

## 3. Acceptance Test Categories

Each test is either **PASS** (must accept) or **FAIL** (must reject / fail closed).

Tests are named `P0-T###` for tracking.

---

### 3.1 Identity Verification

#### PASS

| ID | Case | Expected |
|----|------|----------|
| P0-T001 | Valid identity registration with well-formed `AgentIdentityV0` and matching `permission_root` | Accept; registry entry created with `status = active` |
| P0-T002 | Correct operational public key used to authenticate registration / identity messages | Accept |
| P0-T003 | Derived `AgentId` matches DEC-005 construction from `schema_version` + `operational_public_key` | Accept; stored ID equals derived ID |

#### FAIL

| ID | Case | Expected |
|----|------|----------|
| P0-T010 | Altered operational public key after signing / mismatched key material | Reject |
| P0-T011 | Invalid signature on identity envelope | Reject |
| P0-T012 | Claimed `AgentId` does not match derivation | Reject |
| P0-T013 | Malformed identity object (bad lengths, missing critical fields, invalid CBOR) | Reject |
| P0-T014 | Signing-context / domain mismatch on identity message | Reject |

**Invariants:** 4, 5, 6, 7

---

### 3.2 Permission Root Verification

#### PASS

| ID | Case | Expected |
|----|------|----------|
| P0-T020 | Valid `PermissionRootV0` commitment matches `AgentIdentityV0.permission_root` | Accept |
| P0-T021 | `root_authority_capability_id` matches `SHA-256(canonical_cbor(RootAuthorityCapabilityV0))` | Accept |
| P0-T022 | Authorised root update advances `root_version` monotonically | Accept; prior root historically attributable |

#### FAIL

| ID | Case | Expected |
|----|------|----------|
| P0-T030 | Modified `permission_root` bytes that do not hash to provided `PermissionRootV0` | Reject |
| P0-T031 | Invalid / unknown `root_authority_capability_id` | Reject |
| P0-T032 | Stale `root_version` presented as current for current-state validation | Reject |
| P0-T033 | `PermissionRootV0.agent_id` mismatches registry `AgentId` | Reject |
| P0-T034 | Non-monotonic root-version update | Reject |

**Invariants:** 4, 6, 7, 10

---

### 3.3 Capability Validation

#### PASS

| ID | Case | Expected |
|----|------|----------|
| P0-T040 | Direct capability within root-authority actions and constraints | Accept |
| P0-T041 | Capability with valid time window (`valid_after ≤ now < valid_before`) | Accept |
| P0-T042 | Valid issuer: grant signed by authorised operational key of issuer identity | Accept |
| P0-T043 | Direct grant: `delegation_depth = 0` and `parent_capability_id` absent | Accept |

#### FAIL

| ID | Case | Expected |
|----|------|----------|
| P0-T050 | Forged capability (unsigned / wrong key / fabricated body) | Reject |
| P0-T051 | Post-signature alteration of constraints | Reject (signature or ID mismatch) |
| P0-T052 | Action not in root / parent action set | Reject |
| P0-T053 | Invalid issuer (wrong identity or unauthorised key) | Reject |
| P0-T054 | Expired capability (`now >= valid_before`) | Reject |
| P0-T055 | Not-yet-valid capability (`now < valid_after`) | Reject |
| P0-T056 | Capability presented without prior envelope verification success | Reject |

**Invariants:** 1, 3, 4, 5, 11

---

### 3.4 Delegation Security

**Rule:** Child capabilities may **reduce** authority, never increase it.

#### PASS

| ID | Case | Expected |
|----|------|----------|
| P0-T060 | Parent: `max_spend = 100`, actions = `{A}`; Child: `max_spend = 50`, actions = `{A}`; depth = parent+1 | Accept |
| P0-T061 | Child counterparties are a subset of parent counterparties | Accept |
| P0-T062 | Child `valid_before` ≤ parent `valid_before` | Accept |
| P0-T063 | Child rate limit ≤ parent rate limit within same or narrower window policy | Accept |

#### FAIL

| ID | Case | Expected |
|----|------|----------|
| P0-T070 | Child `max_spend = 200` when parent `max_spend = 100` | Reject |
| P0-T071 | Child adds action `B` not present on parent | Reject |
| P0-T072 | Child claims `delegation_depth` that is not exactly `parent.depth + 1` | Reject |
| P0-T073 | Child depth exceeds root `max_delegation_depth` | Reject |
| P0-T074 | Child widens counterparties or asset scope | Reject |
| P0-T075 | Child extends expiry beyond parent | Reject |
| P0-T076 | Forged / missing / wrong `parent_capability_id` | Reject |

**Invariants:** 1, 2, 11, 12

---

### 3.5 Revocation

#### PASS (correct rejection behaviour)

| ID | Case | Expected |
|----|------|----------|
| P0-T080 | Explicitly revoked capability used in `AuthoriseAction` | Reject |
| P0-T081 | Capability under `frozen` identity | Reject |
| P0-T082 | Capability under `revoked` identity | Reject |
| P0-T083 | Child capability whose parent was revoked | Reject |

#### FAIL (incorrect acceptance — these must not happen)

| ID | Case | Expected |
|----|------|----------|
| P0-T090 | Revoked capability still accepted | **Test failure** if accepted |
| P0-T091 | Stale capability chain accepted after authorised root-version change | **Test failure** if accepted |
| P0-T092 | Expired capability accepted | **Test failure** if accepted |

**Invariants:** 3, 11

---

## 4. Adversarial Tests

Explicit attacker scenarios. Every case must **fail closed**.

### Attack 1 — Capability Escalation

**Attacker modifies** a child grant (or presents a mutated body) to widen authority:

- increases `max_spend`
- adds actions
- widens counterparties
- raises rate limits
- extends expiry

| ID | Case | Expected |
|----|------|----------|
| P0-A01 | Escalated spend / actions / counterparties / rate / expiry | Reject before action authorisation |

**Invariants:** 1, 2, 12

---

### Attack 2 — Forged Parent Link

**Attacker presents** a child capability whose `parent_capability_id`:

- does not exist
- hashes to a different parent
- points to an unrelated grant

| ID | Case | Expected |
|----|------|----------|
| P0-A02 | Forged / missing / wrong parent link | Reject |

**Invariants:** 2, 5, 12

---

### Attack 3 — Excessive Delegation Depth

**Attacker creates** a chain that:

- skips depth steps
- exceeds root `max_delegation_depth`
- reuses a depth value inconsistently

| ID | Case | Expected |
|----|------|----------|
| P0-A03 | Depth not parent+1 or exceeds max | Reject |

**Invariants:** 2, 12

---

### Attack 4 — Expired Capability Replay

**Attacker replays** a previously valid grant after `valid_before`.

| ID | Case | Expected |
|----|------|----------|
| P0-A04 | Expired capability authorises action | Reject |

**Invariants:** 3

---

### Attack 5 — Revoked Capability Use

**Attacker uses** a capability after `CapabilityRevoke` for that `capability_id`, or after parent revoke.

| ID | Case | Expected |
|----|------|----------|
| P0-A05 | Revoked capability or revoked-parent chain | Reject |

**Invariants:** 3, 11

---

### Attack 6 — Stale Permission Root

**Attacker uses** a capability chain issued under `root_version = N` after an authorised update to `N+1`.

| ID | Case | Expected |
|----|------|----------|
| P0-A06 | Stale root-version chain used for current-state `AuthoriseAction` | Reject |

**Invariants:** 4, 6, 7, 10

---

### Attack 7 — Mismatched AgentId

**Attacker binds** a capability or permission root to a different `AgentId` than the registry entry / root material.

| ID | Case | Expected |
|----|------|----------|
| P0-A07 | Issuer / root / registry identity mismatch | Reject |

**Invariants:** 4, 5

---

### Attack 8 — Altered Capability Constraints

**Attacker mutates** actions, spend, counterparties, or expiry **after** the grant was signed.

| ID | Case | Expected |
|----|------|----------|
| P0-A08 | Post-signature constraint mutation | Reject (signature or canonical ID mismatch) |

**Invariants:** 1, 5, 7

---

### Attack 9 — Invalid Signature

**Attacker presents** a grant with:

- wrong signature bytes
- truncated / malformed signature
- signature from an unrelated key

| ID | Case | Expected |
|----|------|----------|
| P0-A09 | Invalid signature on `CapabilityGrant` | Reject before semantic trust |

**Invariants:** 5

---

### Attack 10 — Signing-Context Mismatch

**Attacker reuses** a valid signature under the wrong:

- domain tag
- protocol / schema version binding
- message type

| ID | Case | Expected |
|----|------|----------|
| P0-A10 | Signing-context mismatch | Reject |

**Invariants:** 5, 7

---

### Attack 11 — Identity-Only Economic Bypass

**Attacker claims** an identity is sufficient to authorise a spend / channel-open style action without presenting a valid capability.

| ID | Case | Expected |
|----|------|----------|
| P0-A11 | `AuthoriseAction` with identity only, no valid capability | Reject |

**Invariants:** 4, 11

---

### Attack 12 — Payment Signature Without Capability

**Attacker presents** a valid payment-like signature (or any secondary signature) while capability checks would fail.

| ID | Case | Expected |
|----|------|----------|
| P0-A12 | Secondary signature alone cannot authorise the action | Reject |

**Invariants:** 1, 5, 11

---

## 5. Verification Pipeline Order (Normative for Tests)

Every `AuthoriseAction` test must enforce this order. Skipping a step is itself a failing test condition.

```text
1. Authenticate SignedMessage (canonical body + signature + signing context)
2. Check identity status (not frozen / revoked)
3. Validate permission_root / root_version / root-authority binding
4. Evaluate CapabilityV0 semantics (actions, constraints, expiry, chain)
5. Only then allow simulated economic authorisation
```

A test that accepts at step 5 without passing steps 1–4 is a **PROTO-0 failure**, regardless of other outcomes.

**Invariants:** 1, 3, 4, 5, 11

---

## 6. Invariant Coverage Matrix

| Phase 0 invariant | Required PROTO-0 coverage |
|-------------------|---------------------------|
| 1. Must not exceed active capability envelope | P0-T040–T056, P0-A01, P0-A08, P0-A12 |
| 2. Delegated capability must not exceed parent | P0-T060–T076, P0-A01, P0-A02, P0-A03 |
| 3. Expired/revoked must not authorise | P0-T054–T055, P0-T080–T092, P0-A04, P0-A05 |
| 4. Settlement authority not from identity alone | P0-T001–T014, P0-A07, P0-A11 |
| 5. Signature ≠ truth/safety of outputs | Envelope checks; P0-A08–A10, P0-A12; no AI-output truth tests |
| 6. Identity history across authorised rotation | P0-T022; root update keeps same `AgentId` |
| 7. Deterministic canonical transitions | All encode/decode and accept/reject cases must be deterministic |
| 8. Dispute outcomes reproducible | **Out of PROTO-0 scope** — covered later by PROTO-1/PROTO-2 planning |
| 9. Commitment ≠ private disclosure | PROTO-0 uses commitments/IDs only; no private payload forced exposure |
| 10. Backend failure must not corrupt ID/capability state | PROTO-0 is backend-free; identity/capability state local and isolated |
| 11. Capability before economic authorisation | Pipeline order §5; P0-A11, P0-A12 |
| 12. Child bounded by parent scope/expiry/limits/depth | P0-T060–T076, P0-A01–A03 |

Invariant 8 is explicitly **not** a PROTO-0 acceptance requirement. It remains mapped for Phase 1 overall via PROTO-1/PROTO-2.

---

## 7. Measurable Exit Criteria for PROTO-0

PROTO-0 acceptance is satisfied only when **all** are true:

1. Every `P0-T###` PASS case accepts and every FAIL case rejects.
2. Every `P0-A##` adversarial case rejects (fail closed).
3. Verification pipeline order is enforced in `AuthoriseAction`.
4. No accepted case exceeds configured capability limits.
5. No accepted case uses expired, revoked, or stale-root authority for current-state validation.
6. Fixture-compatible signature/context checks pass where the test uses published signing rules.
7. Results are recorded with reproducible commands.
8. No unresolved violation of invariants 1–7 or 9–12 remains for PROTO-0 scope.
9. Findings are reviewed against `SECURITY_MODEL.md` before PROTO-1 begins.

A passing suite alone is evidence for these claims, not a global proof that Aether is secure.

---

## 8. Implementation Boundary for Later Work

When implementation begins (after this freeze):

- Production-intent logic belongs under `Aether/core/`
- Prototype harnesses may live under `Aether/experiments/proto/`
- `core/` must not depend on experiment code
- Prototype shortcuts must not silently become protocol rules

This document freezes tests only. It does not start that implementation.

---

## 9. Freeze Statement

As of this document:

> PROTO-0 acceptance tests are **frozen** for Phase 1 step 1.

Changes to these acceptance criteria require an explicit update to this file and a note in Phase 1 evidence / decision records. Implementation must target this specification; the specification must not be silently weakened to make tests pass.
