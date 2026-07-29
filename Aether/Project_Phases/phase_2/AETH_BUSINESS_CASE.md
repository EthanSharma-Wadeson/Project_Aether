# AETH Business Case — Phase 2 Design

## Status

**Business analysis only — not pricing, go-to-market, or token launch plan**

Compares **enterprise customers** and the **open autonomous agent economy** to determine where existing money is sufficient, where AETH adds value, and whether customers should ever be **required** to hold AETH.

References:

- [BUSINESS_ALIGNMENT.md](BUSINESS_ALIGNMENT.md)
- [AETH_ECONOMIC_MODEL.md](AETH_ECONOMIC_MODEL.md)
- [ENTERPRISE_DEMO.md](ENTERPRISE_DEMO.md)
- [PHASE_2_WEDGE_DECISION.md](PHASE_2_WEDGE_DECISION.md)

---

## 1. Executive Recommendation

| Question | Answer |
|----------|--------|
| Should enterprises be required to hold AETH? | **No** |
| Should open-network participants be required to hold AETH? | **No** (default) |
| Is AETH justified today? | **Not for the enterprise wedge** — existing ledger/fiat is sufficient |
| Is AETH potentially justified later? | **Maybe** — only for open agent economy portability and micro-settlement, if measured |
| Default adoption stance | **Optional everywhere** |

---

## 2. Enterprise Customers

### 2.1 Profile

Companies deploying autonomous agents under spend control, audit, and compliance constraints. Primary buyer: platform engineering / AI infrastructure. Validated wedge: Enterprise Agent Spend Control.

### 2.2 Why existing money is sufficient

| Need | Existing solution | Aether without AETH |
|------|-------------------|---------------------|
| Treasury control | ERP, corporate banking, virtual cards | PROTO-0 `max_spend` + PROTO-2 escrow |
| Audit | SIEM, accounting systems | Signed CBOR + binding evidence |
| Settlement | Internal cost centres, Stripe, bank transfers | PROTO-4 `enterprise.ledger.v0` / `payments.fiat.v0` |
| Compliance | Fiat AML/KYC stack | No new asset class |
| Budgeting | Finance-approved currencies | Capability constraints per `asset` label (GBP, USD) |

**Demonstrator evidence:** Full lifecycle to hard finality without AETH, using enterprise ledger adapter.

### 2.3 Where AETH might add value (enterprise)

| Scenario | Value | Assessment |
|----------|-------|------------|
| Cross-enterprise agent marketplace | Neutral unit between companies | **Low near-term** — enterprises prefer invoicing in fiat |
| Internal “agent credits” gamification | Motivation / accounting simplification | **Optional** — can use ledger labels without public AETH |
| Crypto-native enterprise unit | Treasury holds digital assets | **Niche** — stablecoin adapter likely preferred over volatile AETH |
| Vendor requires crypto settlement | Pay external provider | **Edge case** — USDC adapter first |

**Enterprise verdict:** AETH provides **marginal** value at **high** regulatory and narrative cost. **Not recommended** as enterprise default.

### 2.4 Should enterprises ever be required to hold AETH?

**No.**

Reasons:

1. Procurement blockers — crypto policies are common in regulated industries.
2. Wedge validation does not require it — undermines proven enterprise demo story.
3. FX and volatility are finance-team objections, not IT objections.
4. Competing products (cloud IAM + billing) do not require a proprietary token.

**Acceptable enterprise pattern:** Enterprise operates entirely on `GBP`/`USD` via `enterprise.ledger.v0`. Agents never touch AETH. Optional future: treasury opens AETH adapter for specific crypto-native vendors — deployment choice.

---

## 3. Open Autonomous Agent Economy

### 3.1 Profile

Multi-agent coordination without a central platform operator: marketplaces, agent swarms, research networks. Strongest **protocol fit**, weakest **near-term revenue**.

### 3.2 Why existing money is often sufficient

| Need | Existing solution | Limit |
|------|-------------------|-------|
| Identity | Platform accounts | Not portable |
| Settlement | Platform escrow | Lock-in |
| Micro-payments | Cards impractical | True gap at scale |
| Cross-platform work | N/A | **Gap** |
| Anti-Sybil | Platform KYC | Centralised |

Stablecoins (USDC) already address **portable settlement** for crypto-native participants without a new AETH.

### 3.3 Where AETH might add value (open economy)

| Scenario | Hypothesised AETH value | Counterargument |
|----------|-------------------------|-----------------|
| Portable settlement across operators | Shared unit without per-platform ledger | **USDC** may suffice; adds issuer dependency either way |
| Machine micro-transactions | Low per-tx cost at scale | Requires chain/backend economics — unproven |
| Neutral bond for Sybil resistance | Open registration cost | Multi-asset bonds; fiat deposits possible |
| Protocol-aligned unit | Ecosystem branding | Branding ≠ protocol need |
| Reduced platform lock-in | Settlement not tied to marketplace balance | **Strongest argument** — still requires liquidity and adoption |

