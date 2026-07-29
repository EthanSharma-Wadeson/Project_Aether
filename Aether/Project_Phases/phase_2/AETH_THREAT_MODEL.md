# AETH Threat Model — Phase 2 Design

## Status

**Threat analysis only — extends [MAINFRAME_THREAT_MODEL.md](MAINFRAME_THREAT_MODEL.md) and [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)**

Analyses risks of introducing an **optional** native asset (AETH) and how optional adoption changes — or does not change — Aether's trust model.

References:

- [AETH_ECONOMIC_MODEL.md](AETH_ECONOMIC_MODEL.md)
- [PROTO_4_SECURITY_REVIEW.md](PROTO_4_SECURITY_REVIEW.md)
- [MULTI_ASSET_SETTLEMENT.md](MULTI_ASSET_SETTLEMENT.md)

---

## 1. Trust Model Baseline (Without AETH)

```text
Authoritative                          Evidence-only
─────────────────────────────────────────────────────────
PROTO-0 identity & capabilities        Settlement adapters
PROTO-2 escrow state & rules             External ledger balances
PROTO-4 binding integrity                Provider reversal reports
```

Participants trust **cryptographic authority and escrow rules** first. They trust **settlement backends** only through verified adapter evidence at finalize time.

**Without AETH:** Trust model is already asymmetric — adapters cannot promote hard finality without fresh confirmation.

---

## 2. Trust Model With Optional AETH

Introducing AETH **does not** move adapter evidence into the authoritative column **if** design principles are followed:

| Layer | Without AETH | With optional AETH |
|-------|--------------|-------------------|
| Identity / capability | PROTO-0 authoritative | **Unchanged** |
| Escrow economics | PROTO-2 authoritative | **Unchanged** |
| Settlement finality | Adapter evidence via PROTO-4 | **Unchanged** — AETH adapter is still evidence-only |
| AETH price / liquidity | N/A (external markets) | **Out of protocol scope** |
| AETH custody | N/A | **Backend assumption** — same as fiat custodian |

**Key invariant:** Optional AETH must not create a code path where “AETH confirmed” bypasses adapter query or capability checks.

---

## 3. Threat Catalogue

### 3.1 Speculative token behaviour

| Threat | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| T-AETH-001 | AETH positioned as investment; protocol confused with token project | **High** (reputational) | Utility-only communications; no protocol price mechanics; enterprise-first wedge |
| T-AETH-002 | Volatility breaks agent budgeting | **High** (operational) | Default to fiat/stablecoin; capability `max_spend` in volatile assets requires explicit opt-in |
| T-AETH-003 | Market manipulation affects settlement timing | **Medium** | Settlement amounts fixed at escrow terms; FX/oracle not in protocol |

### 3.2 Regulatory exposure

| Threat | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| T-AETH-004 | AETH classified as security or e-money in target jurisdiction | **High** | Legal review before implementation; optional adoption; utility framing |
| T-AETH-005 | Enterprise customers blocked by crypto association | **High** | AETH never required; isolated adapter; separate branding for enterprise rail |
| T-AETH-006 | KYC/AML gaps on open AETH rail | **High** | Custodial partner or licensed on-ramp — application layer, not protocol |

### 3.3 Volatility and liquidity

| Threat | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| T-AETH-007 | Insufficient liquidity for agent settlements | **Medium** | Stablecoin adapter as alternative; fail closed on unsettled bindings |
| T-AETH-008 | Liquidity provider exit strands agents | **Medium** | No protocol guarantee of liquidity; multi-asset support |
| T-AETH-009 | Slippage between quote and settle | **Low** (if fixed escrow amounts) | Amount locked in PROTO-2 terms; adapter must match or reject |

### 3.4 Governance capture

| Threat | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| T-AETH-010 | AETH holders capture protocol direction | **High** (if governance token) | **Not recommended** — no token voting in protocol |
| T-AETH-011 | Foundation/treasury discretionary mint affects participants | **High** | No mint in protocol scope; supply policy out of scope but threat if added later |
| T-AETH-012 | Parameter changes favour large holders | **Medium** | Fee/bond parameters versioned; enterprise policy separate |

### 3.5 Sybil attacks and economic spam

| Threat | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| T-AETH-013 | Cheap Sybil identities flood open network | **High** (open economy) | Bonds + rate limits + capabilities; **multi-asset bonds** — AETH not sole option |
| T-AETH-014 | Spam settlement requests exhaust adapters | **Medium** | PROTO-0 capability metering; adapter rate limits |
| T-AETH-015 | AETH airdrop farming distorts reputation | **Medium** | Reputation not yet implemented; design bonds on evidence not balance |

