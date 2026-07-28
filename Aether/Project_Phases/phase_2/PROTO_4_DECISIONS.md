# PROTO_4_DECISIONS.md — Settlement Binding Decisions

## Status

**Design resolution record — not implementation**

Resolves open questions from [PROTO_4_DESIGN.md](PROTO_4_DESIGN.md) for the Enterprise Agent Spend Control wedge.

Owner: Project Lead  
Date: 2026-07-28

| Status | Meaning |
|--------|---------|
| **Locked** | Accepted for PROTO-4 v0 design; change requires explicit revision |
| **Provisional** | Working rule; revisit after spike / acceptance tests |
| **Deferred** | Intentionally out of PROTO-4 |

Preserves: settlement-agnostic architecture ([CONSENSUS_AND_SETTLEMENT.md](../../Aether_docs/CONSENSUS_AND_SETTLEMENT.md)), PROTO-0 capability gates, PROTO-2 soft≠hard finality.

---

## Decision Index

| ID | Question | Status | Confidence |
|----|----------|--------|------------|
| P4-DEC-001 | Settlement abstraction model | Locked | High |
| P4-DEC-002 | Adapter boundary | Locked | High |
| P4-DEC-003 | First spike adapter family | Provisional | Medium-High |
| P4-DEC-004 | Finality semantics / hard placeholder | Locked | High |
| P4-DEC-005 | Binding object split (account vs escrow settle) | Locked | High |
| P4-DEC-006 | Evidence requirements | Provisional | Medium-High |
| P4-DEC-007 | Failure & disagreement handling | Locked | High |
| P4-DEC-008 | Idempotency / duplicate settlement | Locked | High |
| P4-DEC-009 | Capability action tokens | Provisional | Medium-High |
| P4-DEC-010 | Async vs sync adapter calls | Provisional | Medium |
| P4-DEC-011 | Relationship to PROTO-NET-0 | Locked | High |
| P4-DEC-012 | Acceptance-test freeze gate | Locked | High |

---

## P4-DEC-001 — Settlement abstraction model

**Question:** How does Aether relate to external money movement?

**Chosen (Locked):** Settlement-agnostic protocol with pluggable `SettlementAdapterV0`.

Aether defines bindings, evidence commitments, and finality **views**. External systems remain authoritative for account balances and payment success.

**Alternatives rejected for PROTO-4:** native L1; single hard-coded provider in core; treating PROTO-2 simulator balances as real money.

**Aligns with:** DEC-P2-003 Option A; CONSENSUS §2.

---

## P4-DEC-002 — Adapter boundary

**Question:** What may core protocol depend on?

**Chosen (Locked):**

- Core validates Aether objects and **structured adapter results** (status, refs, amounts, commitments)
- Core does **not** embed payment SDKs, bank APIs, or chain clients
- Adapters implement `SettlementAdapterV0` outside protocol-critical validation where possible

**Security implication:** Compromised adapter can lie; PROTO-4 must not pretend otherwise (`hard_settlement` implies adapter trust for v0 spike).

---

## P4-DEC-003 — First spike adapter family

**Question:** Which adapter for the first enterprise wedge experiment?

**Chosen (Provisional):** `enterprise.ledger.v0` — deterministic in-process / sandbox enterprise ledger stub.

**Second candidate (not required for first green suite):** `payments.sandbox.v0`.

**Deferred:** live banking, production Stripe, public stablecoin L2.

**Rationale:** Matches Enterprise Spend Control; avoids crypto/token scope creep; fastest honest demo of soft vs hard finality.

**Review trigger:** Design partner demands fiat sandbox first.

---

## P4-DEC-004 — Finality semantics

**Question:** When may `hard_settlement_placeholder` become true?

**Chosen (Locked):**

| Condition | `hard_settlement_placeholder` |
|-----------|-------------------------------|
| Escrow terminal only (PROTO-2) | `false` |
| Settlement `Requested`…`Accepted` | `false` |
| Settlement `Confirmed` or `Finalized` **and** binding verified | may be `true` |
| `DisputedExternal` / adapter reversal | must return to `false` |

`EconomicFinalityViewV0.finalized` continues to mean **Aether escrow rules completed**, not bank finality.

**Machine-readable:** expose both escrow finality and `settlement_status` distinctly — never collapse into one boolean in APIs.

