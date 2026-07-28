# CONSENSUS_AND_SETTLEMENT.md — Project Aether

## Status

**Draft — Architecture Decision Record**

This document records the initial design decisions and unresolved questions for Aether's consensus, settlement, finality, dispute, evidence, and multi-backend architecture.

These decisions are provisional. They must be tested through simulation, prototyping, threat modelling, and real-world measurement before becoming protocol guarantees.

---

# 1. Purpose

Aether is intended to provide machine-native infrastructure for autonomous agents, including:

* cryptographic identity
* permission enforcement
* task agreements
* escrow and settlement
* attestations
* disputes
* reputation

Before implementing these systems, Aether must determine which responsibilities belong to the Aether protocol and which should be delegated to existing blockchain infrastructure.

The central architectural question is:

> Should Aether operate its own consensus network, use shared modular infrastructure, or record settlement certificates on an existing blockchain?

---

# 2. Proposed Architecture Decision

## Aether v0.1 should not launch as its own Layer 1 blockchain

Aether should initially operate as a **modular, settlement-agnostic agent trust protocol**.

Aether should define the semantics unique to autonomous agents while using existing settlement infrastructure for transaction ordering, consensus, and base-layer security.

Conceptual architecture:

```text
┌─────────────────────────────────────────────┐
│                 AI AGENTS                   │
│                                             │
│  Discover • Negotiate • Act • Pay • Verify │
└──────────────────────┬──────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────┐
│               AETHER PROTOCOL               │
│                                             │
│ Identity • Capabilities • Task Contracts    │
│ Attestations • Disputes • Reputation        │
└──────────────────────┬──────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────┐
│          SETTLEMENT ADAPTER LAYER           │
│                                             │
│  Backend A • Backend B • Enterprise Systems │
│          • Future Aether Chain              │
└─────────────────────────────────────────────┘
```

## Rationale

Aether's unique value is not initially expected to come from:

* block production
* validator coordination
* transaction ordering
* generic token transfers
* maintaining a new consensus network

Aether's unique value is expected to come from defining:

* what an autonomous agent is
* how agents prove identity
* how principals delegate authority
* how permissions are enforced
* how agents negotiate and perform tasks
* how work is attested
* how disputes are resolved
* how reputation is accumulated

Existing networks already provide mature approaches to transaction settlement and consensus.

Launching a new Layer 1 before validating Aether's agent-native primitives would create unnecessary technical, economic, and security burdens.

---

# 3. Long-Term Native Chain Criteria

A native Aether chain is not ruled out.

However, a native chain should only be considered if evidence demonstrates that existing settlement infrastructure cannot satisfy Aether's requirements.

Potential triggers include:

* agent interaction volume exceeds available backend capacity
* settlement costs make autonomous microtransactions impractical
* Aether requires specialised execution semantics unavailable elsewhere
* private attestations require protocol-native support
* task settlement requires tighter integration with verification systems
* existing finality models are unsuitable for agent workflows
* dependency on external networks creates unacceptable security or governance risks

A native chain must be justified by measured requirements, not by branding or token issuance.

---

# 4. Fault Model

Aether must distinguish between faults at different layers.

| Component          | Initial assumption                                                   |
| ------------------ | -------------------------------------------------------------------- |
| Settlement backend | Security assumptions inherited from the selected backend             |
| Autonomous agents  | May be malicious, buggy, compromised, or economically adversarial    |
| Aether verifiers   | May be unavailable, incorrect, malicious, or colluding               |
| Network            | Delays, message loss, temporary outages, and partitions are possible |
| Storage providers  | May become unavailable or refuse to serve evidence                   |
| Principals         | May misconfigure permissions or delegate excessive authority         |

Aether must not assume that AI systems are aligned, honest, or reliable.

Protocol security should derive from:

* cryptographic identity
* capability constraints
* escrow
* bonds
* verification
* evidence
* dispute mechanisms
* explicit economic incentives

---

# 5. Verifier Fault Tolerance

For workflows using Byzantine fault-tolerant verifier committees, Aether may initially use the conventional threshold model:

```text
n ≥ 3f + 1
```

Where:

* `n` is the total number of verifiers
* `f` is the maximum number of Byzantine verifiers tolerated

Under this model, safety generally requires fewer than one-third of the committee to behave maliciously.

Examples:

| Total verifiers | Maximum Byzantine faults tolerated |
| --------------: | ---------------------------------: |
|               4 |                                  1 |
|               7 |                                  2 |
|              10 |                                  3 |
|              13 |                                  4 |

This model is not automatically appropriate for every Aether workflow.

The protocol must define:

* committee size
* verifier selection method
* verifier eligibility
* whether verifiers post bonds
* how conflicts of interest are handled
* how collusion is detected or discouraged
* what happens when verifiers are unavailable
* whether verification requires a supermajority
* whether different task classes require different thresholds

