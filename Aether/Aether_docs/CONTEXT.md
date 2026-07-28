# CONTEXT.md — Project Aether

## Mission

Build a decentralized trust, identity, and payment infrastructure tailored for autonomous AI agents.

## Core Question

What infrastructure will AI agents require when software becomes capable of independent economic activity?

## Fundamental Assumption

Current blockchains and financial rails are designed around human UI/UX, legal identities, and manual signature friction. Aether is built natively for machine-to-machine interaction and autonomous software actors.

## Key Concepts

AI agents require machine-readable primitives for:

| Primitive | Purpose |
|-----------|---------|
| **Identity** | Cryptographic, non-sovereign agent identifiers |
| **Permissions** | Fine-grained, programmatically bound action limits |
| **Reputation** | Provable execution history and fault-resistant scoring |
| **Payments** | Low-latency, micro-fractional settlement channels |
| **Verification** | Zero-knowledge and deterministic execution proofs |
| **Privacy** | Selective disclosure of internal state, weights, and intent |

## Design Philosophy

1. **Infrastructure first, economic mechanisms second.** Protocol utility precedes token narratives.
2. **Zero human-in-the-loop** for standard operations. Agents transact, attest, and settle without UI friction.
3. **Sub-second cryptographic verification** and near-zero settlement overhead.
4. **Tokens as utility** — bonding, anti-spam, resource allocation — not speculative assets.

## Non-Goals

Aether is explicit about what it is **not**:

- A meme coin or speculative trading asset
- A clone or simple EVM layer-2 fork
- An AI chatbot, LLM wrapper, or user-facing agent interface

## Document Map

| Document | Role |
|----------|------|
| [VISION.md](VISION.md) | Long-term north star |
| [PROBLEM_STATEMENT.md](PROBLEM_STATEMENT.md) | Why existing rails fail agents |
| [RESEARCH_AGENDA.md](RESEARCH_AGENDA.md) | Open research questions |
| [ARCHITECTURE.md](ARCHITECTURE.md) | System layers and components |
| [PROTOCOL_DESIGN.md](PROTOCOL_DESIGN.md) | Wire formats, messages, state machines |
| [AI_AGENT_MODEL.md](AI_AGENT_MODEL.md) | What an “agent” is on Aether |
| [IDENTITY_SYSTEM.md](IDENTITY_SYSTEM.md) | Agent identifiers and keys |
| [REPUTATION_SYSTEM.md](REPUTATION_SYSTEM.md) | Execution history and scoring |
| [PAYMENT_LAYER.md](PAYMENT_LAYER.md) | Channels, microsettlement |
| [PRIVACY_MODEL.md](PRIVACY_MODEL.md) | Selective disclosure |
| [CONSENSUS_DESIGN.md](CONSENSUS_DESIGN.md) | Agreement and finality |
| [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) | ADR: settlement strategy, finality, evidence |
| [ECONOMIC_MODEL.md](ECONOMIC_MODEL.md) | Bonding, fees, anti-spam |
| [SECURITY_MODEL.md](SECURITY_MODEL.md) | Threats and mitigations |
| [COMPETITOR_ANALYSIS.md](COMPETITOR_ANALYSIS.md) | Landscape positioning |
| [WHITEPAPER.md](WHITEPAPER.md) | Canonical public narrative |
| [../Project_Phases/phase_0/CONTEXT.md](../Project_Phases/phase_0/CONTEXT.md) | Phase 0: pre-implementation gate |
| [../Project_Phases/phase_0/DECISIONS.md](../Project_Phases/phase_0/DECISIONS.md) | Phase 0 decision log |

Supporting material lives under `research/`, `experiments/`, `roadmap/`, and `Project_Phases/`.

## Working Principles for Contributors

- Prefer machine-verifiable claims over narrative marketing.
- Separate settled design from open questions (`research/unanswered_questions.md`).
- Record hypotheses and prototype outcomes under `experiments/`.
- Advance phases only when prior-phase exit criteria are met (`roadmap/`, `Project_Phases/`).
- Use **Phase N** for project phases and **PROTO-N** for prototypes — never `P0`–`P5`.

## Status

**Living context.** Update this file when mission, assumptions, or non-goals change. All other docs should remain consistent with this source of truth.
