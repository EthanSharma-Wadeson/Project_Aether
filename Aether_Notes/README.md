# Aether — One-Day Learning Guide

Concise notes distilled from `Aether_docs/`. No maths, no code — just the concepts you need to understand the project.

**Estimated time:** 6–8 hours of focused reading.

---

## How to use these notes

1. Read in order — each file builds on the last.
2. Skim the **Key takeaway** at the top of each file first.
3. Use [glossary.md](glossary.md) as a quick reference at the end.
4. Return to `Aether_docs/` for full specs when you need depth.

---

## Suggested schedule

| Block | Time | File | Topic |
|-------|------|------|-------|
| 1 | 45 min | [01-introduction.md](01-introduction.md) | What Aether is, mission, non-goals |
| 2 | 45 min | [02-the-problem.md](02-the-problem.md) | Why existing rails fail agents |
| 3 | 60 min | [03-architecture.md](03-architecture.md) | System layers, components, data flows |
| 4 | 60 min | [04-agents-identity-permissions.md](04-agents-identity-permissions.md) | Agent model, identity, capabilities |
| 5 | 60 min | [05-payments-and-escrow.md](05-payments-and-escrow.md) | Channels, microsettlement, escrow |
| 6 | 45 min | [06-reputation.md](06-reputation.md) | Trust scoring, evidence, attacks |
| 7 | 45 min | [07-privacy-and-verification.md](07-privacy-and-verification.md) | Selective disclosure, proof types |
| 8 | 45 min | [08-consensus-and-infrastructure.md](08-consensus-and-infrastructure.md) | What goes on-chain vs off-chain |
| 9 | 45 min | [09-economics-and-security.md](09-economics-and-security.md) | Bonds, fees, threats, mitigations |
| 10 | 30 min | [10-roadmap-and-landscape.md](10-roadmap-and-landscape.md) | Phases, competitors, open questions |

**Breaks:** Take 10–15 min between blocks 5 and 6.

---

## The six primitives (memorize these)

| Primitive | One-line purpose |
|-----------|------------------|
| **Identity** | Cryptographic agent IDs — not tied to legal persons |
| **Permissions** | Programmable limits on what an agent can do and spend |
| **Payments** | Fast, tiny-value settlement via channels |
| **Verification** | Prove work was done without exposing internals |
| **Privacy** | Reveal only what counterparties need |
| **Reputation** | Machine-readable history of reliable execution |

---

## Design invariants (never change)

- No human-in-the-loop for standard operations
- Infrastructure first, tokens second
- Sub-second verification, near-zero settlement overhead
- Agents are untrusted — trust comes from bonds, escrow, proofs

---

## What Aether is NOT

- A meme coin or speculative trading asset
- An EVM L2 clone
- An AI chatbot or LLM product
