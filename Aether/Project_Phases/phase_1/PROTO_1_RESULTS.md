# PROTO-1 Results — Phase 1

## Status

**PROTO-1 implementation complete — security remediation applied (2026-07-28)**

Command:

```bash
cd Aether/core && cargo test && cargo clippy --all-targets -- -D warnings
```

Date: 2026-07-28

Do **not** begin PROTO-2 automatically from this result set.

### Security remediation (post-review)

| Change | Purpose |
|--------|---------|
| `channel/chain.rs` | Commitment-chain validation for dispute evidence |
| `raise_dispute` | Requires direct chain extension (`seq = applied + 1`) |
| `resolve_dispute` | Highest **chained** dual-signed state; requires `channel.dispute` |
| `finalize_close` | Requires both `channel.close` grants + elapsed dispute window |
| `ChannelStore` | Removed public `get_mut`; `insert_record` is crate-internal only |
| Regression tests `P1-R01`–`P1-R05` | Orphan dispute, skipped raise, terminal auth, dispute window |
| `SECURITY_MODEL.md` | PROTO-1 evidenced scope + terminal trust boundaries |

---

## Implementation Summary

### What was built

A library-first bilateral channel simulator in `aether-core` proving one primitive:

> Two authorised agents can maintain a shared, ordered, cryptographically agreed state history.

Modules under `Aether/core/src/channel/`:

| Module | Role |
|--------|------|
| `model.rs` | `ChannelV0`, open material, `StateUpdateV0`, `DualSignedUpdate`, `ReceiptV0`, status / action tokens |
| `state.rs` | `ChannelStateV0` + commitment (`SHA-256` of canonical CBOR) |
| `verify.rs` | Dual-signature verification + PROTO-0 capability gate |
| `transition.rs` | Open → Activate → Update → Close / Abort state machine |
| `dispute.rs` | Raise / resolve dispute; highest valid dual-signed state wins |

### Architecture choices

- Reuses PROTO-0 `AgentIdentity`, capability grants, `authorise_action`, DEC-004B signing, and schema-locked CBOR.
- Dual signatures over **identical** body bytes required before any accepted transition.
- Validation order: signatures → membership/status → capability → sequence / conservation / transition rules → apply.
- Simulated balances only (conservation of total deposit); not a payment or settlement system.
- Soft local agreement (`soft_local_agreement`) kept distinct from `hard_settlement_placeholder` (always false in PROTO-1).

### How PROTO-0 integrates

Every channel action (`channel.open` / `activate` / `update` / `close` / `dispute`) requires:

1. Registered, Active identities for both parties (where applicable)
2. Valid `CapabilityGrant` covering the action token
3. Fail-closed behaviour on frozen/revoked identity, expired/revoked capability, or stale permission root

---

## Test Results

| Suite | Passed | Failed |
|-------|--------|--------|
| Unit (`agent_id`) | 2 | 0 |
| `identity_tests` | 16 | 0 |
| `capability_tests` | 20 | 0 |
| `adversarial_tests` (PROTO-0) | 12 | 0 |
| `channel_tests` | 27 | 0 |
| `channel_adversarial_tests` | 24 | 0 |
| **Total** | **101** | **0** |

Clippy: clean (`--all-targets -- -D warnings`)

### Acceptance IDs covered

**Lifecycle:** `P1-T001`–`P1-T008`, `P1-T010`–`P1-T014`, `P1-T020`–`P1-T022`

**Adversarial:** `P1-A01`–`P1-A16`, `P1-R01`–`P1-R05` (+ extras)

**PROTO-0 integration:** `P1-I01`–`P1-I10`

**Metrics hooks:** `P1-M01`–`P1-M03`

Sample local metrics (not chain evidence):

```text
P1-M01 simulated_cost=10 direct_baseline=1000
P1-M02 elapsed_ns≈2.2e7 (host-dependent)
P1-M03 soft_local_agreement=true, hard_settlement_placeholder=false
```

PROTO-1 does **not** prove real-chain fee superiority.

---

## Security Properties Demonstrated

| Property | Evidence |
|----------|----------|
| Shared state agreement | Dual-signed open/activate/update; both keys required |
| Replay resistance | Stale / skipped sequence and previous-commitment checks |
| Signature enforcement | Forged, unilateral, mismatched-body, wrong-signer updates rejected |
| Capability before channel auth | Valid channel sigs without matching capability still rejected |
| Deterministic closing | Cooperative close → Closing → Finalized; abort from Open |
| Dispute resolution | Highest valid **chained** mutually signed state wins; terminal ops auth-gated |

---

## Known Limitations

PROTO-1 does **not** prove:

- Real network security (no sockets, no adversaries on the wire)
- Blockchain settlement or hard finality
- Economic incentives, fees, tokens, wallets, or escrow
- Production payment guarantees
- Distributed dispute adjudication (local in-memory store only)
- Operational key rotation mid-channel
- Clock / logical-time integrity against a compromised harness

Remaining risks carried from Phase 0 / PROTO-0 (local revocation only, synthetic H1 baseline, invariant 6 only partially evidenced) still apply.

---

## Next Step Recommendation

**Do not start PROTO-2 yet.**

Recommended next Phase 1 step:

1. Short security review of PROTO-1 against `SECURITY_MODEL.md` and Phase 0 invariants (especially dispute reproducibility and soft-vs-hard finality language).
2. Only after that review is accepted, begin PROTO-2 design freeze (escrow / settlement simulation scope) using the same decision → acceptance-tests → implement gate used for PROTO-0/1.

PROTO-2 should remain blocked until an explicit approval to design or implement.
