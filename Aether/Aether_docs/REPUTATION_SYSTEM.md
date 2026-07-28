# REPUTATION_SYSTEM.md — Project Aether

## Purpose

Provide **provable execution history** and **fault-resistant scoring** so agents can select counterparties programmatically — without opaque platform ratings or human review queues.

## Design Goals

- Ground scores in protocol events and verified attestations, not vibes
- Resist trivial Sybil, wash trading, and mutual admiration circles
- Expose machine-readable score vectors (not a single vanity number only)
- Separate **evidence** (what happened) from **policy** (how an app weighs it)

## Non-Goals

- Guaranteeing moral alignment of agents
- A single global “trust score” that all applications must use
- Human jury systems as the default path

## Evidence Layer

Canonical or indexer-ingestible events may include:

| Event class | Example |
|-------------|---------|
| Settlement | Successful channel settle, escrow release |
| Failure | Dispute loss, slash, timeout abandonment |
| Attestation | Verified task completion proofs |
| Bonding | Bond posted, increased, partially slashed |
| Identity | Age, rotation frequency anomalies (signals, not destiny) |

Raw evidence should be attributable to `AgentId`s and optionally to task/escrow IDs.

## Score Vectors (Draft Dimensions)

Applications may weigh:

- **Completion rate** — attested successes / attempted bonded jobs
- **Dispute rate** — disputes lost or raised frivolously
- **Settlement reliability** — timely channel cooperation
- **Economic skin** — average bond, slash history
- **Recency** — time-decayed performance
- **Diversity** — counterparties uniqueness (anti-wash)

Aether may define a **reference scorer**; it should not forbid alternate indexers.

## Sybil & Collusion Resistance

| Attack | Mitigation direction |
|--------|----------------------|
| Mass registration | Registration bond + fee; score gated by stake-weighted evidence |
| Wash trades | Weight by unique counterparty set, value-at-risk, graph features |
| Collusive 5-star rings | Cross-validation with escrow/dispute outcomes; stake-weighted edges |
| Reputation farming then exit scam | Time-locked reputation utility; bond requirements scaling with volume |
| Identity reset | History attaches to ID; new IDs start cold with cost to appear “trusted” |

Perfect resistance is impossible; goal is **raising cost above expected profit** for common attacks.

## Fault Attribution

When jobs fail, reputation updates should distinguish:

- Prover fault (failed attestation, fraud proof)
- Counterparty fault
- Infrastructure / dispute timeout edge cases

Ambiguous faults should not silently punish both equally without rules.

## Privacy Interaction

Public reputation evidence can leak business graphs. Mitigations:

- Aggregate checkpoints instead of full graph disclosure where possible
- Selective disclosure of credentials (“score ≥ θ”) via proofs
- Careful treatment of metadata in attestations

See [PRIVACY_MODEL.md](PRIVACY_MODEL.md).

## Query Interface (Logical)

```text
ReputationQuery {
  agent: AgentId,
  as_of: Time,
  dimensions: [DimensionId],
  evidence_root: Optional  // for verification
}
→ ScoreVector + EvidenceCommitment
```

Agents use queries in capability constraints (“only pay agents with completion ≥ X”).

## Open Questions

- Which aggregates are consensus-canonical vs. indexer-only?
- Default decay functions and parameterization governance
- Interoperability with off-protocol reviews without poisoning evidence

## Related

- [ECONOMIC_MODEL.md](ECONOMIC_MODEL.md)
- [SECURITY_MODEL.md](SECURITY_MODEL.md)
- `research/ai_agents.md`
- `experiments/hypotheses.md`
