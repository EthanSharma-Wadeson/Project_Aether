# ARCHITECTURE.md — Project Aether

## Overview

Aether is layered infrastructure for autonomous agents. Upper layers consume stable primitives from lower layers. Economic mechanisms sit above core identity, permission, payment, and verification services — never the reverse.

```
┌─────────────────────────────────────────────────────────┐
│  Agent Applications & Markets (out of protocol scope)   │
├─────────────────────────────────────────────────────────┤
│  Reputation · Escrow · Discovery · Policy Oracles       │
├─────────────────────────────────────────────────────────┤
│  Payment Channels · Settlement · Fee / Bond Accounting  │
├─────────────────────────────────────────────────────────┤
│  Identity · Permissions · Attestations · Proof Verify   │
├─────────────────────────────────────────────────────────┤
│  Consensus / Data Availability / Cryptographic Core     │
└─────────────────────────────────────────────────────────┘
```

## Design Constraints

| Constraint | Implication |
|------------|-------------|
| Machine-native | APIs, messages, and state machines — not human wallet UX |
| Sub-second verification target | Heavy work off hot path; proofs/receipts verify quickly |
| Near-zero settlement overhead | Channels, batching, optimistic paths preferred |
| No HITL for standard ops | Automation-safe key use and policy engines |
| Utility-first tokens | Bonding, spam cost, resource meters — not narrative assets |

## Core Components

### 1. Identity Plane

- Agent identifiers (non-sovereign cryptographic IDs)
- Key material, rotation, and delegation
- Capability / permission roots bound to identity

→ [IDENTITY_SYSTEM.md](IDENTITY_SYSTEM.md), [AI_AGENT_MODEL.md](AI_AGENT_MODEL.md)

### 2. Permission Plane

- Fine-grained action limits: spend, counterparties, methods, time, rate
- Machine-enforceable envelopes evaluated before state transitions
- Delegation without transferring root identity

### 3. Payment Plane

- Micro-fractional channels between agents
- Settlement to shared ledger or hub with batch finality
- Fee and bond accounting for anti-spam and resource allocation

→ [PAYMENT_LAYER.md](PAYMENT_LAYER.md)

### 4. Reputation Plane

- Append-only or checkpointed execution/settlement history
- Scoring rules resistant to trivial Sybil and wash patterns
- Queryable reputation objects for counterparty selection

→ [REPUTATION_SYSTEM.md](REPUTATION_SYSTEM.md)

### 5. Verification & Privacy Plane

- Attestation and proof formats for task outcomes
- Selective disclosure of credentials and claims
- Separation of public settlement facts from private agent internals

→ [PRIVACY_MODEL.md](PRIVACY_MODEL.md)

### 6. Consensus & Availability Plane

- Ordering and finality for identity registrations, settlements, disputes
- Tuned for agent transaction patterns, not retail DeFi UX

→ [CONSENSUS_DESIGN.md](CONSENSUS_DESIGN.md)

## Trust Boundaries

| Boundary | Trust assumption |
|----------|------------------|
| Agent runtime | Untrusted; may be malicious or buggy |
| Counterparty agent | Untrusted; rely on bonds, escrow, proofs, reputation |
| Protocol validators / operators | Honest-majority or equivalent security model (TBD) |
| Off-chain evidence providers | Explicit attestation trust; slashable where bonded |

## Data Flows (Canonical)

### Register & bond

1. Agent generates keys → registers identity object  
2. Posts bond / anti-spam stake  
3. Publishes permission root and optional metadata commitments  

### Transact

1. Agents open or reuse payment channel  
2. Exchange signed micro-updates off hot ledger path  
3. Periodically settle or close; disputes resolved on protocol  

### Prove & score

1. Agent produces attestation / proof of work product  
2. Verifier checks proof + permission envelope  
3. Outcome events update reputation under protocol rules  

## Non-Goals (Architecture)

- Embedding an LLM or agent framework in the protocol
- Requiring a single EVM-compatible execution environment
- Making the token the primary product surface

## Open Architectural Choices

**Decided (v0.1):** Settlement-agnostic agent trust protocol with pluggable settlement backends — not a native L1. See [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md).

Still open — tracked in `research/unanswered_questions.md`:

- Which settlement backend(s) for first prototype
- Primary v1 verification backend (receipts / TEE / ZK / optimistic)
- Hub-and-spoke vs. mesh channel topology defaults

## Related Specs

- Wire-level detail: [PROTOCOL_DESIGN.md](PROTOCOL_DESIGN.md)
- Threats: [SECURITY_MODEL.md](SECURITY_MODEL.md)
- Incentives: [ECONOMIC_MODEL.md](ECONOMIC_MODEL.md)
