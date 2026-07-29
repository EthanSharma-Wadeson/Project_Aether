# ARCHITECTURE_DECISIONS.md — Phase 2 Decision Records

## Status

**Provisional decision log — architecture gate; not implementation authority**

Owner: Project Lead  
Date: 2026-07-28

Status meanings:

| Status | Meaning |
|--------|---------|
| **Open** | Requires research and evidence before provisional lock |
| **Provisional** | Working direction for design; may change after spike |
| **Locked** | Accepted for Phase 2+; change requires explicit revision |

---

## Decision Index

| ID | Question | Status | Confidence |
|----|----------|--------|------------|
| DEC-P2-001 | Agent discovery mechanism | Open | Low |
| DEC-P2-002 | Network trust model | Open | Low |
| DEC-P2-003 | Settlement philosophy | Open | Medium |
| DEC-P2-004 | Reputation architecture | **Locked (PROTO-3 design)** | Medium-High |
| DEC-P2-005 | Permission scaling model | Provisional | Medium |
| DEC-P2-006 | First networked prototype | Open | Low |
| DEC-P2-007 | Default network topology | Open | Low |
| DEC-P2-008 | Revocation propagation model | Open | Medium |
| DEC-P2-009 | Key rotation policy | Provisional | Medium |
| DEC-P2-010 | Discovery vs marketplace boundary | Provisional | High |

---

## DEC-P2-001 — Agent Discovery Mechanism

**Question:** How do agents find each other and advertised services?

### Options considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A — Curated directory** | Central or federated registry; agents register `AgentId` + metadata | Simple, enterprise-friendly, Sybil gateable | Centralisation, operator trust |
| **B — DHT / decentralised index** | Distributed hash table keyed by capability or service type | Censorship-resistant, no single operator | Sybil flood, slow convergence, ops complexity |
| **C — Out-of-band only** | Agents exchange IDs via application layer (marketplace, email, config) | Zero protocol work | No interoperable discovery |
| **D — Hybrid** | Curated directory for bootstrapping + optional DHT for open network | Best of both | Two systems to maintain |

### Trade-offs

- Enterprise pilots favour **A** or **D** (curated bootstrap).
- Open agent economy eventually needs **B** or **D**, but not for first networked prototype.
- Discovery advertises **metadata and capability summaries**, not grants themselves.

### Evidence required

- Enterprise pilot interview: is curated directory acceptable?
- Sybil cost model: bond required for directory registration?
- Latency/availability SLO for directory lookup

### Provisional recommendation

**Option D (hybrid), starting with curated directory only** for first networked pilot. DHT deferred to Phase 3.

**Status:** Open  
**Confidence:** Low — needs business wedge input from [BUSINESS_ALIGNMENT.md](BUSINESS_ALIGNMENT.md)

---

## DEC-P2-002 — Network Trust Model

**Question:** What trust assumptions apply when agents communicate across processes or networks?

### Options considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A — Authenticated peer** | Every message requires valid `SignedMessage` + live capability check | Matches Phase 1 model | Requires revocation sync |
| **B — Session-based** | Handshake establishes session keys; messages authenticated per session | Lower per-message cost | Session compromise risk |
| **C — Relay-trusted** | Hub validates and forwards; peers trust relay for delivery not content | Enterprise hub model | Relay becomes bottleneck and trust root |
| **D — A + B combined** | Session for transport; signed envelopes for economic messages | Defence in depth | More complexity |

### Trade-offs

- Economic messages must **always** use full signed envelope path (Phase 1 invariant).
- Session keys may protect non-economic metadata only unless proven equivalent.
- Relay model acceptable for enterprise; open network needs peer-to-peer path.

### Evidence required

- PROTO-NET-0 spike: session + signed envelope interop
- Revocation freshness SLA: max stale-grant window before reject
- Relay compromise threat analysis

### Provisional recommendation

**Option D** — `SecureSessionV0` for transport efficiency; `SignedMessage` mandatory for all economic operations.

**Status:** Open  
**Confidence:** Low

---

## DEC-P2-003 — Settlement Philosophy

