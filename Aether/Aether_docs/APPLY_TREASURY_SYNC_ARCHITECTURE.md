# Apply ↔ Treasury Allocation Sync Architecture — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `APPLY_TREASURY_SYNC_ARCHITECTURE.md` |
| **Phase** | 23 — Apply ↔ Treasury Allocation Sync Design Gate |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Inputs** | [TREASURY_ARCHITECTURE_REVIEW.md](TREASURY_ARCHITECTURE_REVIEW.md), [TREASURY_CONTROL_PLANE_ARCHITECTURE.md](TREASURY_CONTROL_PLANE_ARCHITECTURE.md), [APPLY_ENABLEMENT_READINESS_REVIEW.md](APPLY_ENABLEMENT_READINESS_REVIEW.md), Phases 19–22.5 |
| **Protocol impact** | **None** in this phase — future sync must not fork PROTO-0 semantics |
| **Apply impact** | **None** — `apply_enabled` remains false |
| **Treasury accounting impact** | **None** — journal semantics unchanged |

---

## Executive Summary

Agent **capability** (PROTO-0) and organisation **allocation** (Treasury) are intentionally separate authority planes. Drift between them produces over-permissioned agents, unfunded capabilities, and false confidence in “funded spend.”

This document freezes the **future sync architecture**:

| Principle | Rule |
|-----------|------|
| Runtime spend | **Intersection** — capability ∧ allocation ∧ policy ∧ risk controls (INV-T04/T05) |
| Apply path (spend-related intents) | **Backing required** — CapabilityGrant/Revoke that assert spend ceilings MUST bind a treasury allocation (or explicit unfunded lab waiver) |
| Mutation of PROTO-0 | **Only via Apply** — Treasury never writes protocol stores (INV-T03) |
| Mutation of journals | **Only via Treasury engine** — Apply never posts books |

**Recommended model:** **Hybrid D+B** — Option D (independent planes + runtime intersection) as continuous authority, plus Option B (Apply-time treasury backing) when projecting spend into PROTO-0.

**STOP:** No routes, no Apply flip, no protocol edits, no treasury write changes in Phase 23.

---

## 1. Authority Model

### 1.1 Ownership

| Concern | Owner | Must not |
|---------|-------|----------|
| Assets / title | **Organisation** (via Treasury accounts) | Agents hold title |
| Financial allocation state | **Treasury** (`aether-treasury`) | CP invent balances |
| Human governance / approvals | **Control Plane** | Browser hold PROTO-0 keys |
| Protocol mutation *requests* | **Apply pipeline** | Bypass G1–G14 |
| Protocol authority (identity, capability, freeze) | **PROTO-0** | Treasury/CP write protocol stores |
| Economic contracts | **PROTO-2 / PROTO-4** | Treat journal as escrow/settlement finality |

### 1.2 No-bypass invariants (sync-specific)

| ID | Invariant |
|----|-----------|
| **INV-S01** | Treasury **MUST NOT** grant/revoke/freeze PROTO-0 capabilities. |
| **INV-S02** | Apply **MUST NOT** post, reverse, or invent treasury journal lines. |
| **INV-S03** | Agent-initiated spend **MUST** fail closed unless capability ∧ allocation ∧ applicable policy all pass (INV-T04). |
| **INV-S04** | When PROTO-0 `max_spend` and allocation remaining disagree, the **minimum** binds (INV-T05). |
| **INV-S05** | Sync jobs **MUST NOT** set `apply_enabled=true` or bypass Apply gates. |
| **INV-S06** | Cached “effective spend limit” in CP is a **hint**; live decision re-reads Treasury + PROTO-0 (or attested snapshots with TTL). |
| **INV-S07** | Lab may maintain allocations without PROTO-0 mutation (INV-T08); **production enforced spend** requires Apply path for capability projection **or** an explicitly approved alternative — never a silent Treasury→protocol write. |
| **INV-S08** | Sync correlation **MUST** carry `request_id`, `allocation_id`, `capability_id` / intent, `approval_id`, `organisation_id`. |

### 1.3 Authority verification

