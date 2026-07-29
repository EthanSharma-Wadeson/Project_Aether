# PROTO_4_DESIGN.md — Settlement Binding Layer

## Status

**Design complete + acceptance tests frozen — implementation + remediation complete; security: APPROVE WITH DOCUMENTED LIMITATIONS**

Wedge: [PHASE_2_WEDGE_DECISION.md](PHASE_2_WEDGE_DECISION.md) — **Enterprise Agent Spend Control**.

PROTO-4 defines how Aether **economic decisions** bind to **external settlement systems** without creating a cryptocurrency, token, or public chain.

```text
Aether economic decision (PROTO-2 terminal outcome)
        │
        ▼
Settlement binding (PROTO-4)
        │
        ▼
External payment / ledger / banking adapter
```

References:

- [CONSENSUS_AND_SETTLEMENT.md](../../Aether_docs/CONSENSUS_AND_SETTLEMENT.md) §2, §6–7, §12
- [PROTO_2_DESIGN.md](../phase_1/PROTO_2_DESIGN.md) — `EconomicFinalityViewV0`
- [PROTO_NET_0_SECURITY_REVIEW.md](PROTO_NET_0_SECURITY_REVIEW.md)
- [PROTO_4_DECISIONS.md](PROTO_4_DECISIONS.md)

**Stop:** No Rust until explicit implementation approval. Acceptance tests frozen in [PROTO_4_ACCEPTANCE_TESTS.md](PROTO_4_ACCEPTANCE_TESTS.md).

---

## 1. Purpose

PROTO-2 proves local escrow rules and sets `EconomicFinalityViewV0.finalized = true` on terminal simulator outcomes while keeping:

```text
hard_settlement_placeholder = false
```

PROTO-4 answers:

> How does an authorised Aether terminal outcome become a verifiable request against an external ledger — and how do we distinguish **Aether protocol finality** from **external settlement finality**?

### In scope

- `SettlementAccountBindingV0` — `AgentId` ↔ external account (capability-gated)
- `SettlementBindingV0` — escrow outcome ↔ external settlement attempt/record
- `SettlementAdapterV0` — provider-agnostic adapter interface
- Settlement lifecycle / finality stages
- Evidence commitments and disagreement handling
- Enterprise spend-control requirements
- Threat model classification for settlement

### Out of scope / non-goals

- Cryptocurrency, tokens, token economics
- Public blockchain deployment
- Marketplace, reputation, autonomous open economy
- Live banking integrations (design only; first spike later uses sandbox/stub)
- Weakening PROTO-0/1/2/NET-0 semantics

---

## 2. Authority Split — Who Knows What?

These five questions are normative for PROTO-4.

| # | Question | Answer |
|---|----------|--------|
| 1 | **What does Aether know?** | Identities, capabilities, escrow terms/status, terminal economic outcome (`released` / `refunded`), binding requests, adapter responses **as reported**, evidence commitments, and machine-readable finality **views** |
| 2 | **What does the external system know?** | Accounts, balances/holds, payment intents, ledger entries, its own confirmation IDs and finality rules — **not** Aether capability trees or escrow dispute rules unless explicitly mirrored |
| 3 | **Which system is authoritative for each state?** | See §5 authority matrix |
| 4 | **How are disagreements handled?** | Fail closed; do not set hard settlement; escalate to `Disputed` / `Failed` settlement status; never invent funds in Aether to “match” a backend (§6) |
| 5 | **What evidence proves settlement occurred?** | Adapter-returned external reference + signed Aether `SettlementBindingV0` binding that reference to a specific `escrow_id` + outcome + evidence commitment; backend inclusion proof **when adapter provides one** (§4.4) |

### Authority matrix

| State / fact | Authoritative system |
|--------------|----------------------|
| Agent identity & capability | Aether PROTO-0 |
| Escrow lifecycle & conservation (protocol) | Aether PROTO-2 |
| Soft / simulator economic finality | Aether `EconomicFinalityViewV0` |
| External account balances & payment success | External provider / ledger |
| Mapping AgentId → external account | Aether `SettlementAccountBindingV0` (must be capability-authorised) |
| Mapping escrow outcome → settlement attempt | Aether `SettlementBindingV0` |
| `hard_settlement_placeholder` / settlement-confirmed | **Only** after adapter reports confirmed inclusion **and** Aether verifies binding integrity — never from Aether alone |
| Legal / accounting finality | Out of protocol (enterprise policy / law) |

---

## 3. Minimal Data Models

All protocol-critical objects use schema-locked canonical CBOR, `protocol_version` / `schema_version`, and DEC-004B signing where marked.

