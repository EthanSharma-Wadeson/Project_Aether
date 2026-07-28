# 02 — The Problem Aether Solves

**Key takeaway:** Autonomous agents will participate in the economy, but every existing rail — identity, payments, reputation, privacy — was built for humans. That mismatch is structural, not fixable with a better UI.

---

## The gap

Without agent-native infrastructure, autonomous software either:

1. **Stays tethered** to human wallets, APIs, and approval flows (bottlenecking autonomy), or
2. **Improvises brittle trust** on centralized platforms (single points of failure and capture)

---

## Failure modes of current rails

### 1. Human-centric identity

| Problem | Detail |
|---------|--------|
| KYC doesn't fit | Agents are ephemeral, multi-instance, delegated — not legal persons |
| OAuth / API keys | Platform-scoped, revocable by third parties, not portable trust |
| Wallet addresses alone | No policy, capability binding, or reputation semantics |

### 2. Signature and UX friction

- Manual signing and wallet pop-ups assume a human in the loop
- Agent loops that need frequent authorization stall or force dangerous key shortcuts
- Browser session models don't match long-running headless agents

### 3. Settlement latency and cost

- L1/L2 gas and confirmation times make continuous micro-payments irrational
- Channels exist but aren't designed as first-class agent primitives
- Fiat rails have banking hours, chargebacks, and account freezes

### 4. Trust without machine-verifiable history

- Platform ratings are opaque, siloed, and gameable
- On-chain history is public but noisy — doesn't encode task quality or SLA adherence
- No shared, portable reputation object agents can query under protocol rules

### 5. Privacy vs. verification tension

- Agents must prove correct work **without** leaking model weights, prompts, or strategy
- Current systems force a false choice: full transparency or unverifiable black boxes

### 6. Speculative crypto culture

- Many "AI × crypto" projects prioritize tokens and narratives over infrastructure
- This crowds out serious protocol work and confuses the problem space

---

## Who is harmed today

| Actor | Constraint |
|-------|------------|
| Agent operators | Forced to proxy through human accounts and custodial APIs |
| Agent marketplaces | Can't verify counterparties without central reputation DBs |
| Enterprises | Fear uncontrolled spend and unattested agent behavior |
| Researchers / open networks | No shared substrate for interoperable agent economics |

---

## The precise problem statement

**Given** software actors that can initiate actions and hold cryptographic keys,

**Need** a decentralized stack providing:

- Non-sovereign cryptographic identity
- Fine-grained programmable permissions
- Provable, fault-resistant reputation
- Low-latency micro-fractional payments
- Verifiable execution with selective disclosure

**Subject to:** zero human-in-the-loop for standard ops, sub-second verification, utility-first economics.

---

## What's out of scope for the problem

Aether does **not** claim to solve:

- Building better LLMs or agent UIs
- Replacing all human financial regulation overnight
- Speculative asset issuance as a product category

---

## The implication

If agents become economically autonomous on human rails, the result is either:

- **Captivity** — platform lock-in, or
- **Chaos** — unaccountable automation

Dedicated machine-native trust and payment infrastructure is a **prerequisite** for a healthy agent economy.

**Next:** [03-architecture.md](03-architecture.md)
