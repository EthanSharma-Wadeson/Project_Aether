# PHASE_1_KICKOFF.md — Phase 0

## Status

**Phase 0 planning artifact — implementation-oriented, not implementation itself**

This document defines the ordered execution plan for Phase 1:

- `PROTO-0` — identity + capability validation
- `PROTO-1` — bilateral channel simulator
- `PROTO-2` — escrow + receipt settlement

This document does **not** contain implementation itself. Phase 0 exit review has passed; implementation still follows the ordered sequence in this plan.

---

## 1. Phase 1 Mission

**Objective:**

> Validate Aether’s smallest security-critical primitives through isolated, adversarially tested prototypes before introducing live settlement infrastructure.

Phase 1 is **not**:

- a public network launch
- a native-token launch
- a production blockchain
- a marketplace
- a full agent application

Phase 1 exists to test the smallest honest protocol core under controlled conditions, promote what survives adversarial testing, and discard prototype shortcuts that do not belong in the protocol.

---

## 2. Entry Conditions

Phase 1 inherits the following Phase 0 decisions and artifacts.

| Input | Status at Phase 1 entry | Notes |
|------|--------------------------|-------|
| Rust reference implementation direction | Provisional | `DEC-001` |
| Monorepo structure under `Aether/` | Provisional | `DEC-002` |
| Schema-locked CBOR | Provisional | `DEC-003` |
| Ed25519 signing construction | Provisional | `DEC-004A` / `DEC-004B` |
| Rust/Python fixture interoperability | Evidence complete, decision still Provisional | supports `DEC-003` / `DEC-004B` |
| `AgentIdentityV0` | Provisional | `DEC-006` |
| `permission_root` v0 root-authority commitment | Provisional | `PERMISSION_ROOT_V0.md` |
| `CapabilityV0` model | Provisional | `DEC-007` |
| revocation model | Provisional | `DEC-008` |
| implementation constraints | Active Phase 0 policy | `IMPLEMENTATION_CONSTRAINTS.md` |
| Phase 0 security invariants | Required | binding for Phase 1 work |

### Provisional carry-forward

The following inherited items remain explicitly **Provisional** entering Phase 1:

- Rust/monorepo decisions
- schema-locked CBOR wire definition
- Ed25519 signed-byte construction
- `AgentId` derivation
- `AgentIdentityV0`
- `permission_root` semantics
- capability envelope semantics
- revocation semantics
- time semantics
- signed transport/container details where not already fixed

Phase 1 validates these choices through prototypes; it does **not** silently promote them to `Locked`.

---

## 3. Resolve the Capability Signature Boundary

Phase 1 must make one schema clarification explicit before `PROTO-0` implementation starts.

### Clarification

- `CapabilityV0` is the **unsigned semantic authority object**
- `SignedMessage` is the **canonical signed envelope** carrying a protocol body
- the authoritative signature belongs to the envelope, not as a duplicated authoritative field inside `CapabilityV0`
- verification must reconstruct the canonical signed body and validate the envelope before capability semantics are trusted

### Intent

This is a **Phase 1 schema clarification**, not a new cryptographic redesign.

It resolves the current logical-schema versus transport-envelope ambiguity so that:

- semantic fields are defined once
- signature validation happens once
- capability parsing and trust follow the same verification order as the Phase 0 wire/crypto group

### Required artifact

Before `PROTO-0` implementation begins, draft a minimal v0 schema clarification note or patch that:

- names `CapabilityGrant` as the signed transport form
- defines `CapabilityV0` as the unsigned semantic body
- updates any affected logical examples so they no longer imply two authoritative signature locations

---

## 4. PROTO-0 Plan

### Objective

Validate identity registration and capability enforcement under adversarial conditions, with fail-closed behavior and explicit mapping to Phase 0 invariants.

### Scope

`PROTO-0` covers:

- identity registration state
- permission-root validation
- direct capability validation
- delegated capability validation
- parent-chain validation
- revocation and expiry handling
- root-version invalidation
- signature/context verification at the envelope layer

`PROTO-0` does **not** cover:

- live settlement backend integration
- payment channels
- escrow release logic
- non-deterministic AI-output verification
- marketplace or discovery logic

