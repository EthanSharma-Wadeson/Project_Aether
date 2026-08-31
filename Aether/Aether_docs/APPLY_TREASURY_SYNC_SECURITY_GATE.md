# Apply ↔ Treasury Sync Security Gate — Phase 23 Verdict

| Field | Value |
|-------|--------|
| **Document** | `APPLY_TREASURY_SYNC_SECURITY_GATE.md` |
| **Phase** | 23 — Apply ↔ Treasury Allocation Sync Design Gate |
| **Date** | 2026-08-01 |
| **Scope** | Design & security review of capability ↔ allocation relationship |
| **Non-goals** | Code, routes, Apply enablement, protocol mutations, treasury write changes, accounting-model changes |

**Pack:**

- [APPLY_TREASURY_SYNC_ARCHITECTURE.md](APPLY_TREASURY_SYNC_ARCHITECTURE.md)
- [APPLY_TREASURY_SYNC_THREAT_MODEL.md](APPLY_TREASURY_SYNC_THREAT_MODEL.md)

---

## Executive Summary

Capability authority (PROTO-0 via Apply) and funding authority (Treasury allocations) must remain **separate planes** joined by **runtime intersection** and, for spend-scoped grants, **Apply-time treasury backing**.

This prevents over-permissioned agents and unfunded capabilities **without** giving Treasury a protocol write path or giving Apply a journal write path.

### Gate decision

# **PASS WITH CONDITIONS**

| Option | Result |
|--------|--------|
| PASS | — |
| **PASS WITH CONDITIONS** | **Selected** |
| FAIL | — |

**Meaning:** Sync architecture is sound and consistent with INV-T01…T10 and Apply security. **Implementation is not authorised** until a separate phase closes the conditions below. **Apply enablement is not authorised by this gate.**

---

## 1. Recommended sync architecture

**Hybrid D+B**

| Layer | Model |
|-------|-------|
| **Runtime** | Option **D** — independent planes; spend ALLOW only if capability ∧ allocation ∧ policy ∧ risk |
| **Apply (spend intents)** | Option **B** — CapabilityGrant/Revoke affecting spend MUST bind active `allocation_id` (unless audited lab waiver) |
| **Reconciliation** | Read-only drift reports only — **not** Option C auto-heal |
| **Forbidden** | Option **A** automatic Treasury→PROTO-0 writes |

---

## 2. Security invariants (must hold in any impl)

| ID | Invariant |
|----|-----------|
| INV-S01 | Treasury never mutates PROTO-0 |
| INV-S02 | Apply never posts treasury journal |
| INV-S03 | Spend fail-closed on missing capability or allocation |
| INV-S04 | Numeric disagreement → minimum binds |
| INV-S05 | Sync never flips `apply_enabled` |
| INV-S06 | Caches are hints; live authority re-checked |
| INV-S07 | Production enforced spend needs Apply projection path — no silent protocol writes |
| INV-S08 | Full correlation ids on sync-related decisions |

---

## 3. Compliance position

| Exposure | Created by sync design? |
|----------|-------------------------|
| Custody | **No** |
| Money transmission / banking | **No** |
| External money movement | **No** (still forbidden) |
| Reporting | Org journal remains source for books; no new payment-institution claim |

---

## 4. Blocking issues before implementation

| ID | Blocker | Severity |
|----|---------|----------|
| **S-C1** | Separate **implementation approval** for sync/spend-gate phase | Blocking |
| **S-C2** | Specify spend-gate API placement (library in CP vs agent runtime) without PROTO RFC unless needed | Blocking |
| **S-C3** | Define which Apply intents are “spend-scoped” vs freeze-only | Blocking |
| **S-C4** | Production ban on `lab_unfunded_waiver` | Blocking for prod |
| **S-C5** | Concurrency / TOCTOU test plan (TR-SYNC-006) | Blocking |
| **S-C6** | Drift report UX/API is observation-only | Blocking |
| **S-C7** | Doc amend: no marketing “funded agents” until sync+Apply spend path live | Comms |

Non-blocking residuals: hash-chained audit, HSM, multi-tenant — orthogonal.

---

## 5. Apply enablement sequencing

| Question | Decision |
|----------|----------|
| Should Apply enablement occur **before** sync implementation? | **Not for spend CapabilityGrant/Revoke** in production. FreezeIdentity (and similar non-spend intents) **may** proceed through the separate Apply final enablement gate without sync. |
| Should Apply enablement occur **after** sync implementation? | **Yes** before claiming enforced funded spend or enabling production CapabilityGrant that asserts ceilings. |
| Can sync impl land while Apply is off? | **Yes** — preferred: build spend-gate + backing checks behind `apply_enabled=false`. |

```text
Phase 23 (this gate) ──PASS WITH CONDITIONS──►
    Sync / spend-gate implementation (separate approval)
         │
         ├─► Apply final enablement (freeze-only)     [optional parallel]
         └─► Apply enablement for spend grants        [after sync impl]
```

---

## 6. Conditions checklist

| Criterion | Met? |
|-----------|------|
| Authority model reviewed | **Yes** |
| Sync model recommended | **Yes** (Hybrid D+B) |
| Runtime ALLOW/DENY/ESCALATE defined | **Yes** |
| Lifecycle mapping defined | **Yes** |
| Failure scenarios documented | **Yes** |
| Threat model TR-SYNC-* complete | **Yes** |
| Compliance position stated | **Yes** |
| No code / routes / Apply flip | **Yes** |

---

## 7. Mandatory actions before next implementation phase

1. Explicit approval for **Apply↔Treasury sync / spend-gate implementation**.  
2. Lock intent taxonomy (spend-scoped vs not).  
3. Keep Apply hard-disabled until its own enablement gate; do not bundle enablement into sync coding.  
4. Do not implement Option A or auto-heal Option C.  
5. Do not modify `aether-core` unless a future protocol RFC is separately approved (not required for Hybrid D+B).

---

## 8. Verdict statement

```text
GATE: APPLY ↔ TREASURY ALLOCATION SYNC (Phase 23)
DECISION: PASS WITH CONDITIONS
ARCHITECTURE: Hybrid D+B (runtime intersection + Apply-time backing)
APPLY ENABLEMENT: NOT authorised by this gate
  — freeze-only may use separate Apply final gate
  — spend grants after sync implementation
PROTOCOL / TREASURY ACCOUNTING: unchanged
IMPLEMENTATION: NOT authorised
```

**STOP.** No implementation authorised.
