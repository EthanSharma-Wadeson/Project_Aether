# AETH Economic Model — Phase 2 Design

## Status

**Design only — not an implementation specification**

No Rust, no blockchain, no token contracts, no supply schedule. This document defines whether **AETH** has a justified role and how it coexists with traditional payment rails without weakening Aether's settlement-agnostic protocol.

References:

- [PROTO_4_DESIGN.md](PROTO_4_DESIGN.md) — `SettlementAdapterV0`, asset labels, authority split
- [MULTI_ASSET_SETTLEMENT.md](MULTI_ASSET_SETTLEMENT.md) — adapter-level multi-asset design
- [BUSINESS_ALIGNMENT.md](BUSINESS_ALIGNMENT.md) — enterprise wedge vs open economy
- [ECONOMIC_MODEL.md](../../Aether_docs/ECONOMIC_MODEL.md) — project-wide economic primitives

---

## 1. Executive Conclusion

> **Does AETH solve a genuine protocol problem, or is it merely a cryptocurrency attached to Aether?**

**Answer:** AETH is **not required** for Aether to function. The protocol stack (PROTO-0 authority, PROTO-2 escrow economics, PROTO-4 settlement binding) is **asset-agnostic** and has been validated without AETH — including the Enterprise Spend Control demonstrator using `AETHER_TEST` as a simulated label and `enterprise.ledger.v0` as the settlement backend.

AETH **may** become valuable as an **optional settlement asset and fee/bond denomination** in environments where:

1. Machine-to-machine settlement must be **portable across operators** without a shared enterprise ledger.
2. **Micro-transaction economics** make traditional rails impractical at agent scale.
3. Participants voluntarily choose a **neutral settlement unit** behind the same PROTO-4 semantics as fiat or stablecoin adapters.

AETH is **not recommended** as a protocol primitive, governance token, or mandatory access gate. If introduced, it integrates **only** through `SettlementAdapterV0` and optional economic policy layers — never by changing PROTO-0 / PROTO-2 authority rules.

---

## 2. What AETH Is

| Property | Definition |
|----------|------------|
| **Name** | AETH — optional native settlement unit of the Aether ecosystem |
| **Protocol role** | An **asset label** (`asset: "AETH"`) and a **settlement backend family** (`settlement.aeth.v0` or external chain adapter) consumed by PROTO-4 |
| **Authority** | Aether remains authoritative for identity, capabilities, escrow, and binding integrity; an AETH backend is authoritative only for AETH balance movement **as reported via adapter evidence** |
| **Adoption** | Opt-in per deployment, per escrow terms, per account binding |
| **Semantics** | Identical settlement lifecycle to GBP, USD, USDC, or enterprise ledger: Requested → Submitted → Accepted → Confirmed → Finalized |

AETH is a **settlement denomination choice**, not a new protocol layer.

---

## 3. What AETH Is Not

| AETH is NOT | Why |
|-------------|-----|
| A protocol primitive | PROTO-0/1/2/4 do not require it |
| The product | Control Plane, enterprise dashboards, and adapters are application/infrastructure layers |
| A blockchain requirement | No consensus, no chain, no wallets in this design phase |
| A governance token (by default) | Governance is a separate concern; token voting is not a protocol need |
| Mandatory for identity or escrow | Capability and escrow rules are asset-labelled but asset-agnostic |
| A speculative narrative asset | Utility-first positioning; speculation is a threat, not a goal |
| A replacement for enterprise fiat workflows | Enterprise pilots should continue on ledger/fiat rails |

---

## 4. Why Aether Operates Without AETH

Aether's core problems are **authority, coordination, and verifiable economic release** — not currency issuance.

```text
Problem                          Solved by (today)              Requires AETH?
─────────────────────────────────────────────────────────────────────────────
Who may this agent act as?       PROTO-0 identity + caps        No
What economic action is allowed? PROTO-0 authorisation            No
Are terms agreed and funded?     PROTO-2 escrow                 No
Was work evidenced?              PROTO-2 receipt                  No
Did external money move?         PROTO-4 adapter evidence         No (any asset)
```

**Enterprise wedge proof:** The demonstrator completes identity → delegation → escrow → settlement → hard finality using an enterprise ledger adapter and a test asset label. No native token, no chain, no AETH.

**Design principle (locked):** Settlement backends are pluggable. Choosing GBP over AETH does not change protocol semantics — only the adapter implementation and `asset` field in terms/bindings.

---

## 5. Why AETH May Become Valuable (Specific Environments)

