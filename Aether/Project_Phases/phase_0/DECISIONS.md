# DECISIONS.md — Phase 0 Decision Log

## Purpose

Record critical implementation decisions with alternatives, evidence, risks, confidence, dependencies, and review triggers.

See [CONTEXT.md](CONTEXT.md) for decision precedence, ownership rules, and exit criteria.

**Wire/crypto group (DEC-003–DEC-008):** recorded as one coherent design in [V0_WIRE_CRYPTO_GROUP.md](V0_WIRE_CRYPTO_GROUP.md). Do not promote individual members to `Locked` without group consistency review.

---

## Repository Consistency Check

**Last checked:** 2026-07-28 (post review acceptance)

| Check | Result |
|-------|--------|
| Old prototype IDs (`P0`–`P5`) in canonical docs | **Pass** — only in "do not use" warnings |
| `PROTO-0`–`PROTO-5` in roadmap and prototypes | **Pass** |
| Duplicate old/new patch lines | **Pass** |
| Decision experiment isolation from `core/` | **Pass** — experiments in `experiments/decision/` only |

---

## Status and confidence definitions

| Status | Meaning |
|--------|---------|
| **Locked** | Accepted for v0; changes require ADR-style review |
| **Provisional** | Working direction; specs and experiments allowed; not final |
| **Deferred** | Intentionally postponed |
| **Blocked** | Cannot progress until dependency resolved |

| Confidence | Meaning |
|------------|---------|
| **High** | Strong evidence; unlikely to change soon |
| **Medium** | Reasonable choice; needs Phase 1 validation |
| **Low** | Temporary; high change likelihood |
| **Unknown** | Insufficient evidence |

---

## Decision index

| ID | Title | Status | Confidence | Owner | Depends on |
|----|-------|--------|------------|-------|------------|
| DEC-001 | Implementation language (Rust) | Provisional | Medium–High | Project Lead | — |
| DEC-002 | Repository layout | Provisional | Medium–High | Project Lead | DEC-001 |
| DEC-003 | Canonical serialization (CBOR) | Provisional | Medium-High | Project Lead | DEC-001, DEC-002 |
| DEC-004A | Signature primitive (Ed25519) | Provisional | Medium-High | Project Lead | DEC-001 |
| DEC-004B | Signed-byte construction | Provisional | Medium | Project Lead | DEC-003, DEC-004A, DEC-006 |
| DEC-006 | Minimal identity object | Provisional | Medium | Project Lead | DEC-003 |
| DEC-007 | Capability envelope schema | Provisional | Medium | Project Lead | DEC-003, DEC-004A, DEC-006 |
| DEC-005 | Agent ID derivation | Provisional | Low | Project Lead | DEC-003, DEC-006, DEC-007 |
| DEC-008 | Capability revocation (v0) | Provisional | Medium | Project Lead | DEC-006, DEC-007 |

Full specifications: [V0_WIRE_CRYPTO_GROUP.md](V0_WIRE_CRYPTO_GROUP.md)

---

## Approved wire/crypto sequence

```text
DEC-003 → DEC-004A/B → DEC-006 → DEC-007 → DEC-005 → DEC-008
```

- **DEC-004A** (Ed25519 primitive) evaluated before DEC-003 is Locked.
- **DEC-004B** (signed bytes) stays Provisional until canonical serialization fixtures exist.
- **DEC-005** follows **DEC-007** so `AgentId` vs `permission_root` scope is reviewed with capability design.

