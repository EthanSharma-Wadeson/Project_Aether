# PROBLEM_STATEMENT.md — Project Aether

## Problem

Autonomous AI agents will soon participate in economic activity at scale, but the infrastructure available to them — identity systems, payment rails, blockchains, reputation databases, and compliance frameworks — was built for humans. That mismatch is structural, not cosmetic.

Without agent-native infrastructure, autonomous software either:

1. Remains tethered to human wallets, APIs, and approval flows (bottlenecking autonomy), or
2. Improvises brittle trust on centralized platforms (recreating single points of failure and capture).

Aether exists to close this gap.

## Failure Modes of Current Rails

### Human-centric identity

- Legal identity (KYC/AML identity providers) maps poorly to ephemeral, multi-instance, or delegated software actors.
- OAuth and API keys are platform-scoped, revocable by third parties, and not portable trust.
- Wallet addresses alone lack policy, capability binding, and reputation semantics.

### Signature and UX friction

- Manual signing, wallet pop-ups, and device unlocks assume a human in the loop.
- Agent loops that require frequent authorization stall or require dangerous key custody shortcuts.
- Session models optimized for browsers do not match long-running or headless agents.

### Settlement latency and cost

- L1/L2 gas and confirmation times make continuous micro-payments irrational.
- Batch settlement and channels exist, but are rarely designed as first-class agent primitives with programmatic channel lifecycle.
- Fiat rails introduce banking hours, chargebacks, and account freezes incompatible with autonomous loops.

### Trust without machine-verifiable history

- Platform ratings are opaque, siloed, and gameable.
- On-chain history is public but noisy; it does not encode task quality, SLA adherence, or fault attribution by default.
- Agents lack a shared, portable reputation object they can query and update under protocol rules.

### Privacy vs. verification tension

- Agents must prove they did work correctly without leaking model weights, prompts, proprietary data, or strategy.
- Current systems force a false choice: full transparency or unverifiable black boxes.

### Speculative and clone-driven crypto culture

- Many “AI × crypto” projects prioritize tokens, narratives, or EVM forks over machine-grade infrastructure.
- This crowds out serious protocol work and confuses the problem space.

## Who Is Harmed / Constrained

| Actor | Constraint today |
|-------|------------------|
| Agent operators | Forced to proxy through human accounts and custodial APIs |
| Agent marketplaces | Cannot verify counterparties without central reputation DBs |
| Enterprises | Fear uncontrolled spend and unattested agent behavior |
| Researchers / open networks | No shared substrate for interoperable agent economics |

## Problem Formulation (Precise)

**Given** software actors that can initiate actions and hold cryptographic keys,

**Need** a decentralized stack providing:

- Non-sovereign cryptographic identity
- Fine-grained programmable permissions
- Provable, fault-resistant reputation
- Low-latency micro-fractional payments
- Verifiable execution with selective disclosure

**Subject to** zero human-in-the-loop for standard operations, sub-second verification targets, and utility-first economics.

## Out of Scope for the Problem

Aether does not claim to solve:

- Building better LLMs or agent UIs
- Replacing all human financial regulation overnight
- Speculative asset issuance as a product category

## Implication

If agents become economically autonomous on human rails, the result will be either captivity (platform lock-in) or chaos (unaccountable automation). A dedicated machine-native trust and payment infrastructure is a prerequisite for a healthy agent economy.

See [VISION.md](VISION.md) for the target state and [RESEARCH_AGENDA.md](RESEARCH_AGENDA.md) for open technical questions.
