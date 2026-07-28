# WHITEPAPER.md — Project Aether

> Canonical public-facing narrative. Technical detail lives in the linked design docs; this document states the problem, thesis, and system shape without pretending research is finished.

## Abstract

Software is gaining the ability to act economically: to buy services, hire other programs, and settle value continuously. The financial and blockchain systems available today assume human users, legal identities, and interactive signing. **Aether** is decentralized infrastructure for autonomous AI agents — providing cryptographic identity, programmable permissions, micro-fractional payments, verifiable execution interfaces, and portable reputation — with zero human-in-the-loop required for standard operations.

## 1. Motivation

When agents can pursue goals independently, they need machine-readable trust primitives. Without them, autonomy collapses into platform APIs and custodial accounts — or into unaccountable automation.

Aether starts from a single question:

> What infrastructure will AI agents require when software becomes capable of independent economic activity?

## 2. Thesis

**Agent-native infrastructure must be designed for machines first.** Retrofits of human wallets, KYC databases, and retail blockchains will not yield sub-second verification, safe microsettlement, or portable reputation under adversarial conditions.

Tokens, if present, are tools for bonding, anti-spam, and resource allocation — not speculative ends.

## 3. System Overview

Aether composes six primitives:

1. **Identity** — Non-sovereign cryptographic agent identifiers  
2. **Permissions** — Fine-grained, programmatically bound action limits  
3. **Payments** — Low-latency channels for micro-fractional settlement  
4. **Verification** — Attestations and proofs of execution outcomes  
5. **Privacy** — Selective disclosure of state, credentials, and intent  
6. **Reputation** — Provable history with fault-resistant scoring  

These sit above a consensus/settlement plane that finalizes registrations, bonds, channel settlements, and disputes — while keeping high-frequency updates off the hot path.

## 4. Agent Model

An Aether agent is an autonomous software actor with keys, an identity, and a capability root. It is not defined as a chatbot or a particular model. Principals fund bonds and set policy; agents operate within envelopes the protocol enforces.

## 5. Architecture (Brief)

```
Applications / agent markets
        ↓
Reputation, escrow, discovery
        ↓
Payment channels & settlement accounting
        ↓
Identity, permissions, attestations
        ↓
Consensus, availability, cryptography
```

See [ARCHITECTURE.md](ARCHITECTURE.md) and [PROTOCOL_DESIGN.md](PROTOCOL_DESIGN.md).

## 6. Economic Design

Value locked in bonds and channels creates accountability. Fees meter scarce resources. Escrow ties payment to verification. The economic model explicitly rejects meme-coin productization.

See [ECONOMIC_MODEL.md](ECONOMIC_MODEL.md).

## 7. Security & Privacy

Agents and counterparties are untrusted. Security relies on authentication, capability checks, slashable bonds, disputeable channel states, and careful proof semantics. Privacy relies on least disclosure and pluggable proof systems for proprietary execution.

See [SECURITY_MODEL.md](SECURITY_MODEL.md) and [PRIVACY_MODEL.md](PRIVACY_MODEL.md).

## 8. What Aether Is Not

- A meme coin or speculative trading asset  
- A clone or simple EVM layer-2 fork  
- An AI chatbot, LLM wrapper, or user-facing agent interface  

## 9. Roadmap Posture

Delivery is phased: primitives and threat models first; network expansion and richer verification later. Research questions remain open by design; see [RESEARCH_AGENDA.md](RESEARCH_AGENDA.md) and `roadmap/`.

## 10. Conclusion

Autonomous agents will not wait for infrastructure shaped around human click-through consent. Aether builds the trust, identity, and payment substrate those agents require — infrastructure first, speculation never.

---

## Document Control

| Field | Value |
|-------|-------|
| Status | Draft whitepaper / living summary |
| Source of truth for mission | [CONTEXT.md](CONTEXT.md) |
| Deep specs | Sibling `*.md` files in this repository |