**Group promotion rule:** DEC-003–DEC-008 remain Provisional until [internal consistency review](V0_WIRE_CRYPTO_GROUP.md#internal-consistency-review) and promotion criteria are satisfied. None are `Locked`.

---

## DEC-001 — Implementation language and runtime

**Decision:** Rust for the v0 reference implementation (`Aether/core/`).

**Status:** Provisional  
**Confidence:** Medium–High  
**Owner:** Project Lead  
**Date:** 2026-07-28 (evidence updated 2026-07-28)

### Experiment evidence

| Experiment | Result | Reference |
|------------|--------|-----------|
| `rust_sign_verify` | PASS — `cargo test` 2/2, sign/verify OK | [RESULTS.md](../../experiments/decision/RESULTS.md) |
| `core/` scaffold | PASS — `cargo build` | [RESULTS.md](../../experiments/decision/RESULTS.md) |

### Options considered

Rust (selected provisional), Go, TypeScript, Python, spec-only.

### Rationale

Rust + `ed25519-dalek` validated in-repo. Memory safety and deterministic parsing align with security invariants.

### Known risks

Iteration speed; cross-language conformance vector work.

### Review trigger

Move toward **Locked** after DEC-004B canonical-byte sign test passes. Reassess if Rust blocks Phase 0 exit.

---

## DEC-002 — Repository layout

**Decision:** Monorepo under `Aether/` — `core/`, `schemas/v0/`, `experiments/decision/`, `experiments/proto/`.

**Status:** Provisional  
**Confidence:** Medium–High  
**Owner:** Project Lead  
**Date:** 2026-07-28 (scaffold created 2026-07-28)

### Options considered

Monorepo under `Aether/` (selected provisional), split multi-repo, root-level flat structure without package boundaries.

### Rationale

The monorepo keeps protocol-intent code, compatibility fixtures, and disposable experiments in one reviewable tree while preserving explicit boundaries between `core/`, `schemas/`, and `experiments/`.

### Experiment evidence

| Check | Result |
|-------|--------|
| `Aether/core/` compiles | PASS |
| `experiments/decision/rust_sign_verify` isolated | PASS |
| `experiments/decision/serialization_compare` isolated | PASS |

### Known risks

`Aether_Notes/` at repo root; no root README yet.

### Review trigger

**Locked** candidate after `IMPLEMENTATION_CONSTRAINTS.md` documents paths and one promoted pattern from experiments → `schemas/v0/`.

---

## DEC-003 – DEC-008 — Wire / crypto group

**Status:** Provisional (group)  
**Confidence:** Mixed (see per-decision in group doc)  
**Owner:** Project Lead  
**Date:** 2026-07-28

Recorded together in **[V0_WIRE_CRYPTO_GROUP.md](V0_WIRE_CRYPTO_GROUP.md)**.

### Summary

| ID | Provisional direction |
|----|----------------------|
| DEC-003 | Schema-locked CBOR primary wire; JSON fixtures for audit only |
| DEC-004A | Ed25519 via `ed25519-dalek` 2.x |
| DEC-004B | Sign `SHA-256(preimage)` where preimage binds domain, versions, message type, and canonical CBOR body |
| DEC-006 | `AgentIdentityV0` — single operational key, permission_root as field |
| DEC-007 | `CapabilityV0` — issuer, subject, actions, constraints, delegation_depth |
| DEC-005 | `AgentId` = hash of **stable material only** (schema_version + operational_public_key); excludes permission_root |
| DEC-008 | Expiry + `CapabilityRevoke` + identity freeze/revoke |

### Serialization experiment evidence (DEC-003)

See [RESULTS.md](../../experiments/decision/RESULTS.md). Key findings:

- Sorted-key JSON: order-independent, deterministic; needs explicit binary encoding rule.
- CBOR+serde: deterministic with fixed field order; ~40% smaller on stub fixture.
- serde_json default: unsuitable as canonical wire.

### Signing pipeline evidence (DEC-004B)

See [RESULTS.md](../../experiments/decision/RESULTS.md). Key findings:

- Real canonical CBOR objects now drive the full sign/verify pipeline.
- 10/10 signing-pipeline tests pass, including all requested negative cases.
- Fixtures are published in `Aether/schemas/v0/fixtures/`.
- DEC-004B remains Provisional because group-level risks still remain.

### Python interoperability evidence (DEC-003 / DEC-004B)

See [RESULTS.md](../../experiments/decision/RESULTS.md). Key findings:

- Independent Python verifier reproduced canonical CBOR bytes from fixture source objects.
- Reconstructed preimages and SHA-256 digests matched the published fixture values for every fixture.
- Valid and invalid fixture verification results matched expected outputs in Python.
- The Python verifier used explicit schema order, not dict insertion order or Rust map behavior.

### Internal consistency review

Provisional pass against `IDENTITY_SYSTEM.md`, `AI_AGENT_MODEL.md`, `PROTOCOL_DESIGN.md`, `SECURITY_MODEL.md`, and Phase 0 invariants — see group doc table. **Not promotion to Locked.**

### Review trigger (group)

- second-language verifier reproduces fixture outputs
- PROTO-0 H2 test plan mapped to invariants
- permission-root tree shape and time semantics reviewed

---

## Superseded decisions

| ID | Superseded by | Date | Reason |
|----|---------------|------|--------|
| — | — | — | — |

---

## Next steps

1. Fold `PERMISSION_ROOT_V0.md` into future Phase 1 schema clarification work without promoting it to `Locked`.
2. Draft the minimal signed-envelope versus logical-schema clarification for `CapabilityGrant` / `CapabilityV0`.
3. Use `PHASE_1_KICKOFF.md` as the ordered execution plan for PROTO-0 → PROTO-2.
4. Reassess time semantics and current-state versus historical authority validation during Phase 1 reviews.