### Required inputs

- `AgentIdentityV0`
- `PermissionRootV0`
- `RootAuthorityCapabilityV0`
- `CapabilityV0`
- `CapabilityGrant` signed-envelope clarification
- `CapabilityRevoke`
- canonical CBOR and signing rules from the wire/crypto group
- implementation constraints and published fixtures

### Minimal state model

Minimum in-memory model:

- identity registry:
  - `AgentId`
  - `operational_public_key`
  - `permission_root`
  - `root_version`
  - `status = active | frozen | revoked`
- permission-root material store:
  - `PermissionRootV0`
  - `RootAuthorityCapabilityV0`
- capability store:
  - capability body
  - envelope metadata
  - derived capability ID
  - parent link if any
- revocation store:
  - revoked capability IDs
  - revoked-at metadata if needed
- deterministic evaluation context:
  - current logical time
  - current active root version

### Identity registration flow

1. construct canonical `AgentIdentityV0`
2. verify identity envelope signature rules
3. derive and validate `AgentId` under current provisional decision
4. bind `permission_root` to the identity entry
5. insert identity into deterministic registry state

**Invariant mapping:** 4, 5, 6, 7

### Permission-root validation

Validation must prove:

- `permission_root == SHA-256(canonical_cbor(PermissionRootV0))`
- `PermissionRootV0.agent_id` matches the registered identity
- `PermissionRootV0.root_authority_capability_id` matches the canonical root-authority descriptor
- `root_version` is active and monotonic

**Invariant mapping:** 4, 6, 7, 10

### Direct capability validation

For a direct capability:

- envelope signature must verify
- signed body must decode to canonical `CapabilityV0`
- `issuer` must match the active identity
- `delegation_depth` must be `0`
- `parent_capability_id` must be absent
- actions/constraints must be a subset of the active root authority
- time validity must pass
- identity must not be frozen/revoked

**Invariant mapping:** 1, 3, 4, 5, 11

### Delegated capability validation

For a delegated capability:

- envelope signature must verify
- parent capability must exist and validate first
- child depth must equal `parent.depth + 1`
- child actions must be a subset of parent actions
- child constraints must be equal to or narrower than parent constraints
- child validity window must be within the parent validity window
- child must remain within root-authority delegation limits

**Invariant mapping:** 1, 2, 3, 11, 12

### Parent-chain validation

Validation must walk the full chain from presented capability to the active root authority, rejecting on the first failure.

The chain must reject:

- missing parent
- wrong parent hash
- parent revoked or expired
- parent/root version mismatch
- widening authority
- invalid signature at any hop

**Invariant mapping:** 1, 2, 3, 5, 11, 12

### Subset / narrowing checks

Each child must remain bounded by parent and root on:

- actions
- spend/resource ceilings
- asset scope
- counterparties
- rate limits
- validity window
- delegation depth

**Invariant mapping:** 1, 2, 12

### Delegation-depth checks

Reject if:

- child depth is not exactly parent depth + 1
- child depth exceeds root-authority maximum
- further delegation occurs after maximum depth is reached

**Invariant mapping:** 2, 12

### Expiry checks

Reject if:

- `now < valid_after`
- `now >= valid_before`
- child validity exceeds parent validity

**Invariant mapping:** 3, 12

### Revoke checks

Reject if:

- capability ID is explicitly revoked
- any parent in the chain is revoked
- identity status is `frozen` or `revoked`

**Invariant mapping:** 3, 11

### Root-version checks

Reject if:

- capability chain is presented under a stale root version for current-state validation
- root version is non-monotonic during an identity update
- root-authority commitment does not match the active identity state

**Invariant mapping:** 4, 6, 7, 10

### Fail-closed behavior

Unknown, malformed, missing, or inconsistent inputs must reject the action.

Reject on:

- unknown critical fields
- invalid canonical encoding
- signature failure
- signing-context mismatch
- missing parent evidence
- time ambiguity
- revocation ambiguity

**Invariant mapping:** 1, 3, 4, 5, 7, 11, 12

### Adversarial tests

Required adversarial cases:

- forged parent links
- widened child authority
- excessive delegation depth
- expired capability
- revoked capability
- stale root version
- mismatched `AgentId`
- altered capability constraints
- invalid signature
- signing-context mismatch

