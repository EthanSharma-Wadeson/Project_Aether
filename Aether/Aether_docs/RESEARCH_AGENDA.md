# RESEARCH_AGENDA.md — Project Aether

## Purpose

Define the research program that must be answered before and while building Aether. Settled design lives in architecture and protocol docs; open work lives here and in `research/`.

## Research Pillars

### 1. Distributed systems for agent workloads

- Consensus and finality models that match high-frequency, low-value agent transactions
- Channel networks and off-chain coordination with enforceable on-protocol settlement
- Availability and partition behavior when agents are ephemeral

→ See `research/distributed_systems.md`

### 2. Cryptography for verification and privacy

- Efficient proofs of execution / integrity suitable for agent task outcomes
- Selective disclosure of credentials, permissions, and attestations
- Key hierarchies, delegation, and rotation for non-human principals

→ See `research/cryptography.md`

### 3. Blockchain and settlement analysis

- What existing L1/L2/channel designs get right and wrong for agents
- Settlement batching, fee markets, and anti-spam without human UX assumptions
- Interoperability vs. building agent-native chains or modules

→ See `research/blockchain_analysis.md`

### 4. AI agent economics and behavior

- Agent lifecycle: spawn, bond, act, settle, retire
- Incentive compatibility under adversarial or misaligned agents
- Reputation as a control surface (and failure modes: collusion, wash activity)

→ See `research/ai_agents.md`

### 5. Unanswered / contested questions

Explicit list of decisions not yet locked.

→ See `research/unanswered_questions.md`

## Priority Questions (Near Term)

1. What is the minimal viable identity object for an agent (keys, metadata, capability root)?
2. What payment abstraction hits sub-second, micro-fractional settlement without unsafe custody?
3. How should reputation be updated: purely on-protocol events, attested off-chain evidence, or hybrid?
4. Which verification model is realistic for v1 (deterministic receipts, TEE attestations, ZK, optimistic fraud proofs)?
5. What economic bonding prevents sybil floods without pricing out legitimate micro-agents?

## Method

| Method | Use |
|--------|-----|
| Literature & competitor review | Ground claims; avoid reinventing settled primitives |
| Formal threat modeling | Security and economic attacks before token design |
| Hypotheses + prototypes | Empirically test latency, cost, and UX-for-machines |
| Spec freezes | Promote answers into ARCHITECTURE / PROTOCOL docs |

## Success Criteria for Research

A research thread is “done enough” when:

- Assumptions are explicit and falsifiable
- A recommendation is recorded with alternatives rejected and why
- Downstream docs (`ARCHITECTURE.md`, subsystem docs) are updated
- Remaining unknowns are listed in `research/unanswered_questions.md`

## Anti-Patterns

- Designing tokenomics before settlement and identity primitives
- Treating “put it on an EVM L2” as an architecture
- Confusing agent product UX with protocol research
- Shipping reputation formulas without adversarial analysis

## Related

- Hypotheses: `experiments/hypotheses.md`
- Prototypes: `experiments/prototypes.md`
- Roadmap phases: `roadmap/phase_1.md` et seq.
