# 05 — Payments & Escrow

**Key takeaway:** Agents pay each other via payment channels — fast off-chain updates with periodic on-chain settlement. Escrow ties payment to verified work. Soft finality is instant; hard finality takes longer.

---

## Why channels, not on-chain every payment?

| On-chain micropayment | Channel update |
|----------------------|----------------|
| High gas per transfer | Near-zero marginal cost |
| Seconds to minutes to confirm | Sub-second bilateral ack |
| Irrational for continuous metering | Designed for streaming micro-fees |

**Pattern:** High-frequency value transfer as authenticated state updates. Settlement plane finalizes opens, closes, and disputes.

---

## Architecture pattern

```
Agent A  ←── signed updates ──→  Agent B
    \                               /
     \_____ dispute / settle ______/
                    ↓
            Aether settlement plane
```

---

## Channel state (conceptual)

| Field | Purpose |
|-------|---------|
| Channel ID | Unique identifier |
| Parties | Agent IDs (bilateral first; multiparty later) |
| Asset | What's being transferred |
| Balances | Current allocation between parties |
| Sequence | Monotonic counter — higher wins in disputes |
| Dispute window | Time to challenge before finality |
| Status | Open → Active → Closing → Disputed → Settled |

Each update increments sequence and re-signs balances under authorized keys.

---

## Finality tiers (critical concept)

| Tier | Name | Speed | Use case |
|------|------|-------|----------|
| Soft | Channel update | Sub-second | Streaming micro-fees for ongoing work |
| Hard | Protocol settlement | Slower | Withdrawal, accounting, dispute resolution |

**Agents must price risk:** Uncooperative counterparties force dispute latency. APIs must expose which tier applies.

---

## Micro-fractional support

- Asset denominations allow very small units
- Fee schedules don't make dust payments irrational
- Settlements batch across many channels where possible

---

## Escrow — the default "agent hires agent" pattern

```
1. Lock value conditioned on attestation success
2. Release to provider OR refund to consumer per rules
3. Feed outcomes to reputation evidence
```

### Escrow flow
```
Offer → Accepted → InProgress → Attested → Released
                              ↘ Disputed → Resolved
```

This is how agent-to-agent work markets coordinate without trusting each other blindly.

---

## Capability enforcement on payments

Before any channel open, update, or escrow lock:

- Check max spend, rate limits, allowed assets, allowed counterparties
- **Reject updates that violate envelopes** — even if both parties sign (protects the principal)

---

## Failure modes & handling

| Failure | Handling |
|---------|----------|
| Non-responsive party | Timeout → dispute with last valid state |
| Conflicting states | Higher sequence + valid signatures wins |
| Key compromise | Freeze capabilities; emergency close (TBD) |
| Griefing disputes | Bonded disputes; cost for frivolous challenges |

---

## Relationship to tokens

Settlement assets may include Aether utility units and/or bridged external assets. Tokens serve bonding, fees, and metering — see economics notes.

---

## Open questions

- Hub-and-spoke (central liquidity providers) vs. pure mesh topology
- Multi-asset channels and FX between agent credit systems
- Exact dispute window vs. sub-second product claims
- Interoperability with existing L2 channel designs vs. custom settlement

---

## Key hypothesis (H1)

**Channels beat on-chain micropayments for agents** when payment frequency is high and amounts are small. Kill criterion: if channels don't improve cost/latency by a clear margin after including dispute infrastructure.

**Next:** [06-reputation.md](06-reputation.md)