## Provisional fault-model statement

> Aether treats agents as potentially fully adversarial. For threshold-verification workflows, safety assumptions must explicitly define the maximum tolerated number of Byzantine verifiers. Settlement security is inherited from the selected backend.

---

# 6. Agent Finality

Blockchain finality and agent economic finality are not necessarily the same.

A transaction may be final on a settlement backend while the underlying task remains open to challenge.

Aether should therefore define multiple finality states.

```text
PROPOSED
    ↓
ACCEPTED
    ↓
SETTLEMENT_FINAL
    ↓
DISPUTE_WINDOW_FINAL
    ↓
ECONOMIC_FINAL
```

## 6.1 Proposed

A task, payment, attestation, or settlement request has been submitted but is not yet accepted by the relevant protocol component.

## 6.2 Accepted

The request satisfies initial protocol validation and has entered the relevant workflow.

Acceptance does not imply settlement or economic completion.

## 6.3 Settlement Final

The relevant transaction is final according to the selected settlement backend.

The protocol should expose the backend-specific finality assumptions rather than representing all backends as equally final.

## 6.4 Dispute-Window Final

The applicable dispute period has expired without an accepted challenge.

The task outcome is no longer reversible through the normal Aether dispute process.

## 6.5 Economic Final

Funds, bonds, reputation effects, and other economic consequences have been applied according to protocol rules.

The workflow is complete unless an exceptional recovery mechanism applies.

---

# 7. Machine-Readable Finality

Agents must not infer finality from informal descriptions.

Aether should expose finality through machine-readable protocol states.

Conceptual example:

```json
{
  "workflow_id": "aether:workflow:example",
  "state": "DISPUTE_WINDOW_FINAL",
  "settlement_final": true,
  "challenge_expiry": "2030-01-01T12:00:00Z",
  "economic_final": false,
  "reversal_conditions": []
}
```

The exact schema remains to be designed.

---

# 8. Dispute Windows

Aether should not use one universal dispute period.

Different task types have different verification characteristics.

Initial task classes may include:

| Task class                            | Illustrative dispute period |
| ------------------------------------- | --------------------------: |
| Deterministic machine-verifiable task |          Seconds to minutes |
| Standard API or compute service       |            Minutes to hours |
| Complex AI-generated output           |               Hours to days |
| Human-reviewed or high-value work     |              Days or longer |

These values are examples, not final protocol parameters.

A task contract should specify its dispute configuration before work begins.

Conceptual example:

```text
Task:
GPU computation

Settlement:
Provisional release

Dispute window:
30 minutes

Economic finality:
After the dispute period expires
```

---

# 9. Network Assumptions

Aether should initially assume:

* normal network propagation occurs within a reasonable operational period
* temporary outages are possible
* agents may be offline
* network partitions are possible
* messages may arrive late or out of order
* no participant is guaranteed to be continuously available

Exact latency and availability parameters should be determined through benchmarking rather than assumed during early design.

The protocol should define:

* message expiry
* retry behaviour
* timeout rules
* evidence submission deadlines
* dispute deadlines
* behaviour during backend outages

---

# 10. Dispute Evidence

Large AI outputs, datasets, execution logs, model artefacts, and other evidence should not necessarily be stored directly on-chain.

Aether should use a split evidence model.

```text
ON-CHAIN

- Evidence commitment or hash
- Content identifier
- Submission timestamp
- Submitter identity
- Dispute status
- Verification result
- Retention deadline

OFF-CHAIN

- Full task output
- Execution logs
- Datasets
- Model artefacts
- Supporting evidence
- Private or encrypted materials
```

The on-chain commitment provides integrity.

It can demonstrate that evidence has not been altered after commitment.

However:

> A cryptographic hash proves integrity, not availability.

Aether must separately address whether evidence remains retrievable.

---

# 11. Evidence Availability

Evidence required for a dispute must remain available throughout:

1. the active task period
2. the dispute window
3. the required retention period

Potential availability mechanisms include:

* replicated content-addressed storage
* multiple independent storage providers
* requester and provider retention
* archival services
* enterprise storage systems
* encrypted evidence replication

## Proposed availability rule

> A participant responsible for required evidence must maintain its availability until the applicable retention deadline. Failure to provide required evidence may result in an adverse dispute outcome or economic penalty.

The exact penalty model remains open.

Possible outcomes include:

* loss of the dispute
* partial bond forfeiture
* reduced reputation
* temporary capability restrictions

---

# 12. Multi-Backend Identity

Aether identity should be independent of any single blockchain address.

An agent should not be defined solely by:

> "Its Ethereum address"

or:

> "Its account on a particular settlement network"

Instead:

