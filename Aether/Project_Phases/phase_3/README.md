# Phase 3 — Control Plane Architecture

**Status:** Milestone 1 complete (security approved) — Milestone 2 design preparation

Phase 3 builds the Enterprise Agent Governance Console: the first human-facing interface for observing and governing autonomous AI agents operating through Aether protocol layers.

## Artifacts

| Document | Role |
|----------|------|
| [PHASE_3_CONTEXT.md](PHASE_3_CONTEXT.md) | Purpose, scope, exit criteria |
| [CONTROL_PLANE_ARCHITECTURE.md](CONTROL_PLANE_ARCHITECTURE.md) | Full architecture: user types, dashboard views, data flow, APIs, security |
| [CONTROL_PLANE_THREAT_MODEL.md](CONTROL_PLANE_THREAT_MODEL.md) | Protocol-level threat classification (VALIDATED / OPEN) |
| [CONTROL_PLANE_BUSINESS_CASE.md](CONTROL_PLANE_BUSINESS_CASE.md) | Enterprise, research, and developer value |
| [PHASE_3_WEDGE_DECISION.md](PHASE_3_WEDGE_DECISION.md) | Product wedge selection and MVP scope |
| [CONTROL_PLANE_MVP_ARCHITECTURE.md](CONTROL_PLANE_MVP_ARCHITECTURE.md) | Frontend, backend, protocol integration, storage, auth, deployment |
| [CONTROL_PLANE_SECURITY_MODEL.md](CONTROL_PLANE_SECURITY_MODEL.md) | Application-layer threat classification and milestone security gates |
| [CONTROL_PLANE_MILESTONE_1_RESULTS.md](CONTROL_PLANE_MILESTONE_1_RESULTS.md) | Milestone 1 implementation results and security checklist |
| [CONTROL_PLANE_MILESTONE_2_PLAN.md](CONTROL_PLANE_MILESTONE_2_PLAN.md) | Milestone 2 Policy Management plan (design gate) |

## Key Question

> How do humans observe and govern autonomous AI agents without becoming a protocol authority?

## Authority Invariant

The Control Plane's **only write path** to the protocol stack is through PROTO-0 capability issuance. It cannot modify escrow, settlement, channel, or reputation stores. Loss of the Control Plane does not affect protocol operation.

## Progress

| Item | Status |
|------|--------|
| Phase 3 context | Complete |
| Control Plane architecture | Complete |
| Threat model (protocol-level) | Complete |
| Business case | Complete |
| Product wedge decision | Complete — Enterprise Governance Console |
| MVP architecture | Complete — frozen |
| Security model | Complete — milestone gates defined |
| Milestone 1 — Read-Only Observatory | **Complete** — 18 tests pass |
| Milestone 1 security review | **Approved** |
| Milestone 2 design (threat / authority / write security) | **Approved** — sequence refined (Phases 1–5) |
| Milestone 2 — Policy Management implementation | **Not started** — begins at Phase 1 only |

## Milestones

**Milestone 1 — Read-Only Observatory** — Complete

**Milestone 2 — Policy Management** — Design prepared; implementation gated

Design docs (Aether_docs):

- [CONTROL_PLANE_POLICY_THREAT_MODEL.md](../../Aether_docs/CONTROL_PLANE_POLICY_THREAT_MODEL.md)
- [CONTROL_PLANE_AUTHORITY_MODEL.md](../../Aether_docs/CONTROL_PLANE_AUTHORITY_MODEL.md)
- [CONTROL_PLANE_WRITE_SECURITY.md](../../Aether_docs/CONTROL_PLANE_WRITE_SECURITY.md)
- [CONTROL_PLANE_SECURITY_MODEL.md](../../Aether_docs/CONTROL_PLANE_SECURITY_MODEL.md) — signing roadmap index

**Implementation sequence (frozen):** Phase 1 Secure Write Foundation → Phase 2 Policy Templates → Phase 3 Signer Abstraction → Phase 4 PROTO-0 Integration → Phase 5 Security Validation.

## Related

- [Phase 2 README](../phase_2/README.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
