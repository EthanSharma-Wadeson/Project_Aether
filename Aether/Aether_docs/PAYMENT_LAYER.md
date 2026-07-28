# PAYMENT_LAYER.md — Project Aether

## Purpose

Enable **low-latency, micro-fractional settlement** between agents so continuous machine-to-machine economic coordination is rational.

## Design Goals

- Sub-second *update* latency on the hot path (off-chain channel updates)
- Near-zero marginal overhead per micro-payment
- Programmatic channel lifecycle (open, update, settle, dispute) with no human UI
- Enforce spend limits via [IDENTITY_SYSTEM.md](IDENTITY_SYSTEM.md) capabilities
- Utility fees/bonds for spam resistance, not speculative velocity games

## Non-Goals

- Replacing all retail consumer payments
- Maximizing MEV surface area
- Requiring on-chain finality for every micro-transfer

## Architecture Pattern

**Payment channels (or channel networks) + periodic settlement.**

```
Agent A  ←── signed updates ──→  Agent B
    \                               /
     \_____ dispute / settle ______/
                    ↓
            Aether settlement plane
```

High-frequency value transfer occurs as authenticated state updates. The settlement plane finalizes opens, cooperative closes, and disputes.

## Channel State (Logical)

```text
Channel {
  channel_id,
  parties: [AgentId; 2+] ,      // bilateral first; multipart later
  asset,
  balances,
  sequence,
  dispute_window,
  latest_signatures,
  status: Open | Active | Closing | Disputed | Settled
}
```

Each `ChannelUpdate` increments `sequence` and re-signs balances under channel keys authorized by capabilities.

## Latency & Finality Tiers

Aether defines five protocol finality states (see [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) §6):

| Tier | Name | Target use |
|------|------|------------|
| Soft (update) | Channel update — bilateral, sub-second | Streaming micro-fees for work |
| Settlement final | Backend-confirmed transaction | Withdrawal, accounting |
| Dispute-window final | Challenge period expired | Task outcome locked |
| Economic final | Funds, bonds, reputation applied | Workflow complete |

Agents price risk: uncooperative counterparties force dispute latency. Finality MUST be exposed as machine-readable protocol state.

## Micro-Fractional Support

- Asset denominations must allow very small units (or abstract usage credits redeemable in aggregate)
- Fee schedules must not make dust payments irrational
- Batching of settlements across many channels where possible

## Escrow Composition

Payments compose with attestation:

1. Lock value conditioned on `AttestationVerify` success  
2. Release to provider or refund to consumer per rules  
3. Feed outcomes to reputation evidence  

This is the default pattern for “agent hires agent.”

## Capability Enforcement

Before `ChannelOpen` / `ChannelUpdate` / escrow lock:

- Check `max_spend`, rate limits, allowed assets, allowed counterparties
- Reject updates that would violate envelopes even if both parties sign (principal protection)

## Failure Modes

| Failure | Handling |
|---------|----------|
| Non-responsive party | Timeout → dispute with last valid state |
| Conflicting states | Higher sequence + valid sigs wins; fraud proofs for revoked states |
| Key compromise | Freeze capabilities; emergency close policies TBD |
| Griefing disputes | Bonded disputes; cost for frivolous challenges |

## Relationship to Tokens

Settlement assets may include Aether utility units and/or external assets bridged under policy. Token design serves bonding, fees, and metering — see [ECONOMIC_MODEL.md](ECONOMIC_MODEL.md).

## Open Questions

- Hub-and-spoke (LSP-like) vs. pure mesh for agent topologies
- Multi-asset channels and FX between agent credit systems
- Exact dispute window vs. sub-second product claims (product honesty about soft vs hard finality)
- Interoperability with existing L2 channel designs vs. custom settlement

## Related

- [PROTOCOL_DESIGN.md](PROTOCOL_DESIGN.md)
- [CONSENSUS_DESIGN.md](CONSENSUS_DESIGN.md)
- `research/blockchain_analysis.md`
