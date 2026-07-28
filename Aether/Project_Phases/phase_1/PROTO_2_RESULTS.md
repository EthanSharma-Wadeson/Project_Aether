# PROTO-2 Results — Phase 1

## Status

**PROTO-2 implementation complete (2026-07-28)**

Command:

```bash
cd Aether/core && cargo fmt --check && cargo test && cargo clippy --all-targets -- -D warnings
```

Date: 2026-07-28

Do **not** begin PROTO-3 automatically from this result set.

---

## Implementation Summary

### Modules created

`Aether/core/src/escrow/`:

| Module | Role |
|--------|------|
| `terms.rs` | `EscrowTermsV0` — dual-signed bilateral agreement |
| `state.rs` | `EscrowV0`, `EscrowStatus`, `EconomicFinalityViewV0` |
| `model.rs` | Funding/release/refund/cancel intent messages, action tokens |
| `receipt.rs` | `SettlementReceiptV0`, `SettlementEvidenceV0` (distinct from PROTO-1 `ReceiptV0`) |
| `fee.rs` | `FeeQuoteV0`, `FeeLedgerEntryV0`, `BalanceLedger` |
| `store.rs` | `EscrowStore` — crate-internal mutation only (`insert_record`) |
| `verify.rs` | Dual-signed terms, receipt verification, capability gates |
| `transition.rs` | Create, fund, submit receipt, release, refund, cancel, expire |
| `dispute.rs` | Trusted-local raise/resolve with deterministic rule |

### State transitions

```text
Proposed → Funded → ReceiptAccepted → Released
Proposed → Cancelled
Funded → Refunded (cooperative, payer)
Funded → Expired → Refunded (timeout)
Funded / ReceiptAccepted → Disputed → ResolvedReleased | ResolvedRefunded
```

### Receipt pipeline

- Provider-only `SettlementReceiptV0` signatures (`escrow.submit_receipt`)
- `receipt_id = SHA-256(canonical body without receipt_id)`
- Binding to `escrow_id`, `terms_version`, monotonic `receipt_nonce`
- Signature proves bounded claim only — not output correctness

### Authorisation integration

Every economically meaningful operation uses PROTO-0 `authorise_action` before escrow semantics:

```text
Verify SignedMessage → identity status → capability → escrow rules → mutate
```

Action tokens: `escrow.create`, `escrow.fund`, `escrow.submit_receipt`, `escrow.release`, `escrow.refund`, `escrow.dispute`, `escrow.resolve`, `escrow.cancel`.

### Fee accounting

- `FeeQuoteV0` at create (fund + release quotes)
- Fee reserved on fund; consumed on release/resolve-release
- Fee refunded on refund paths
- `FeeLedgerEntryV0` audit per consumption
- Checked arithmetic; overflow fails closed

### Economic finality

`EconomicFinalityViewV0.finalized = true` on terminal statuses. `hard_settlement_placeholder` always `false`. Not blockchain or legal finality.

### PROTO-1 isolation

Separate `EscrowStore`; no mutation of `ChannelStore`. PROTO-1 `ReceiptV0` rejected as settlement evidence (P2-I11).

---

## Test Results

| Suite | Passed | Failed |
|-------|--------|--------|
| Unit (`agent_id`) | 2 | 0 |
| `identity_tests` (PROTO-0) | 16 | 0 |
| `capability_tests` (PROTO-0) | 20 | 0 |
| `adversarial_tests` (PROTO-0) | 12 | 0 |
| `channel_tests` (PROTO-1) | 27 | 0 |
| `channel_adversarial_tests` (PROTO-1) | 24 | 0 |
| `escrow_tests` (P2-T) | 20 | 0 |
| `escrow_receipt_tests` (P2-R) | 14 | 0 |
| `escrow_economic_tests` (P2-E) | 12 | 0 |
| `escrow_adversarial_tests` (P2-A) | 16 | 0 |
| `escrow_integration_tests` (P2-I/H/M) | 20 | 0 |
| **Total** | **183** | **0** |

Clippy: clean (`--all-targets -- -D warnings`)

### Acceptance IDs covered

All frozen PROTO-2 IDs: P2-T001–T020, P2-R001–R014, P2-E001–E012, P2-A01–A16, P2-I01–I12, P2-H001–H006, P2-M001–M002.

---

## Security Properties Evidenced

### Locally evidenced

- Dual-signed terms required for escrow creation
- Provider-only settlement receipt signatures independently verified
- Capability gating on every economic operation (no identity-only authority)
- Receipt replay rejected via monotonic nonce
- Terminal outcomes mutually exclusive
- Value conservation with checked arithmetic
- Dispute window blocks premature release
- PROTO-1 channel state unchanged by escrow operations
- Deterministic dispute resolution rule (valid receipt → release; else refund)

### Design assumptions

- Logical time (`now`) is harness-injected and trusted
- Simulated balance ledger represents abstract units only
- Fee quotes are deterministic functions of terms at create time

### Trusted-local assumptions

- Dispute resolver is a local authorised participant with `escrow.resolve` — not distributed arbitration
- No network adversary in test harness
- No distributed revocation propagation

### Out of scope

- Real money, cryptocurrency, tokens, blockchain settlement
- Production economic security
- Receipt signatures proving output correctness or truthfulness
- Legal finality

---

## H4 Results

**What the prototype demonstrates:**

- Bounded hire→work→settle loops complete via fund → receipt → release
- Timeout paths refund safely without receipt
- Invalid/forged receipts rejected; adversarial wrongful-release count = 0 in suite
- Dispute paths resolve deterministically

**What it does not prove:**

- Real-market escrow sufficiency for agent task markets
- Production safety against colluding authorised parties creating misleading bounded claims
- Sufficiency without non-deterministic proofs in unbounded real-world tasks

---

## H5 Results

**What the prototype demonstrates:**

- Fee quotes bound to escrow terms at create
- Funding reserves fee within `max_protocol_fee`
- Terminal operations consume fees within reserved budget
- Insufficient budget blocks release (P2-E010)
- Fee ledger matches consumed totals (P2-M002)
- Agent loop completes without manual budget intervention in harness (P2-H005)

**What it does not prove:**

- Real fee markets or token economics
- Autonomous fee planning in production environments

---

## Remaining Risks

- Logical-time trust in harness
- Local-only resolver (not decentralised arbitration)
- No distributed revocation
- No network adversary testing
- No real settlement backend
- No real-money custody
- No blockchain finality
- Receipt signatures do not prove output correctness
- Colluding authorised parties can create bounded but misleading claims
- No production economic security

---

## Assumptions documented

- `Expired` is a transitional status (not terminal) to allow payer refund step (P2-T006)
- Fee split: fund quote = `max_protocol_fee / 2`, release quote = remainder
- Cooperative refund limited to payer before receipt acceptance
