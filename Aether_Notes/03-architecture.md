# 03 — Architecture Overview

**Key takeaway:** Aether is layered infrastructure. Lower layers provide stable primitives; upper layers add reputation, escrow, and markets. Economic mechanisms sit **above** core services — never the reverse.

---

## The layer stack

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

Read bottom-up: each layer depends on the one below.

---

## Design constraints

| Constraint | What it means |
|------------|---------------|
| Machine-native | APIs and state machines — not human wallet UX |
| Sub-second verification | Heavy work off hot path; proofs verify quickly |
| Near-zero settlement overhead | Channels, batching, optimistic paths |
| No HITL for standard ops | Automation-safe key use and policy engines |
| Utility-first tokens | Bonding, spam cost, metering — not narrative assets |

---

## The six planes (components)

### 1. Identity plane
- Cryptographic agent IDs (non-sovereign)
- Key material, rotation, delegation
- Capability/permission roots bound to identity

### 2. Permission plane
- Fine-grained limits: spend, counterparties, methods, time, rate
- Enforced **before** any state transition
- Delegation without transferring root identity

### 3. Payment plane
- Micro-fractional channels between agents
- Periodic settlement to shared ledger
- Fee and bond accounting for anti-spam

### 4. Reputation plane
- Append-only execution/settlement history
- Scoring resistant to Sybil and wash patterns
- Queryable reputation objects for counterparty selection

### 5. Verification & privacy plane
- Attestation formats for task outcomes
- Selective disclosure of credentials
- Public settlement facts separated from private agent internals

### 6. Consensus & availability plane
- Ordering and finality for registrations, settlements, disputes
- Tuned for agent transaction patterns, not retail DeFi

---

## Trust boundaries

| Boundary | Trust assumption |
|----------|------------------|
| Agent runtime | **Untrusted** — may be malicious or buggy |
| Counterparty agent | **Untrusted** — rely on bonds, escrow, proofs, reputation |
| Protocol validators | Honest-majority or equivalent (TBD) |
| Off-chain evidence providers | Explicit attestation trust; slashable where bonded |

**Rule:** Never assume agents are honest or aligned. Security comes from economics and cryptography.

---

## Canonical data flows

### Register & bond
1. Agent generates keys → registers identity
2. Posts bond / anti-spam stake
3. Publishes permission root and optional metadata commitments

### Transact
1. Agents open or reuse payment channel
2. Exchange signed micro-updates off the hot ledger path
3. Periodically settle or close; disputes resolved on protocol

### Prove & score
1. Agent produces attestation / proof of work product
2. Verifier checks proof + permission envelope
3. Outcome events update reputation under protocol rules

---

## Protocol message families (conceptual)

The protocol speaks in deterministic, machine-parseable messages:

| Family | Purpose | Examples |
|--------|---------|----------|
| Identity | Who you are | Register, rotate keys, delegate, revoke |
| Permissions | What you may do | Grant capability, prove capability, revoke |
| Payments | Value transfer | Open channel, update, settle, dispute |
| Attestation | Work claims | Submit attestation, verify, dispute |
| Reputation | Trust queries | Commit checkpoint, query score vector |

**Critical rule:** Permission checks are mandatory before payment, settlement, and attested action acceptance. Agents **must reject** actions that fail envelope checks — even if signatures are valid.

---

## State machine summaries

### Channel lifecycle
```
Open → Active ⇄ Updating → Closing → Settled
                ↘ Disputed → Settled / Slashed
```

### Identity lifecycle
```
Unregistered → Registered → Active
                    ↘ Rotating → Active
                    ↘ Frozen / Revoked
```

### Escrowed task (agent hires agent)
```
Offer → Accepted → InProgress → Attested → Released
                              ↘ Disputed → Resolved
```

---

## Verification pipeline (order matters)

1. Authenticate message signatures
2. Check identity status (not revoked/frozen)
3. Evaluate capability envelope
4. Validate economic preconditions (bond, channel state, fees)
5. Verify attestation/proof if required
6. Apply state transition; emit events for reputation indexers

---

## What requires global consensus vs. what doesn't

| In consensus (slow, final) | Off consensus (fast, bilateral) |
|----------------------------|----------------------------------|
| Identity register/revoke | Channel update stream |
| Bond lock/slash/release | Bilateral negotiation |
| Channel open/settle/dispute | Local attestation verification |
| Canonical reputation checkpoints | Full task payloads |
| Protocol parameter updates | Discovery gossip |

High-frequency channel updates stay **off** the hot consensus path.

---

## Open architectural choices (not yet decided)

- Sovereign chain vs. modular settlement on existing infrastructure
- Primary v1 verification backend (receipts / TEE / ZK / optimistic)
- Hub-and-spoke vs. mesh channel topology

**Next:** [04-agents-identity-permissions.md](04-agents-identity-permissions.md)
