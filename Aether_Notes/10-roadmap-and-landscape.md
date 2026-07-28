# 10 — Roadmap, Landscape & Open Questions

**Key takeaway:** Delivery is phased — primitives first, networked settlement second, open substrate third. Aether's wedge is the integrated machine-native stack. Many design decisions remain open by design.

---

# Part A — Three-Phase Roadmap

## Phase 1 — Primitives & Threat Foundations

**Theme:** Smallest honest stack: identity, capabilities, channels, receipts, written threat model.

### In scope
- Agent identity register/rotate/delegate
- Capability envelopes (spend/rate/counterparty limits)
- Bilateral channel state machine (simulation)
- Receipt-based escrow flow
- Reputation indexer experiments (not consensus-canonical yet)

### Out of scope
- Native token generation events
- ZKML production proving
- Full discovery marketplace
- Human chat/agent UI
- Multiparty channel mesh at scale

### Exit criteria (must pass to advance)
- Capabilities contain loss in adversarial tests
- Channel lifecycle including dispute path simulated with metrics
- Escrow + receipt loop completes headlessly
- Settlement backend shortlist documented
- Security model reviewed against prototype findings

---

## Phase 2 — Networked Settlement & Economic Safety

**Theme:** Move from simulators to networked settlement; bonds/fees make spam and cheap fraud unprofitable.

### In scope
- Channel open/settle/dispute against chosen settlement backend
- Bond lock/slash/release flows
- Watchtower or principal-mirror reference design
- Reputation scorer v1 with published attack ROI results
- Attestation type registry (receipts required; TEE/ZK optional)
- Light-client / inclusion proof path

### Out of scope
- Mass retail consumer wallet UX
- Complex governance token politics
- Guaranteed ZK proving for arbitrary ML
- Fully decentralized discovery at global scale

### Exit criteria
- Headless agents settle micro-payments with measured soft/hard finality
- Dispute path recovers funds under crash tests
- Economic parameter sheet (bonds/fees/windows) versioned
- Security review of payment + bond paths

---

## Phase 3 — Open Agent Economy Substrate

**Theme:** Harden into substrate others build on — portable reputation, richer verification, multi-agent markets.

### In scope
- Stable identity + payment APIs for external agent frameworks
- Pluggable verification backends (TEE/ZK/optimistic) behind common interface
- Reputation queries as reliable constraint surface for capabilities
- Recursive "agent hires agent" escrow at meaningful volume
- Security response, parameter governance, light-client tooling
- Builder documentation, SDKs, conformance tests
- Published performance SLOs (p50/p95)

### Success metrics (illustrative)
- Autonomous loops completed without HITL
- Cost per 1k micro-settlements
- Dispute rate and wrongful release rate
- Time from register → first bonded job for new agents
- External builder retention

---

# Part B — Competitive Landscape

## Aether's wedge

The **integrated machine-native stack**:

```
Identity + Capabilities + Channels + Attestations + Reputation
```

Competitors often excel at one slice. Aether's risk is scope; mitigation is phased delivery.

---

## Landscape categories

| Category | Strengths | Gaps for agents |
|----------|-----------|-----------------|
| **General L1/L2 blockchains** | Security, tooling, liquidity | Human-retail UX; identity = address; weak capabilities |
| **Payment channel networks** | Micro-payments, speed | Weak agent identity/reputation integration |
| **Decentralized identity (DID/VC)** | Portable credentials, selective disclosure | Human/org centric; rarely paired with microsettlement |
| **Agent marketplaces (Web2)** | Fast iteration | Platform lock-in, opaque reputation, HITL accounts |
| **"AI × crypto" token projects** | Attention, fundraising | Thin infra; speculative primary; chatbot wrappers |
| **Confidential compute & proofs** | Execution privacy/integrity | Not a full identity+payment stack |
| **Reputation systems** | Signal for humans/apps | Not tied to agent escrow outcomes |

