# 01 — Introduction to Project Aether

**Key takeaway:** Aether is decentralized infrastructure for autonomous AI agents to identify themselves, constrain their actions, pay each other, prove their work, and build reputation — all without a human clicking "approve" on every transaction.

---

## The core question

> What infrastructure will AI agents need when software can act economically on its own?

Software is shifting from **tools that wait for humans** to **actors that initiate work, negotiate, and pay**. Aether builds the substrate for that world.

---

## Mission

Build decentralized **trust, identity, and payment** infrastructure tailored for autonomous AI agents.

---

## Fundamental assumption

Today's blockchains and financial rails were designed for:

- Humans holding phones
- Legal identities (KYC)
- Manual wallet signing
- Multi-second confirmation UX

Agents need something different: **machine-native** primitives they can use programmatically at high speed.

---

## The six primitives

| Primitive | What it solves |
|-----------|----------------|
| Identity | "Who is this agent?" — cryptographic, portable IDs |
| Permissions | "What may this agent do?" — spend limits, allowed actions |
| Reputation | "Can I trust them?" — provable execution history |
| Payments | "How do they settle value?" — micro-payments at machine speed |
| Verification | "Did they do the work?" — proofs without leaking secrets |
| Privacy | "What do they reveal?" — selective disclosure of state and intent |

---

## Design philosophy

1. **Infrastructure first, economics second** — protocol utility before token narratives
2. **Zero human-in-the-loop** — agents transact, attest, and settle autonomously
3. **Sub-second verification** — fast enough for automated loops
4. **Tokens as utility** — bonding, anti-spam, resource metering — not speculation

---

## What success looks like

Success is **not** TVL theater or social engagement. Success is:

- Agents registering, bonding, and transacting without human session friction
- Counterparties verifying identity, permissions, and proofs programmatically
- Settlement costs low enough for continuous micro-coordination
- Reputation that correlates with reliable execution under attack
- Clear separation between Aether (infrastructure) and apps built on top

---

## Horizon framing

| Horizon | Focus |
|---------|-------|
| **Near** | Primitives: identity, permissions, channels, attestation formats |
| **Mid** | Networks of agents coordinating work and payment reliably |
| **Long** | Default economic substrate for autonomous software across domains |

---

## Non-goals (important)

Aether explicitly is **not**:

| Not this | Why |
|----------|-----|
| Meme coin / speculative asset | Utility infrastructure, not trading narrative |
| EVM L2 fork | Agent protocol, not another chain clone |
| AI chatbot / LLM wrapper | Infrastructure layer, not a product UI |
| Better LLMs | Model quality is out of scope |
| Replacing all regulation | Legal bridges are optional, not mandatory |

---

## Document map (source docs)

| Topic | Source in `Aether_docs/` |
|-------|--------------------------|
| Vision | `VISION.md` |
| Problem | `PROBLEM_STATEMENT.md` |
| Architecture | `ARCHITECTURE.md` |
| Protocol | `PROTOCOL_DESIGN.md` |
| Agents | `AI_AGENT_MODEL.md` |
| Identity | `IDENTITY_SYSTEM.md` |
| Payments | `PAYMENT_LAYER.md` |
| Reputation | `REPUTATION_SYSTEM.md` |
| Privacy | `PRIVACY_MODEL.md` |
| Consensus | `CONSENSUS_DESIGN.md` |
| Economics | `ECONOMIC_MODEL.md` |
| Security | `SECURITY_MODEL.md` |
| Competitors | `COMPETITOR_ANALYSIS.md` |
| Public narrative | `WHITEPAPER.md` |

**Next:** [02-the-problem.md](02-the-problem.md)
