# IMPLEMENTATION_CONSTRAINTS.md — Phase 0

## Status

**Phase 0 draft — implementation policy, not protocol authority**

This document converts current Phase 0 decisions into implementation policy for Aether v0. It constrains how code, schemas, fixtures, experiments, and tests are produced while Phase 0 remains active.

It does **not** replace:

- protocol specifications in `Aether_docs/`
- architecture decisions in `Project_Phases/phase_0/DECISIONS.md`
- the coherent wire/crypto group in `Project_Phases/phase_0/V0_WIRE_CRYPTO_GROUP.md`

This document does **not** authorise the start of PROTO-0.

---

## 1. Purpose and Scope

This document defines implementation constraints for Aether v0:

- toolchain expectations
- repository/package boundaries
- dependency policy
- cryptographic implementation requirements
- canonical encoding requirements
- fixture policy
- testing obligations
- security obligations
- versioning and change-control rules

It is intentionally narrower than the protocol docs. It describes **how to implement and verify current provisional decisions**, not how to redesign them.

### Scope In

- reference implementation policy under `Aether/core/`
- schema and fixture policy under `Aether/schemas/v0/`
- decision experiment policy under `Aether/experiments/`
- compatibility testing against published fixtures

### Scope Out

- production network design
- settlement backend selection
- token issuance or economics changes
- protocol redesign of identity, capabilities, reputation, or finality

---

## 2. Rust Toolchain Policy

Rust is the Phase 0 reference implementation language per `DEC-001`.

### Toolchain

- **Edition:** Rust `2021`
- **Minimum supported toolchain (MSRV):** Rust `1.97.1` or newer in the `1.97.x` stable line for current Phase 0 work
- **Channel:** stable only for Phase 0 reference implementation and decision experiments unless a decision record explicitly allows otherwise

### Formatting and linting

- `cargo fmt` is required for all Rust code under `Aether/core/` and `Aether/experiments/`
- `cargo test` must pass before considering a Phase 0 experiment or scaffold valid
- `cargo clippy --all-targets -- -D warnings` is the intended Phase 0 lint target once `IMPLEMENTATION_CONSTRAINTS.md` is adopted in active code paths
- Warnings in isolated decision experiments should be removed where practical; do not normalize noisy builds

### Compiler-version changes

- MSRV changes must be documented in the same change set that introduces them
- The reason for the change must be recorded in `DECISIONS.md` or a follow-up decision note if the change affects implementation policy
- Do not silently raise the compiler floor through transitive dependency drift

---

## 3. Repository and Package Rules

### `Aether/core/`

`core/` is the Phase 0 and Phase 1 home for production-intent reference implementation code.

Responsibilities:

- canonical protocol parsing and validation
- canonical encoding / decoding implementation
- signing and verification helpers
- fixture-consumption tests
- protocol-state and envelope logic once Phase 1 begins

Non-responsibilities during Phase 0:

- disposable decision experiments
- settlement backend spikes
- exploratory one-off scripts with no stable ownership

### `Aether/schemas/v0/`

`schemas/v0/` is the normative home for:

- wire-format schema notes
- published compatibility fixtures
- fixture manifests and hashes
- explicit field-order rules

Anything under `schemas/v0/fixtures/` is treated as a normative compatibility vector, not as loose sample data.

### `Aether/experiments/`

`experiments/` exists for:

- isolated decision experiments
- prototype-specific spike code once Phase 1 begins
- temporary evaluators and comparison harnesses

Subdirectories:

- `experiments/decision/` — disposable Phase 0 decision experiments
- `experiments/proto/` — reserved for Phase 1+ prototype work

### Dependency direction

Allowed dependency direction:

```text
schemas/v0/fixtures  → consumed by core and experiments
experiments/decision → may read schemas/v0 fixtures
core                 → may read schemas/v0 fixtures
core                 → MUST NOT depend on experiments
```

Rules:

- `core/` MUST NOT import code from `experiments/`
- `experiments/decision/` MUST NOT become a hidden production dependency
- Anything promoted from experiment to production-intent code must be moved or rewritten under `core/` with explicit review

### Experiment promotion rule

Experiments MUST NOT become production dependencies without:

- documented review
- identified ownership
- tests in `core/`
- fixture coverage where applicable
- updates to `DECISIONS.md` and affected specs

---

## 4. Dependency Policy

### Approved dependency categories

Allowed dependency categories in Phase 0:

- standard serialization / parsing libraries
- Ed25519 and hashing libraries
- test harness and assertion utilities
- fixture loading / JSON reporting helpers
- narrowly scoped CBOR support

### Criteria for adding a dependency

A dependency should only be added if it materially improves one of:

- security
- determinism
- auditability
- standards compliance
- testability
- implementation clarity

Preference order:

1. standard library
2. small, mature external crate/package
3. custom code only when it reduces ambiguity or avoids unsafe abstraction

### Version policy