**Question:** How does Aether relate to real value movement?

### Options considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A — Settlement-agnostic (current)** | Protocol defines binding + finality stages; backends pluggable | Flexible, no chain lock-in | Adapter work per backend |
| **B — Single backend first** | One chosen backend (e.g. stablecoin L2 or enterprise ledger) | Faster pilot | May bias protocol design |
| **C — Custodial bridge** | Licensed custodian holds funds; agents authorise via capabilities | Regulatory clarity | Counterparty risk |
| **D — Fiat rails via partner** | Traditional payment API as backend | Enterprise familiarity | Slow, human-centric dispute paths |

### Trade-offs

- CONSENSUS_AND_SETTLEMENT.md already commits to **A** at v0.1.
- First pilot needs **one concrete adapter** (B, C, or D) without closing the architecture.
- `hard_settlement_placeholder` becomes true only when backend confirms inclusion.

### Evidence required

- PROTO-4 spike: latency, finality, fee per backend candidate
- Regulatory review for custodial vs non-custodial path
- Enterprise buyer preference (fiat vs stablecoin vs internal ledger)

### Provisional recommendation

**Maintain A (settlement-agnostic).** First PROTO-4 spike: **`enterprise.ledger.v0`** stub (see [PROTO_4_DECISIONS.md](PROTO_4_DECISIONS.md) P4-DEC-003). Optional second candidate: payments sandbox. No public chain required for enterprise wedge.

**Status:** Provisional (design recorded in PROTO-4 docs)  
**Confidence:** Medium-High

---

## DEC-P2-004 — Reputation Architecture

**Question:** How is agent trust computed and queried?

### Options considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A — Indexer-derived only** | Scores computed off-chain from signed protocol events | Flexible, no consensus needed | Indexer trust, fragmentation |
| **B — Consensus-canonical scores** | On-chain or BFT-agreed reputation state | Portable, verifiable | Expensive, slow updates |
| **C — Hybrid** | Canonical events on-chain; scores derived by competing indexers | Auditability + flexibility | Two layers |
| **D — Bond-only (no scores)** | Trust = stake size; no history | Simple anti-Sybil | No quality signal |

### Trade-offs

- Reputation without bonds is farmable (Sybil).
- Scores without evidence are opaque (platform lock-in repeat).
- PROTO-3 should implement **A** with evidence refs; upgrade path to **C**.

### Evidence required

- PROTO-3 adversarial suite: wash trading, fake escrow, score manipulation ROI
- Query latency SLO for agent hire decisions
- Minimum event set for meaningful score

### Provisional recommendation

**Option A for PROTO-3**, with `ReputationEventV0` schema locked and evidence refs mandatory in query responses. Upgrade path to C if indexer trust becomes blocker.

**Status:** Provisional  
**Confidence:** Medium

---

## DEC-P2-005 — Permission Scaling Model

**Question:** When and how do permission roots scale beyond linear verification?

### Options considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A — Linear (current)** | Full grant verification per action | Simple, already implemented | O(n) revoke checks at scale |
| **B — Merkle permission root** | Root commitment; grants are Merkle proofs | Compact, auditable history | Implementation complexity |
| **C — Accumulator-based revoke** | RSA/Verkle accumulator for revocation | Compact revoke set | Crypto complexity, setup |
| **D — Time-bounded grants only** | Short TTL; no long-lived grants | Limits stale exposure | Operational burden |

### Trade-offs

- Enterprise single-tenant: **A + D** sufficient for years.
- Multi-issuer federation: **B** becomes necessary.
- Revocation propagation (DEC-P2-008) matters more than Merkle for first pilot.

### Evidence required

- Grant volume projection for enterprise fleet (agents × capabilities)
- Revoke freshness requirements under network delay
- Merkle proof verification cost benchmark

### Provisional recommendation

**Stay on A for first networked pilot.** Design `PermissionRootMerkleV0` interface now; implement before public federation.

**Status:** Provisional  
**Confidence:** Medium

---

## DEC-P2-006 — First Networked Prototype Selection

**Question:** After architecture gate, which prototype is built first?

### Options considered

