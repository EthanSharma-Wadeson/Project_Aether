# Phase 2 — Mainframe Architecture & Protocol Expansion

**Status:** Active — PROTO-3 **implemented** (34 tests pass); PROTO-4 security **APPROVE WITH DOCUMENTED LIMITATIONS** (post-remediation)

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
| [AETH_ECONOMIC_MODEL.md](AETH_ECONOMIC_MODEL.md) | AETH role, classification, decision gates (design only) |
| [AETH_TOKEN_ARCHITECTURE.md](AETH_TOKEN_ARCHITECTURE.md) | AETH as SettlementAdapter integration (no chain/contracts) |
| [MULTI_ASSET_SETTLEMENT.md](MULTI_ASSET_SETTLEMENT.md) | Multi-asset adapter architecture (GBP, USD, USDC, AETH, …) |
| [AETH_THREAT_MODEL.md](AETH_THREAT_MODEL.md) | Optional native asset risks and trust model |
| [AETH_BUSINESS_CASE.md](AETH_BUSINESS_CASE.md) | Enterprise vs open economy; optional adoption default |
| [PROTO_3_DESIGN.md](PROTO_3_DESIGN.md) | Evidence-based reputation layer (design only) |
| [PROTO_3_DECISIONS.md](PROTO_3_DECISIONS.md) | PROTO-3 design decisions (P3-DEC-###) |
| [PROTO_3_ACCEPTANCE_TESTS.md](PROTO_3_ACCEPTANCE_TESTS.md) | Frozen PROTO-3 acceptance tests |
| [PROTO_3_THREAT_MODEL.md](PROTO_3_THREAT_MODEL.md) | Reputation threat model (VALIDATED vs OPEN) |
| [PROTO_3_RESEARCH.md](PROTO_3_RESEARCH.md) | Research background and open questions |
| [PROTO_3_SECURITY_REVIEW.md](PROTO_3_SECURITY_REVIEW.md) | PROTO-3 security review — **APPROVE WITH DOCUMENTED LIMITATIONS** |

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
| AETH economic architecture (design only) | Complete — optional asset; no implementation |
| PROTO-3 reputation architecture (design only) | Complete — acceptance tests frozen |
| **PROTO-3 security review** | **APPROVE WITH DOCUMENTED LIMITATIONS** (P3-SEC-001–003) |
| **PROTO-3 implementation** | **Complete** — 34 acceptance tests pass; `cargo fmt` + `cargo clippy -D warnings` clean |

**STOP:** Design freeze only. No PROTO-3 implementation, AETH implementation, marketplace, dashboard, governance, or production integrations until explicitly authorised.

Run the demonstrator:

```bash
cd Aether/demo && cargo run
```

## Constraints

No live banking, blockchain, or token work in the PROTO-4 spike scope. AETH documents are **architecture only** — no Rust, no contracts, no supply schedule.

## AETH design summary

> Does AETH solve a genuine protocol problem, or is it merely a cryptocurrency attached to Aether?

**Answer:** Aether functions entirely without AETH. For enterprise customers, existing fiat/ledger rails are sufficient (proven by the demonstrator). AETH may optionally matter later for open agent-economy portability — but stablecoins may suffice; evidence is required before any implementation. **AETH is never mandatory.**

See [AETH_ECONOMIC_MODEL.md](AETH_ECONOMIC_MODEL.md) and [AETH_BUSINESS_CASE.md](AETH_BUSINESS_CASE.md).

## PROTO-3 design summary

> How can autonomous agents evaluate counterparty trust using only protocol evidence?

**Answer:** PROTO-3 is a **read-only reputation indexer** that derives `ReputationEventV0` from signed PROTO-1/2/4 artifacts and exposes **deterministic metrics** — not social ratings or mandatory scores. **Authority remains PROTO-0.** Reputation is **optional** for enterprise deployments.

See [PROTO_3_DESIGN.md](PROTO_3_DESIGN.md) and [PROTO_3_ACCEPTANCE_TESTS.md](PROTO_3_ACCEPTANCE_TESTS.md).

## Related

- [Phase 1 results](../phase_1/PROTO_2_RESULTS.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
- [roadmap/phase_2.md](../../Aether_docs/roadmap/phase_2.md)