### 3.1 `SettlementAccountBindingV0` (protocol-critical)

Links an Aether agent to an external settlement account. Aligns with CONSENSUS_AND_SETTLEMENT §12.

| Field | Type | Purpose |
|-------|------|---------|
| `protocol_version` / `schema_version` | u32 | Wire versions |
| `binding_id` | bytes[32] | `SHA-256(canonical body without binding_id)` |
| `agent_id` | AgentId | Aether identity |
| `settlement_provider` | text | Adapter id, e.g. `enterprise.ledger.v0`, `payments.sandbox.v0` |
| `external_account_ref` | text | Opaque provider account / ledger code |
| `asset` | text | Asset label (e.g. `USD`, `AETHER_TEST`) |
| `scope` | text | e.g. `escrow.settle`, `spend` |
| `valid_after` / `valid_before` | u64 | Logical validity window |
| `created_at` | u64 | Logical time |

**Authority:** Creating/updating requires PROTO-0 capability (proposed token: `settlement.bind`).  
**Not:** proof of KYC, solvency, or banking relationship quality.

### 3.2 `SettlementBindingV0` (protocol-critical)

Binds a **specific PROTO-2 terminal (or settleable) escrow outcome** to an **external settlement attempt**.

| Field | Type | Purpose |
|-------|------|---------|
| `protocol_version` / `schema_version` | u32 | |
| `binding_id` | bytes[32] | Commitment over canonical fields below (excluding mutable status/history) |
| `escrow_id` | bytes[32] | PROTO-2 escrow |
| `terms_version` | u64 | Must match escrow terms |
| `economic_outcome` | enum | `release_to_provider` \| `refund_to_payer` \| `fee_only` (extensible) |
| `principal_amount` | u64 | Abstract units mirrored from escrow |
| `fee_amount` | u64 | Fee to consume/settle if any |
| `asset` | text | Must match escrow asset |
| `payer_agent_id` / `provider_agent_id` | AgentId | From escrow terms |
| `settlement_provider` | text | Adapter id |
| `payer_account_binding_id` | bytes[32] | Ref to `SettlementAccountBindingV0` |
| `provider_account_binding_id` | optional bytes[32] | Required for release path |
| `external_settlement_ref` | optional text | Provider reference once submitted |
| `settlement_status` | enum | See §5 |
| `aether_escrow_status` | text | Snapshot of PROTO-2 status at bind time |
| `requested_at` | u64 | Logical time |
| `submitted_at` / `accepted_at` / `confirmed_at` / `finalized_at` | optional u64 | Lifecycle stamps |
| `evidence_commitment` | bytes[32] | `SHA-256` of evidence package (§4.4) |
| `adapter_receipt_commitment` | optional bytes[32] | Commitment to adapter response bytes |
| `correlation_id` | bytes[32] | Idempotency key for adapter calls |

**Invariant:** One active settlement binding per `(escrow_id, economic_outcome)` unless prior binding is `Failed` / `Cancelled` (no silent double settle).

### 3.3 `SettlementAdapterV0` (interface — design)

Provider-agnostic boundary. Implementations are **not** part of core protocol semantics.

```text
SettlementAdapterV0
  provider_id() -> text
  capabilities() -> { supports_hold, supports_release, supports_refund, ... }

  validate_account(account_ref) -> Result
  request_settlement(SettlementBindingV0) -> AdapterSubmitResult
  query_settlement(external_ref) -> AdapterStatusResult
  cancel_settlement(external_ref) -> Result   // if supported
```

| Adapter family (examples) | Role in enterprise wedge |
|---------------------------|--------------------------|
| `enterprise.ledger.v0` | Internal cost centre / virtual wallet (preferred first spike) |
| `payments.sandbox.v0` | Sandbox payment API (second candidate) |
| `banking.partner.v0` | Deferred — licensed partner |
| `chain.stablecoin.v0` | Deferred — not required for enterprise wedge |

**Rule:** Core Aether must not import provider SDKs into protocol validation. Adapters live behind the interface; protocol verifies **shape and commitments** of responses, not provider business logic.

### 3.4 Evidence package (unsigned wrapper, simulator / audit)

| Field | Purpose |
|-------|---------|
| `settlement_binding_id` | |
| `escrow_terminal_commitment` | Hash of canonical escrow snapshot at bind |
| `capability_grant_ids` | Grants used to authorise settle request |
| `adapter_response_bytes` | Raw or canonical adapter payload (may be redacted in exports) |
| `logical_time` | |

`evidence_commitment = SHA-256(canonical_cbor(evidence fields used for binding))`.

---

## 4. Settlement Lifecycle & Finality

### 4.1 Settlement status machine (`SettlementBindingV0.settlement_status`)

