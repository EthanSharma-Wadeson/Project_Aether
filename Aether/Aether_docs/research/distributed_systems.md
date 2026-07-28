# research/distributed_systems.md

## Focus

Distributed systems questions that determine whether Aether can deliver machine-speed coordination with enforceable settlement.

## Topics

### Consensus placement

- What state must be totally ordered vs. bilaterally agreed?
- Can channel dispute domains be parallelized/sharded safely?
- How do we provide light-client proofs agents can verify cheaply?

### Partial synchrony & disputes

- Dispute windows vs. realistic network adversary delays
- Watchtower availability models for ephemeral agents
- Behavior under partitions: safety over liveness for funds

### Availability of evidence

- Where dispute evidence lives (DA layer, content-addressed stores)
- Retention policies compatible with long channel lifetimes

### Ephemeral participants

- Agents that vanish mid-job
- Reattachment via identity keys vs. lost operational state
- Multiparty protocols when party sets churn

### Throughput patterns

- Workloads: many tiny bilateral updates, rare global settles
- Avoid designs optimized for NFT mints and retail DEX swaps
- Batching certificates without creating latency cliffs

## Working Hypotheses

1. Bilateral channels + rare consensus touchpoints beat “every micropayment on L1/L2.”
2. Agent networks benefit more from parallel dispute domains than from monolithic global mempools.
3. Watchtowers (or principal mirrors) are necessary for honest offline safety — economics must be designed, not assumed free.

## References to Capture

- Payment channel / Lightning-style security proofs literature
- BFT and DAG-based mempool research
- Modular DA + execution rollup designs
- Actor system / capability-system literature (for permission inspiration)

## Exit Criteria

- Written recommendation for settlement architecture option (A/B/C in CONSENSUS_DESIGN)
- Parameter ranges for dispute windows under stated network assumptions
- Explicit failure mode table for partitions and crash-only agents

## Links

- [CONSENSUS_DESIGN.md](../CONSENSUS_DESIGN.md)
- [PAYMENT_LAYER.md](../PAYMENT_LAYER.md)
- [unanswered_questions.md](unanswered_questions.md)
