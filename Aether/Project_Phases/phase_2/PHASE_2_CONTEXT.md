# PHASE_2_CONTEXT.md — Mainframe Architecture & Protocol Expansion

## Status

**Active — design and planning only; no implementation authorised**

Phase 1 (PROTO-0, PROTO-1, PROTO-2) is complete. Phase 2 begins with architecture review before any networked or production-shaped code.

| Artifact | Role |
|----------|------|
| [MAINFRAME_ARCHITECTURE.md](MAINFRAME_ARCHITECTURE.md) | Long-term protocol stack map |
| [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) | Open decision records (DEC-P2-###) |
| [MAINFRAME_THREAT_MODEL.md](MAINFRAME_THREAT_MODEL.md) | Expanded adversary and assumption model |
| [BUSINESS_ALIGNMENT.md](BUSINESS_ALIGNMENT.md) | User-segment value mapping (not product design) |
| [PROTO_NET_0_RESULTS.md](PROTO_NET_0_RESULTS.md) | PROTO-NET-0 transport foundation results |

References: [Phase 1 results](../phase_1/PROTO_2_RESULTS.md), [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md), [roadmap/phase_2.md](../../Aether_docs/roadmap/phase_2.md).

---

## Phase 2 Purpose

Phase 2 answers one question before implementation resumes:

> **What infrastructure must exist for autonomous agents to safely coordinate economically at scale?**

Phase 1 proved that **local rule engines** for identity, bilateral state, and escrow can be written correctly and fail closed. Phase 2 defines the **mainframe** — the protocol operating system — that sits between those primitives and a real agent economy.

**Progress:** Architecture gate documents complete. **PROTO-NET-0** (secure transport foundation) implemented as local simulation — see [PROTO_NET_0_RESULTS.md](PROTO_NET_0_RESULTS.md). Awaiting security review before PROTO-4 or further networked prototypes.

---

## Why Phase 2 Exists

Phase 1 closed the "can we write the rules?" question. It opened the "what must exist around the rules?" question.

```text
Phase 1 answered:     Can authorised agents follow deterministic protocol rules locally?
Phase 2 must answer:  What distributed infrastructure makes those rules meaningful at scale?
```

Without Phase 2 architecture:

- PROTO-0/1/2 remain a library demo, not a coordination substrate
- Settlement, discovery, and reputation get bolted on ad hoc
- Security claims from local tests get over-interpreted for production
- Product decisions precede protocol boundaries

---

## What Phase 1 Proved

Evidence: 183 tests, clippy clean, frozen acceptance suites, `SECURITY_MODEL.md` updated.

| Layer | Prototype | Demonstrated (local harness) |
|-------|-----------|------------------------------|
| Authority | PROTO-0 | Identity registration, capability grants, delegation narrowing, revoke/freeze/expiry, fail-closed authorisation |
| Bilateral state | PROTO-1 | Dual-signed channel history, commitment chain, dispute selection, capability-gated terminal ops |
| Economic coordination | PROTO-2 | Dual-signed terms, receipt-based escrow, timeout/refund, fee budgeting, value conservation |

Cross-cutting:

- Signature proves key control, not truth or output quality
- Capability enforcement precedes economic authorisation
- Deterministic state transitions under injected logical time
- Separate stores with transition-only mutation (no public `get_mut`)

---

## What Phase 1 Did Not Prove

| Gap | Implication |
|-----|-------------|
| No networking | Rules untested against delay, partition, replay across peers |
| No settlement backend | No real-money custody, finality, or chargeback handling |
| Local revocation only | Capability revoke does not propagate across processes |
| Trusted logical time | Clock adversary not modelled |
| Local dispute resolver | Not distributed arbitration or legal enforcement |
| No discovery | Agents cannot find each other without out-of-band IDs |
| No reputation | Sybil, wash trading, and trust bootstrapping unaddressed |
| No key rotation mid-session | Operational compromise model incomplete |
| Receipt ≠ work quality | Colluding authorised parties can sign bounded false claims |
| Channels and escrow isolated | No integrated agent budgeting loop across primitives |

A passing Phase 1 suite is **not** evidence of production agent-economy security.

---

## Non-Goals (Phase 2 Design)

Phase 2 architecture work must **not**:

- implement networking, transports, or peer discovery code
- deploy blockchain, tokens, or smart contracts
- build marketplaces, UIs, or the Control Plane product
- choose a settlement backend (research options only)
- claim production readiness from Phase 1 evidence
- weaken Phase 0 security invariants or Phase 1 remediations
- begin PROTO-3+ implementation without architecture gate approval

---

## Relationship to Project Phases

```text
Phase 0  →  Pre-implementation readiness        [Closed]
Phase 1  →  Primitives & threat foundations     [Closed — PROTO-0/1/2]
Phase 2  →  Mainframe architecture + expansion   [Active — design gate]
Phase 3  →  Open agent economy substrate         [Pending]
```

Phase 2 has two logical stages:

1. **Architecture gate (current)** — documents in this folder; no code
2. **Networked expansion (future)** — PROTO-3/4/5 and settlement integration; only after architecture gate passes

See [roadmap/phase_2.md](../../Aether_docs/roadmap/phase_2.md) for implementation-oriented objectives that follow this gate.

---

## Exit Criteria

Phase 2 architecture gate passes when:

1. [MAINFRAME_ARCHITECTURE.md](MAINFRAME_ARCHITECTURE.md) defines the full stack, missing primitives, dependency graph, and prototype candidates
2. [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) records all major unknowns with options, trade-offs, and evidence requirements
3. [MAINFRAME_THREAT_MODEL.md](MAINFRAME_THREAT_MODEL.md) separates prototype-evidenced claims from future assumptions
4. [BUSINESS_ALIGNMENT.md](BUSINESS_ALIGNMENT.md) maps protocol value to at least one credible user segment
5. [unanswered_questions.md](../../Aether_docs/research/unanswered_questions.md) updated with Phase 2 backlog
6. Explicit review approves **which prototype to build first** after the gate
7. No implementation begins until items 1–6 are accepted

Phase 2 **fails** if architecture documents are skipped in favour of ad hoc coding.

---

## Success Statement

Phase 2 succeeds when the project can answer, with documented trade-offs:

> What infrastructure must exist for autonomous agents to safely coordinate economically at scale?

Only then does networked implementation restart.
