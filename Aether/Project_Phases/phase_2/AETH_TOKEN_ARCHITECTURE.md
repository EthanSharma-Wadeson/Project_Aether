# AETH Token Architecture — Phase 2 Design

## Status

**Architecture only — no token implementation, no chain, no contracts**

This document describes how **AETH** would integrate with Aether **if** a decision is made to support it. It deliberately does **not** define supply schedules, consensus, wallets, or smart contracts.

References:

- [PROTO_4_DESIGN.md](PROTO_4_DESIGN.md) — `SettlementAdapterV0`, `SettlementAccountBindingV0`
- [MULTI_ASSET_SETTLEMENT.md](MULTI_ASSET_SETTLEMENT.md) — shared multi-asset interface
- [AETH_ECONOMIC_MODEL.md](AETH_ECONOMIC_MODEL.md) — role and classification

---

## 1. Architectural Position

AETH is **not** a new protocol layer. It is a **settlement backend + asset label** sitting below PROTO-4:

```text
PROTO-0  Identity + capability authority
    ↓
PROTO-2  Escrow terms (asset = "AETH" | "USD" | …)
    ↓
PROTO-4  Settlement binding + evidence
    ↓
SettlementAdapterV0
    ├── enterprise.ledger.v0   (GBP/USD internal)
    ├── payments.fiat.v0       (banking partner)
    ├── settlement.stablecoin.v0 (USDC, etc.)
    └── settlement.aeth.v0     (AETH — optional)
```

**Rule:** Core protocol code imports **no** AETH-specific logic. Only an adapter implementation (outside or adjacent to `aether-core`) understands AETH custody, addresses, or ledger queries.

---

## 2. What “Token Architecture” Means Here

In this design phase, “token architecture” means:

| In scope | Out of scope |
|----------|--------------|
| Adapter interface contract | ERC-20 / SPL / custom contract bytecode |
| Asset identifier conventions | Mint/burn policy |
| Account binding shape | Wallet UX |
| Evidence / finality mapping | Consensus algorithm |
| Capability gating for bind/settle | Token sale or distribution |
| Failure modes and reversal handling | Bridge security implementation |

AETH is treated as **an external settlement asset** whose movements are **reported** to Aether through the same adapter boundary as fiat.

---

## 3. Identifier Conventions

### 3.1 Asset label (PROTO-2 / PROTO-4)

Escrow terms and settlement bindings carry:

```text
asset: "AETH"
```

This is an opaque label matched across escrow, capability constraints, and binding. It is **not** a chain contract address at the protocol layer.

### 3.2 Settlement provider id

```text
settlement_provider: "settlement.aeth.v0"
```

Or, if AETH is hosted on an external chain:

```text
settlement_provider: "chain.aeth.mainnet.v0"   # example only — not selected
```

Provider id is chosen at **adapter registration** time in the application/control plane — not hardcoded in protocol.

### 3.3 External account reference

`SettlementAccountBindingV0.external_account_ref` remains **opaque** to the protocol:

```text
external_account_ref: "<adapter-defined account handle>"
```

Examples (adapter-internal, not protocol-normative):

- `aeth:account:0x…`
- `custodian:wallet:uuid:…`

Protocol validates **binding authorisation** and **shape**; adapter validates **account existence**.

---

## 4. SettlementAdapter Mapping for AETH

AETH uses the **same** `SettlementAdapterV0` interface as every other backend:

```text
SettlementAdapterV0
  provider_id()           → "settlement.aeth.v0"
  capabilities()        → { supports_hold, supports_release, supports_refund, … }

  validate_account(ref)   → Result
  request_settlement(binding) → AdapterSubmitResult
  query_settlement(external_ref) → AdapterStatusResult
  cancel_settlement(external_ref) → Result   // if supported
```

### 4.1 Semantic parity

