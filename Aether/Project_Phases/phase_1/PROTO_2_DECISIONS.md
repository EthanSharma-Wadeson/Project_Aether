# PROTO_2_DECISIONS.md — Phase 1

## Status

**Design resolution record — not implementation**

Resolves open questions from [PROTO_2_DESIGN.md](PROTO_2_DESIGN.md) so acceptance tests and later coding are deterministic.

Owner: Project Lead  
Date: 2026-07-28

Status meanings:

| Status | Meaning |
|--------|---------|
| **Locked** | Accepted for PROTO-2 v0 simulator; change requires explicit decision revision |
| **Provisional** | Working rule for PROTO-2; review after tests / metrics |
| **Deferred** | Intentionally out of PROTO-2 |

PROTO-0 and PROTO-1 assumptions are preserved. PROTO-1 dispute-chain and terminal-auth remediations must not be weakened.

---

## Decision index

| ID | Question | Status | Confidence |
|----|----------|--------|------------|
| P2-DEC-001 | Escrow state ownership / store model | Locked | High |
| P2-DEC-002 | PROTO-1 integration model | Locked | High |
| P2-DEC-003 | Terms agreement / creation quorum | Locked | High |
| P2-DEC-004 | Settlement receipt signer model | Locked | High |
| P2-DEC-005 | Receipt-to-escrow binding | Locked | High |
| P2-DEC-006 | Release authority model | Provisional | Medium-High |
| P2-DEC-007 | Refund authority model | Locked | Medium-High |
| P2-DEC-008 | Timeout semantics | Locked | High |
| P2-DEC-009 | Dispute resolution / resolver trust | Provisional | Medium |
| P2-DEC-010 | v0 escrow capability action tokens | Locked | High |
| P2-DEC-011 | Economic finality definition | Locked | High |
| P2-DEC-012 | Fee-budget accounting model | Provisional | Medium |
| P2-DEC-013 | H4 success criteria (prototype) | Provisional | Medium |
| P2-DEC-014 | H5 success criteria (prototype) | Provisional | Medium |

---

## P2-DEC-001 — Escrow state ownership

**Question:** Where does escrow state live and how is it mutated?

**Chosen approach (Locked):**

- In-memory `EscrowStore` in `aether-core` (PROTO-2 module)
- External read via `get(escrow_id)` only
- Mutations only through transition functions (`insert_record` crate-internal)
- No public `get_mut`

**Reasoning:** Mirrors PROTO-1 remediation; prevents bypass of transition rules.

**Security implications:** Terminal and economic transitions cannot be skipped via direct store mutation.

**Review trigger:** First networked persistence design (Phase 2+).

---

## P2-DEC-002 — PROTO-1 integration model

**Question:** Does escrow use channels, a separate ledger, or both?

**Chosen approach (Locked): Option B — separate in-memory escrow ledger**

PROTO-2 escrow funding, receipts, and outcomes are **independent** of PROTO-1 channel balances.

**Alternatives considered:**

| Option | Verdict |
|--------|---------|
| A — channel-only | Rejected: couples escrow correctness to channel; expands scope |
| B — separate ledger | **Chosen** |
| C — both | Rejected for PROTO-2: unnecessary duplication |

**Conceptual reuse:** `EconomicFinalityViewV0` fields align with PROTO-1 `FinalityViewV0` semantics.

**Deferred:** Optional escrow funding from channel snapshot (post-PROTO-2 integration spike).

**Review trigger:** If H4 tests show escrow/channel split confuses agent budgeting loops.

---

## P2-DEC-003 — Terms agreement / creation quorum

**Question:** Who must sign escrow creation?

**Chosen approach (Locked):**

Both payer and provider must dual-sign identical canonical `EscrowTermsV0` body before `Proposed` status is recorded.

Message type: `escrow.create`.

**Reasoning:** Bilateral agreement on amount, deadlines, and claim profile before funding.

**Security implications:** Prevents unilateral escrow terms imposed on an unknowing counterparty.

**Review trigger:** Asymmetric hire flows requiring delegated creation (PROTO-3+).

---

## P2-DEC-004 — Settlement receipt signer model

**Question:** Who signs `SettlementReceiptV0`?

**Chosen approach (Locked):**

- **Provider only** signs the settlement receipt (`escrow.submit_receipt`)
- Payer does **not** co-sign the receipt body
- Payer may dispute or withhold release via separate authorised operations

**Reasoning:** Receipt is a claimant assertion. Payer protection comes from terms, timeout refund, and dispute — not co-signing provider claims.

**Security implications:** Preserves invariant 5. Signature proves provider signed bounded bytes, not correctness.

**Review trigger:** If H4 shows excessive wrongful release in adversarial tests.

---

## P2-DEC-005 — Receipt-to-escrow binding

**Question:** How is a receipt bound to an escrow?

**Chosen approach (Locked):**

Receipt must include:

- `escrow_id` = `SHA-256(canonical_cbor(EscrowTermsV0))`
- `terms_version` matching live escrow
- `payer` / `provider` matching terms
- `receipt_nonce` strictly greater than any prior accepted nonce for that escrow

Verification order: signature → capability → field binding → terms match → nonce → timeout.

**Security implications:** Prevents cross-escrow replay and wrong-escrow submission.

**Review trigger:** Multi-escrow batch flows.

---

## P2-DEC-006 — Release authority model

**Question:** Is release automatic after valid receipt?

**Chosen approach (Provisional):**

1. Valid receipt → status `ReceiptAccepted`
2. Optional `dispute_window` must elapse (if non-zero)
3. **Either** payer **or** provider may invoke `escrow.release` with valid capability
4. No payer co-signature on receipt; release is a separate authorised operation

