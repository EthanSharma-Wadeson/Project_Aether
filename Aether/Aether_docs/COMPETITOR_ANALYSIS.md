# COMPETITOR_ANALYSIS.md — Project Aether

## Purpose

Position Aether relative to adjacent work. The goal is clarity, not tribal dismissal. Many projects solve nearby problems; few target **machine-native trust + identity + microsettlement** as one infrastructure stack.

## Evaluation Criteria

| Criterion | Question |
|-----------|----------|
| Agent-native | Built for autonomous software, not human wallet UX? |
| Identity | Cryptographic agent identity with permissions? |
| Payments | Micro-fractional, low-latency settlement path? |
| Reputation | Provable history, not platform ratings alone? |
| Verification / privacy | Proofs or selective disclosure for work products? |
| Non-speculative posture | Utility infrastructure vs. meme/AI-ticker narrative? |
| Independence from HITL | Standard ops without human click-signing? |

## Landscape Categories

### 1. General-purpose L1/L2 blockchains

**Examples (representative):** Ethereum and rollups, Solana, Cosmos appchains, Bitcoin + L2 experiments.

| Strengths | Gaps for agents |
|-----------|-----------------|
| Shared security, tooling, liquidity | Fee/latency UX still human-retail skewed |
| Smart contracts enable escrow | Identity = address; weak capability model |
| Ecosystem gravity | Reputation not first-class; AI narratives bolted on |

**Aether stance:** Reuse settlement/DA where it helps; do not mistake an EVM fork for an agent protocol.

### 2. Payment channel / streaming payment networks

**Examples (representative):** Lightning-class networks, L2 payment channels, streaming money protocols.

| Strengths | Gaps |
|-----------|------|
| Micro-payments, speed | Often human node operation UX |
| Mature dispute patterns | Weak agent identity/reputation integration |

**Aether stance:** Channel designs are highly relevant; Aether binds them to agent identity, capabilities, and attested work.

### 3. Decentralized identity (DID / VC) systems

**Examples (representative):** W3C DID/VC ecosystems, various blockchain identity projects.

| Strengths | Gaps |
|-----------|------|
| Portable credentials, selective disclosure research | Often human/org identity centric |
| Rich credential formats | Rarely paired with agent microsettlement |

**Aether stance:** Align with credential crypto where useful; root use case is agent economic actors, not SSO replacement.

### 4. Agent marketplaces & orchestrators (Web2 / hybrid)

**Examples (representative):** Centralized agent stores, API broker platforms, tooling frameworks.

| Strengths | Gaps |
|-----------|------|
| Fast product iteration | Platform lock-in, opaque reputation |
| Easy payments via cards/Stripe | HITL accounts; freeze risk; not portable trust |

**Aether stance:** Application layer may thrive; Aether provides non-custodial portable substrate underneath.

### 5. “AI × crypto” token projects

**Examples:** Varied; high churn.

| Strengths | Gaps |
|-----------|------|
| Attention, fundraising | Often thin infra; speculative primary |
| Experimentation energy | Chatbot wrappers, copy-paste chains |

**Aether stance:** Explicit non-goal to compete as a meme/AI ticker. Differentiate on primitives and measurements.

### 6. Confidential compute & proof systems

**Examples (representative):** TEE clouds, ZKML research, optimistic ML verification.

| Strengths | Gaps |
|-----------|------|
| Execution privacy / integrity | Not a full identity+payment stack |
| Rapid research progress | Cost and tooling may not fit v1 hot path |

**Aether stance:** Pluggable verification backends under a common attestation interface.

### 7. Reputation & crediting systems

**Examples:** Platform ratings, on-chain activity scores, Eigen-style restaked trust experiments (adjacent).

| Strengths | Gaps |
|-----------|------|
| Signal for humans/apps | Often not tied to agent escrow outcomes |
| Some crypto-economic security | Different problem domains |

**Aether stance:** Reputation grounded in agent settlement and attestation evidence.

## Differentiation Summary

Aether’s wedge is the **integrated machine-native stack**:

```
Identity + Capabilities + Channels + Attestations + Reputation
```

Competitors often excel at one slice. Aether’s risk is scope; mitigation is phased delivery (`roadmap/`) with infrastructure-first discipline.

## Tracking Process

- Revisit this document each phase gate  
- Prefer primary sources (docs, specs, measurements) over CT narratives  
- Record concrete latency/cost comparisons in `experiments/` when prototypes exist  

## Open Questions

- Which existing channel stack to interoperate with first  
- Whether DID methods should be adopted or mapped  
- Partnership vs. rebuild decisions for proof backends  
