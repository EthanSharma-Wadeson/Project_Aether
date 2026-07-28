# VISION.md — Project Aether

## North Star

A world where autonomous software agents can discover each other, prove who they are, constrain what they may do, settle value at machine speed, and leave verifiable trails of behavior — without relying on human operators for every interaction.

Aether is the substrate for that world: a decentralized trust, identity, and payment fabric designed for machines first.

## The Shift

Software is crossing a threshold: from tools that wait for humans to actors that initiate work, negotiate, and pay. Markets, supply chains, research pipelines, and digital services will increasingly be composed of agents that:

- Spin up and tear down in seconds
- Transact at micro-fractional amounts and high frequency
- Need cryptographic identity without KYC theater
- Must prove execution without exposing proprietary internals
- Cannot tolerate multi-second confirmation UX designed for wallets and browsers

Existing financial and blockchain systems optimize for humans holding phones. Aether optimizes for agents holding keys.

## Desired End State

By maturity, Aether should enable:

1. **Agent-native identity** — Stable, portable, non-sovereign identifiers bound to cryptographic material and policy, not to legal persons by default.
2. **Programmable permissions** — Capability envelopes that agents and principals attach to actions, spend limits, counterparties, and time windows.
3. **Provable reputation** — Execution and settlement history that is auditable, sybil-resistant, and usable as a machine-readable trust signal.
4. **Microsettlement rails** — Channels and batching that make sub-cent, sub-second economic coordination rational.
5. **Selective disclosure** — Agents reveal only what a counterparty or verifier needs; weights, prompts, and internal state stay private unless proven otherwise.
6. **Composable markets** — Agents hire agents, escrow outcomes, and route work across open networks with deterministic dispute and settlement rules.

## Design Invariants

These do not change with roadmap phase:

- No dependency on human UI for standard agent operations
- Verification and settlement latency suitable for automated loops
- Utility-first economics; speculation is not a product goal
- Infrastructure precedes complex economic games

## What Success Looks Like

Success is not TVL theater or social engagement. Success looks like:

- Agents registering, bonding, and transacting without human session friction
- Counterparties verifying identity, permissions, and proofs programmatically
- Settlement costs and latency low enough for continuous micro-coordination
- Reputation scores that correlate with reliable execution under adversarial conditions
- Clear separation between Aether (infrastructure) and applications built on top

## Horizon Framing

| Horizon | Focus |
|---------|--------|
| Near | Primitives: identity, permissions, channels, attestation formats |
| Mid | Networks of agents coordinating work and payment with measurable reliability |
| Long | Default economic substrate for autonomous software actors across domains |

See `roadmap/` for phased delivery. See [PROBLEM_STATEMENT.md](PROBLEM_STATEMENT.md) for why this vision is not served by today’s rails.
