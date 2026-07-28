# V0 Wire / Crypto Group — Phase 0

**Status:** Provisional (group) — **not Locked**  
**Owner:** Project Lead  
**Date:** 2026-07-28

Coherent design for DEC-003 through DEC-008. These decisions are recorded together and must pass [internal consistency review](#internal-consistency-review) before any member is promoted to `Locked`.

**Experiment evidence:** [experiments/decision/RESULTS.md](../../experiments/decision/RESULTS.md)

---

## Approved sequence

| Step | ID | Topic |
|------|-----|-------|
| 1 | DEC-003 | Canonical serialization |
| 2 | DEC-004A / DEC-004B | Signature primitive; signed-byte construction |
| 3 | DEC-006 | Minimal identity object |
| 4 | DEC-007 | Capability envelope schema |
| 5 | DEC-005 | Agent ID derivation and identity commitment |
| 6 | DEC-008 | Capability revocation (v0) |

DEC-004A may be evaluated before DEC-003 is final. DEC-004B remains provisional until canonical serialization is selected. DEC-005 follows DEC-007 so capability/permission-root binding is reviewed before ID commitment scope is set.

---

## DEC-003 — Canonical serialization

**Status:** Provisional  
**Confidence:** Medium  
**Dependencies:** DEC-001 (Rust reference), DEC-002 (schemas path)

### Options considered

| Option | Summary |
|--------|---------|
| A. Schema-locked CBOR (RFC 8949) | Binary wire; field order fixed in `schemas/v0/`; byte strings native |
| B. Canonical JSON (sorted keys + explicit binary rules) | Human-auditable; needs base64url rule for keys/hashes |
| C. serde_json default | Rejected by experiment — field order not safe across implementations |
| D. Protobuf / other IDL | Deferred — heavier toolchain; not needed for PROTO-0 |

### Provisional decision

**Primary wire encoding: schema-locked CBOR (Option A).**

- Field order is **normative** in `schemas/v0/` (not implementation-defined serde order).
- The v0 canonical field order for `IdentityRegisterV0` is explicitly:
  1. `protocol_version`
  2. `schema_version`
  3. `operational_public_key`
  4. `permission_root`
  5. `metadata_commitment`
- `protocol_version` is the first field in every v0 message body.
- Unknown **critical** fields → reject; unknown **optional** fields → ignore per version policy (`PROTOCOL_DESIGN.md`).
- Conformance fixtures published as hex CBOR + derived hashes in `schemas/v0/fixtures/`.

**Secondary (non-wire):** canonical JSON fixtures for human audit and cross-language test vectors, using **base64url** (no padding) for binary fields — not used as the signed on-wire bytes unless explicitly marked as a JSON-wire profile (deferred).

### Evidence

| Source | Finding |
|--------|---------|
| `serialization_compare` | CBOR ~40% smaller on stub message (214 vs 355 bytes) |
| `serialization_compare` | CBOR deterministic when field order is spec-locked; fails order-independence test without spec |
| `serialization_compare` | Sorted-key JSON is order-independent; byte encoding needs explicit rule |
| `python_interop` | Independent Python verifier reproduced CBOR bytes, preimages, digests, and verification results for all fixtures |
| `PROTOCOL_DESIGN.md` | Requires deterministic, machine-parseable messages |

### Known risks

- CBOR less human-readable; debugging relies on JSON fixtures.
- Rust `ciborium` serde mapping must not become the spec — normative order lives in schema doc.
- Cross-language implementers must follow the published schema order exactly, not local map/dict insertion behavior.

### Review trigger

Reassess when: a future non-Rust consumer fails fixture match; PROTO-0 needs JSON-only settlement adapter; or CBOR library gaps appear in target environments.

---

## DEC-004 — Signatures

Split into linked sub-decisions per review.

### DEC-004A — Signature primitive and library suitability

**Status:** Provisional  
**Confidence:** Medium  
**Dependencies:** DEC-001

#### Options considered

| Option | Summary |
|--------|---------|
| Ed25519 | `ed25519-dalek` 2.x — experiment PASS |
| secp256k1 | EVM alignment; not tested in Phase 0 experiment |
| Defer | Blocks all signed messages |

#### Provisional decision

**Ed25519** for v0 operational signatures using audited library (`ed25519-dalek` 2.x in reference implementation).

#### Evidence

- `rust_sign_verify`: keygen, sign, verify, wrong-payload rejection — all PASS.
- Aligns with `PROTOCOL_DESIGN.md` draft ("Ed25519 / equivalent").

#### Known risks

- Independent Python verification succeeded with `cryptography` Ed25519 verification.
- Settlement backends on secp256k1 may require separate binding keys (`SettlementBinding`).
- Threshold / multi-sig not in v0; may need algorithm agility later.

#### Review trigger

First settlement-backend key-type conflict; first additional non-Rust verifier mismatch; PROTO-0 delegation patterns.

---

### DEC-004B — Exact signed-byte construction

**Status:** Provisional  
**Confidence:** Medium  
**Dependencies:** DEC-003, DEC-004A, DEC-006 (message types)

#### Options considered

| Option | Summary |
|--------|---------|
| Sign canonical CBOR body bytes directly | Simple; body = entire message minus signature envelope |
| Sign `SHA-256(domain || canonical_cbor_body)` | Domain separation; fixed-length sign input |
| Sign hash of JSON canonical form | Rejected unless DEC-003 selects JSON wire |

#### Provisional decision

**Sign `SHA-256(preimage)`** where the preimage is built exactly as:

```text
preimage =
  UTF8("aether:v0:sign:v1")
  || 0x00
  || protocol_version_u32be
  || 0x00
  || schema_version_u32be
  || 0x00
  || message_type_len_u16be
  || UTF8(message_type)
  || 0x00
  || canonical_cbor_body_len_u32be
  || canonical_cbor_body
```

Rules:

- `canonical_cbor_body` is the schema-locked CBOR encoding of the message body only
- `message_type` is outside the body and explicitly bound into the preimage
- protocol and schema versions are outside the body **and** present inside the body; verifiers must reject any mismatch
- lengths are unsigned big-endian integers
- body bytes are included exactly as emitted by the canonical CBOR encoder

Signature envelope (logical outer transport):

```text
SignedMessage {
  protocol_version: u32,
  message_type: string,
  body: bytes,
  signer_key_id: string,
  signature: bytes
}
```

`signature = Ed25519_sign(signing_key, SHA-256(preimage))`

**Still not Locked.** The full CBOR signing pipeline now passes, but group-level open risks remain below.

#### Evidence

- `signing_pipeline` experiment now uses **real canonical CBOR objects**, not stub bytes.
- Automated tests pass for:
  - identical object → identical bytes
  - equivalent construction paths → identical bytes
  - encode → decode → encode byte identity
  - valid signature verification
  - failure on modified message data, domain, message type, signature, and public key
- Reproducible fixtures are published under `schemas/v0/fixtures/`.
- Domain-binding bug detected on first run and fixed before acceptance; this increased confidence in the negative-test coverage.

#### Known risks

- `message_type` and version fields exist both in transport context and in body semantics; mismatch handling must remain fail-closed.
- The logical `SignedMessage` envelope is specified, but transport/container details remain Phase 1 work.
- Future hash-agility or signature-agility requirements may change the outer signing envelope version.

#### Review trigger

Reassess after:

- first non-Rust verifier reproduces fixture hashes
- first PROTO-0 identity register roundtrip using the same fixtures
- any evidence that body/context duplication creates ambiguity in implementation

---

## DEC-006 — Minimal identity object (v0)

**Status:** Provisional  
**Confidence:** Medium  
**Dependencies:** DEC-003

### Provisional decision

v0 `AgentIdentity` (registration material):

```text
AgentIdentityV0 {
  protocol_version: u32,           // = 1
  schema_version: u32,             // = 1
  operational_public_key: bytes,   // 32 bytes Ed25519
  permission_root: bytes,          // 32 bytes — commitment to capability tree
  metadata_commitment: optional bytes,  // omitted if absent (CBOR null)
}
```

**Runtime / registry fields** (not in ID derivation — see DEC-005):

```text
AgentRegistryEntry {
  id: AgentId,
  identity: AgentIdentityV0,
  status: enum { active, frozen, revoked },
  registered_at: uint64,           // logical time; units TBD at PROTO-1
}
```

v0 supports **one operational key** per identity. Root/recovery and channel keys are **Deferred** to post-PROTO-0.

### Rationale

- Matches `IDENTITY_SYSTEM.md` logical object with v0 scope reduction.
- `permission_root` is present at registration but commitment scope for `AgentId` is decided in DEC-005 after capabilities (DEC-007).
- Supports invariant 6 (rotation) by separating `AgentId` from rotatable keys and permission roots.

### Evidence

- `IDENTITY_SYSTEM.md`, `AI_AGENT_MODEL.md` registration flow.
- Deliberately excludes display names, settlement bindings (separate messages).

### Known risks

- Single operational key may be insufficient for PROTO-1 channel volume — channel keys deferred.
- `registered_at` units not fixed — must align with settlement adapter later.

### Review trigger

PROTO-0 register/rotate; principal recovery requirements.

---

## DEC-007 — Capability envelope schema (v0)

**Status:** Provisional  
**Confidence:** Medium  
**Dependencies:** DEC-003, DEC-004A, DEC-006

### Provisional decision

v0 `CapabilityV0` (matches `PROTOCOL_DESIGN.md` logical schema, reduced):

```text
CapabilityV0 {
  protocol_version: u32,
  schema_version: u32,
  issuer: AgentId,
  subject: SubjectRef,             // agent_id OR raw public key bytes
  actions: [ActionSelector],       // v0: list of string tokens, e.g. "payment.channel.open"
  constraints: {
    max_spend: optional uint64,    // smallest units; asset in separate field
    asset: optional string,
    counterparties: optional [AgentId],
    rate_limit: optional RateLimit, // { max_ops, window_seconds }
    valid_after: optional uint64,
    valid_before: optional uint64,
  },
  delegation_depth: uint32,        // 0 = issuer direct grant
  parent_capability_id: optional bytes,  // hash of parent cap; required if depth > 0
}
```

`CapabilityGrant` message = `SignedMessage` wrapping `CapabilityV0`.

`CapabilityV0` is the unsigned semantic authority object. The authoritative signature is carried by the enclosing `SignedMessage`, not duplicated as a second authoritative field inside `CapabilityV0`.

For the current v0 permission-root model, direct and delegated capabilities are validated against the active `PermissionRootV0` / `RootAuthorityCapabilityV0` described in [PERMISSION_ROOT_V0.md](PERMISSION_ROOT_V0.md).

**Enforcement (invariant 1, 11, 12):** verifier MUST evaluate envelope before authorising economic action. Payment signatures alone are insufficient.

### Rationale

- Directly supports H2 (capability envelopes contain loss).
- `parent_capability_id` + `delegation_depth` enforce delegation chain (invariant 12).
- Informs DEC-005: `permission_root` commits to the active root-authority descriptor and root version — **not** part of `AgentId` (see below).

### Evidence

- `PROTOCOL_DESIGN.md` permission envelope and verification pipeline order.
- `AI_AGENT_MODEL.md` capability binding requirements.

### Known risks

- `ActionSelector` string tokens need registry — v0 list may be minimal (3–5 actions).
- the exact root-authority descriptor is still provisional and may need refinement during PROTO-0 tests.

### Review trigger

PROTO-0 adversarial tests (H2); first delegation chain test.

---

## DEC-005 — Agent ID derivation and identity commitment

**Status:** Provisional  
**Confidence:** Low  
**Dependencies:** DEC-003, DEC-004A, DEC-006, DEC-007

### Question under review

Does `AgentId` commit to **stable identity material only**, or also to `permission_root`?

### Options considered

| Option | AgentId includes | Effect |
|--------|------------------|--------|
| A. Stable material only | `schema_version` + `operational_public_key` | ID stable across permission-root rotation; reputation persists |
| B. Include permission_root | A + `permission_root` | ID changes when root changes — breaks reputation continuity |
| C. Include metadata_commitment | A + optional metadata | ID changes when metadata commitment changes |

### Provisional decision

**Option A: stable material only.**

```text
id_material = AgentIdMaterialV0 {
  schema_version: u32,
  operational_public_key: bytes,
}

AgentId = "aether:" + lowercase_hex(SHA-256(
  UTF8("aether:v0:agent-id:") || canonical_cbor(id_material)
))
```

`permission_root` is carried in `IdentityRegister` and `AgentRegistryEntry` but **does not affect `AgentId`**. Updating permissions uses `IdentityRotate` / capability grants without minting a new ID.

`permission_root` (v0) = `SHA-256(canonical_cbor(PermissionRootV0))` where:

```text
PermissionRootV0 {
  agent_id: AgentId,               // binds root to identity
  root_version: uint64,            // monotonic on rotation
  root_authority_capability_id: bytes32
}
```

`root_authority_capability_id = SHA-256(canonical_cbor(RootAuthorityCapabilityV0))`

The current minimal v0 model is a **root-authority commitment + signed parent chain**, not a production Merkle-set optimisation. See [PERMISSION_ROOT_V0.md](PERMISSION_ROOT_V0.md).

### Rationale

- Matches `IDENTITY_SYSTEM.md`: "ID derived from canonical public material"; registration **commits** to permission root separately.
- Supports invariant 6 (history across operational-key rotation) and Sybil cost via registration bond (not ID churn).
- `AI_AGENT_MODEL.md`: reputation attaches to identity; rotating policy should not reset ID.

### Evidence

- Design docs separate `id` from `permission_root` in `AgentIdentity` struct.
- DEC-007 defines capabilities as signed grants — root is summary commitment, not identity.

### Known risks

- If permission_root is not in ID, verifier must check root matches registry on every action (not infer from ID alone) — supports invariant 4.
- root-authority descriptor shape may still need refinement before Locked.

### Review trigger

PROTO-0 register + rotate + grant integration test; internal consistency review sign-off.

---

## DEC-008 — Capability revocation (v0)

**Status:** Provisional  
**Confidence:** Medium  
**Dependencies:** DEC-006, DEC-007

### Provisional decision

v0 uses **three complementary mechanisms**:

| Mechanism | Purpose |
|-----------|---------|
| **Expiry** | `valid_before` on `CapabilityV0` — primary automatic revocation |
| **Explicit `CapabilityRevoke`** | Signed message referencing `capability_id = SHA-256(canonical_cbor(CapabilityV0 without signature))` |
| **Identity status** | `frozen` / `revoked` on registry entry revokes all active capabilities |

**Not in v0:** global revocation accumulators, CRL downloads, or on-chain status lists (Deferred).

Revocation store (reference impl): in-memory map `capability_id → revoked_at` for PROTO-0.

### Rationale

- Supports invariants 1, 3, 11.
- Expiry-heavy per Phase 0 guidance; explicit revoke for compromise.
- Identity freeze for principal-level kill switch (`SECURITY_MODEL.md`).

### Evidence

- `PROTOCOL_DESIGN.md`: `CapabilityRevoke`, identity freeze/revoke messages.
- `IDENTITY_SYSTEM.md`: delegations expirable and revocable.

### Known risks

- Without distributed revocation gossip, verifiers must share state or query issuer — acceptable for PROTO-0 local tests only.
- Revocation ID derivation must match grant ID derivation exactly.

### Review trigger

PROTO-0 H2 adversarial tests; compromised-key scenario tests.

---

## Internal consistency review

**Review status:** Provisional pass — **not promotion to Locked**

| Source | Check | Result |
|--------|-------|--------|
| `IDENTITY_SYSTEM.md` | Non-sovereign crypto ID; permission root separate from key material | Pass with DEC-005 Option A |
| `IDENTITY_SYSTEM.md` | Key hierarchy (root/channel deferred) | Pass — v0 explicitly reduced |
| `AI_AGENT_MODEL.md` | Agent = keys + identity + permission root; capabilities enforced | Pass |
| `AI_AGENT_MODEL.md` | Reputation survives process death | Pass — stable AgentId |
| `PROTOCOL_DESIGN.md` | Verification pipeline order (auth → status → capability → economic) | Pass — DEC-007 + DEC-008 |
| `PROTOCOL_DESIGN.md` | Message families for register/revoke/delegate | Pass — v0 subset named |
| `SECURITY_MODEL.md` | Fail closed; capability bypass prevention | Pass — invariants 1, 11 |
| `SECURITY_MODEL.md` | Key compromise containment | Pass — rotate + revoke + freeze |
| Invariant 1 | Envelope limits spend | Pass — DEC-007 constraints |
| Invariant 2 | Delegation ≤ parent | Pass — depth + parent_capability_id |
| Invariant 3 | Expired/revoked cannot authorise | Pass — DEC-008 |
| Invariant 4 | Settlement not from identity alone | Pass — capabilities + bindings |
| Invariant 5 | Signature ≠ correctness | Pass — documented in DEC-004 |
| Invariant 6 | History across key rotation | Pass — DEC-005 stable ID |
| Invariant 7 | Deterministic transitions | Pass — DEC-003 + DEC-004B |
| Invariant 8 | Reproducible dispute outcomes | Deferred — PROTO-2 scope |
| Invariant 9 | Commitment ≠ disclosure | Pass — hashes only in ID material |
| Invariant 10 | Backend failure ≠ silent ID corrupt | Pass — identity layer separate per ADR |
| Invariant 11 | Capability before economic action | Pass — verification pipeline |
| Invariant 12 | Child bounded by parent | Pass — DEC-007 + PERMISSION_ROOT_V0 parent-chain rules |

**Open gaps before Locked:**

1. root-authority descriptor shape is still provisional.
2. `registered_at` / time semantics remain unresolved.
3. Only one independent second-language verifier has reproduced the canonical bytes and signatures so far.

---

## Promotion criteria (group → Locked)

No member of DEC-003–DEC-008 may move to `Locked` until **all** are true:

- [x] DEC-004B sign/verify test uses canonical CBOR from DEC-003
- [x] Fixtures in `schemas/v0/fixtures/` with published hashes
- [ ] PROTO-0 H2 test plan maps to invariants 1, 3, 11, 12
- [ ] This consistency table re-run with no Fail rows
- [ ] `unanswered_questions.md` updated for resolved identity/serialization items

---

## Related

- [DECISIONS.md](../DECISIONS.md)
- [CONTEXT.md](CONTEXT.md)
- [schemas/v0/fixtures/](../../schemas/v0/fixtures/)
