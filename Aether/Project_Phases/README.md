# Project Phases — Aether

Operational phase context for moving from design docs to implementation.

## Naming Convention

| Kind | IDs | Example |
|------|-----|---------|
| **Project phases** | Phase 0–3 | Phase 0 exit criteria |
| **Prototypes** | PROTO-0–PROTO-5 | PROTO-0 adversarial tests |

Do not use `P0`–`P5` for prototypes — they collide with phase numbers.

## Phases

| Phase | Status | Context |
|-------|--------|---------|
| **0** | Closed | [phase_0/CONTEXT.md](phase_0/CONTEXT.md) — Pre-implementation readiness |
| **1** | Closed | [phase_1/README.md](phase_1/README.md) — PROTO-0/1/2 primitives |
| **2** | Complete | [phase_2/README.md](phase_2/README.md) — Mainframe architecture + protocol implementation |
| **3** | Complete (Control Plane + Governance Console) | [phase_3/README.md](phase_3/README.md) — Control Plane; Apply disabled |
| **15** | **Design frozen** | Agent Treasury & Financial Architecture — [design pack](../Aether_docs/TREASURY_ARCHITECTURE.md) |
| **16** | **Review complete** | [TREASURY_ARCHITECTURE_REVIEW.md](../Aether_docs/TREASURY_ARCHITECTURE_REVIEW.md) — **PASS WITH CONDITIONS** |
| **17** | **Foundation implemented** | [`treasury/`](../treasury/) — internal journal/allocations/reservations; **no HTTP/rails/Apply** |
| **18** | **Design frozen** | [TREASURY_CONTROL_PLANE_ARCHITECTURE.md](../Aether_docs/TREASURY_CONTROL_PLANE_ARCHITECTURE.md) — **PASS WITH CONDITIONS** |
| **19** | **Read observatory implemented** | [TREASURY_READ_OBSERVATORY_IMPLEMENTATION.md](../Aether_docs/TREASURY_READ_OBSERVATORY_IMPLEMENTATION.md) — CP + Console read-only; **no mutations** |

**Current focus:** Phase 19 complete — treasury observation via Control Plane and Console. Do **not** add funding/mutation APIs, custody, ERP, or Apply sync until separately approved. Apply remains disabled; no protocol changes.

| Treasury design pack | Role |
|----------------------|------|
| [../Aether_docs/TREASURY_ARCHITECTURE.md](../Aether_docs/TREASURY_ARCHITECTURE.md) | Hierarchy, assets, funding, spending authority |
| [../Aether_docs/TREASURY_ACCOUNTING_MODEL.md](../Aether_docs/TREASURY_ACCOUNTING_MODEL.md) | Immutable journal + attribution |
| [../Aether_docs/TREASURY_SECURITY_MODEL.md](../Aether_docs/TREASURY_SECURITY_MODEL.md) | Controls + regulatory extension points |
| [../Aether_docs/TREASURY_THREAT_MODEL.md](../Aether_docs/TREASURY_THREAT_MODEL.md) | TR-THR catalogue |
| [../Aether_docs/TREASURY_BUSINESS_CASE.md](../Aether_docs/TREASURY_BUSINESS_CASE.md) | Enterprise spend-control business case |
| [../Aether_docs/TREASURY_OPEN_QUESTIONS.md](../Aether_docs/TREASURY_OPEN_QUESTIONS.md) | Open questions + Phase 16 resolutions |
| [../Aether_docs/TREASURY_ARCHITECTURE_REVIEW.md](../Aether_docs/TREASURY_ARCHITECTURE_REVIEW.md) | Phase 16 review + gate decision |
| [../Aether_docs/TREASURY_CONTROL_PLANE_ARCHITECTURE.md](../Aether_docs/TREASURY_CONTROL_PLANE_ARCHITECTURE.md) | Phase 18 CP ↔ Treasury integration freeze |
| [../Aether_docs/TREASURY_READ_OBSERVATORY_IMPLEMENTATION.md](../Aether_docs/TREASURY_READ_OBSERVATORY_IMPLEMENTATION.md) | Phase 19 read-only implementation |