| Operation | GBP adapter | AETH adapter |
|-----------|-------------|--------------|
| Bind account | Enterprise treasury code | AETH account handle |
| Request settle | Post payment intent | Post transfer intent |
| Query status | Ledger confirmation | Backend confirmation |
| Confirm | Funds posted | Units moved |
| Reverse | Chargeback / reversal | Backend reversal (adapter reports) |
| Hard finality | Fresh query at finalize | **Identical PROTO-4 rule** |

No special-case finality path for AETH.

### 4.2 Capability actions (unchanged)

| Action | Owner |
|--------|-------|
| `settlement.bind` | Account binding |
| `settlement.settle` | Settlement request |
| `settlement.query` | Status poll |
| `settlement.cancel` | Pre-confirm cancel |

PROTO-0 semantics are **identical** whether `asset` is `AETH` or `USD`.

---

## 5. Custody Models (Design Options — Not Selected)

AETH custody is an **implementation choice behind the adapter**, not a protocol decision.

| Model | Description | Enterprise fit | Open economy fit |
|-------|-------------|----------------|------------------|
| **Custodial partner** | Licensed custodian holds AETH; agents authorise via capabilities | Low — redundant with ledger | Medium |
| **Enterprise sub-ledger** | AETH as internal unit on `enterprise.ledger.v0` | Medium — competes with fiat labels | Low |
| **External chain** | On-chain asset with light-client or RPC adapter | Low — regulatory/complexity | High — if chain justified separately |
| **Synthetic / credit** | Platform-issued balance not on public chain | Medium | Medium — centralisation risk |

**Design default:** Do not select a custody model until [AETH_ECONOMIC_MODEL.md](AETH_ECONOMIC_MODEL.md) decision gates pass.

---

## 6. Fee and Bond Denomination

If AETH is used for fees or bonds (optional):

```text
Fee meter (application layer)
    → charges N units of asset X
    → asset X may be "AETH" | "USD" | "USDC"
    → settlement via corresponding adapter
```

Protocol-critical objects reference `asset` only. **Fee policy** is application or control-plane configuration.

Bonds (future `BondV0` — not implemented) should support **multi-asset** locks:

```text
bond_asset: "AETH" | "USD" | …
bond_amount: uint64
```

Never require AETH as the sole bond asset.

---

## 7. Interaction with PROTO-2 Escrow

Escrow creation already requires matching assets:

| Field | AETH example |
|-------|--------------|
| `terms.asset` | `"AETH"` |
| `terms.principal_amount` | smallest units (adapter defines decimals) |
| `capability.constraints.asset` | `"AETH"` (optional scope) |

Funding escrow in AETH:

1. Payer holds AETH balance **outside** protocol (custodian/chain).
2. PROTO-2 records funded escrow in simulated or adapter-backed ledger.
3. Release triggers PROTO-4 settlement intent with `asset: "AETH"`.
4. Adapter moves AETH; PROTO-4 records evidence.

**No change** to escrow state machine semantics.

---

## 8. Evidence and Finality

AETH settlements produce the same evidence package as fiat:

| Field | Role |
|-------|------|
| `external_settlement_ref` | Adapter-issued confirmation id |
| `adapter_response_bytes` | Canonical provider payload |
| `evidence_commitment` | SHA-256 over evidence fields |

Hard finality rules from PROTO-4 remediation apply unchanged:

- Finalize **re-queries** adapter; stale or reversed reports reject.
- Adapter is evidence source, **not** protocol authority.

---

## 9. What Would NOT Be Built (This Phase)

| Item | Reason |
|------|--------|
| AETH mint contract | Token implementation out of scope |
| Aether L1 chain | No measured requirement |
| In-protocol wallet | Application layer |
| DEX / liquidity pools | Not protocol semantics |
| AETH-only capability roots | Violates optional adoption |
| Protocol-enforced price oracles | Speculation surface |

---

## 10. Summary

AETH “token architecture” is **adapter architecture**:

- One asset label (`AETH`)
- One or more `SettlementAdapterV0` implementations
- Identical PROTO-0 / PROTO-2 / PROTO-4 authority and lifecycle
- Zero protocol primitive changes

If AETH cannot be expressed as an adapter without special cases, it does not belong in Aether.