| Option | Prototype | Best for |
|--------|-----------|----------|
| **A** | PROTO-NET-0 (transport + session) | Open agent coordination, marketplace wedge |
| **B** | PROTO-4 (settlement backend spike) | Enterprise spend control wedge |
| **C** | Networked PROTO-1 (channel sync) | High-frequency bilateral state use case |
| **D** | PROTO-3 (reputation indexer) | Trust-bootstrapping use case |

### Evidence required

- Business alignment review outcome
- Customer interview or design partner commitment
- Technical dependency analysis (see MAINFRAME_ARCHITECTURE.md §6)

### Provisional recommendation

**Wedge locked (pending acceptance):** Enterprise Agent Spend Control — see [PHASE_2_WEDGE_DECISION.md](PHASE_2_WEDGE_DECISION.md).

Sequence: PROTO-NET-0 (**done**) → **PROTO-4** (settlement binding + one adapter) → multi-host NET integration as needed.

Agent-to-agent economy remains strategic follow-on, not the next build.

**Status:** Provisional (wedge decision recorded; awaiting explicit acceptance)  
**Confidence:** Medium-High

---

## DEC-P2-007 — Default Network Topology

**Question:** Mesh peer-to-peer vs hub-and-spoke?

### Options considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A — Hub-and-spoke** | Agents connect to regional relay/coordinator | Enterprise policy enforcement, simpler NAT | Hub trust, SPOF |
| **B — Full mesh** | Direct peer connections | Decentralised | NAT, scale, discovery hard |
| **C — Hybrid** | Hub for discovery/routing; direct for economic messages | Balanced | Two connection modes |

### Provisional recommendation

**Option C** — hub for `AgentDirectoryV0` and optional routing; direct signed messages for economic ops where policy allows.

**Status:** Open  
**Confidence:** Low

---

## DEC-P2-008 — Revocation Propagation Model

**Question:** How do capability revokes reach all verifiers before stale grants are accepted?

### Options considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A — Pull on verify** | Verifier checks revocation registry on each authorisation | Fresh | Latency, registry availability |
| **B — Push gossip** | Revokes broadcast to known peers | Fast local reject | Partition stale window |
| **C — Short TTL + pull** | Grants expire quickly; pull for early revoke | Limits exposure | Re-issue overhead |
| **D — B + C combined** | Gossip + TTL cap | Best freshness bound | Most complex |

### Evidence required

- Max acceptable stale-grant window (security review)
- Registry availability SLO
- Gossip convergence time under partition

### Provisional recommendation

**Option D** for production path; **Option C** sufficient for first two-party networked pilot.

**Status:** Open  
**Confidence:** Medium

---

## DEC-P2-009 — Key Rotation Policy

**Question:** When must agents rotate operational keys?

### Provisional policy (not yet locked)

| Trigger | Rotation required |
|---------|-------------------|
| Before networked deployment | Operational key rotation support in protocol |
| Suspected compromise | Immediate revoke + rotate |
| Scheduled | Configurable TTL (enterprise policy) |
| Mid-channel / mid-escrow | Grace period with dual-key acceptance window |

**Status:** Provisional  
**Confidence:** Medium

---

## DEC-P2-010 — Discovery vs Marketplace Boundary

**Question:** Is agent discovery a protocol service or application-layer concern?

### Decision

**Discovery directory is a protocol primitive (`AgentDirectoryV0`).**  
**Marketplace matching, pricing, and UI are application layer.**

Rationale: discovery requires standard schema and trust rules; marketplaces compete on top.

**Status:** Provisional  
**Confidence:** High

---

## Review Triggers

| Decision | Revisit when |
|----------|--------------|
| DEC-P2-001 | First design partner named |
| DEC-P2-003 | PROTO-4 spike complete |
| DEC-P2-004 | PROTO-3 adversarial results |
| DEC-P2-006 | Business alignment review complete |
| DEC-P2-005 | Grant volume exceeds 10k active grants per issuer |

---

## Freeze Statement

> Phase 2 decisions are provisional until evidence from spikes and business alignment review. Locked status requires explicit architecture gate approval.