```text
                 AETHER AGENT ID
                         │
          ┌──────────────┼──────────────┐
          │              │              │
          ▼              ▼              ▼
      Backend A      Backend B      Backend C
```

Settlement accounts should be bound to the Aether identity through authorised capabilities.

Conceptual model:

```text
SettlementBinding {
    backend_id
    account_identifier
    authorised_key
    capability_scope
    expiry
}
```

This enables an agent to:

* settle on multiple networks
* change settlement providers
* use different assets
* preserve one Aether identity
* retain portable reputation

A settlement address is a bound account or capability, not the root identity.

---

# 13. Settlement Backend Interface

Aether should define an abstract settlement interface.

Conceptual operations:

```text
SettlementBackend

- lock_escrow()
- update_escrow()
- submit_attestation()
- open_dispute()
- submit_evidence_commitment()
- resolve_dispute()
- release_funds()
- apply_slashing()
- get_finality()
```

The final interface should be designed only after task and attestation flows are specified.

Backends may differ in:

* transaction cost
* settlement latency
* finality model
* supported assets
* privacy capabilities
* smart-contract functionality
* security assumptions

Aether should expose these differences rather than hiding them behind misleadingly identical guarantees.

---

# 14. Consensus Strategy

## Aether v0.1

Aether should not initially select a native consensus algorithm.

Instead, Aether should:

1. define settlement-independent protocol semantics
2. create a backend adapter interface
3. test multiple settlement environments
4. compare cost, latency, finality, security, and developer experience
5. identify requirements that existing infrastructure cannot satisfy

The protocol should remain compatible with the possibility of a future native chain.

---

# 15. Initial Backend Evaluation Criteria

Potential settlement backends should be evaluated against:

| Criterion            | Question                                                        |
| -------------------- | --------------------------------------------------------------- |
| Cost                 | Can agents settle economically at expected transaction volumes? |
| Latency              | Is settlement fast enough for agent workflows?                  |
| Finality             | How clearly and reliably can finality be represented?           |
| Security             | What assumptions protect settlement?                            |
| Scalability          | Can the backend support large numbers of agents?                |
| Interoperability     | Can agents interact with other ecosystems?                      |
| Programmability      | Can Aether's task and dispute logic be implemented safely?      |
| Privacy              | Can sensitive evidence and commercial activity be protected?    |
| Reliability          | How does the backend behave during congestion or outages?       |
| Developer experience | Can the protocol be built and audited effectively?              |

No backend should be selected solely because it is popular or has a native token ecosystem.

---

# 16. Open Questions

The following questions remain unresolved:

* What settlement backends should be included in the first prototype?
* Should Aether support one backend initially or multiple adapters?
* What verifier committee sizes are appropriate for different task classes?
* How are verifiers selected?
* How are verifier conflicts of interest handled?
* Which task types can use immediate cryptographic verification?
* Which task types require delayed dispute windows?
* What are the minimum and maximum allowed dispute periods?
* Who is responsible for evidence availability?
* How should evidence availability be audited?
* How long should evidence be retained?
* Can private evidence be verified without public disclosure?
* How should Aether represent backend-specific finality?
* How should cross-backend settlement affect reputation?
* Under what measurable conditions would a native Aether chain become justified?

---

# 17. Provisional Conclusion

> **Aether v0.1 is a settlement-agnostic agent trust protocol, not a new Layer 1 blockchain.**
>
> Aether defines agent identity, capabilities, task agreements, attestations, disputes, and reputation while delegating transaction ordering and base settlement security to pluggable settlement backends.
>
> Finality is multi-stage. Settlement finality, dispute-window finality, and economic finality are distinct protocol states.
>
> Agents are treated as potentially adversarial. Verification workflows use explicit fault assumptions, while settlement security is inherited from the selected backend.
>
> Evidence is committed through cryptographic references but stored off-chain with explicit availability obligations and economic consequences for non-availability.
>
> Aether identity is independent of settlement addresses and may bind authorised accounts across multiple settlement backends.
>
> A native Aether chain remains a future possibility only if simulations, prototypes, and real-world measurements demonstrate that existing settlement infrastructure cannot satisfy Aether's requirements.

---

## Related

* [CONSENSUS_DESIGN.md](CONSENSUS_DESIGN.md)
* [AI_AGENT_MODEL.md](AI_AGENT_MODEL.md)
* [IDENTITY_SYSTEM.md](IDENTITY_SYSTEM.md)
* [ECONOMIC_MODEL.md](ECONOMIC_MODEL.md)
* [PAYMENT_LAYER.md](PAYMENT_LAYER.md)
* [PROTOCOL_DESIGN.md](PROTOCOL_DESIGN.md)
* [SECURITY_MODEL.md](SECURITY_MODEL.md)
* [ARCHITECTURE.md](ARCHITECTURE.md)