| Claim | Status |
|-------|--------|
| No layer bypasses another | **VALIDATED (design)** — Hybrid D+B preserves plane separation |
| Organisation owns assets | **VALIDATED** (INV-T01) |
| PROTO-0 sole protocol authority | **VALIDATED** (INV-T03 / INV-S01) |
| Treasury sole accounting authority | **VALIDATED** (INV-S02) |

---

## 2. Sync Model Options

| Option | Description | Pros | Cons | Verdict |
|--------|-------------|------|------|---------|
| **A** — Allocation creates capability constraints | Treasury write auto-drives PROTO-0 | Strong funding link | Violates INV-T03 unless Apply-shaped; couples ledgers to protocol | **Reject** as automatic path |
| **B** — Capability approval requires treasury backing | Apply prepare/execute checks allocation | Prevents unfunded grants | Needs Apply on for projection; doesn't alone stop runtime overspend if grants pre-exist | **Adopt for Apply-time** |
| **C** — Bidirectional reconciliation | Periodic heal both ways | Detects drift | Dangerous auto-heal; race-prone; may invent authority | **Reject as primary**; **allow read-only drift reports** |
| **D** — Independent planes + runtime intersection | Each plane authoritative in domain; spend checks both | Matches INV-T04/T05; works Apply-off | Drift visible until reconciled | **Adopt for runtime** |

### 2.1 Recommendation: Hybrid D+B

```text
                    ┌─────────────────────┐
                    │  Runtime spend gate │
                    │  D: intersection    │
                    └─────────▲───────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
   PROTO-0 capability   Treasury allocation   CP policy
         │                    │
         │         ┌──────────┴──────────┐
         │         │ Apply path (B)      │
         │         │ Grant/Revoke must   │
         └────────►│ bind allocation_id  │
                   │ (when spend-scoped) │
                   └─────────────────────┘
```

| Mode | When | Behaviour |
|------|------|-----------|
| **Observation / lab** | Apply off | Allocations + capabilities independent; spend gate still intersects if spend path exists in product |
| **Enforced production** | After sync impl + Apply enablement for grant intents | CapabilityGrant (spend) requires active allocation; runtime still intersects |

**Not recommended:** Option A auto-sync; Option C auto-heal that mutates either plane without human/Apply governance.

---

## 3. Runtime Decision Model

### 3.1 Decision flow (agent action / spend intent)

```text
Agent action request
  → (1) Authenticate agent / session
  → (2) Policy allowed?          else DENY
  → (3) Capability granted?      else DENY
  → (4) Treasury allocation active & sufficient remaining?
                                  else DENY (or ESCALATE if policy says)
  → (5) Budget / treasury not frozen?
                                  else DENY
  → (6) Approval / attestation valid (if required)?
                                  else DENY / ESCALATE
  → (7) Risk controls (velocity, allowlist, MFA step-up)?
                                  else DENY / ESCALATE
  → ALLOW (then reserve / proceed under separate economic protocols)
```

### 3.2 Outcomes

| Outcome | Meaning |
|---------|---------|
| **ALLOW** | All mandatory checks passed; may proceed to reservation / downstream protocol |
| **DENY** | Hard fail-closed; audit reason codes |
| **ESCALATE** | Human dual-control or break-glass path (policy-configured); not silent allow |

### 3.3 Effective limit

```text
effective_max = min(
  capability.max_spend (if present),
  allocation.remaining_minor,
  policy.ceiling (if present),
  risk.dynamic_cap (if present)
)
```

---

## 4. Allocation ↔ Capability Lifecycle

