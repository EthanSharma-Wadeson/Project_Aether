# ECONOMIC_MODEL.md — Project Aether

## Purpose

Specify how economic mechanisms support infrastructure: **bonding, anti-spam, resource allocation**, and aligned incentives. Tokens (if any) are utility instruments — not the product.

## Design Philosophy Recap

1. Infrastructure first, economic mechanisms second  
2. Utility over speculation  
3. Price abuse above honest micro-agent costs where possible (hard tradeoff; be explicit)

## Non-Goals

- Designing a meme coin or high-velocity trading asset
- Using “number go up” as a protocol success metric
- Complex reflexive token games before primitives work

## Economic Primitives

### Bonds

Agents (or principals) lock value to:

- Register identity / access message classes
- Open channels above certain volumes
- Raise disputes
- Underwrite attested work (performance bonds)

Bonds are slashable for proven fraud, equivocation, or rule violations.

### Fees

Small fees meter scarce resources:

- Settlement inclusion
- Identity registration
- Dispute processing
- Optional relay / routing services

Fees should be predictable for automated budgeting.

### Escrow

Conditional value transfer tied to attestations and timeouts. Primary coordination tool for agent-to-agent work markets.

### Reputation-linked limits

Economic capacity may scale with evidence-backed reputation to reduce cold-start abuse without permanent caste systems (parameters TBD).

## Token Utility (If Issued)

If Aether issues a native unit, acceptable utilities include:

| Utility | Role |
|---------|------|
| Bonding asset | Skin-in-the-game for identity and disputes |
| Fee payment | Meter settlement and registration |
| Resource credits | Prepaid inclusion / bandwidth tickets |
| Staking for services | Secure relays, indexers, verifiers (if decentralized) |

Unacceptable primary utilities:

- Pure governance theater without protocol need
- Mandatory speculation to access basic agent identity

Bridged external assets may serve as bonds/fees if security and UX-for-machines allow.

## Incentive Constraints

| Actor | Desired behavior | Failure if mispriced |
|-------|------------------|----------------------|
| Honest micro-agents | Participate cheaply enough | Market dies |
| Sybil attacker | Unprofitable floods | Reputation/payment spam |
| Malicious provider | Slash > expected cheat profit | Exit scams dominate |
| Frivolous disputer | Loses dispute bond | Griefing freezes funds |
| Validators / services | Reliable inclusion | Censorship / extractive MEV |

## Anti-Spam Policy (Draft)

1. Minimum registration bond or fee  
2. Rate limits enforced by capabilities and protocol meters  
3. Stake-weighted tolerance for burst traffic  
4. Escalating cost for repeated failed attestations / disputes  

## Value Flow (Conceptual)

```
Principal funds agent bond & channel deposits
    → Agent pays micro-fees for work / services
    → Escrow releases on attestation
    → Settlements finalize
    → Slashes redistribute or burn per rules (TBD)
```

## Governance of Parameters

Fee, bond, and dispute window parameters affect security and access. Changes require:

- Explicit versioning
- Simulation / experimental evidence when possible
- No silent swaps that break agent automation assumptions

## Open Questions

- Native token vs. multi-asset utility from day one
- Burn vs. redistribute slashed funds
- Cold-start subsidies without creating farmable rewards
- Avoiding regulatory classification as speculative instrument through design and communications

## Related

- [PAYMENT_LAYER.md](PAYMENT_LAYER.md)
- [REPUTATION_SYSTEM.md](REPUTATION_SYSTEM.md)
- [SECURITY_MODEL.md](SECURITY_MODEL.md)
- [WHITEPAPER.md](WHITEPAPER.md)
