# PROTO_2_DESIGN.md — Phase 1

## Status

**Design resolved + acceptance tests frozen — implementation not started**

PROTO-0 and PROTO-1 are complete. PROTO-2 implementation may begin only after this package is accepted:

- state model in this document
- decisions in [PROTO_2_DECISIONS.md](PROTO_2_DECISIONS.md)
- frozen tests in [PROTO_2_ACCEPTANCE_TESTS.md](PROTO_2_ACCEPTANCE_TESTS.md)

This document does **not** authorise live settlement, networking, tokens, or PROTO-3+ work.

References:

- [PHASE_1_KICKOFF.md](../phase_0/PHASE_1_KICKOFF.md) §6
- [PROTO_0_RESULTS.md](PROTO_0_RESULTS.md)
- [PROTO_1_RESULTS.md](PROTO_1_RESULTS.md)
- [PROTO_1_DESIGN.md](PROTO_1_DESIGN.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
- [PROTOCOL_DESIGN.md](../../Aether_docs/PROTOCOL_DESIGN.md)
- [ECONOMIC_MODEL.md](../../Aether_docs/ECONOMIC_MODEL.md)
- Phase 0 invariants in [CONTEXT.md](../phase_0/CONTEXT.md)

---

## 1. Purpose

PROTO-2 explores **receipt-based conditional settlement** through a local escrow simulator.

It asks:

> Can authorised agents commit simulated value, present deterministic completion evidence, and reach a mutually auditable terminal outcome (release, refund, or dispute resolution) without live settlement infrastructure?

### Why an escrow primitive is needed

Channels (PROTO-1) prove bilateral shared-state agreement. Agent work markets need a separate primitive:

- one party locks value against **bounded, deterministic completion claims**
- the counterparty performs work and presents **evidence**
- release or refund follows **rules + authority**, not identity alone

### What a receipt represents (PROTO-2)

A **settlement receipt** (`SettlementReceiptV0`) is a signed claim that:

- binds to a specific escrow ID and terms version
- identifies the claimant (provider) and payer
- asserts a **deterministic completion claim** (`claim_type` + `result_code` and/or `output_commitment`)
- records logical time of claim

It proves: *the provider signed this bounded claim under their operational key at this time for this escrow.*

It does **not** prove: output correctness, usefulness, truth, safety, legal completion, or blockchain finality.

### What conditional release means

Funds move from the simulated escrow ledger to the provider only when:

1. PROTO-0 authorisation passes for the release operation
2. escrow status and timeout rules permit release
3. a valid `SettlementReceiptV0` is bound to the escrow
4. deterministic acceptance conditions in `EscrowTermsV0` are satisfied
5. no blocking dispute state exists
6. fee budget permits the operation

### Economic finality (simulator)

**Simulated economic finality** means: within the local `EscrowStore`, a terminal status (`Released`, `Refunded`, `ResolvedReleased`, `ResolvedRefunded`) is recorded, value conservation holds, and `EconomicFinalityViewV0.finalized = true`.

This is **not**:

- blockchain finality
- legal settlement
- production payment safety
- irreversible real-money transfer

Agents must treat `hard_settlement_placeholder` (always false in PROTO-2) as distinct from simulator finality.

### Hypothesis mapping

- **H4** — deterministic receipt + timeout escrow is sufficient for early bounded agent task flows (prototype evidence only)
- **H5** — agents can plan and execute hire→work→settle loops with predictable fee quotes and budgets (prototype evidence only)

---

## 2. Scope

### In scope

- in-memory escrow ledger (separate from PROTO-1 channels)
- escrow creation with explicit terms
- simulated funding / value lock
- provider-signed settlement receipts
- receipt verification and escrow binding
- conditional release and refund paths
- timeout handling (logical time)
- dispute raise and local resolution
- simulated economic finality view
- fee quote, reservation, consumption, and budget enforcement (abstract units)
- PROTO-0 capability gating on every economically meaningful operation
- deterministic unit and adversarial tests
- H4 / H5 measurement hooks

### Out of scope / non-goals

PROTO-2 is **not**:

- real money, cryptocurrency, or token issuance
- blockchain deployment or live L1/L2 settlement
- networking, smart contracts, or production custody
- legal enforcement or oracle networks
- TEEs, ZK proofs, or non-deterministic AI-output adjudication
- reputation scoring, marketplace UI, or autonomous agent behaviour
- slashing economics or watchtowers
- multiparty escrows beyond payer + provider (+ local resolver role)

Do **not** introduce:

- native token
- real payments
- settlement backend adapters
- production infrastructure
- weakening PROTO-0 or PROTO-1 security guarantees

---

## 3. Relationship to PROTO-0 and PROTO-1

### PROTO-0 (authority layer)

PROTO-2 **consumes** PROTO-0 unchanged. Every economically meaningful escrow operation requires:

- registered identity with appropriate status
- valid `CapabilityGrant` for the action token
- fail-closed on freeze/revoke/expiry/stale root

Verification order (normative):

```text
1. Verify SignedMessage envelope(s) where applicable
2. PROTO-0 identity status
3. PROTO-0 capability / root checks
4. Escrow transition rules (status, timeout, conservation, receipt rules)
5. Apply state / emit audit record
```

### PROTO-1 (channel layer) — Option B (separate ledger)

**Chosen direction (P2-DEC-002):** PROTO-2 uses a **separate in-memory escrow ledger**. It does not require channel balances or channel state transitions for correctness.

| Concern | Owner |
|---------|-------|
| Identity / capability | PROTO-0 |
| Bilateral channel state | PROTO-1 (optional future integration) |
| Escrow lifecycle, receipts, fees | PROTO-2 |

Rationale:

- Kickoff requires escrow correctness independent of channel logic
- Avoids duplicating a second payment system inside channels
- Preserves clear prototype boundaries

**Conceptual reuse only:** `EconomicFinalityViewV0` mirrors PROTO-1 `FinalityViewV0` patterns (`soft_local_agreement`, `hard_settlement_placeholder`, dispute window fields).

**Explicit distinction:**

```text
PROTO-1 ReceiptV0        = channel transition audit record (not independently verified)
PROTO-2 SettlementReceiptV0 = work-completion claim unlocking escrow (signed, verified)
```

Optional future hook (deferred): escrow funding may reference a channel balance snapshot; not required for PROTO-2.

---

## 4. Minimal data models

All protocol-critical objects use schema-locked canonical CBOR (DEC-003), `protocol_version` / `schema_version`, and DEC-004B signing where marked signed.

### 4.1 `EscrowTermsV0` (unsigned, protocol-critical)

| Field | Type | Purpose |
|-------|------|---------|
| `protocol_version` | u32 | Wire version |
| `schema_version` | u32 | Schema version |
| `payer` | AgentId | Funding party |
| `provider` | AgentId | Work party |
| `asset` | text | Simulated asset label |
| `principal_amount` | u64 | Escrow principal (abstract units) |
| `max_protocol_fee` | u64 | Fee ceiling authorised for this escrow |
| `claim_type` | text | Deterministic claim profile (e.g. `deterministic.result_code.v0`) |
| `required_result_code` | text | Expected result code for auto-release eligibility |
| `accept_output_commitment` | bool | Whether `output_commitment` must match terms |
| `expected_output_commitment` | optional bytes | Required commitment when flag true |
| `fund_before` | u64 | Logical deadline to fund |
| `receipt_before` | u64 | Logical deadline to submit valid receipt |
| `dispute_window` | u64 | Logical duration after receipt/refund trigger |
| `terms_version` | u64 | Monotonic terms revision |

**Validation:** `payer != provider`; amounts > 0; `fund_before < receipt_before`; fee ≤ principal; canonical re-encode.

**Capability:** `escrow.create` for proposer; counterparty must co-sign terms body (P2-DEC-003).

### 4.2 `EscrowV0` (unsigned snapshot, protocol-critical)

| Field | Type | Purpose |
|-------|------|---------|
| `protocol_version` | u32 | |
| `schema_version` | u32 | |
| `escrow_id` | bytes[32] | `SHA-256(canonical_cbor(EscrowTermsV0))` |
| `terms` | EscrowTermsV0 | Embedded terms |
| `status` | enum | See state machine |
| `funded_amount` | u64 | Principal locked |
| `fee_reserved` | u64 | Reserved protocol fee |
| `fee_consumed` | u64 | Consumed fee |
| `receipt_id` | optional bytes[32] | Bound settlement receipt hash |
| `created_at` | u64 | Logical time |
| `funded_at` | optional u64 | |
| `receipt_submitted_at` | optional u64 | |
| `finalized_at` | optional u64 | |
| `finality` | EconomicFinalityViewV0 | Machine-readable finality |

**State ownership:** `EscrowStore` (crate-internal mutation only; same pattern as PROTO-1 remediation).

### 4.3 `EscrowFundingV0` (signed, protocol-critical)

| Field | Type | Purpose |
|-------|------|---------|
| `escrow_id` | bytes[32] | Target escrow |
| `payer` | AgentId | Must match terms |
| `amount` | u64 | Must equal `principal_amount` |
| `fee_reservation` | u64 | ≤ `max_protocol_fee` |
| `logical_time` | u64 | |

**Signed by:** payer operational key (`escrow.fund` message type).

**Validation:** status `Proposed`; `now <= fund_before`; payer simulated balance sufficient (principal + fee reservation).

### 4.4 `SettlementReceiptV0` (signed body, protocol-critical)

| Field | Type | Purpose |
|-------|------|---------|
| `protocol_version` | u32 | |
| `schema_version` | u32 | |
| `receipt_id` | bytes[32] | `SHA-256(canonical body without receipt_id)` |
| `escrow_id` | bytes[32] | Binding |
| `terms_version` | u64 | Must match escrow terms |
| `payer` | AgentId | |
| `provider` | AgentId | Claimant / signer |
| `claim_type` | text | Must match terms |
| `result_code` | text | Deterministic completion code |
| `output_commitment` | optional bytes | Private output commitment |
| `claimed_amount` | u64 | ≤ principal |
| `logical_time` | u64 | |
| `receipt_nonce` | u64 | Replay resistance per escrow |

**Signed by:** provider only (`escrow.submit_receipt`).

**Proves:** provider signed this bounded claim. **Does not prove** work quality or truth.

**Validation:** signature → membership → capability → escrow binding → claim_type/result_code/commitment match terms → `now <= receipt_before` → nonce monotonicity.

### 4.5 `SettlementEvidenceV0` (unsigned wrapper, simulator)

| Field | Purpose |
|-------|---------|
| `escrow_id` | |
| `receipt` | `SignedMessage<SettlementReceiptV0>` |
| `logical_time_presented` | Harness timestamp |

Used for release/dispute paths; evidence store is local.

### 4.6 `FeeQuoteV0` (unsigned, protocol-critical)

| Field | Purpose |
|-------|---------|
| `protocol_version` / `schema_version` | |
| `operation` | e.g. `escrow.fund`, `escrow.release` |
| `asset` | |
| `quoted_fee` | Abstract units |
| `valid_after` / `valid_before` | Logical window |

Returned at create time; stored on escrow record.

### 4.7 `FeeLedgerEntryV0` (unsigned, simulator audit)

| Field | Purpose |
|-------|---------|
| `escrow_id` | |
| `agent_id` | Payer charged |
| `operation` | |
| `amount` | Fee consumed |
| `logical_time` | |
| `remaining_budget` | After operation |

Simulator-only audit trail for H5; not a payment receipt.

### 4.8 `EconomicFinalityViewV0` (unsigned, protocol-critical)

| Field | Purpose |
|-------|---------|
| `soft_local_agreement` | Rules satisfied locally |
| `dispute_window_open` | |
| `dispute_deadline` | optional logical time |
| `hard_settlement_placeholder` | always `false` in PROTO-2 |
| `finalized` | terminal escrow status |
| `outcome` | `released` \| `refunded` \| `none` |

### 4.9 `EscrowReleaseV0` / `EscrowRefundV0` (signed, protocol-critical)

Minimal signed intent messages for terminal transitions:

- **Release:** payer signs release authorization (`escrow.release`) referencing `escrow_id`, `receipt_id`, `logical_time`
- **Refund:** payer or resolver signs (`escrow.refund` / `escrow.resolve`) with reason code

Auto-release path (P2-DEC-006): after valid receipt + dispute window, either party may invoke `escrow.release` with capability; no payer co-signature on receipt body.

### 4.10 Channel `ReceiptV0` (PROTO-1, not PROTO-2)

Unchanged. PROTO-2 must not overload or extend `ReceiptV0` for settlement claims.

---

## 5. Escrow state machine

### 5.1 Statuses

```text
Proposed          — terms agreed, not yet funded
Funded            — principal (+ fee reservation) locked
AwaitingReceipt   — funded, before valid receipt (may equal Funded operationally)
ReceiptAccepted   — valid receipt bound; dispute window may be open
ReleasePending    — release authorized; pre-terminal checkpoint (optional, may merge with ReceiptAccepted)
Released          — terminal; provider receives principal (fee consumed)
RefundPending     — refund authorized; pre-terminal checkpoint
Refunded          — terminal; payer receives principal (fee rules applied)
Disputed          — dispute raised with admissible evidence
ResolvedReleased  — terminal via dispute → release
ResolvedRefunded  — terminal via dispute → refund
Cancelled         — terminal; aborted before funding (dual-signed cancel)
Expired           — terminal; timed out without valid receipt → refund path
```

Minimal operational set for implementation: `Proposed`, `Funded`, `ReceiptAccepted`, `Released`, `Refunded`, `Disputed`, `ResolvedReleased`, `ResolvedRefunded`, `Cancelled`, `Expired`.

### 5.2 Allowed transitions

| Transition | Authority | Evidence | Preconditions | Fail closed |
|------------|-----------|----------|---------------|-------------|
| `Proposed` → `Funded` | payer `escrow.fund` | `EscrowFundingV0` sig | terms valid; `now <= fund_before`; budget OK | no cap; late fund; wrong amount; double fund |
| `Proposed` → `Cancelled` | both `escrow.create` | dual-signed cancel | unfunded | after fund |
| `Funded` → `ReceiptAccepted` | provider `escrow.submit_receipt` | `SettlementReceiptV0` sig | receipt matches terms; `now <= receipt_before`; nonce fresh | wrong escrow; bad sig; replay; expired |
| `ReceiptAccepted` → `Released` | payer or provider `escrow.release` | release intent + bound receipt | dispute window elapsed if set; no dispute | before window; disputed; wrong receipt |
| `Funded` → `Expired` | either party `escrow.refund` or auto timeout | timeout proof | `now > receipt_before`; no valid receipt | receipt already accepted |
| `Expired` → `Refunded` | payer `escrow.refund` | refund intent | status Expired | double refund |
| `Funded/ReceiptAccepted` → `Disputed` | either party `escrow.dispute` | `SettlementEvidenceV0` or challenge | admissible evidence | invalid state; bad evidence |
| `Disputed` → `ResolvedReleased` | resolver `escrow.resolve` | valid receipt evidence wins | deterministic rule (P2-DEC-009) | no authority |
| `Disputed` → `ResolvedRefunded` | resolver `escrow.resolve` | missing/invalid receipt | rule-based | no authority |
| `*` → `Refunded` (cooperative) | payer `escrow.refund` | refund intent | before release; terms allow | after release |

Invalid transitions (must reject): any from terminal states; release before fund; refund after release; duplicate terminal ops.

### 5.3 Value conservation

At all times:

```text
escrow_pool = principal_locked + fee_reserved
```

On terminal release: `provider_gets = principal - fee_consumed` (fee to simulator treasury account); on refund: `payer_gets = principal - fee_consumed_if_applicable`.

No operation may create value. Double release and double refund are impossible (status guards).

---

## 6. Receipt semantics

### 6.1 PROTO-1 vs PROTO-2

| | PROTO-1 `ReceiptV0` | PROTO-2 `SettlementReceiptV0` |
|--|---------------------|-------------------------------|
| Purpose | Channel transition audit | Work-completion claim |
| Signers | Copies channel dual-sigs | Provider signs claim |
| Independent verify | No | Yes (full SignedMessage path) |
| Escrow binding | No | Required |
| Proves completion | No | Bounded claim only |

### 6.2 Required receipt properties (P2-DEC-004)

- Separately signed `SignedMessage<SettlementReceiptV0>`
- Single signer: **provider** (claimant)
- `escrow_id` + `terms_version` binding
- `claim_type` + `result_code` (+ optional `output_commitment`)
- `receipt_nonce` monotonic per escrow
- `logical_time` from harness
- Optional expiry via `receipt_before` on terms (not separate receipt TTL in v0)

### 6.3 Valid receipt proves

- Provider operational key controlled the signed bytes
- Claim is scoped to a specific escrow and terms version
- Claim uses the deterministic profile named in terms

### 6.4 Valid receipt does not prove

- Output correctness, usefulness, or safety (invariant 5)
- Legal delivery or real-world performance
- Backend settlement or hard finality

---

## 7. Authority and capability requirements

### 7.1 Action tokens (P2-DEC-010)

| Token | Who | Operation |
|-------|-----|-----------|
| `escrow.create` | payer + provider | Create/agree terms |
| `escrow.fund` | payer | Lock principal |
| `escrow.submit_receipt` | provider | Submit settlement receipt |
| `escrow.release` | payer or provider | Execute release after rules |
| `escrow.refund` | payer (cooperative) | Refund before release |
| `escrow.dispute` | payer or provider | Raise dispute |
| `escrow.resolve` | resolver participant | Resolve dispute locally |
| `escrow.cancel` | both parties | Cancel unfunded escrow |

No identity-only authority. Escrow ID possession is insufficient without capability + signature.

### 7.2 Resolver trust model (P2-DEC-009)

PROTO-2 uses a **trusted local resolver function** in the simulator: an authorised participant presenting `escrow.resolve` capability. This is not a distributed arbitrator. Resolution rule: valid bound receipt present → `ResolvedReleased`; else → `ResolvedRefunded` if timeout or invalid receipt.

---

## 8. Timeout and dispute model

### 8.1 Timeouts (logical `now`)

| Deadline | Effect |
|----------|--------|
| `fund_before` | Unfunded escrow → `Cancelled` or reject fund |
| `receipt_before` | No valid receipt → `Expired` → refund path |
| `dispute_window` after receipt | Blocks immediate release (mirrors PROTO-1 pattern) |

### 8.2 Dispute

- Admissible raise: participant with `escrow.dispute`; escrow in `Funded`, `ReceiptAccepted`, or `ReleasePending`
- Evidence: signed receipt and/or challenge record
- Resolution: local deterministic rule; no chain submission
- Must not weaken PROTO-1-style fail-closed invalid evidence rejection

---

## 9. H4 and H5 measurement

### 9.1 H4 — Receipt-based escrow

**Scenarios tested:**

| Scenario | Success criterion |
|----------|-------------------|
| Happy path | fund → receipt → release |
| Timeout | fund → no receipt → refund |
| Invalid receipt | rejected; no release |
| Dispute with valid receipt | resolved release |
| Dispute without valid receipt | resolved refund |

**Would falsify H4 (prototype):** high wrongful-release rate in adversarial suite; agents require non-deterministic proofs even in bounded tasks (documented, not auto-kill).

**Not claimed:** real market sufficiency; passing tests ≠ production escrow safety.

### 9.2 H5 — Autonomous fee budgeting

**Model:**

1. `FeeQuoteV0` at create
2. `max_protocol_fee` cap in terms
3. Reserve on fund (`fee_reserved`)
4. Consume on terminal ops (`fee_consumed`)
5. Reject if `quoted_fee > max_protocol_fee` or insufficient payer budget
6. `FeeLedgerEntryV0` audit per operation

**Metrics:** loop completion without budget exhaustion; stuck-funds count from fee mis-estimation (simulated).

**Not claimed:** real fee markets or token economics.

---

## 10. Security invariant mapping

| # | Invariant | Coverage | Enforcement | Failure |
|---|-----------|----------|-------------|---------|
| 1 | Capability envelope | Covered | PROTO-0 on every op | `CapabilityDenied` |
| 2 | Delegation narrowing | Covered | Inherited PROTO-0 | reject grant |
| 3 | Expiry/revoke | Covered | PROTO-0 + time rules | reject |
| 4 | No identity-only settlement | Covered | grant required | `CapabilityDenied` |
| 5 | Signature ≠ truth | Covered | receipt verifies sig only | no quality claim |
| 6 | Identity continuity | Partial | AgentId binding on terms | mismatch reject |
| 7 | Deterministic transitions | Covered | state machine tests | reject invalid |
| 8 | Dispute reproducibility | Covered | deterministic resolve rule | same inputs → same outcome |
| 9 | Commitment ≠ disclosure | Covered | `output_commitment` only | no raw payload required |
| 10 | Backend isolation | Covered | no backend in harness | N/A |
| 11 | Capability before economic auth | Covered | pipeline order | reject before transition |
| 12 | Delegation bounds | Covered | inherited PROTO-0 | reject |

---

## 11. Open questions

Resolved or provisional-locked in [PROTO_2_DECISIONS.md](PROTO_2_DECISIONS.md). Implementation must not proceed on unresolved Locked items.

---

## 12. PROTO-2 exit criteria

See [PROTO_2_ACCEPTANCE_TESTS.md](PROTO_2_ACCEPTANCE_TESTS.md) §9.

Summary:

- All frozen acceptance tests pass
- `cargo test` green; clippy `-D warnings`
- Every economic op capability-gated
- Deterministic transitions; value conservation; no double release/refund
- Receipt replay rejected; terminal outcomes mutually exclusive
- H4/H5 evidence recorded without overclaim
- `SECURITY_MODEL.md` reviewed after implementation
- No live network/token/chain/real-money code

---

## 13. Implementation notes (non-normative)

- New module: `Aether/core/src/escrow/` (not started)
- Reuse: `authorise_action`, `SignedMessage`, CBOR helpers, error taxonomy
- Store pattern: mirror PROTO-1 `EscrowStore` with `insert_record` only
- Do not modify PROTO-1 channel semantics to accommodate escrow

---

## Freeze statement

> PROTO-2 design and acceptance tests are **frozen** for review. Implementation requires explicit approval after design review.

Changes require edits to this file, `PROTO_2_DECISIONS.md`, and `PROTO_2_ACCEPTANCE_TESTS.md` with recorded rationale.