```text
Requested
    → Submitted      (adapter accepted request; external_ref may be set)
    → Accepted       (provider acknowledges intent / hold)
    → Confirmed      (provider reports funds moved / ledger posted)
    → Finalized      (Aether records settlement-final alignment)

Branches:
    * → Failed
    * → Cancelled     (before Confirmed, if adapter allows)
    Confirmed/Finalized → DisputedExternal  (adapter later reverses / conflict)
```

Exact transition table:

| From | To | Trigger | Fail closed if |
|------|-----|---------|----------------|
| — | Requested | Authorised settle request; escrow terminal/settleable | Missing binding accounts; wrong outcome; duplicate |
| Requested | Submitted | Adapter returns submit ok + ref | Adapter error |
| Submitted | Accepted | Adapter status = accepted/held | Unknown ref |
| Accepted | Confirmed | Adapter status = confirmed/posted | Partial amount mismatch |
| Confirmed | Finalized | Aether verifies binding + updates finality view | Evidence missing |
| any pre-Confirmed | Failed | Adapter failure / timeout policy | — |
| any pre-Confirmed | Cancelled | Authorised cancel | Already confirmed |
| Confirmed/Finalized | DisputedExternal | Adapter reversal or conflicting query | — |

### 4.2 Relationship to PROTO-2 `EconomicFinalityViewV0`

| Field | PROTO-2 today | PROTO-4 rule |
|-------|---------------|--------------|
| `finalized` | true on escrow terminal | Unchanged meaning: **Aether escrow rules completed** |
| `hard_settlement_placeholder` | always `false` | Remains **`false` until** settlement status ≥ `Confirmed` **and** binding verification succeeds |
| `outcome` | released / refunded / none | Must match `SettlementBindingV0.economic_outcome` |
| Soft agreement | local rules satisfied | Does **not** imply external money moved |

**Normative:**

```text
Aether escrow finalized  ≠  external settlement confirmed
hard_settlement_placeholder == true  only after external confirmation + verified binding
```

Never set `hard_settlement_placeholder = true` from escrow terminal alone.

### 4.3 Alignment with CONSENSUS multi-stage finality

Map conceptual stages (CONSENSUS §6) onto PROTO-4 + PROTO-2:

| CONSENSUS stage | PROTO-4 / PROTO-2 meaning |
|-----------------|---------------------------|
| PROPOSED | Settlement binding `Requested` |
| ACCEPTED | Escrow terminal accepted in Aether **or** settlement `Accepted` (expose both distinctly) |
| SETTLEMENT_FINAL | Settlement status `Confirmed` / `Finalized` per backend assumptions |
| DISPUTE_WINDOW_FINAL | PROTO-2 dispute window elapsed (escrow path); independent of bank chargeback windows |
| ECONOMIC_FINAL | Escrow terminal **and** settlement `Finalized` **and** no open `DisputedExternal` |

Agents must read machine-readable fields; they must not infer finality from prose.

### 4.4 What proves settlement occurred?

Minimum evidence set for `hard_settlement_placeholder = true`:

1. Valid PROTO-2 terminal escrow consistent with `economic_outcome`  
2. Valid `SettlementBindingV0` with matching `escrow_id`, amounts, agents  
3. Non-empty `external_settlement_ref`  
4. Adapter query (or signed adapter receipt) reporting **Confirmed** (or provider-equivalent)  
5. Matching `evidence_commitment` / `adapter_receipt_commitment`  
6. Idempotent `correlation_id` recorded (no second success for same key)

**Does not prove:** legal discharge, absence of future chargebacks, adapter honesty if the adapter is compromised (see §7).

---

## 5. Pipeline & Capability Tokens

### 5.1 Validation order for settle request

```text
1. Verify SignedMessage (settle intent) if signed
2. PROTO-0 identity Active
3. PROTO-0 capability (settlement.settle / settlement.bind)
4. PROTO-2 escrow status + outcome consistency
5. Account bindings valid + provider match
6. Idempotency / duplicate binding checks
7. Call adapter (side effect) OR record Requested for async submit
8. Mutate SettlementStore only through transition functions
```

### 5.2 Proposed action tokens (P4-DEC)

| Token | Who | Operation |
|-------|-----|-----------|
| `settlement.bind` | Principal / treasury agent | Create/update `SettlementAccountBindingV0` |
| `settlement.settle` | Payer or authorised settler | Request settlement for escrow outcome |
| `settlement.query` | Participant / auditor | Refresh status from adapter |
| `settlement.cancel` | Authorised party | Cancel pre-confirmed settlement if supported |

No identity-only settlement. Possession of `escrow_id` is insufficient.