| Event | Treasury | PROTO-0 (via Apply only) | Runtime |
|-------|----------|---------------------------|---------|
| **Allocation create** | Ceiling + optional reserved books | No automatic grant | Spend still needs capability |
| **Allocation increase** | Raise ceiling / remaining | Optional Apply to raise `max_spend` if projecting | Intersection uses min |
| **Allocation decrease** | Lower ceiling (≥ committed) | Optional Apply to lower/revoke excess capability | Prefer decrease allocation first or simultaneous Apply |
| **Capability grant (spend)** | Must reference `allocation_id` (mode B) | Apply CapabilityGrant | After success, both required |
| **Capability revoke** | Allocation may remain (funding recovery separate) | Apply CapabilityRevoke | Spend DENY |
| **Capability reduction** | May precede or follow allocation decrease | Apply with lower max_spend | min() binds |
| **Agent / identity freeze** | Optional treasury freeze (ops) | Apply FreezeIdentity | Both DENY spend |
| **Budget exhaustion** | remaining → 0; new reserves fail | Capability may still exist | DENY at (4) |
| **Emergency stop** | Admin freeze treasury + dual-control unfreeze | Apply freeze/revoke | Fail closed |
| **Allocation expire** | Status expired; releases per policy | Stale capability → DENY at intersection; Apply revoke recommended | DENY |

**Ordering preference for reductions:** reduce/freeze treasury allocation (stops funding) **and** Apply revoke/reduce capability (stops protocol authority). Either alone is insufficient for production narrative.

---

## 5. Failure Scenarios (design responses)

| Scenario | Response |
|----------|----------|
| Capability exists, no allocation | Runtime **DENY**; drift report; optional ESCALATE to fund or revoke |
| Allocation exists, capability missing | Runtime **DENY**; operator may Apply grant with backing (B) |
| Budget exhausted mid-operation | Reservation/settle fail closed; no overdraft; compensating release if held |
| Apply succeeds, accounting fails | **No automatic journal invent**; compensating ops; capability may be revoked via new Apply; audit gap alert |
| Accounting succeeds, Apply fails | Allocation remains; no protocol authority; safe fail; retry Apply or reduce allocation |
| Protocol unavailable | Fail closed on spend; treasury observation/mutations may continue (ledger-only) |
| Treasury unavailable | Fail closed on spend and on Apply intents that require backing check |

**Forbidden:** Compensating by writing the other plane’s store directly.

---

## 6. Binding record (future implementation shape)

When sync is implemented, spend-scoped Apply intents carry:

| Field | Role |
|-------|------|
| `allocation_id` | Required for CapabilityGrant with spend semantics |
| `organisation_id` | Tenancy |
| `asset_id` | Must match allocation asset |
| `requested_ceiling_minor` | Must be ≤ allocation remaining/ceiling policy |
| `execution_hash` | Existing Apply binding |
| `sync_mode` | `backed` \| `lab_unfunded_waiver` (waiver admin-only, audited) |

---

## 7. Compliance position

| Question | Position |
|----------|----------|
| Custody exposure? | **No** — sync does not hold customer keys or bank accounts |
| Money transmission? | **No** — still internal ledger + protocol authority; no rails |
| Financial reporting? | Org books remain treasury journal; sync does not create payment institution duties by itself |
| External money movement? | **Forbidden** in this design |

Mis-marketing funded capabilities as “money in agent wallet” remains a **comms/compliance risk**, not a new legal product class by sync alone.

---

## 8. Sequencing vs Apply enablement

| Question | Answer |
|----------|--------|
| Enable Apply **before** sync impl? | **Only** for non-spend intents (e.g. FreezeIdentity) if final Apply gate passes — **not** for production CapabilityGrant that claims funded spend |
| Enable Apply **after** sync? | **Required** before marketing “funded agent spend” or enforced production spend control |
| Implement sync while Apply off? | **Yes** — spend-gate library + Apply-time backing checks can ship behind `apply_enabled=false` |

**Normative sequencing for CapabilityGrant (spend):**

1. This design gate (Phase 23) — **done when PASS**  
2. Sync implementation phase (spend gate + Apply binding) — separate approval  
3. Apply final enablement gate — may proceed for freeze-only earlier; **full spend grants after step 2**

---

## 9. Freeze statement

Hybrid D+B, INV-S01…S08, runtime ALLOW/DENY/ESCALATE, and sequencing rules are **frozen**. Implementation requires a separate phase. Option A automatic Treasury→PROTO-0 writes remain **forbidden**.

**This document does not authorise implementation.**
