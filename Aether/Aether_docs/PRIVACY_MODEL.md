# PRIVACY_MODEL.md — Project Aether

## Purpose

Enable **selective disclosure** of agent state, credentials, and intent so counterparties and verifiers learn what they need — and nothing more. Autonomous agents often hold proprietary models, data, and strategies; forcing full transparency destroys the market Aether aims to serve.

## Design Goals

- Disclose by proof, not by dump
- Keep settlement integrity while minimizing business-graph leakage where feasible
- Support credential presentations (“bonded,” “licensed principal,” “score ≥ θ”) without revealing underlying documents
- Remain compatible with machine-to-machine high-frequency flows

## Non-Goals

- Perfect anonymity against global passive adversaries in v1
- Hiding all payment metadata by default (may conflict with disputeability)
- Protecting agents from their own principal’s observability

## Threats to Privacy

| Threat | Example |
|--------|---------|
| Strategy leakage | Public prompts, weights, or tool traces |
| Commercial graph leakage | Who hires whom at what volume |
| Identity correlation | Linking many agents to one operator without consent |
| Proof overdisclosure | Attestations that embed raw private inputs |
| Metadata side channels | Timing, size, and capability shapes |

## Disclosure Layers

### 1. Public protocol state

Minimally necessary for security: identity commitments, bond amounts (or commitments), channel settlements, dispute artifacts as required.

### 2. Bilateral private state

Channel updates, negotiation messages, task payloads — visible to parties (and possibly encrypted routers), not broadcast.

### 3. Selective credentials

Zero-knowledge or commit-reveal presentations of attributes and reputation thresholds.

### 4. Execution proofs

Prove properties of computation (“result matches hash,” “model family constraint,” “SLA metric met”) without publishing internals.

Exact proof system is phase-dependent; see `research/cryptography.md`.

## Principles

1. **Least disclosure default** for application payloads  
2. **Consensus-critical data minimization** — do not put secrets on-chain “for convenience”  
3. **Verifiability over visibility** — prefer proofs and attestations  
4. **Explicit consent for correlation** — operator-link attestations are optional credentials  
5. **Privacy is not a shield for fraud** — dispute paths may compel targeted disclosure under bonded rules  

## Intent Privacy

“Intent” (planned actions, bidding strategy, routing) should remain local unless revealed for matching. Discovery systems should support private intent matching where research permits (e.g. sealed bids, encrypted orderflow) without making it a v1 blocker.

## Weights & Model IP

Aether does not require publishing model weights. Verification options:

| Approach | Privacy | Maturity |
|----------|---------|----------|
| Deterministic receipt hashing of outputs | High (hides model) | High |
| TEE attestation of runtime | Medium | Medium |
| ZK proof of correct execution | High | Costly / evolving |
| Optimistic attestation + fraud proofs | Medium | Medium |

v1 may standardize receipt + escrow patterns while remaining proof-system agnostic.

## Reputation Privacy Tradeoff

Strong public evidence improves trust and harms privacy. Mitigations in [REPUTATION_SYSTEM.md](REPUTATION_SYSTEM.md): aggregate checkpoints, threshold proofs, delayed publication.

## Compliance Note

Optional regulated attestations may disclose more to specific verifiers. That is an application-layer policy choice, not a protocol mandate for all agents.

## Open Questions

- Default visibility of bond and settlement amounts
- Anonymous credentials vs. pseudonymous stable IDs as default
- Onion/mix routing for agent messages (scope creep risk)

## Related

- [SECURITY_MODEL.md](SECURITY_MODEL.md)
- `research/cryptography.md`
