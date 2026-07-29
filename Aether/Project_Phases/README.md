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
| **2** | Active | [phase_2/README.md](phase_2/README.md) — Mainframe architecture (design gate) |
| 3 | Pending | [Aether_docs/roadmap/phase_3.md](../Aether_docs/roadmap/phase_3.md) |

**Current focus:** Phase 2 — PROTO-4 implemented; PROTO-3 reputation **implemented** (34 tests pass).

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