### Measurable exit criteria

`PROTO-0` is complete only when:

- deterministic registration + capability validation tests pass
- all required adversarial tests pass
- every relevant invariant is mapped to at least one test
- fixture-based signature/context verification tests pass where applicable
- no failing case can exceed configured capability limits
- no unresolved invariant violation remains open
- findings are recorded in experiment results and reviewed against `SECURITY_MODEL.md`

---

## 5. PROTO-1 Plan

### Objective

Validate a bilateral channel state machine that makes high-frequency agent payments cheaper and faster than direct settlement while keeping finality states explicit and machine-rational.

### Scope

`PROTO-1` covers:

- in-memory bilateral channel lifecycle
- signed state updates
- sequence/nonce monotonicity
- replay prevention
- invalid-state rejection
- explicit soft vs hard finality state exposure
- closure and dispute-window simulation

`PROTO-1` does **not** cover:

- live settlement backend integration
- global routing / channel mesh
- production liquidity management
- public-network deployment

### In-memory state model

Minimum model:

- channel participants
- asset / unit
- opening balances / deposits
- dispute-window parameter
- channel state sequence / nonce
- latest signed state
- status: `open | active | closing | disputed | settled`
- machine-readable finality fields:
  - soft acceptance / local agreement
  - hard settlement placeholder
  - dispute-window state

### Channel opening

Opening must validate:

- both parties identified
- authority to open channel exists
- opening balances are valid
- channel parameters are deterministic

### Signed state updates

Every update must:

- be signed by the required parties
- increase the monotonic sequence or nonce
- conserve total balance
- preserve asset identity
- reject malformed or partial state

### Monotonic sequence / nonce requirements

The simulator must reject:

- replay of an older state
- duplicate stale state after a later state exists
- non-monotonic updates

### Balance conservation

For every accepted update:

- total balances remain conserved
- no participant may create value by malformed update

### Replay prevention

State acceptance must depend on current highest valid sequence / nonce.

### Invalid-state rejection

Reject if:

- signatures invalid
- sequence stale
- balances negative or non-conserving
- participant set mismatched
- asset mismatched
- channel status transition invalid

### Channel closure behavior

Closure simulation must support:

- cooperative close from latest agreed state
- disputed close using highest valid signed state
- explicit machine-readable soft/hard/dispute-window finality outputs

### Hypothesis mapping

- **H1:** measure simulated per-update cost/latency against a direct-settlement baseline model
- **H6:** expose finality states explicitly and test whether agents can distinguish local agreement from harder settlement/dispute outcomes

### Measurable exit criteria

`PROTO-1` is complete only when:

- opening, update, replay, dispute, and closure tests pass
- monotonic sequence tests pass
- balance-conservation tests pass
- invalid-state rejection tests pass
- H1 comparison metrics are recorded
- H6 finality-behavior tests are recorded
- no Phase 0 invariant is violated by the channel simulator design

---

## 6. PROTO-2 Plan

### Objective

Validate an early escrow + receipt flow that allows deterministic, bounded work claims to unlock conditional settlement without requiring non-deterministic AI outputs to be objectively verifiable.

### Scope

`PROTO-2` covers:

- escrow creation
- deterministic receipt structure
- receipt verification
- conditional release
- timeout handling
- dispute/failure path
- outcome recording
- bounded fee/bond budgeting checks

`PROTO-2` does **not** cover:

- live backend settlement integration
- non-deterministic AI-output truth adjudication
- production reputation scoring
- full market coordination

### Escrow creation

Creation must define:

- payer
- provider
- asset / amount
- timeout / expiry
- deterministic acceptance conditions
- fee/bond assumptions used in the simulation

### Conditional release

Release must require:

- a valid deterministic receipt
- required signatures / authority
- timeout not exceeded
- no blocking dispute state

### Receipt structure

The receipt should be versioned and deterministic, with bounded claims such as:

- task or escrow ID
- protocol/schema versions
- claimant / counterparty IDs
- completion claim type
- referenced deterministic output commitment or result code
- timestamp / logical time fields
- settlement-relevant amount or status if needed

### Receipt verification

