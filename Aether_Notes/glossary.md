# Glossary — Quick Reference

Terms you'll encounter across Aether docs and notes.

---

## Core concepts

| Term | Definition |
|------|------------|
| **Agent** | Autonomous software actor with keys, identity, and capabilities — not an LLM |
| **Principal** | Entity that funds bonds and sets root policy (human org, DAO, or parent agent) |
| **Counterparty** | Another agent in a bilateral economic flow |
| **Verifier** | Party or network rule that checks attestations/proofs |
| **Indexer** | Reads protocol events; may compute reputation views |

---

## Identity & permissions

| Term | Definition |
|------|------------|
| **Agent ID** | Cryptographic identifier derived from public key material |
| **Non-sovereign identity** | Not issued by nation-state or single platform as root of trust |
| **Permission root** | Root of the capability tree bound to an identity |
| **Capability envelope** | Signed, scoped allowance: spend limits, actions, counterparties, expiry |
| **Delegation** | Issuing scoped capability to another key/agent without identity transfer |
| **HITL** | Human-in-the-loop — explicitly avoided for standard operations |

---

## Payments & settlement

| Term | Definition |
|------|------------|
| **Payment channel** | Bilateral off-chain state for fast value transfer; settles on-chain periodically |
| **Channel update** | Signed off-chain balance/state change with monotonic sequence number |
| **Soft finality** | Counterparty-authenticated channel update — instant, bilateral |
| **Settlement final** | Transaction final per settlement backend rules |
| **Dispute-window final** | Challenge period expired without accepted dispute |
| **Economic final** | Funds, bonds, reputation applied — workflow complete |
| **SettlementBinding** | Capability linking Aether Agent ID to a backend account |
| **Escrow** | Value locked conditional on attestation success or timeout |
| **Dispute window** | Time period to challenge task outcome before economic finality |
| **Watchtower** | Service that monitors channels and submits disputes on behalf of offline parties |

---

## Trust & reputation

| Term | Definition |
|------|------------|
| **Attestation** | Signed claim about work/outcome, optionally with proof payload |
| **Receipt** | Deterministic hash of outputs — simplest v1 verification approach |
| **Reputation evidence** | Protocol events (settlements, disputes, attestations) feeding scores |
| **Score vector** | Multi-dimensional reputation output (not just one number) |
| **Sybil attack** | Many fake identities to game reputation or spam |
| **Wash trading** | Fake transactions between colluding parties to inflate reputation |

---

## Privacy & verification

| Term | Definition |
|------|------------|
| **Selective disclosure** | Revealing only attributes a verifier needs, not full data |
| **ZK proof** | Zero-knowledge proof — proves property without revealing inputs |
| **TEE** | Trusted Execution Environment — hardware-enclosed computation attestation |
| **Optimistic verification** | Assume valid unless challenged with fraud proof |
| **Claim type** | Machine-checkable condition for escrow (output hash, metric threshold) |

---

## Economics

| Term | Definition |
|------|------------|
| **Bond** | Locked value slashable for fraud, equivocation, or rule violations |
| **Slash** | Forfeiture of bonded value as penalty |
| **Anti-spam stake** | Minimum bond/fee to register or access message classes |
| **Utility token** | Token for bonding, fees, metering — not speculation |

---

## Infrastructure

| Term | Definition |
|------|------------|
| **Consensus** | Global agreement on identity, bonds, settlements, disputes |
| **Settlement plane** | Layer that finalizes channel opens, closes, and dispute resolutions |
| **Light client** | Minimal-trust verification of finalized state via compact proofs |
| **DA (Data Availability)** | Guarantee that dispute evidence is retrievable |
| **BFT** | Byzantine Fault Tolerance — consensus despite malicious participants |

---

## Acronyms

| Acronym | Expansion |
|---------|-----------|
| HITL | Human-in-the-loop |
| KYC | Know Your Customer |
| AML | Anti-Money Laundering |
| DID | Decentralized Identifier |
| VC | Verifiable Credential |
| TEE | Trusted Execution Environment |
| ZK | Zero-Knowledge |
| ZKML | Zero-Knowledge Machine Learning |
| DA | Data Availability |
| BFT | Byzantine Fault Tolerance |
| SLO | Service Level Objective |
| ROI | Return on Investment |
| MEV | Maximal Extractable Value |

---

## Message types (protocol)

| Message | Purpose |
|---------|---------|
| IdentityRegister | Publish agent ID, keys, permission root |
| IdentityRotate | Rotate keys under existing ID |
| IdentityDelegate | Issue scoped capability |
| CapabilityGrant | Scoped allowances with constraints |
| ChannelOpen | Start payment channel |
| ChannelUpdate | Off-chain balance update |
| ChannelSettle | Submit state for on-protocol finalization |
| ChannelDispute | Challenge state within dispute window |
| AttestationSubmit | Claim about work with proof |
| ReputationQuery | Request score vector for an agent |

---

## Status values

| Status | Context |
|--------|---------|
| Active | Identity or channel in normal operation |
| Frozen | Identity temporarily suspended |
| Revoked | Identity or capability permanently invalidated |
| Open / Active / Closing / Disputed / Settled | Channel lifecycle states |
