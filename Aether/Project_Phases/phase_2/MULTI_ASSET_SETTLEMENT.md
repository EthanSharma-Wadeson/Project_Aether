# Multi-Asset Settlement — Phase 2 Design

## Status

**Design only — extends PROTO-4 settlement binding without protocol code changes**

Describes how `SettlementAdapterV0` supports multiple settlement assets (fiat, stablecoin, enterprise ledger, optional AETH) with **identical protocol semantics** and an unchanged PROTO-0 / PROTO-2 authority model.

References:

- [PROTO_4_DESIGN.md](PROTO_4_DESIGN.md)
- [PROTO_4_DECISIONS.md](PROTO_4_DECISIONS.md) — P4-DEC-003 settlement-agnostic adapter
- [AETH_TOKEN_ARCHITECTURE.md](AETH_TOKEN_ARCHITECTURE.md)

---

## 1. Design Principle

> **One protocol. Many settlement assets. Adapter-specific backends.**

Aether does not embed currency logic. It embeds **authorisation, escrow rules, and binding integrity**. Value denomination is a field (`asset`) plus a pluggable adapter.

---

## 2. Architecture Overview

```text
                    ┌─────────────────────────────────┐
                    │  PROTO-0 — Authority            │
                    │  identity · capabilities        │
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────▼─────────────────┐
                    │  PROTO-2 — Economic rules       │
                    │  escrow · receipt · asset label │
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────▼─────────────────┐
                    │  PROTO-4 — Settlement binding │
                    │  bind · request · finalize      │
                    └───────────────┬─────────────────┘
                                    │
              ┌─────────────────────┼─────────────────────┐
              │                     │                     │
              ▼                     ▼                     ▼
     ┌────────────────┐   ┌────────────────┐   ┌────────────────┐
     │ enterprise.    │   │ payments.      │   │ settlement.    │
     │ ledger.v0      │   │ fiat.v0        │   │ stablecoin.v0  │
     │ GBP · USD ·    │   │ GBP · USD ·    │   │ USDC · EURC    │
     │ EUR (internal) │   │ EUR            │   │                │
     └────────────────┘   └────────────────┘   └────────────────┘
                                    │
                                    ▼
                          ┌────────────────┐
                          │ settlement.    │
                          │ aeth.v0        │
                          │ AETH (optional)│
                          └────────────────┘
```

Each box is an **adapter implementation**. The protocol sees only `settlement_provider` + `asset` + opaque `external_account_ref`.

---

## 3. Shared Interface — SettlementAdapterV0

Already defined in PROTO-4. Multi-asset support requires **no interface change** — only multiple implementations and consistent metadata.

### 3.1 Provider registration

| Field | Purpose |
|-------|---------|
| `provider_id` | Stable adapter name, e.g. `payments.fiat.v0` |
| `supported_assets` | Adapter advertises `["GBP","USD"]` etc. (application metadata) |
| `capabilities` | hold, release, refund, cancel support flags |

### 3.2 Account binding

`SettlementAccountBindingV0` per (agent, provider, asset):

| agent_id | settlement_provider | asset | external_account_ref |
|----------|---------------------|-------|----------------------|
| `aether:…` | `enterprise.ledger.v0` | `GBP` | `treasury-uk-001` |
| `aether:…` | `payments.fiat.v0` | `USD` | `stripe:acct:…` |
| `aether:…` | `settlement.stablecoin.v0` | `USDC` | `0x…` (opaque) |
| `aether:…` | `settlement.aeth.v0` | `AETH` | `aeth:…` (opaque) |

Binding requires `settlement.bind` capability — same for all assets.

### 3.3 Settlement request

`SettlementBindingV0` carries:

- `asset` — must match escrow `terms.asset`
- `principal_amount` / `fee_amount` — must match escrow terminal state
- `payer_account_binding_id` / `provider_account_binding_id` — must reference bindings for **same asset and compatible provider**

**Cross-asset settlement is rejected** at validation — not converted by protocol.

---

## 4. Identical Protocol Semantics

These behaviours are **invariant** across GBP, USD, EUR, USDC, enterprise ledger, and AETH:

