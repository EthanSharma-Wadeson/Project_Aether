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
| 1 | Active | [Aether_docs/roadmap/phase_1.md](../Aether_docs/roadmap/phase_1.md) |
| 2 | Pending | [Aether_docs/roadmap/phase_2.md](../Aether_docs/roadmap/phase_2.md) |
| 3 | Pending | [Aether_docs/roadmap/phase_3.md](../Aether_docs/roadmap/phase_3.md) |

**Current focus:** PROTO-0 complete; security model aligned. PROTO-1 not started — awaiting explicit approval.

| Resource | Role |
|----------|------|
| [phase_0/DECISIONS.md](phase_0/DECISIONS.md) | Decision log |
| [phase_0/V0_WIRE_CRYPTO_GROUP.md](phase_0/V0_WIRE_CRYPTO_GROUP.md) | DEC-003–008 coherent spec |
| [phase_0/PERMISSION_ROOT_V0.md](phase_0/PERMISSION_ROOT_V0.md) | Minimal provisional permission-root design |
| [phase_0/PHASE_1_KICKOFF.md](phase_0/PHASE_1_KICKOFF.md) | Ordered Phase 1 execution plan |
| [phase_1/PROTO_0_ACCEPTANCE_TESTS.md](phase_1/PROTO_0_ACCEPTANCE_TESTS.md) | Frozen PROTO-0 acceptance tests |
| [phase_1/PROTO_0_RESULTS.md](phase_1/PROTO_0_RESULTS.md) | PROTO-0 test results |
| [experiments/decision/RESULTS.md](../experiments/decision/RESULTS.md) | Experiment evidence |
| [../schemas/v0/fixtures/manifest.json](../schemas/v0/fixtures/manifest.json) | Published signing fixtures |
| [../experiments/decision/python_interop/verify_fixtures.py](../experiments/decision/python_interop/verify_fixtures.py) | Independent Python verifier |
