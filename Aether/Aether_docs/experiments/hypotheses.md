# experiments/hypotheses.md

## Purpose

Falsifiable claims that guide prototypes. Each hypothesis should state expected measurement and kill criteria.

## H1 — Channels beat on-chain micropayments for agents

**Claim:** For agent workloads with ≥ N payments/minute at dust sizes, bilateral channels achieve lower p95 cost and latency than posting each transfer on a candidate L2.

**Measure:** $/payment, time-to-soft-ack, time-to-hard-finality, failure rate under fee spikes.

**Kill if:** Channels do not improve cost/latency by a clear margin after including dispute infrastructure cost.

## H2 — Capability envelopes prevent principal loss under compromised agents

**Claim:** Spend/rate/counterparty envelopes stop theft beyond configured limits when operational keys are attacker-controlled.

**Measure:** Max loss vs. envelope ceilings in adversarial simulation.

**Kill if:** Envelope checks can be bypassed via payment paths or delegation tricks.

## H3 — Bonded evidence resists trivial wash reputation

**Claim:** A stake-and-outcome-weighted scorer makes naive wash trading unprofitable at bond levels tolerable for honest agents.

**Measure:** Attacker ROI under wash strategies; honest agent onboarding cost.

**Kill if:** Cheap Sybil rings still mint high scores without real escrow risk.

## H4 — Receipt-based escrow is enough for early agent markets

**Claim:** Versioned output receipts + timeouts unlock useful agent-to-agent markets before ZK/TEE are mandatory.

**Measure:** Dispute rate, wrongful release rate, user/agent drop-off in prototype market.

**Kill if:** Counterparties refuse to transact without stronger integrity proofs.

## H5 — Agents can budget protocol fees autonomously

**Claim:** Predictable fee/bond schedules allow agents to complete hire→work→settle loops without human fee management.

**Measure:** Loop completion rate; frequency of stuck funds due to fee mis-estimation.

**Kill if:** Fee variance forces HITL intervention as the normal case.

## H6 — Soft vs hard finality can be made machine-rational

**Claim:** Agents can price counterparty risk when APIs expose finality tier explicitly.

**Measure:** Whether agents select channels/timeouts consistently with documented risk; incident rate from misunderstanding finality.

**Kill if:** Ambiguous finality causes systematic fund loss in tests.

## Hypothesis Process

1. Write claim + metrics before coding  
2. Implement minimal prototype (`prototypes.md`)  
3. Record results: confirm / reject / revise  
4. Promote confirmed findings into design docs  

## Results Log

| ID | Result | Date | Notes |
|----|--------|------|-------|
| — | — | — | — |
