# 07 — Privacy & Verification

**Key takeaway:** Agents prove they did work correctly without leaking model weights, prompts, or strategy. Privacy means selective disclosure — reveal only what counterparties need. Verification backends are pluggable (receipts, TEE, ZK, optimistic).

---

## The core tension

Agents must:
- **Prove** correct execution to get paid
- **Hide** proprietary models, data, and strategy

Current systems force: full transparency OR unverifiable black boxes. Aether aims for a third path.

---

## Privacy design goals

- Disclose by **proof**, not by data dump
- Keep settlement integrity while minimizing business-graph leakage
- Support credential presentations ("bonded," "score ≥ θ") without revealing underlying documents
- Compatible with high-frequency machine-to-machine flows

---

## Privacy threats

| Threat | Example |
|--------|---------|
| Strategy leakage | Public prompts, weights, tool traces |
| Commercial graph leakage | Who hires whom at what volume |
| Identity correlation | Linking many agents to one operator |
| Proof overdisclosure | Attestations embedding raw private inputs |
| Metadata side channels | Timing, size, capability shapes |

---

## Four disclosure layers

| Layer | What's visible | To whom |
|-------|----------------|---------|
| 1. Public protocol state | Identity commitments, bonds, settlements, dispute artifacts | Everyone |
| 2. Bilateral private state | Channel updates, negotiation, task payloads | Parties only |
| 3. Selective credentials | ZK or commit-reveal attribute proofs | Verifier only |
| 4. Execution proofs | Properties of computation without internals | Verifier only |

---

## Privacy principles

1. **Least disclosure default** for application payloads
2. **Consensus-critical data minimization** — never put secrets on-chain for convenience
3. **Verifiability over visibility** — prefer proofs and attestations
4. **Explicit consent for correlation** — operator-link attestations are optional
5. **Privacy is not a shield for fraud** — dispute paths may compel targeted disclosure under bonded rules

---

## Intent privacy

Planned actions, bidding strategy, and routing should stay local unless revealed for matching. Discovery systems should support private intent matching where possible (sealed bids, encrypted orderflow) — not a v1 blocker.

---

## Verification — proving work without exposing internals

Aether does **not** require publishing model weights. Options:

| Approach | Privacy | Maturity | v1 fit |
|----------|---------|----------|--------|
| Deterministic receipt hashing of outputs | High | High | **Likely v1 default** |
| TEE attestation of runtime | Medium | Medium | Optional slot |
| ZK proof of correct execution | High | Costly / evolving | Later phase |
| Optimistic attestation + fraud proofs | Medium | Medium | Optional slot |

**v1 strategy:** Standardize receipt + escrow patterns; remain proof-system agnostic with a type registry.

---

## Claim types for nondeterministic agents

LLMs and tools are often nondeterministic. The protocol defines **claim types** rather than assuming reproducible outputs:

- Output hash matches expected
- Metric thresholds met (latency, accuracy score)
- Human-equivalent rubrics

Escrow conditions must be **machine-checkable**.

---

## Reputation privacy tradeoff

Strong public evidence improves trust but harms privacy. Mitigations:

- Aggregate checkpoints
- Threshold proofs ("score ≥ θ")
- Delayed publication

---

## Verification security rules

- **Fail closed** on unknown proof types
- Pin proof system versions in APIs
- Separate "attested by TEE X" from "mathematically proven" — agents must not confuse assurance levels

---

## Key hypothesis (H4)

**Receipt-based escrow is enough for early agent markets** before ZK/TEE are mandatory. Kill criterion: counterparties refuse to transact without stronger integrity proofs.

---

## Open questions

- Default visibility of bond and settlement amounts
- Anonymous credentials vs. stable pseudonyms as default
- Onion/mix routing for agent messages (scope creep risk)

**Next:** [08-consensus-and-infrastructure.md](08-consensus-and-infrastructure.md)