| Resource | Role |
|----------|------|
| [phase_2/PROTO_3_ACCEPTANCE_TESTS.md](phase_2/PROTO_3_ACCEPTANCE_TESTS.md) | Frozen PROTO-3 acceptance tests |
| [phase_2/PROTO_3_DESIGN.md](phase_2/PROTO_3_DESIGN.md) | Evidence-based reputation design |
| [phase_2/PROTO_3_DECISIONS.md](phase_2/PROTO_3_DECISIONS.md) | PROTO-3 decisions (P3-DEC-###) |
| [phase_2/PROTO_3_SECURITY_REVIEW.md](phase_2/PROTO_3_SECURITY_REVIEW.md) | PROTO-3 security review (APPROVE) |
| [phase_2/PROTO_4_ACCEPTANCE_TESTS.md](phase_2/PROTO_4_ACCEPTANCE_TESTS.md) | Frozen PROTO-4 acceptance tests |
| [phase_2/PROTO_4_DESIGN.md](phase_2/PROTO_4_DESIGN.md) | Settlement binding design |
| [phase_2/PROTO_4_DECISIONS.md](phase_2/PROTO_4_DECISIONS.md) | PROTO-4 decisions |
| [phase_2/PROTO_NET_0_RESULTS.md](phase_2/PROTO_NET_0_RESULTS.md) | PROTO-NET-0 transport foundation results |
| [phase_2/PHASE_2_WEDGE_DECISION.md](phase_2/PHASE_2_WEDGE_DECISION.md) | Enterprise spend-control wedge |
| [phase_2/MAINFRAME_ARCHITECTURE.md](phase_2/MAINFRAME_ARCHITECTURE.md) | Protocol stack map + missing primitives |
| [phase_0/DECISIONS.md](phase_0/DECISIONS.md) | Decision log |
| [phase_0/V0_WIRE_CRYPTO_GROUP.md](phase_0/V0_WIRE_CRYPTO_GROUP.md) | DEC-003–008 coherent spec |
| [phase_0/PERMISSION_ROOT_V0.md](phase_0/PERMISSION_ROOT_V0.md) | Minimal provisional permission-root design |
| [phase_0/PHASE_1_KICKOFF.md](phase_0/PHASE_1_KICKOFF.md) | Ordered Phase 1 execution plan |
| [phase_1/PROTO_0_ACCEPTANCE_TESTS.md](phase_1/PROTO_0_ACCEPTANCE_TESTS.md) | Frozen PROTO-0 acceptance tests |
| [phase_1/PROTO_0_RESULTS.md](phase_1/PROTO_0_RESULTS.md) | PROTO-0 test results |
| [experiments/decision/RESULTS.md](../experiments/decision/RESULTS.md) | Experiment evidence |
| [../schemas/v0/fixtures/manifest.json](../schemas/v0/fixtures/manifest.json) | Published signing fixtures |
| [../experiments/decision/python_interop/verify_fixtures.py](../experiments/decision/python_interop/verify_fixtures.py) | Independent Python verifier |
| [phase_3/PHASE_3_CONTEXT.md](phase_3/PHASE_3_CONTEXT.md) | Phase 3 purpose and scope |
| [phase_3/CONTROL_PLANE_ARCHITECTURE.md](phase_3/CONTROL_PLANE_ARCHITECTURE.md) | Control Plane full architecture |
| [phase_3/CONTROL_PLANE_THREAT_MODEL.md](phase_3/CONTROL_PLANE_THREAT_MODEL.md) | Control Plane threat classification |
| [phase_3/CONTROL_PLANE_MILESTONE_1_RESULTS.md](phase_3/CONTROL_PLANE_MILESTONE_1_RESULTS.md) | Milestone 1 implementation results |
| [phase_3/CONTROL_PLANE_MILESTONE_2_PLAN.md](phase_3/CONTROL_PLANE_MILESTONE_2_PLAN.md) | Milestone 2 Policy Management design gate |
| [../Aether_docs/CONTROL_PLANE_POLICY_THREAT_MODEL.md](../Aether_docs/CONTROL_PLANE_POLICY_THREAT_MODEL.md) | M2 policy threat model |
| [../Aether_docs/CONTROL_PLANE_AUTHORITY_MODEL.md](../Aether_docs/CONTROL_PLANE_AUTHORITY_MODEL.md) | Observer vs authorised mutation |
| [../Aether_docs/CONTROL_PLANE_WRITE_SECURITY.md](../Aether_docs/CONTROL_PLANE_WRITE_SECURITY.md) | CSRF and write API boundary |