### 3.6 Protocol abuse

| Threat | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| T-AETH-016 | Fake AETH adapter reports confirmed status | **High** | Same as PROTO-4: fresh query on finalize; fail closed |
| T-AETH-017 | AETH-only escrow traps enterprise in crypto | **High** | Never require AETH; terms.asset chosen at escrow creation |
| T-AETH-018 | Capability escalation via AETH-specific actions | **Medium** | No AETH-specific capability tokens; reuse `settlement.*` |
| T-AETH-019 | Double-settle across AETH and fiat adapters | **Medium** | One active binding per `(escrow_id, outcome)` — already PROTO-4 rule |

### 3.7 Narrative and product risks

| Threat | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| T-AETH-020 | “AI × crypto” association dilutes enterprise credibility | **High** | Separate messaging; demonstrator uses enterprise ledger |
| T-AETH-021 | Premature AETH launch before open-network need | **Medium** | Decision gates in AETH_ECONOMIC_MODEL §9 |

---

## 4. How Optional Asset Changes Attack Surface

### 4.1 Does NOT increase (if designed correctly)

- PROTO-0 forgery or capability bypass — no AETH code in authority path
- PROTO-2 conservation violations — asset label is opaque string
- PROTO-4 stale finalize — same remediation for all adapters

### 4.2 DOES increase (operational / ecosystem)

| New surface | Why |
|-------------|-----|
| Custody risk | Someone must hold AETH balances |
| Market risk | Volatility if used as settlement unit |
| Regulatory scrutiny | Crypto classification |
| Social engineering | “Send AETH to verify agent” scams |
| Adapter diversity | More backends → more integration bugs |

### 4.3 Optional vs mandatory — trust difference

| Model | Trust implication |
|-------|-------------------|
| **AETH mandatory** | Protocol becomes dependent on AETH liquidity, price, and regulation — **weakens** enterprise proposition |
| **AETH optional** | Participants who never bind AETH accounts have **identical trust model** to today |
| **AETH default for open network** | Soft pressure to hold AETH — treat as product policy risk, mitigate with stablecoin parity |

**Recommendation:** Optional only. Enterprise deployments should default to zero AETH exposure.

---

## 5. Adversary Profiles

| Adversary | AETH-specific goal | Protocol response |
|-----------|-------------------|-------------------|
| Speculator | Hype-driven adoption | No protocol coupling to price |
| Regulator | Classify protocol as MSB/securities issuer | Settlement-agnostic design; licensed partners |
| Sybil farmer | Cheap identities + AETH faucets | Bonds in any asset; capability limits |
| Malicious adapter | False confirmation | Fresh finalize query; hard flag gated |
| Enterprise CISO | Block crypto on policy grounds | Operate without AETH adapter |
| Governance attacker | Token-weighted control | No governance token |

---

## 6. Security Invariants (AETH-Specific)

If AETH is ever implemented, these invariants must hold:

1. **INV-AETH-01:** No `aether-core` import of AETH custody or pricing logic.
2. **INV-AETH-02:** AETH adapter implements `SettlementAdapterV0` without privileged protocol hooks.
3. **INV-AETH-03:** Hard finality promotion path is identical to `enterprise.ledger.v0`.
4. **INV-AETH-04:** Escrow `asset: "AETH"` cannot be created without payer/provider consent in signed terms.
5. **INV-AETH-05:** Capability grants for settlement never implicitly convert assets.
6. **INV-AETH-06:** No protocol object embeds AETH supply, price, or APY.

---

## 7. Residual Risk Statement

Even with optional AETH and correct adapter design:

- **Reputational risk** from crypto association may persist for enterprise sales.
- **Volatility** remains a participant problem if they choose AETH denomination.
- **Liquidity** is not a protocol guarantee.

These are acceptable **only if** AETH remains optional and enterprise rails remain the primary wedge.

---

## 8. Conclusion

Optional AETH **does not inherently weaken** protocol security if it stays behind `SettlementAdapterV0`. It **does weaken** business positioning and **does expand** operational/regulatory attack surface if promoted prematurely or made mandatory.

**Threat model verdict:** Proceed with AETH **design only**. Do not implement until open-network evidence outweighs T-AETH-004, T-AETH-005, and T-AETH-020.

---

## 9. Related Reviews

| Document | Relationship |
|----------|--------------|
| PROTO_4_SECURITY_REVIEW | Adapter evidence rules apply to AETH adapter |
| MAINFRAME_THREAT_MODEL | Distributed agent threats |
| SECURITY_MODEL.md | Project-wide assets and adversaries |