AETH earns a role only where **existing money is insufficient** for the coordination pattern, not where it is merely fashionable.

| Environment | Pain without neutral unit | AETH value hypothesis |
|-------------|---------------------------|------------------------|
| **Open agent economy** | No shared enterprise ledger; each operator has its own accounting | Portable settlement unit across marketplaces and agent runtimes |
| **High-volume micro-settlement** | Card/ACH minimums and per-tx fees dominate | Sub-cent machine payments if backend economics support it |
| **Cross-jurisdiction agent work** | Fiat rails are fragmented; stablecoins help but add issuer dependency | Optional third denomination behind same PROTO-4 binding |
| **Bond/fee denomination** | Anti-spam bonds in fiat require banking per participant | Unified bond asset for open networks (optional) |

These are **hypotheses requiring measurement** — not commitments. Each must pass a gate: *does this problem remain unsolved with USDC/enterprise ledger + PROTO-4?*

---

## 6. Economic Responsibilities — Classification

| Use case | Classification | Rationale |
|----------|----------------|-----------|
| **Escrow principal settlement** | **Optional** | Any asset works via PROTO-2 `asset` + PROTO-4 binding. AETH is one adapter option. |
| **Protocol fees** | **Optional** | Fees can be levied in fiat, stablecoin, or internal credits. AETH only if open-network metering needs a neutral unit. |
| **Agent-to-agent settlement** | **Optional** | Strongest AETH case in open economy; enterprise uses treasury rails. |
| **Staking** | **Not Recommended** (as protocol default) | Implies validator/chain economics not in scope; defer unless native chain is justified by evidence. |
| **Security bonds** | **Optional** | Legitimate anti-Sybil tool in open networks; enterprise can use fiat holds or IAM. Multi-asset bonds preferred over AETH-only. |
| **Anti-Sybil mechanisms** | **Optional** | Bonds + rate limits + capabilities suffice; asset choice is policy. |
| **Governance** | **Not Recommended** | No protocol need for token voting; creates regulatory and capture risk. |
| **Liquidity** | **Not Recommended** (as protocol obligation) | Liquidity is market infrastructure, not protocol semantics. |
| **Enterprise internal accounting** | **Not Recommended** | Enterprises already have ERP/ledger; `enterprise.ledger.v0` path is preferred. |

**Recommended default:** Enterprise and pilot deployments use **fiat / enterprise ledger / stablecoin** adapters. AETH is an **additional capability**, never the default requirement.

---

## 7. Asset-Agnostic Invariants

These hold regardless of whether settlement is in GBP, USDC, or AETH:

1. **PROTO-0** authorises actions; it does not custody settlement assets.
2. **PROTO-2** escrow conservation is defined over an `asset` label; mismatched assets reject.
3. **PROTO-4** verifies binding integrity and adapter evidence; it does not trust provider state as authority.
4. **Capability `max_spend`** is evaluated against the requested spend amount **in the stated asset**.
5. **Hard finality** promotes only after fresh adapter confirmation — same rule for all backends.

---

## 8. Relationship to Other Documents

| Document | Focus |
|----------|-------|
| [AETH_TOKEN_ARCHITECTURE.md](AETH_TOKEN_ARCHITECTURE.md) | How AETH maps to SettlementAdapter (no chain design) |
| [MULTI_ASSET_SETTLEMENT.md](MULTI_ASSET_SETTLEMENT.md) | Multi-asset adapter architecture |
| [AETH_BUSINESS_CASE.md](AETH_BUSINESS_CASE.md) | Enterprise vs open economy adoption |
| [AETH_THREAT_MODEL.md](AETH_THREAT_MODEL.md) | Risks of optional native asset |

---

## 9. Decision Gates Before Any AETH Implementation

| Gate | Question |
|------|----------|
| G1 | Is there measured open-network volume that fiat/stablecoin adapters cannot serve? |
| G2 | Does AETH improve portability without weakening enterprise optional adoption? |
| G3 | Can bonds/fees be multi-asset so AETH is never mandatory? |
| G4 | Does legal review accept utility positioning in target jurisdictions? |
| G5 | Would AETH strengthen or dilute the enterprise spend-control wedge narrative? |

**If G1–G5 are not satisfied:** Do not implement AETH. Aether remains complete without it.

---

## 10. Freeze Statement

> AETH is an **optional settlement and economic denomination**, not a protocol requirement. Aether's authority model does not change when AETH is absent. Any future AETH work is adapter and policy layer only — never a fork of PROTO-0/2/4 semantics.
