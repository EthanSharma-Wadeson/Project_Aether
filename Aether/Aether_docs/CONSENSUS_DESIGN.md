# CONSENSUS_DESIGN.md — Project Aether

> **Architecture decision:** See [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) for the v0.1 ADR. Aether is a settlement-agnostic agent trust protocol — not a new L1. This document covers consensus touchpoints and finality semantics at the protocol layer.

## Purpose

Define how Aether reaches agreement on the small set of states that require global ordering and finality — without forcing every agent micro-payment through a slow human-oriented blockchain UX.

## Design Goals

- Finalize identity, bonds, settlements, and disputes with clear safety rules
- Keep high-frequency channel updates off the consensus hot path
- Target verification and settlement overhead compatible with automated agents
- Avoid "EVM L2 clone" as the architecture identity

## What Consensus Must Cover

| In consensus | Out of consensus (typically) |
|--------------|------------------------------|
| Identity register / freeze / revoke | Channel update stream |
| Bond lock / slash / release | Bilateral negotiation |
| Channel open / settle / dispute resolve | Local attestation verification |
| Canonical reputation checkpoints (if any) | Full task payloads |
| Protocol parameter updates | Discovery gossip |

## Safety & Liveness (Targets)

- **Safety:** No conflicting finalized settlements for the same channel sequence under the assumed fault model
- **Liveness:** Honest agents can eventually settle and recover funds despite crashed counterparties (via disputes)
- **Accountability:** Equivocation and fraud are attributable and slashable where bonds exist

Fault model and verifier thresholds are defined in [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) §4–5. Settlement security is inherited from the selected backend; agents and verifiers are treated as potentially adversarial.

## Architectural Decision (v0.1)

**Decided:** Aether v0.1 operates as a **modular, settlement-agnostic agent trust protocol** with a pluggable settlement adapter layer. It does not launch as its own L1.

A native Aether chain remains a future option only if measured requirements justify it — see [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) §3.

## Finality States (Aligned with Payments)

Aether defines multi-stage finality — not a single "confirmed" flag:

```
PROPOSED → ACCEPTED → SETTLEMENT_FINAL → DISPUTE_WINDOW_FINAL → ECONOMIC_FINAL
```

Channel updates remain bilateral soft finality off the settlement hot path. Product and protocol docs MUST expose finality as machine-readable states so agents can reason about risk. See [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) §6–7.

## Fee Market for Agents

Human NFT mint UX is the wrong fee market template. Prefer:

- Predictable metering for registration and settlement batching
- Anti-spam prepaid bonds / tickets for message inclusion classes
- Priority based on bonded stake or explicit urgency fields without unnecessary public auction drama (design TBD)

## Parallelism

Agent workloads are highly parallel (many bilateral channels). Consensus design should:

- Shard or otherwise isolate channel dispute domains where safe
- Avoid global lockstep for unrelated agent pairs
- Batch settlement proofs/certificates

## Light Clients & Agents

Agents must verify relevant finalized state with minimal trust:

- Compact proofs of inclusion for identity and channel settlements
- Clear commitment schemes for capability roots and reputation checkpoints

## Open Questions

See [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) §16. Remaining work includes:

- Which settlement backends to include in the first prototype
- Verifier committee design per task class
- Exact dispute period bounds and evidence retention rules
- Backend-specific finality representation in the protocol schema

## Related

- [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) — ADR: settlement strategy, finality, evidence, multi-backend identity
- [PAYMENT_LAYER.md](PAYMENT_LAYER.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- `research/distributed_systems.md`
- `research/blockchain_analysis.md`