Verification must check:

- canonical decoding
- signature validity
- escrow binding
- receipt field completeness
- deterministic condition satisfaction
- capability preconditions before release

### Timeout handling

The prototype must define:

- release before timeout
- timeout without valid receipt
- timeout with conflicting claims

### Dispute or failure path

The prototype must model at least:

- refusal to release on invalid receipt
- expired escrow path
- conflicting or missing receipt path
- deterministic resolution outcome for prototype purposes

### Settlement outcome recording

Record at minimum:

- released
- refunded
- disputed
- failed / expired

### Hypothesis mapping

- **H4:** determine whether deterministic receipt-based escrow is sufficient for early market-style task flows
- **H5:** determine whether agents can plan around fee/bond requirements without human intervention in the simulated loop

### Measurable exit criteria

`PROTO-2` is complete only when:

- escrow creation / release / timeout / failure tests pass
- deterministic receipt verification tests pass
- invalid-receipt rejection tests pass
- bounded fee/budget planning tests are recorded
- H4 and H5 results are recorded
- no prototype path relies on unverifiable non-deterministic output claims

---

## 7. Prototype Boundaries

### `PROTO-0`

May depend on:

- `core/` protocol parsing/validation code
- `schemas/v0/` fixtures and schema notes
- Phase 0 decision artifacts

Must not depend on:

- live settlement backends
- channel simulator code for authority correctness
- escrow business logic

Reusable outputs:

- identity validation primitives
- signed-envelope verification helpers
- capability evaluation logic
- revocation and root-version checks

Disposable experiment code:

- one-off harnesses
- malformed-input generators without long-term ownership

### `PROTO-1`

May depend on:

- `PROTO-0` identity/capability validation outputs
- deterministic state-machine helpers

Must not depend on:

- live backend integration
- escrow receipt logic as a prerequisite for channel correctness
- production networking assumptions

Reusable outputs:

- channel state transition rules
- monotonic update validation
- finality-state exposure patterns

Disposable experiment code:

- baseline-cost calculators
- prototype-only metrics collection harnesses

### `PROTO-2`

May depend on:

- `PROTO-0` identity/capability checks
- `PROTO-1` finality-state representation only where needed conceptually
- deterministic receipt-validation helpers

Must not depend on:

- live settlement adapters
- production verifier networks
- non-deterministic AI-output adjudication

Reusable outputs:

- escrow state transition rules
- receipt structure and verification rules
- timeout / failure-path handling

Disposable experiment code:

- task-specific stubs
- prototype-only market-flow scripts

### Boundary rule

Prototype shortcuts, mock fields, or harness-only assumptions MUST NOT silently become protocol rules. If a prototype artifact is promoted, it must be rewritten or explicitly adopted under `core/` with documentation and tests.

---

## 8. Test and Evidence Policy

Phase 1 requires:

- deterministic unit tests
- adversarial tests
- fixture compatibility tests where applicable
- reproducible commands
- recorded results
- invariant-to-test mapping
- documented failures and fixes

### Evidence rules

- each prototype must declare its commands and expected outputs
- each failed adversarial case must either be fixed or recorded as an unresolved blocker
- fixture reproduction must be re-run when schema/signing changes affect covered artifacts
- security findings must be summarized back into `RESULTS.md` and reviewed against `SECURITY_MODEL.md`

### Security caution

A passing test suite alone is **not** proof that the protocol is secure.

Tests provide evidence about specific claims and failure paths. They do not replace threat modeling, consistency review, or later formal analysis.

### Phase 0 invariant coverage matrix

All 12 Phase 0 invariants must have a named Phase 1 test or review target.

