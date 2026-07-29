# Enterprise Agent Spend Control — Demonstration Results

## Status

**All four scenarios pass** (expected outcomes met: 4/4)

Command:

```bash
cd Aether/demo && cargo run
```

Date: 2026-07-29

No protocol code (`PROTO-0`, `PROTO-1`, `PROTO-2`, `PROTO-NET-0`, `PROTO-4`) was modified for this demonstrator.

---

## Successful scenario

### Scenario 1 — Happy path

Stages completed: `[1]`–`[10]`

| Stage | Event | Protocol |
|-------|-------|----------|
| 1–3 | Enterprise, agent, provider identities active; delegated grant stored | PROTO-0 |
| 4–5 | Escrow created and funded (principal 1,000) | PROTO-2 |
| 6 | Provider receipt accepted | PROTO-2 |
| — | Enterprise releases escrow | PROTO-2 |
| 7 | Settlement requested | PROTO-4 |
| 8 | Mock adapter submit → accepted → confirmed | PROTO-4 |
| 9 | Finalize with fresh adapter report | PROTO-4 |
| 10 | Hard finality `TRUE` | PROTO-4 |

**Summary snapshot:**

```text
Escrow:       released
Settlement:   finalized
Soft finality: TRUE  (PROTO-2)
Hard finality: TRUE  (PROTO-4)
```

---

## Rejected scenarios (expected)

### Scenario 2 — Agent exceeds spend policy

| Field | Value |
|-------|-------|
| Rejection stage | `[7] Settlement requested` |
| Protocol | PROTO-0 |
| Error | `CapabilityDenied` |
| Cause | Agent delegated `max_spend` = 500, settlement principal = 1,000 |
| Hard finality | FALSE |

**Guarantee demonstrated:** spend caps are enforced at authorisation time before settlement mutation or adapter calls.

### Scenario 3 — Settlement adapter reverses

| Field | Value |
|-------|-------|
| Rejection stage | `[9] Hard settlement verified` |
| Protocol | PROTO-4 |
| Error | `InvalidSettlementEvidence` |
| Cause | Adapter marked reversed; finalize re-queries adapter and rejects stale cached report |
| Settlement status at end | `confirmed` (not finalized) |
| Hard finality | FALSE |

**Guarantee demonstrated:** external provider state cannot promote hard finality without fresh confirmed evidence at finalize time (P4-SEC-002 remediation).

### Scenario 4 — Receipt replay attack

| Field | Value |
|-------|-------|
| Rejection stage | `[6] Receipt accepted` (second submit) |
| Protocol | PROTO-2 |
| Error | `InvalidEscrowStatus` |
| Cause | Escrow already left `Funded` after first receipt; duplicate submission rejected |
| Hard finality | FALSE |

**Guarantee demonstrated:** PROTO-2 prevents duplicate receipt acceptance on a terminal receipt path. (Same-nonce replay on an open funded escrow returns `ReceiptReplay`; post-acceptance resubmit returns `InvalidEscrowStatus` — both are PROTO-2 rejections per `p2_r008`.)

---

## Protocol interactions (full wedge)

```text
PROTO-0
  register_bundle (enterprise, agent, provider)
  grant_capability (enterprise self, provider self, agent delegation)
  authorise_action on every mutating transition

PROTO-2
  create_escrow / fund_escrow
  submit_receipt / release_escrow
  economic finality view (soft) on release

PROTO-4
  bind_account (enterprise + provider ledger accounts)
  request_settlement / submit_settlement
  query_settlement (fresh adapter evidence)
  finalize_settlement → promote_verified_hard_settlement (crate-private)
```

---

## Demonstrated guarantees

| Guarantee | Mechanism | Scenario |
|-----------|-----------|----------|
| Agent cannot exceed delegated spend | PROTO-0 `max_spend` on `settlement.settle` | 2 |
| Funds are escrow-bound, not wallet-loose | PROTO-2 funded escrow + release rules | 1 |
| Receipt integrity / no double-claim | PROTO-2 status + nonce rules | 4 |
| Settlement requires validated escrow terminal state | PROTO-4 escrow cross-check | 1 |
| Adapter is evidence, not authority | PROTO-4 fresh query on finalize; reversal blocks hard | 3 |
| Hard finality is protocol-verified | Only `finalize_settlement` promotes hard flag | 1 |

---

## Known limitations

1. **Settlement signer** — Successful settlement in scenario 1 is payer-signed (enterprise). Agent delegation is proven by rejection in scenario 2, matching current PROTO-4 participant rules.
2. **Mock adapter only** — No real enterprise ERP, bank, or card rail.
3. **No PROTO-NET-0** — All actors are in-process; no networked agent session.
4. **Fixed seeds** — Deterministic identities (`seed = 42`) for reproducible output.
5. **Single asset** — `AETHER_TEST` test asset throughout.

---

## Next decision

This demonstrator is a **proof-of-value** exercise. The next step is a product/strategy decision: does this wedge convincingly validate Enterprise Agent Spend Control before any expansion (reputation, marketplace, second backend, production integrations)?

**STOP** at this boundary unless explicitly authorised to continue.