- Prefer explicit versions compatible with the current stable toolchain
- Avoid unbounded version ranges
- Do not upgrade dependencies just to chase latest versions during Phase 0
- When a dependency choice materially affects protocol behavior, document it in `DECISIONS.md`

### Audit and maintenance expectations

- security-sensitive dependencies must have visible maintenance activity and broad ecosystem use
- dependencies used for cryptography, canonical encoding, or parsing must be scrutinized more heavily than general utilities
- remove dependencies that are only convenience wrappers if they obscure protocol-critical behavior

### Cryptographic dependency restrictions

- do not add multiple overlapping Ed25519 libraries without a documented reason
- do not add unnecessary cryptographic “grab-bag” libraries
- do not depend on experimental signature frameworks unless Phase 0 records a reason
- avoid dependencies that implicitly change canonical encoding or signature behavior

---

## 5. Cryptographic Requirements

### Ed25519 requirements

- v0 operational signatures use **Ed25519**
- Rust reference implementation uses `ed25519-dalek` 2.x per current provisional decision
- independent verification has been demonstrated in Python using `cryptography` Ed25519

### Key and signature validation rules

Implementations MUST:

- reject malformed public keys
- reject malformed signature lengths or encodings
- reject invalid signatures
- reject verification attempts where the signing context does not match the message context
- treat signature verification failure as a hard failure, not a soft warning

### Domain separation requirements

Implementations MUST bind signatures to the documented domain-separation context. Domain tags are protocol data, not comments.

### Exact signing construction

Authoritative current reference: `V0_WIRE_CRYPTO_GROUP.md` (`DEC-004B`).

The current provisional construction is:

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