---

## P4-DEC-005 — Binding object split

**Question:** One object or two?

**Chosen (Locked):**

| Object | Role |
|--------|------|
| `SettlementAccountBindingV0` | `AgentId` → external account (CONSENSUS §12) |
| `SettlementBindingV0` | Escrow outcome → external settlement attempt/record |

Do not overload a single type for both concerns.

---

## P4-DEC-006 — Evidence requirements

**Question:** What evidence is required to promote hard settlement?

**Chosen (Provisional):** Minimum set in PROTO_4_DESIGN.md §4.4:

- terminal escrow consistency  
- `SettlementBindingV0` field integrity  
- `external_settlement_ref`  
- adapter Confirmed (or equivalent)  
- evidence / adapter receipt commitments  
- idempotent `correlation_id`  

**Not required for v0:** ZK proofs, multi-provider attestation, legal notarisation.

**Review trigger:** If false-confirmation risk blocks enterprise design partner.

---

## P4-DEC-007 — Failure & disagreement handling

**Question:** What if Aether and adapter disagree?

**Chosen (Locked):** Fail closed.

- Never mutate PROTO-2 conservation to match adapter  
- Never set hard settlement on adapter success without escrow consistency  
- On post-finalization reversal → `DisputedExternal` + clear hard flag + audit event  
- Partial non-atomic settlement → `Failed` unless adapter guarantees atomicity  

---

## P4-DEC-008 — Idempotency / duplicate settlement

**Question:** How to prevent double pay?

**Chosen (Locked):**

1. Client-supplied or derived `correlation_id` unique per settle intent  
2. At most one non-terminal/successful binding per `(escrow_id, economic_outcome)`  
3. Retry with same `correlation_id` returns existing binding (idempotent)  
4. New correlation after Confirmed/Finalized → reject  

---

## P4-DEC-009 — Capability action tokens

**Question:** Which PROTO-0 actions?

**Chosen (Provisional):**

| Token | Use |
|-------|-----|
| `settlement.bind` | Account bindings |
| `settlement.settle` | Create/submit settlement binding |
| `settlement.query` | Refresh from adapter |
| `settlement.cancel` | Cancel pre-confirmed |

All economically meaningful settle paths require `authorise_action` before adapter side effects.

**Review trigger:** Enterprise wants settle subsumed under `escrow.release` only — rejected: separates protocol release from external money movement.

---

## P4-DEC-010 — Async vs sync adapter calls

**Question:** Must settlement complete in one call?

**Chosen (Provisional):**

- Support async lifecycle: `Requested → Submitted → … → Finalized`  
- Sync adapters may skip intermediate statuses in one transition **recorded as multi-step audit** or collapse Submitted+Accepted+Confirmed in a single adapter response **only if** all evidence fields are present  
- Finality promotion still requires explicit Confirmed+verify step in Aether  

**Rationale:** Enterprise ledgers and payment APIs are often async.

---

## P4-DEC-011 — Relationship to PROTO-NET-0

**Question:** Is networking required for PROTO-4?

**Chosen (Locked):** **No** for the first enterprise spike.

Single-host (or single-process) settle path is sufficient. Network envelopes may carry settle intents later without changing binding semantics.

---

## P4-DEC-012 — Acceptance-test freeze gate

**Question:** When may Rust begin?

**Chosen (Locked):** Only after:

1. Design review acceptance of PROTO_4_DESIGN.md + this file  
2. Frozen `PROTO_4_ACCEPTANCE_TESTS.md` → **Done (2026-07-28)**  
3. Explicit implementation approval ← **blocking**

**Status:** Locked (tests frozen; implementation not authorised)

---

## Updates to Related Decisions

| Prior ID | Update |
|----------|--------|
| DEC-P2-003 | First spike preference: enterprise ledger stub; remain settlement-agnostic |
| DEC-P2-006 | Next build = PROTO-4 (unchanged by this file) |

---

## Freeze Statement

> PROTO-4 decisions are recorded for design review. Acceptance tests are **frozen** in [PROTO_4_ACCEPTANCE_TESTS.md](PROTO_4_ACCEPTANCE_TESTS.md). Implementation requires explicit approval. No tokens, public chains, or live banking integrations are authorised.