---

## 6. Disagreement & Failure Handling

| Scenario | Aether behaviour |
|----------|------------------|
| Adapter says success, escrow not terminal | **Reject** — do not bind |
| Escrow terminal, adapter fails | Binding → `Failed`; escrow remains terminal; **no** hard settlement; retry policy is operational |
| Adapter confirms wrong amount | → `Failed` or `DisputedExternal`; never mutate PROTO-2 conservation to “fix” |
| Duplicate settle with same `correlation_id` | Return existing binding; no second debit |
| Duplicate settle with new correlation for same escrow+outcome | **Reject** if prior binding Confirmed/Finalized/Submitted-in-flight |
| Adapter later reverses after Finalized | → `DisputedExternal`; set `hard_settlement_placeholder = false`; emit audit event; do not silently rewrite escrow |
| Query timeout | Leave status unchanged; fail closed on promoting finality |
| Partial settlement (provider paid, fee not) | Treat as failure unless adapter atomicity guaranteed; record evidence |

**Principle:** Aether does not create value to reconcile with a lying or broken adapter.

---

## 7. Security Model (Settlement Threats)

| Threat | Status | Notes |
|--------|--------|-------|
| Duplicate settlement | **Provisional** | Design: correlation_id + one active binding per escrow/outcome; tests at implementation |
| False settlement confirmation | **Open** | Trusted adapter assumption for spike; production needs attestation / multi-query / custodian SLA |
| Settlement replay | **Provisional** | Idempotency keys + binding_id; mirror NET-0 replay lessons |
| Incorrect escrow binding | **Provisional** | Field binding to escrow_id, terms_version, amounts, agents |
| Adapter compromise | **Open** | Host/provider trust; out of PROTO-4 crypto scope |
| Provider failure / timeout | **Provisional** | Failed status; no hard settlement |
| Partial settlement | **Provisional** | Reject non-atomic results unless adapter declares atomic |
| Capability bypass | **Validated** (by PROTO-0 reuse intent) | Must call `authorise_action`; evidenced when implemented |
| Setting hard finality from Aether alone | **Provisional** | Explicitly forbidden in this design |

---

## 8. Enterprise Requirements Mapping

| Enterprise need | PROTO-4 contribution |
|-----------------|----------------------|
| AI agent spending controls | Settle only under `settlement.settle` + PROTO-0 spend/action caps; escrow bounds principal |
| Approval policies | Principals issue short-lived grants; revoke/freeze stops new settle requests |
| Audit trails | Canonical CBOR bindings + evidence commitments + adapter refs exportable to GRC |
| Compliance review | Clear split: protocol decision vs external ledger confirmation |
| Financial governance | Cost-centre accounts via `SettlementAccountBindingV0`; FinOps reconciles using `external_settlement_ref` |

**Demo narrative (post-implementation):**

```text
Grant (max_spend) → Agent opens/funds escrow → receipt/release
  → SettlementBinding Requested→…→Confirmed
  → hard_settlement_placeholder true
  → Revoke grant → further settle rejected
```

---

## 9. Store & Isolation Rules (non-normative for implementers)

- `SettlementStore` — crate-internal mutation only (`insert_record`); no public `get_mut`
- Must **not** mutate PROTO-1 `ChannelStore`
- May **read** PROTO-2 escrow; promote hard-finality fields only via defined transition
- Adapters invoked only after capability checks
- First spike adapter: in-process `enterprise.ledger.v0` stub with deterministic ledger — still behind `SettlementAdapterV0`

---

## 10. Relationship to Other Prototypes

| Prototype | Relation |
|-----------|----------|
| PROTO-2 | Source of economic outcomes; soft finality |
| PROTO-0 | Authority for bind/settle |
| PROTO-NET-0 | Optional carrier for settle messages later; not required for single-host enterprise spike |
| PROTO-3 | Out of scope |
| PROTO-1 | Unchanged; channels not required for PROTO-4 wedge |

---

## 11. Open Questions → PROTO_4_DECISIONS.md

See P4-DEC-001 … P4-DEC-010.

---

## 12. Exit Criteria for Design Gate

Design gate passes when:

1. This document answers the five authority questions (§2)  
2. [PROTO_4_DECISIONS.md](PROTO_4_DECISIONS.md) records locked/provisional decisions  
3. Explicit review accepts design  
4. Acceptance tests frozen in a follow-on `PROTO_4_ACCEPTANCE_TESTS.md`  
5. **Then** implementation may begin  

---

## Freeze Statement

> PROTO-4 design is ready for review. It does not authorise Rust, live adapters, banking integrations, tokens, or blockchain deployment.
