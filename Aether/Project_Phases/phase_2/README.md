# Phase 2 — Mainframe Architecture & Protocol Expansion

**Status:** Active — PROTO-4 security **APPROVE WITH DOCUMENTED LIMITATIONS** (post-remediation)

Phase 1 (PROTO-0, PROTO-1, PROTO-2) is complete. Phase 2 defines the protocol operating system and settlement binding for the Enterprise Spend Control wedge.

## Artifacts

| Document | Role |
|----------|------|
| [PHASE_2_CONTEXT.md](PHASE_2_CONTEXT.md) | Purpose, scope, exit criteria |
| [MAINFRAME_ARCHITECTURE.md](MAINFRAME_ARCHITECTURE.md) | Full stack map, missing primitives, dependency graph, roadmap |
| [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) | DEC-P2-### decision records |
| [MAINFRAME_THREAT_MODEL.md](MAINFRAME_THREAT_MODEL.md) | Distributed threat model (extends SECURITY_MODEL.md) |
| [BUSINESS_ALIGNMENT.md](BUSINESS_ALIGNMENT.md) | User-segment value mapping |
| [PROTO_NET_0_RESULTS.md](PROTO_NET_0_RESULTS.md) | PROTO-NET-0 transport results |
| [PROTO_NET_0_SECURITY_REVIEW.md](PROTO_NET_0_SECURITY_REVIEW.md) | PROTO-NET-0 vs SECURITY_MODEL.md |
| [PHASE_2_WEDGE_DECISION.md](PHASE_2_WEDGE_DECISION.md) | First real-world wedge (Enterprise Spend Control → PROTO-4) |
| [PROTO_4_DESIGN.md](PROTO_4_DESIGN.md) | Settlement binding design |
| [PROTO_4_DECISIONS.md](PROTO_4_DECISIONS.md) | PROTO-4 design decisions |
| [PROTO_4_ACCEPTANCE_TESTS.md](PROTO_4_ACCEPTANCE_TESTS.md) | Frozen PROTO-4 acceptance tests |
| [PROTO_4_RESULTS.md](PROTO_4_RESULTS.md) | PROTO-4 implementation + remediation results |
| [PROTO_4_SECURITY_REVIEW.md](PROTO_4_SECURITY_REVIEW.md) | Post-remediation review — **APPROVE WITH DOCUMENTED LIMITATIONS** |
| [ENTERPRISE_DEMO.md](ENTERPRISE_DEMO.md) | Enterprise Spend Control demonstrator — purpose, architecture, scenarios |
| [ENTERPRISE_DEMO_RESULTS.md](ENTERPRISE_DEMO_RESULTS.md) | Demonstrator run results (4/4 scenarios) |

## Key Question

> What infrastructure must exist for autonomous agents to safely coordinate economically at scale?

## Progress

| Item | Status |
|------|--------|
| Architecture gate | Complete |
| PROTO-NET-0 | Complete + security APPROVE |
| Wedge | Enterprise Agent Spend Control |
| PROTO-4 design | Complete |
| PROTO-4 acceptance tests | Frozen |
| PROTO-4 implementation | Complete |
| P4-SEC-001 / P4-SEC-002 remediation | Complete (P4-SEC-R01–R03) |
| **PROTO-4 security review (post-remediation)** | **APPROVE WITH DOCUMENTED LIMITATIONS** |
| Enterprise Spend Control demonstrator | Complete (`Aether/demo/`, 4/4 scenarios) |

**STOP:** Await explicit acceptance of the enterprise spend-control wedge. No reputation, marketplace, second settlement backend, or production integrations until authorised.

Run the demonstrator:

```bash
cd Aether/demo && cargo run
```

## Constraints

No live banking, blockchain, or token work in the PROTO-4 spike scope.

## Related

- [Phase 1 results](../phase_1/PROTO_2_RESULTS.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
- [roadmap/phase_2.md](../../Aether_docs/roadmap/phase_2.md)