| Behaviour | Rule |
|-----------|------|
| Authorisation order | Capability → escrow validation → adapter call |
| Participant signing | Payer or provider signs settlement request |
| Idempotency | Correlation id deduplication |
| Duplicate prevention | One active binding per `(escrow_id, economic_outcome)` |
| Status machine | Requested → Submitted → Accepted → Confirmed → Finalized |
| Failure | Fail closed; no invented funds |
| Reversal | `DisputedExternal`; hard finality blocked |
| Finalize | Fresh adapter query required |
| Hard finality | Only via verified finalize path |

### 4.1 PROTO-0 authority model (unchanged)

```text
authorise_action(agent_id, grant, ActionRequest { action, spend, asset })
```

- `settlement.settle` checks `max_spend` against **requested amount in stated asset**
- Asset scope in capability constraints must match or reject
- Delegation depth and expiry rules unchanged

### 4.2 PROTO-2 authority model (unchanged)

- Escrow terms fix `asset` at creation
- Funding, receipt, release conserve value in that asset label
- Fee budget applies in escrow's asset unit

### 4.3 PROTO-4 authority model (unchanged)

- Aether authoritative for bindings and escrow alignment
- External system authoritative for balance movement
- Adapter reports are evidence, not truth overrides

---

## 5. Asset Catalogue (Illustrative)

| Asset label | Typical adapter | Primary segment | Notes |
|-------------|-----------------|-----------------|-------|
| `GBP` | `enterprise.ledger.v0` / `payments.fiat.v0` | Enterprise | Preferred enterprise wedge |
| `USD` | `enterprise.ledger.v0` / `payments.fiat.v0` | Enterprise | Same semantics |
| `EUR` | `payments.fiat.v0` | Enterprise | Same semantics |
| `USDC` | `settlement.stablecoin.v0` | Crypto-native pilots | Lower volatility than AETH |
| `AETHER_TEST` | `enterprise.ledger.v0` | Development | Demonstrator only |
| `AETH` | `settlement.aeth.v0` | Open economy (optional) | Not required for enterprise |

Labels are **strings** in protocol objects. ISO codes and ticker symbols are conventions enforced by adapters and control plane — not by core validation logic beyond equality matching.

---

## 6. Multi-Asset Escrow Scenarios

### 6.1 Single-asset escrow (normative)

One escrow, one asset, one adapter family:

```text
terms.asset = "USD"
payer binding → payments.fiat.v0 / USD
provider binding → payments.fiat.v0 / USD
settlement → USD only
```

### 6.2 Enterprise multi-currency fleet (application layer)

Enterprise operates GBP and USD escrows as **separate escrows** with separate bindings. Protocol does not convert FX.

### 6.3 Agent capability across assets

Enterprise may grant:

```text
settlement.settle · max_spend 1000 · asset USD
settlement.settle · max_spend 800 · asset GBP
```

Separate grants — no implicit conversion.

---

## 7. Adapter Selection Policy (Control Plane)

Not protocol-critical. Suggested application rules:

| Policy | Recommendation |
|--------|----------------|
| Default enterprise pilot | `enterprise.ledger.v0` + fiat labels |
| Default open network | Stablecoin or AETH adapter **if** participant opts in |
| Required asset | **Never** at protocol level; deployment config only |
| Fallback | If adapter unavailable, settlement fails — no silent asset switch |

---

## 8. Testing Strategy (Design)

When implementing additional adapters (future):

| Test class | Assert |
|------------|--------|
| Parity | Same acceptance tests as PROTO-4 per asset |
| Mismatch | USD escrow + GBP binding rejects |
| Capability | `max_spend` enforced per asset |
| Finalize | Stale evidence rejects for every adapter |
| Idempotency | Same correlation returns same binding |

Demonstrator today: single asset `AETHER_TEST` on `enterprise.ledger.v0` — sufficient for wedge validation.

---

## 9. Non-Goals

| Non-goal | Reason |
|----------|--------|
| In-protocol DEX / FX | Application or external market |
| Automatic asset conversion | Hides economic risk |
| Asset priority ranking | Policy, not protocol |
| Global asset registry in core | Control plane / directory service |

---

## 10. Freeze Statement

> Multi-asset settlement is achieved by **multiple adapters and asset labels**, not by extending PROTO-0/2/4. AETH is one label among many. Enterprise workflows on GBP/USD remain the reference path; optional assets add capability without replacing it.
