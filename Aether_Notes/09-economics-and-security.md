# 09 — Economics & Security

**Key takeaway:** Economics uses bonds, fees, and escrow to make attacks unprofitable. Security assumes all agents are untrusted. Tokens (if any) are utility instruments — bonding, metering, anti-spam — never the product.

---

# Part A — Economic Model

## Design philosophy

1. Infrastructure first, economic mechanisms second
2. Utility over speculation
3. Price abuse above honest micro-agent costs where possible

---

## Economic primitives

### Bonds
Agents lock value to:
- Register identity / access message classes
- Open channels above certain volumes
- Raise disputes
- Underwrite attested work (performance bonds)

Bonds are **slashable** for proven fraud, equivocation, or rule violations.

### Fees
Small fees meter scarce resources:
- Settlement inclusion
- Identity registration
- Dispute processing
- Optional relay / routing services

Fees must be **predictable** for automated budgeting.

### Escrow
Conditional value transfer tied to attestations and timeouts. Primary coordination tool for agent-to-agent work markets.

### Reputation-linked limits
Economic capacity may scale with evidence-backed reputation to reduce cold-start abuse without permanent caste systems.

---

## Token utility (if issued)

| Acceptable utility | Role |
|--------------------|------|
| Bonding asset | Skin-in-the-game for identity and disputes |
| Fee payment | Meter settlement and registration |
| Resource credits | Prepaid inclusion / bandwidth tickets |
| Staking for services | Secure relays, indexers, verifiers |

| Unacceptable primary utility | Why |
|------------------------------|-----|
| Pure governance theater | No protocol need |
| Mandatory speculation for basic identity | Excludes legitimate micro-agents |

Bridged external assets may serve as bonds/fees if security and machine UX allow.

---

## Incentive alignment

| Actor | Desired behavior | Failure if mispriced |
|-------|------------------|----------------------|
| Honest micro-agents | Participate cheaply | Market dies |
| Sybil attacker | Unprofitable floods | Reputation/payment spam |
| Malicious provider | Slash > cheat profit | Exit scams dominate |
| Frivolous disputer | Loses dispute bond | Griefing freezes funds |
| Validators / services | Reliable inclusion | Censorship / extractive MEV |

---

## Anti-spam policy (draft)

1. Minimum registration bond or fee
2. Rate limits via capabilities and protocol meters
3. Stake-weighted tolerance for burst traffic
4. Escalating cost for repeated failed attestations / disputes

---

## Value flow (conceptual)

```
Principal funds agent bond & channel deposits
    → Agent pays micro-fees for work / services
    → Escrow releases on attestation
    → Settlements finalize
    → Slashes redistribute or burn per rules (TBD)
```

---

## Key hypothesis (H5)

**Agents can budget protocol fees autonomously** with predictable fee/bond schedules. Kill criterion: fee variance forces human intervention as the normal case.

---

# Part B — Security Model

## Protected assets

| Asset | Why it matters |
|-------|----------------|
| Agent keys | Control of identity actions and funds |
| Channel state integrity | Theft via stale or forged updates |
| Bonded value | Direct economic loss |
| Attestation validity | Wrongful escrow release / reputation poison |
| Capability envelopes | Unauthorized spend or action scope |
| Private payloads | Model IP, data, strategy |
| Protocol liveness | Agents must settle and dispute |

---

## Adversary classes

1. **Malicious agent** — cheats, farms reputation, griefs disputes
2. **Compromised agent host** — malware extracts operational keys
3. **Sybil operator** — many identities to overwhelm discovery/reputation
4. **Colluding cartel** — wash activity, censorship if they run infrastructure
5. **Network adversary** — delay, partition, replay
6. **Malicious principal** — configures agents to externalize harm
7. **Curious counterparty** — extracts private data from overdisclosure

---

## Trust assumptions

- Cryptographic primitives behave as specified
- Consensus fault threshold holds (TBD)
- Agents verify capabilities and proofs — don't trust free-text claims
- Off-chain channel safety depends on timely dispute participation or watchtowers
- TEEs / ZK systems carry their own trust assumptions — must be explicit

---

## Security properties

| Property | Intent |
|----------|--------|
| Authentication | Messages bound to authorized keys |
| Authorization | Capabilities constrain actions |
| Integrity | Settlements match authorized latest states |
| Accountability | Fraud attributable and slashable |
| Availability | Honest parties can eventually finalize |
| Privacy | Selective disclosure |
| Non-repudiation | Signed updates bind parties |

---

## Key risk mitigations

| Risk | Mitigation |
|------|------------|
| Key compromise | Hierarchical keys + rapid rotation |
| Blast radius | Capability scopes with low limits |
| Principal requirements | Hardware / enclave custody where needed |
| Emergency | Freeze paths for root holders |
| Hot root on agent hosts | Avoid single long-lived hot root key |

---

## Payment & dispute security

- Monotonic sequence numbers — reject stale state
- Dispute windows sized to adversary delay assumptions
- Watchtowers or principal backups for offline agents
- Slash for submitting revoked/fraudulent state
- Capability checks prevent "both parties agree to violate principal limits"

---

## Key hypothesis (H2)

**Capability envelopes prevent principal loss** when operational keys are compromised. Kill criterion: envelope checks bypassable via payment paths or delegation tricks.

---

## Incident response (protocol level)

- Identity freeze / mass revoke procedures
- Channel emergency close
- Parameter circuit breakers for fee/bond misconfigurations
- Disclosure process for critical vulnerabilities

---

## Project security norms

- Threat model before feature design
- Prefer simplicity on the hot path
- Explicit status of cryptographic assumptions in release notes
- No "AI safety theater" substituting for protocol security

**Next:** [10-roadmap-and-landscape.md](10-roadmap-and-landscape.md)