digest = SHA-256(preimage)
signature = Ed25519_sign(signing_key, digest)
```

Rules:

- protocol and schema versions are part of the signing context
- message type is explicitly bound outside the body
- body bytes must be the authoritative canonical CBOR bytes
- verifiers MUST reject any mismatch between signing context and body semantics

### Signature semantics

A signature proves **control of an authorised key only**.

A signature does **not** prove:

- output correctness
- truthfulness
- safety
- policy compliance by itself

This is mandatory per Phase 0 invariant 5.

---

## 6. Canonical Encoding Requirements

### Authoritative wire encoding

**Schema-locked CBOR** is the authoritative v0 wire encoding.

JSON is for:

- fixtures
- inspection
- debugging
- human-readable reports

JSON is **not** the authoritative v0 wire encoding.

### Field order is protocol-critical

Implementations MUST NOT rely on language-specific map or dict insertion order.

Where schema order is required, it must be explicitly specified and implemented.

Current explicit v0 field order for `IdentityRegisterV0`:

1. `protocol_version`
2. `schema_version`
3. `operational_public_key`
4. `permission_root`
5. `metadata_commitment`

If this order changes, fixture bytes change. Such a change is protocol-relevant.

### Unknown-field and malformed-input handling

Implementations MUST:

- reject malformed CBOR
- reject fields of the wrong type
- reject malformed protocol objects
- reject unknown critical fields
- only ignore unknown optional fields where the spec explicitly allows it

Implementations MUST NOT silently fall back to a non-canonical parse path.

### Encode/decode stability

For canonical protocol objects, implementations MUST satisfy:

```text
encode(x) == encode(decode(encode(x)))
```

for all supported v0 canonical objects in scope.

Equivalent construction paths for the same logical object MUST produce identical canonical bytes.

---

## 7. Fixture Policy

Fixtures under `Aether/schemas/v0/fixtures/` are **normative compatibility vectors** for v0 signing and encoding behavior.

### Normative requirements

- production-intent implementations MUST consume and verify them
- fixture comparisons are compatibility checks, not optional examples
- negative fixtures remain mandatory members of the verification suite

### Fixture change rules

Any fixture change requires:

- documented review
- regenerated fixture hashes in the same change set
- updated results if behavior changes
- explanation of whether the change is bug-fix, clarification, or protocol change

### Atomic update rule

If canonical bytes, preimage construction, digest rules, public keys, or signature rules change, then:

- the fixture files
- the manifest
- any published hashes
- relevant tests
- relevant decision/spec documents

must be updated atomically in the same change set.

---

## 8. Testing Requirements

Phase 0 and later production-intent implementations must support the following test classes.

### Required test categories

- unit tests for encoding and signing helpers
- deterministic encoding tests
- fixture compatibility tests
- malformed-input tests
- signature-context mismatch tests
- capability and delegation adversarial tests
- encode → decode → encode stability tests

### Minimum expectations

- canonical bytes must be stable
- negative fixtures must fail for the documented reason
- no invalid signature or malformed object may be accepted
- no capability check may be bypassed due to parsing, ordering, or missing-field ambiguity

### Invariant-to-test mapping

| Invariant | Implementation obligation | Minimum test obligation |
|-----------|---------------------------|-------------------------|
| 1. Active capability envelope not exceeded | enforce constraints before action | adversarial over-limit tests |
| 2. Delegation not greater than parent | enforce parent bounds and depth | child > parent rejection tests |
| 3. Expired/revoked capabilities rejected | check expiry/revocation before action | expired + revoked negative tests |
| 4. No settlement authority from identity alone | require capability + context | identity-only authorization failure tests |
| 5. Signature != correctness | keep verification separate from truth claims | tests that signature success does not skip policy/proof checks |
| 6. Identity history attributable across rotation | stable identity semantics preserved | rotation/continuity tests |
| 7. Deterministic state transitions | canonical encoding + strict parsing | encode/decode stability and fixture tests |
| 8. Reproducible dispute outcomes | deterministic evidence-handling code | deferred until dispute-scope code exists |
| 9. Commitment != disclosure | no private material emitted by default | fixture/output inspection tests |
| 10. Backend failure must not corrupt identity/capability state | keep identity/capability logic separate from backend failures | simulated backend-failure isolation tests |
| 11. Capability enforcement before economic action | order verification pipeline correctly | sequencing tests |
| 12. Child delegation remains bounded | track scope/expiry/depth/limits | delegation chain adversarial tests |

---

## 9. Security Requirements

Implementations must map the 12 Phase 0 invariants into both code behavior and tests.

### Fail-closed requirements

The implementation MUST fail closed on:

- invalid signatures
- mismatched signing context
- mismatched protocol/schema version expectations
- invalid capabilities
- expired authority
- revoked authority
- malformed protocol objects
- malformed binary fields
- malformed canonical encodings

### Security posture requirements

- prefer simple, explicit implementations on hot paths
- separate authentication from authorization
- separate signature validity from output validity
- do not log sensitive private key material
- do not leak raw secret material in structured errors

---

## 10. Versioning and Compatibility

### Protocol version rules

- every canonical object in scope must carry `protocol_version`
- `protocol_version` changes are protocol-significant
- mismatched or unsupported protocol versions must fail closed unless the spec explicitly defines compatibility behavior

### Schema version rules

- canonical object schemas must carry `schema_version`
- schema changes that alter bytes, field meaning, or ordering require explicit fixture review

### Compatibility expectations

- compatible implementations must reproduce the published fixture bytes and verification results exactly
- compatibility claims without fixture reproduction are insufficient

### Wire-format change process

Any change to:

- canonical CBOR layout
- field order
- signing preimage
- hashing rules
- identity commitment fields

requires:

- fixture regeneration
- comparison against prior behavior
- decision review
- documentation update in the same change set

---

## 11. Error-Handling Policy

Implementations MUST:

- avoid silent fallback to non-canonical encoding
- avoid automatic acceptance of unknown fields unless explicitly specified
- return structured errors suitable for debugging and testing
- avoid leaking sensitive key material in errors, logs, or panics

Structured errors should distinguish categories such as:

- malformed object
- unsupported version
- signature failure
- context mismatch
- invalid capability
- revoked or expired authority
- internal consistency failure

---

## 12. Change-Control Rules

### Same-change-set rule

Protocol-relevant implementation changes require documentation and fixtures to be updated in the same change set.

This includes changes to:

- canonical encoding
- signing construction
- identity semantics
- capability semantics
- fixture hashes

### Decision-record review rule

Changes to canonical encoding, signing construction, or identity semantics require decision-record review before they are treated as normal implementation edits.

At minimum, update:

- `DECISIONS.md`
- `V0_WIRE_CRYPTO_GROUP.md`
- affected `Aether_docs/` specs
- fixture set and manifest

---

## Assumptions Inherited from Current Provisional Decisions

This document currently assumes, but does not independently lock:

- Rust is the reference implementation language
- schema-locked CBOR is the authoritative v0 wire encoding
- Ed25519 is the v0 signature primitive
- the current DEC-004B preimage construction remains in force
- `AgentId` remains stable across permission-root rotation
- `CapabilityV0` and revocation semantics remain as currently drafted

If any of those decisions change, this document must be updated.

---

## Unresolved Risks

- `permission_root` root-authority commitment shape remains provisional
- `registered_at` / time semantics remain unresolved
- only one second-language verifier has reproduced the fixture set so far
- transport/container details around the logical `SignedMessage` envelope remain Phase 1 work
- capability revocation distribution is still only defined at the current provisional v0 level

---

## Review Triggers

Reassess this document when any of the following occur:

- a non-Rust implementation fails fixture reproduction
- canonical field order changes
- signing preimage construction changes
- `permission_root` commitment shape changes
- versioning rules change
- PROTO-0 test planning reveals an invariant that cannot be enforced under these constraints

---

## Explicit Non-Authorisation

This document does **not** authorise the start of PROTO-0.

Phase 0 remains active until exit criteria in `CONTEXT.md` are satisfied. This document only constrains implementation behavior for current provisional decisions.