---

## Evaluation criteria for competitors

| Criterion | Question |
|-----------|----------|
| Agent-native | Built for autonomous software, not human wallet UX? |
| Identity | Cryptographic agent identity with permissions? |
| Payments | Micro-fractional, low-latency settlement? |
| Reputation | Provable history, not platform ratings alone? |
| Verification / privacy | Proofs or selective disclosure? |
| Non-speculative posture | Utility infrastructure vs. meme narrative? |
| Independence from HITL | Standard ops without human click-signing? |

---

# Part C — Research Agenda

## Five research pillars

1. **Distributed systems** — consensus, channels, availability for ephemeral agents
2. **Cryptography** — proofs, selective disclosure, key hierarchies
3. **Blockchain analysis** — what to adopt, wrap, or avoid from existing systems
4. **AI agent economics** — lifecycle, incentives, adversarial behavior
5. **Unanswered questions** — explicit backlog of undecided design

## Priority near-term questions

1. Minimal viable identity object for an agent?
2. Payment abstraction for sub-second micro-fractional settlement?
3. Reputation updates: on-protocol events, off-chain evidence, or hybrid?
4. Realistic v1 verification model?
5. Economic bonding that prevents Sybil floods without pricing out micro-agents?

## Research anti-patterns

- Designing tokenomics before settlement and identity primitives
- Treating "put it on an EVM L2" as an architecture
- Confusing agent product UX with protocol research
- Shipping reputation formulas without adversarial analysis

---

# Part D — Key Hypotheses to Validate

| ID | Claim | Kill if |
|----|-------|---------|
| H1 | Channels beat on-chain micropayments for agents | No clear cost/latency margin after dispute infra |
| H2 | Capability envelopes prevent principal loss | Bypassable via payment paths or delegation |
| H3 | Bonded evidence resists wash reputation | Cheap Sybil rings mint high scores without escrow risk |
| H4 | Receipt-based escrow enough for early markets | Counterparties refuse without stronger proofs |
| H5 | Agents can budget fees autonomously | Fee variance forces HITL as normal case |
| H6 | Soft vs hard finality is machine-rational | Ambiguous finality causes systematic fund loss |

---

# Part E — Open Questions

## Consensus & settlement — partially resolved (2026-07-28)

**Decided** (see `Aether_docs/CONSENSUS_AND_SETTLEMENT.md`):
- v0.1 is settlement-agnostic with pluggable backends — not a native L1
- Layered fault model; agents fully adversarial; BFT threshold for verifier committees
- Five-stage finality: PROPOSED → ACCEPTED → SETTLEMENT_FINAL → DISPUTE_WINDOW_FINAL → ECONOMIC_FINAL
- Split evidence model: on-chain commitments, off-chain storage with availability obligations
- Multi-backend identity via SettlementBinding

**Still open:**
- Which backend(s) for first prototype
- Exact dispute period bounds per task class
- Verifier committee design and evidence retention rules

## Identity & capabilities
- Exact Agent ID derivation scheme?
- Serialization format?
- Capability revocation mechanism?
- Recovery model without breaking headless ops?

## Payments
- Mesh vs. hub-and-spoke topology?
- Multiparty channels in v1?
- Native asset only vs. multi-asset at launch?
- Watchtower in-core or out-of-core?

## Reputation
- Consensus-canonical vs. indexer-derived aggregates?
- Default decay and anti-wash parameters?

## Verification & privacy
- v1 attestation profile: receipts only?
- Claims for nondeterministic model outputs?
- Default visibility of bond/settlement amounts?

## Economics
- Native utility token at genesis or delay?
- Slash redistribution policy?
- Cold-start subsidies without farmable inflation?

---

## Where to go next

- **Full specs:** `Aether_docs/` sibling files
- **Quick reference:** [glossary.md](glossary.md)
- **Day plan:** [README.md](README.md)
