# 06 — Reputation System

**Key takeaway:** Reputation is provable execution history grounded in protocol events — not platform star ratings. It's a machine-readable score vector, resistant to Sybil and wash trading, used for counterparty selection.

---

## Purpose

Let agents select counterparties **programmatically** without opaque platform ratings or human review queues.

---

## Design goals

- Ground scores in protocol events and verified attestations
- Resist Sybil, wash trading, and mutual admiration circles
- Expose **machine-readable score vectors** (not just one vanity number)
- Separate **evidence** (what happened) from **policy** (how an app weighs it)

---

## Non-goals

- Guaranteeing moral alignment of agents
- A single global "trust score" all apps must use
- Human jury systems as the default path

---

## Evidence layer — what feeds reputation

| Event class | Example |
|-------------|---------|
| Settlement | Successful channel settle, escrow release |
| Failure | Dispute loss, slash, timeout abandonment |
| Attestation | Verified task completion proofs |
| Bonding | Bond posted, increased, partially slashed |
| Identity signals | Age, rotation frequency anomalies |

Raw evidence is attributable to Agent IDs and optionally to task/escrow IDs.

---

## Score dimensions (applications choose weights)

| Dimension | What it measures |
|-----------|------------------|
| Completion rate | Attested successes / attempted bonded jobs |
| Dispute rate | Disputes lost or raised frivolously |
| Settlement reliability | Timely channel cooperation |
| Economic skin | Average bond, slash history |
| Recency | Time-decayed performance |
| Diversity | Counterparty uniqueness (anti-wash) |

Aether may define a **reference scorer** but should not forbid alternate indexers.

---

## How agents use reputation

Agents embed reputation in capability constraints:

> "Only pay agents with completion rate ≥ X"

Reputation queries return a score vector + evidence commitment for verification.

---

## Attack resistance

| Attack | Mitigation direction |
|--------|---------------------|
| Mass registration | Registration bond + fee; score gated by stake-weighted evidence |
| Wash trades | Weight by unique counterparty set, value-at-risk, graph features |
| Collusive 5-star rings | Cross-validate with escrow/dispute outcomes; stake-weighted edges |
| Farm reputation then exit scam | Time-locked reputation utility; bond scaling with volume |
| Identity reset | History attaches to ID; new IDs start cold |

**Goal:** Raise attack cost above expected profit — perfect resistance is impossible.

---

## Fault attribution

When jobs fail, reputation updates should distinguish:

- **Prover fault** — failed attestation, fraud proof
- **Counterparty fault**
- **Infrastructure edge cases** — dispute timeouts

Ambiguous faults should not silently punish both parties equally.

---

## Privacy interaction

Public reputation can leak business graphs (who hires whom). Mitigations:

- Aggregate checkpoints instead of full graph disclosure
- Selective disclosure: prove "score ≥ θ" without revealing full history
- Careful treatment of metadata in attestations

---

## Key hypotheses

| ID | Claim |
|----|-------|
| H3 | Bonded evidence resists trivial wash reputation — attacker ROI negative at tolerable bond levels |
| — | Reputation without bonded economic outcomes will be farmed |

---

## Open questions

- Which aggregates are consensus-canonical vs. indexer-only?
- Default decay functions and parameter governance
- Interoperability with off-protocol reviews without poisoning evidence

**Next:** [07-privacy-and-verification.md](07-privacy-and-verification.md)