**Open economy verdict:** AETH is **plausibly useful** but **not proven necessary**. USDC + PROTO-4 may capture most value with lower narrative risk.

### 3.4 Should open-network participants be required to hold AETH?

**No** — default answer.

Optional participation patterns:

| Pattern | Description |
|---------|-------------|
| **Fiat-only agents** | Bind `payments.fiat.v0`; never see AETH |
| **Stablecoin agents** | Bind `settlement.stablecoin.v0` |
| **AETH-opt-in agents** | Bind `settlement.aeth.v0` when counterparty agrees |
| **Hybrid principals** | Enterprise payer in USD; open provider accepts USDC — separate escrows, no conversion |

Mandatory AETH creates a **cold-start liquidity trap**: agents cannot participate without acquiring AETH from somewhere — contradicts infrastructure-first positioning.

---

## 4. Side-by-Side Comparison

| Dimension | Enterprise customers | Open agent economy |
|-----------|---------------------|-------------------|
| Primary currency | Fiat / internal ledger | Platform balance or crypto |
| AETH required? | **No** | **No** |
| AETH recommended? | **No** | **Optional only** |
| Best settlement adapter | `enterprise.ledger.v0` | `stablecoin` > `aeth` > `fiat` (TBD by evidence) |
| Unique value without AETH | **High** (spend control) | **Medium** (portability gap remains) |
| Risk of AETH promotion | Damages enterprise sales | Acceptable if segment-specific |
| Proof today | Enterprise demo complete | Not built |

---

## 5. Value Proposition Test

Aether creates unique value when:

1. Agents act **autonomously**
2. Authority is **delegable, revocable, auditable**
3. Counterparties verify trust **without shared platform operator**

| Segment | Condition 3 met without AETH? | AETH strengthens proposition? |
|---------|------------------------------|------------------------------|
| Enterprise (internal agents) | Partially — same org trust | **No** — fiat sufficient |
| Enterprise (cross-vendor agents) | Improving via portable `AgentId` | **Marginal** |
| Marketplace | Weak without reputation | **Maybe** — neutral settlement |
| Open ecosystem | **Weak** without portable settlement | **Maybe** — if USDC insufficient |

**Conclusion:** AETH is a **potential enhancer** for segment 4 only — not a foundation for segments 1–2 where revenue lives today.

---

## 6. Competitive Positioning

| Competitor pattern | Aether without AETH | Aether with mandatory AETH |
|--------------------|----------------------|----------------------------|
| Cloud IAM + billing | Comparable for enterprise | **Worse** — crypto friction |
| “AI × crypto” tokens | **Differentiated** — infrastructure seriousness | **Commoditised** — narrative collision |
| Marketplace escrow | Portable identity + rules | Unclear advantage |
| Stablecoin payment rails | Complementary | AETH competes with USDC — needs clear reason |

**Strategic recommendation:** Position against **token projects** by remaining settlement-agnostic. Introduce AETH only if it solves a problem **stablecoins and fiat cannot** — and prove it with data.

---

## 7. Decision Framework

```text
                    ┌─────────────────────────┐
                    │  Does use case need     │
                    │  portable machine       │
                    │  settlement?            │
                    └───────────┬─────────────┘
                          No    │    Yes
                    ┌───────────┴───────────┐
                    ▼                       ▼
            Use fiat /               Can USDC + PROTO-4
            enterprise ledger        solve it?
            (no AETH)                      │
                                    Yes ───┴─── No
                                     │            │
                                     ▼            ▼
                              No AETH      Evaluate AETH
                              required     as optional adapter
                                           (still not required)
```

---

## 8. What Success Looks Like (Business)

| Outcome | Metric |
|---------|--------|
| Enterprise adoption without AETH | Design partners on ledger/fiat adapters |
| No crypto procurement blockers | Zero AETH requirement in enterprise contracts |
| Open economy optionality | Participants choose asset at escrow creation |
| Honest positioning | No “token required” messaging |
| Evidence before implementation | Volume/cost data from networked pilot |

---

## 9. Final Answer

> **Does AETH solve a genuine protocol problem, or would it merely be a cryptocurrency attached to Aether?**

For **enterprise customers today:** It would **largely be a cryptocurrency attached to Aether** — the protocol problem is already solved without it.

For the **open autonomous agent economy:** AETH **might** solve portability and micro-settlement problems that fiat cannot — but **stablecoins may suffice**, and the hypothesis is **unproven**.

**Business case verdict:**

- **Do not implement AETH** until open-network pilots produce evidence that multi-asset adapters excluding AETH are insufficient.
- **Do not require AETH** for any customer segment.
- **Preserve optional adoption** as a permanent architectural constraint.

---

## 10. Freeze Statement

> This document does not authorise token issuance, chain development, or wallet products. It records business reasoning for optional AETH as a future settlement adapter — nothing more.