| Invariant | Planned Phase 1 coverage |
|-----------|--------------------------|
| 1. Capability envelope cannot be exceeded | `PROTO-0` subset/narrowing and adversarial escalation tests |
| 2. Delegated capability cannot exceed parent | `PROTO-0` parent-chain and child-boundedness tests |
| 3. Expired/revoked capabilities cannot authorise new actions | `PROTO-0` expiry, revoke, and stale-root tests |
| 4. Settlement authority not inferred from identity alone | `PROTO-0` capability-presentment checks before channel/escrow authority |
| 5. Signature proves key control, not truth/safety | `PROTO-0` envelope verification; `PROTO-2` bounded deterministic receipts only |
| 6. Identity history remains attributable across authorised rotation | `PROTO-0` identity update and root-version continuity review |
| 7. Canonical state transitions are deterministic | `PROTO-0`, `PROTO-1`, and `PROTO-2` deterministic state-machine tests |
| 8. Dispute outcomes reproducible from rules + admissible evidence | `PROTO-1` dispute-state resolution rules; `PROTO-2` deterministic dispute/failure path |
| 9. Evidence commitment must not automatically expose private evidence | `PROTO-2` receipt structure review: commitments/result codes, no forced raw private payload disclosure |
| 10. Settlement-backend failure must not silently corrupt identity/capability state | `PROTO-0` identity/capability state isolated from live backend dependencies; `PROTO-1` / `PROTO-2` remain backend-free |
| 11. Capability enforcement occurs before economic authorisation | `PROTO-0` verification order; `PROTO-1` channel open/update authority checks; `PROTO-2` release precondition checks |
| 12. Child delegation remains bounded by parent scope/expiry/limits/depth | `PROTO-0` delegation-depth, expiry-window, and narrowing tests |

---

## 9. Phase 1 Exit Criteria

Phase 1 is complete only when **all** are true:

- `PROTO-0`, `PROTO-1`, and `PROTO-2` are completed
- all required adversarial tests pass
- no unresolved violation of a Phase 0 security invariant remains
- experiment outcomes are recorded
- `SECURITY_MODEL.md` is reviewed against prototype findings
- wire/schema changes are documented and fixtures updated where applicable
- Phase 2 blockers are identified, named, and owned

Supporting completion evidence should also include:

- result summaries for H1, H2, H4, H5, and H6
- reusable primitive inventory promoted from prototype work
- disposable experiment inventory not promoted into protocol code

---

## 10. Deferred Work

Deferred beyond Phase 1:

- live settlement backend selection
- production validator network
- native token
- public launch
- full reputation system
- distributed revocation propagation
- advanced Merkle permission structures
- multi-party authority
- full agent marketplace

---

## 11. Risks and Review Triggers

Carry forward the following risks:

- provisional permission-root semantics
- root-version update semantics
- time semantics
- transport/container details
- capability signature-boundary clarification
- current-state versus historical authority validation

### Review triggers

Reassess before Phase 2 if:

- `PROTO-0` reveals ambiguity in capability-chain or root-version validation
- fixture reproduction changes because of schema/container clarification
- `PROTO-1` shows that finality-state APIs are not machine-rational in practice
- `PROTO-2` shows deterministic receipts are insufficient even for bounded early flows
- any prototype requires relaxing a Phase 0 invariant to proceed

---

## 12. Ordered Execution Sequence

1. Freeze `PROTO-0` acceptance tests. → **Done:** [PROTO_0_ACCEPTANCE_TESTS.md](../phase_1/PROTO_0_ACCEPTANCE_TESTS.md)
2. Draft the minimal v0 identity/capability schema clarification. → **Done** in `PROTOCOL_DESIGN.md` / wire group (Phase 0 exit)
3. Implement and test `PROTO-0`. → **Done:** [PROTO_0_RESULTS.md](../phase_1/PROTO_0_RESULTS.md)
4. Review security findings. → **Done:** [PROTO_0_SECURITY_REVIEW.md](../phase_1/PROTO_0_SECURITY_REVIEW.md); `SECURITY_MODEL.md` aligned
5. Implement and test `PROTO-1`. → **Not started** — requires explicit approval
6. Review channel findings.
7. Implement and test `PROTO-2`.
8. Review escrow/receipt findings.
9. Update experiment evidence and security documentation.
10. Run the Phase 1 exit review.
11. Only then consider Phase 2 work.

---

## Acceptance-Sequence Notes

- `PROTO-1` must not start until `PROTO-0` security findings are reviewed.
- `PROTO-2` must not start until `PROTO-1` channel findings are reviewed.
- Any wire/schema change discovered during prototype work must update relevant decision docs and fixtures before the prototype is considered complete.
- Phase 1 may refine provisional decisions, but it must not silently rewrite them.
