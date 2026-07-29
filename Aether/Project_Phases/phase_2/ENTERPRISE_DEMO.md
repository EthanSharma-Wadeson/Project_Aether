# Enterprise Agent Spend Control Demonstrator

## Purpose

This is the **first end-to-end proof-of-value** for Aether. It shows that the existing protocol stack (PROTO-0 identity and capability, PROTO-2 escrow economics, PROTO-4 settlement binding) can enforce an enterprise workflow where an autonomous agent operates under delegated spend authority without unrestricted access to company funds.

This is **not** a production application, marketplace, reputation system, or live payment integration.

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│  Aether/demo/  (demonstration layer only)                   │
│    enterprise_demo/                                         │
│      harness.rs   — identity + capability + account setup   │
│      scenario.rs  — four scenario definitions               │
│      runner.rs    — lifecycle orchestration                 │
│      reporting.rs — protocol-attributed summary             │
│      cli.rs       — `enterprise-demo` entry point           │
└──────────────────────────┬──────────────────────────────────┘
                           │ consumes (read-only)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  aether-core                                                │
│    PROTO-0  identity registry + capability grants           │
│    PROTO-2  escrow create / fund / receipt / release        │
│    PROTO-4  account bind / settle / adapter / finalize      │
└─────────────────────────────────────────────────────────────┘
```

### Roles

| Role | Protocol identity | Responsibility |
|------|-------------------|----------------|
| Enterprise | Payer / treasury | Registers, funds escrow, releases after receipt, initiates settlement |
| Agent | Delegated principal | Holds bounded `settlement.settle` capability (demonstrated in scenario 2) |
| Provider | Service vendor | Submits work receipt |
| Mock ledger | `enterprise.ledger.v0` | External evidence source only (PROTO-4) |

### Demonstration flow

```text
Enterprise
      │
      ▼
Register identities (PROTO-0)
      │
      ▼
Grant spend capability to agent (PROTO-0)
      │
      ▼
Create + fund escrow (PROTO-2)
      │
      ▼
Provider submits receipt (PROTO-2)
      │
      ▼
Enterprise releases escrow (PROTO-2)
      │
      ▼
Settlement requested (PROTO-4)
      │
      ▼
Mock adapter submit / query (PROTO-4)
      │
      ▼
Finalize with fresh adapter evidence (PROTO-4)
      │
      ▼
Verified hard settlement
```

## Running

```bash
cd Aether/demo
cargo run                    # all four scenarios
cargo run -- happy-path      # scenario 1 only
cargo run -- spend           # scenario 2
cargo run -- reversal        # scenario 3
cargo run -- replay          # scenario 4
```

Each scenario prints ten lifecycle stages `[1]`–`[10]`. Failures name the rejecting protocol layer.

## Scenarios

| # | Name | Expected outcome | Rejecting layer |
|---|------|------------------|-----------------|
| 1 | Happy path | Full lifecycle through hard finality | — |
| 2 | Agent exceeds spend policy | `CapabilityDenied` on settlement request | PROTO-0 |
| 3 | Adapter reverses | Finalize rejected; hard finality stays false | PROTO-4 |
| 4 | Receipt replay | Second receipt rejected | PROTO-2 |

### Scenario notes

**Happy path (1):** Enterprise treasury completes escrow and settlement after provider delivery. Settlement is signed by the enterprise payer (a PROTO-4 participant requirement). Agent delegation is established at setup and enforced in scenario 2.

**Spend policy (2):** The autonomous agent attempts `request_settlement` for principal 1,000 with a delegated `max_spend` of 500. PROTO-0 rejects before any adapter side effect.

**Adapter reversal (3):** After adapter confirmation, the mock ledger marks the settlement reversed. `finalize_settlement` re-queries the adapter and rejects stale evidence (P4-SEC-002 remediation). Hard finality never promotes.

**Receipt replay (4):** Provider submits a valid receipt, then resubmits. PROTO-2 rejects the second submission (`InvalidEscrowStatus` once the escrow has left `Funded`; `ReceiptReplay` when nonce rules are violated on an open funded escrow).

## Reporting

Each scenario ends with a summary attributing decisions to protocol layers:

```text
Identity:      PROTO-0
Capability:    PROTO-0 (delegated grant)
Escrow:        PROTO-2
Settlement:    PROTO-4
Hard Finality: PROTO-4 (adapter evidence + finalize)
```

## Limitations

- **Local simulation only** — no network transport (PROTO-NET-0 is not exercised here).
- **Mock settlement provider** — `enterprise.ledger.v0` is deterministic test infrastructure, not a bank API.
- **Settlement actor constraint** — PROTO-4 requires the settlement request signer to be the escrow payer or provider. The agent’s delegated authority is demonstrated when the agent *attempts* settlement (scenario 2), not by having a non-participant sign a successful settlement.
- **No PROTO-1 channels** — this wedge uses escrow-only economic coordination.
- **Demonstration layer may be duplicated** — harness patterns mirror `core/tests/common` but do not import test helpers.

## Why this demonstrates enterprise spend control

1. **Delegated authority is explicit** — PROTO-0 grants carry `max_spend`, asset, and action bounds; the agent never receives root treasury keys.
2. **Economic exposure is bounded** — PROTO-2 escrow terms cap principal, fees, and timing before funds move.
3. **Work is evidenced** — PROTO-2 receipts bind provider claims to signed terms; replay is rejected.
4. **Settlement is verified, not trusted** — PROTO-4 treats the external ledger as evidence; hard finality requires fresh confirmed adapter state at finalize time.
5. **Providers are not protocol authority** — adapter reversal blocks hard settlement even when soft escrow finality already occurred.

## Completion boundary

After this demonstrator: **stop**. Do not begin reputation, marketplace, governance, a second settlement backend, or production payment integrations until the wedge is explicitly accepted.

See [ENTERPRISE_DEMO_RESULTS.md](ENTERPRISE_DEMO_RESULTS.md) for captured run output and guarantees.
