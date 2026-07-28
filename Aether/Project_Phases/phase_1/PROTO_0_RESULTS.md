# PROTO-0 Results — Phase 1

## Status

**PROTO-0 implementation complete — awaiting review before PROTO-1**

Command:

```bash
cd Aether/core && cargo test && cargo clippy --all-targets -- -D warnings
```

Date: 2026-07-28

---

## Test Results

| Suite | Passed | Failed |
|-------|--------|--------|
| Unit (`agent_id`) | 2 | 0 |
| `identity_tests` | 16 | 0 |
| `capability_tests` | 20 | 0 |
| `adversarial_tests` | 12 | 0 |
| **Total** | **50** | **0** |

Clippy: clean (`-D warnings`)

---

## Acceptance IDs Covered

### Identity / root

`P0-T001`, `P0-T002`, `P0-T003`, `P0-T010`, `P0-T011`, `P0-T012`, `P0-T013`, `P0-T014`, `P0-T020`, `P0-T021`, `P0-T022`, `P0-T030`, `P0-T031`, `P0-T033`, `P0-T034`

`P0-T032` (stale root version) covered by adversarial `P0-A06`.

### Capability / delegation / revocation

`P0-T040`–`P0-T043`, `P0-T050`–`P0-T055`, `P0-T060`, `P0-T070`–`P0-T073`, `P0-T076`, `P0-T080`–`P0-T083`

### Adversarial

`P0-A01`–`P0-A12` (all reject / fail closed)

---

## Dependencies Added (policy-aligned)

| Crate | Reason | Policy basis |
|-------|--------|--------------|
| `ed25519-dalek` 2.x + `rand_core` | Ed25519 sign/verify | DEC-004A / constraints §5 |
| `rand` 0.8 | Key generation for tests/runtime | required by dalek keygen |
| `sha2` 0.10 | SHA-256 digests / IDs / roots | DEC-004B / DEC-005 |
| `ciborium` 0.2 | Schema-locked CBOR encode/decode | DEC-003 |
| `hex` 0.4 | AgentId hex encoding + test diagnostics | fixture/test utility |

No architectural dependency changes beyond Phase 0 provisional decisions.

---

## Review Gate

Security findings reviewed; `SECURITY_MODEL.md` aligned (see [PROTO_0_SECURITY_REVIEW.md](PROTO_0_SECURITY_REVIEW.md)).

PROTO-1 remains **blocked until explicitly approved**. Do not start PROTO-1 automatically from this result set.