**Alternatives:**

| Option | Verdict |
|--------|---------|
| Auto-release immediately on receipt | Rejected: no dispute window |
| Payer-only release | Rejected: provider stuck if payer unresponsive post-receipt |
| Dual-signed release | Deferred: adds friction; revisit if H4 fails |

**Review trigger:** H4 wrongful-release metrics; payer griefing scenarios.

---

## P2-DEC-007 — Refund authority model

**Question:** Who may refund?

**Chosen approach (Locked):**

| Path | Authority |
|------|-----------|
| Cooperative refund (pre-release) | payer `escrow.refund` |
| Timeout (`receipt_before` elapsed, no valid receipt) | either party `escrow.refund` with timeout proof |
| Post-dispute refund | resolver `escrow.resolve` → `ResolvedRefunded` |

Provider cannot unilaterally refund funded escrow to themselves.

**Review trigger:** Multiparty escrow (deferred).

---

## P2-DEC-008 — Timeout semantics

**Question:** How do logical timeouts work?

**Chosen approach (Locked):**

Harness-supplied logical `now` (inherited PROTO-0 assumption):

| Deadline | Rule |
|----------|------|
| `fund_before` | Reject fund if `now > fund_before`; allow cancel |
| `receipt_before` | Reject receipt if `now > receipt_before`; allow expire/refund path |
| `dispute_window` | Block `escrow.release` until `now >= receipt_accepted_at + dispute_window` |

**Security implications:** Time integrity is a trusted-local assumption, not network-secure.

**Review trigger:** First networked deployment; NTP/wall-clock policy.

---

## P2-DEC-009 — Dispute resolution / resolver trust

**Question:** Who resolves disputes in PROTO-2?

**Chosen approach (Provisional):**

- **Trusted local resolver:** an authorised participant (payer or provider) presenting `escrow.resolve` capability
- **Deterministic rule:**
  - If admissible `SettlementReceiptV0` verifies and matches terms → `ResolvedReleased`
  - Else if timeout or invalid/missing receipt → `ResolvedRefunded`
- No external oracle, committee, or AI adjudication

**Alternatives considered:** payer-only resolver (rejected: provider lock-out); third-party resolver (deferred).

**Security implications:** Dispute fairness is not production-grade; reproducibility is local only (invariant 8 partial).

**Review trigger:** PROTO-3+ escrow disputes; first watchtower design.

---

## P2-DEC-010 — v0 escrow capability action tokens

**Question:** What capability action strings are used?

**Chosen approach (Locked):**

```text
escrow.create
escrow.fund
escrow.submit_receipt
escrow.release
escrow.refund
escrow.dispute
escrow.resolve
escrow.cancel
```

Follows PROTO-1 `channel.*` namespacing pattern.

**Review trigger:** Capability registry consolidation across prototypes.

---

## P2-DEC-011 — Economic finality definition

**Question:** What is “final” in the simulator?

**Chosen approach (Locked):**

Terminal statuses: `Released`, `Refunded`, `ResolvedReleased`, `ResolvedRefunded`, `Cancelled`, `Expired` (when mapped to refund complete).

`EconomicFinalityViewV0.finalized = true` only on terminal statuses.

`hard_settlement_placeholder = false` always in PROTO-2.

**Reasoning:** Parallel PROTO-1 H6 lesson — agents must not confuse local simulator finality with chain finality.

---

## P2-DEC-012 — Fee-budget accounting model

**Question:** How are fees represented?

**Chosen approach (Provisional):**

- Abstract value units (same simulated asset label as escrow principal)
- `FeeQuoteV0` at create; `max_protocol_fee` cap in terms
- Reserve `fee_reserved` on fund; consume `fee_consumed` on terminal ops
- Payer simulated balance debited: `principal + fee_reserved` on fund
- Insufficient budget → reject fund/release
- `FeeLedgerEntryV0` audit log per fee event

**No token.** No external fee market.

**Review trigger:** First real backend fee samples (Phase 2+).

---

## P2-DEC-013 — H4 success criteria (prototype)

**Question:** What does PROTO-2 need to show for H4?

**Chosen approach (Provisional):**

Record in `PROTO_2_RESULTS.md`:

- Happy-path completion rate (fund→receipt→release)
- Timeout refund success rate
- Wrongful-release count in adversarial suite (target: 0)
- Invalid-receipt rejection rate (target: 100%)

**Kill criterion (documentation only):** sustained wrongful-release in adversarial tests or inability to complete bounded deterministic flows without stronger proofs.

**Not claimed:** real market sufficiency.

---

## P2-DEC-014 — H5 success criteria (prototype)

**Question:** What does PROTO-2 need to show for H5?

**Chosen approach (Provisional):**

Record:

- Full loop completion without manual fee intervention
- Count of operations rejected for insufficient fee budget
- Fee quote vs consumed delta within `max_protocol_fee`
- Stuck-funds incidents attributable to fee mis-estimation

**Kill criterion (documentation only):** fee variance forces human intervention as the normal case.

---

## Dependencies

| Decision | Depends on |
|----------|------------|
| P2-DEC-004–006 | P2-DEC-003 terms shape |
| P2-DEC-009 | P2-DEC-004 receipt model |
| P2-DEC-012 | P2-DEC-003 `max_protocol_fee` |
| All | PROTO-0 authority pipeline unchanged |

---

## Freeze statement

> PROTO-2 decisions are **frozen** for acceptance-test lock. Locked items require explicit revision to change.

Implementation must target this record; the record must not be silently weakened to make tests pass.
