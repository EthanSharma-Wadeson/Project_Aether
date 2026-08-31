# Treasury Write Security Gate — Phase 20 Verdict

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_WRITE_SECURITY_GATE.md` |
| **Phase** | 20 — Treasury Write Security Gate |
| **Date** | 2026-08-01 |
| **Scope** | Design & security review of future CP → `aether-treasury` mutations |
| **Non-goals** | Implementation, Apply enablement, protocol changes, custody/rails, accounting-model changes |

**Pack:**

- [TREASURY_WRITE_SECURITY_MODEL.md](TREASURY_WRITE_SECURITY_MODEL.md)
- [TREASURY_WRITE_THREAT_MODEL.md](TREASURY_WRITE_THREAT_MODEL.md)
- [TREASURY_WRITE_API_SPEC.md](TREASURY_WRITE_API_SPEC.md)

---

## Executive Summary

Phase 19 delivered a **read-only** treasury observatory. Phase 20 freezes how **writes** may later appear: organisation-owned funds, engine-owned journal truth, Control Plane governance (JWT, RBAC, CSRF, dual-control, audit), agents never mutating treasury.

The designed write surface is **narrow and labelled**: internal book funding, allocations, reservations, settlement *posts*, refunds/chargebacks/adjustments, freeze/unfreeze — **without** payment rails, custody, or Apply↔capability sync.

### Gate decision

# **PASS WITH CONDITIONS**

| Option | Result |
|--------|--------|
| PASS | — |
| **PASS WITH CONDITIONS** | **Selected** |
| FAIL | — |

**Meaning:** Design is coherent with prior freezes (Phases 15–19) and is **safe to implement under a separate Phase 21 approval**, provided the mandatory conditions below are met. **Phase 20 does not authorise mutation code.**

---

## 1. Architecture decisions (frozen)

| ID | Decision |
|----|----------|
| **AD-W1** | Writes flow **Console → CP HTTP → Treasury Adapter → `aether-treasury` only**. |
| **AD-W2** | Backend roles remain `admin` \| `operator` \| `viewer`; personas map as today. |
| **AD-W3** | High-risk ops use **request → approve → execute** with **requester ≠ approver**. |
| **AD-W4** | Low-risk ops (reserve / release / settlement_post) are **single-shot** with CSRF consume + idempotency. |
| **AD-W5** | Emergency **freeze** is admin solo; **unfreeze** is dual-control. |
| **AD-W6** | Phase 21 funding is **`internal_acknowledgment` only** (INV-TW14). |
| **AD-W7** | Settlement_post updates **treasury journal only** — never PROTO-4. |
| **AD-W8** | Apply stays **disabled**; no sync endpoints. |
| **AD-W9** | Tenancy remains deployment-bound org id (Phase 19); cross-org → 404. |
| **AD-W10** | Engine transaction is the unit of accounting atomicity; CP audit is correlated after success (gap ADR). |

---

## 2. Allowed future write surface (Phase 21 candidate)

| Allowed | Disallowed (still) |
|---------|-------------------|
| Fund (internal acknowledgment) | Bank/crypto deposit confirmation |
| Allocation create / increase / reduce / close / expire | Agent-owned wallets |
| Reserve / escrow_reserve / release | Payment initiation / payouts |
| Settlement journal post | PROTO-0/2/4 mutations; Apply execute |
| Refund / chargeback / adjustment | Custody / ERP adapters |
| Freeze; dual-control unfreeze | Multi-tenant org switching |
| Treasury node create/close; asset register (lab) | Anything setting `apply_enabled=true` |
| Dual-control force-release (stuck) | UI → SQLite direct access |

---

## 3. Authority model — validation

| Claim | Status |
|-------|--------|
| Org owns funds; agents never own | **VALIDATED** (INV-TW01) |
| Treasury owns journal truth | **VALIDATED** (INV-TW02) |
| CP authenticates & governs only | **VALIDATED** (INV-TW03–05, 17) |
| No PROTO bypass via treasury writes | **VALIDATED** (INV-TW05, AD-W8) |
| Dual-control for high-risk | **VALIDATED** (INV-TW08, AD-W3) |
| Idempotent, auditable mutations | **VALIDATED** (INV-TW09–10) |

Invariants **INV-TW01…TW17** are accepted as the write-path authority freeze.

---

## 4. Security findings

| ID | Finding | Severity | Disposition |
|----|---------|----------|-------------|
| **SF-W1** | Exposing fund/adjust without SoD would enable insider minting | Critical | Dual-control mandatory on H ops |
| **SF-W2** | CSRF gaps (observed historically in console sessions) would allow browser-driven fraud | High | CSRF consume on execute + L mutations; tests required |
| **SF-W3** | Stale JWTs missing `iss` yield noisy 401s / confused ops | Medium | Existing claim verification + session clear |
| **SF-W4** | Engine success / audit failure creates forensic holes | High | `audit_gap` event + reconcile by `request_id` (ADR) |
| **SF-W5** | Lab funding confused with real money movement | High | `funding_kind` enum + console copy INV-TW14 |
| **SF-W6** | Viewer/Auditor role conflation (Phase 18 C2) | Low for writes | Viewers cannot write; balance masking still deferred |
| **SF-W7** | No MFA / HSM in lab CP | Medium | Accepted for lab; **condition** before production fiat narrative |
| **SF-W8** | Allocation increase may need engine ADR | Medium | Phase 21 may extend `aether-treasury` helpers **without** changing accounting principles |

---

## 5. Threat findings

| ID | Summary | Gate treatment |
|----|---------|----------------|
| **TR-TW-001…004** | Unauthorised / mislabelled funding | Mitigated in design |
| **TR-TW-010…015** | Replay / duplicate / races | Mitigated; refund race tests mandatory in Phase 21 |
| **TR-TW-020…022** | Tenant / CSRF / viewer | Mitigated |
| **TR-TW-030** | Colluding admins | **ACCEPTED** residual |
| **TR-TW-032** | Freeze storm / fleet DoS | **OPEN** — accept or add fleet dual-control ADR |
| **TR-TW-033 / 043** | Host audit/DB tampering | **ACCEPTED** host trust |
| **TR-TW-060** | Stuck fake reservations (TR-THR-003) | **OPEN** — **mandatory** sweeper ADR before/with Phase 21 |
| **TR-TW-070…072** | Custody / Apply sync / rails | Out of scope |

---

## 6. Failure model (normative summary)

| Scenario | Required behaviour |
|----------|-------------------|
| DB failure inside engine txn | Full rollback; HTTP 503; no success audit claiming post |
| Partial “CP said OK” without batch id | Forbidden — response must carry engine `journal_batch_id` or fail |
| Duplicate request (same idempotency key) | Return original success (`duplicate: true`) |
| Approval expiry | 409 `APPROVAL_EXPIRED`; no engine call |
| Stale / frozen / expired allocation | 409 mapped codes; no post |
| Journal imbalance | Engine rejects; never commit |
| Adapter failure after commit | Emit `treasury.mutation.audit_gap`; operator reconstructs from journal |
| Operator cancellation | Pending → cancelled; execute rejected |
| SoD violation | 403; `treasury.mutation.denied` |

---

## 7. Audit model — gate check

Reconstruction path **VALIDATED** at design level:

```text
CP audit (request_id, actor, approval_id, outcome)
        ↔
mutation store (payload_hash, status)
        ↔
journal batch (batch_id, event_type, lines, idempotency_key)
```

Phase 21 **MUST** ship a test: fund → approve → execute → GET journal by `request_id` / `batch_id` round-trip.

---

## 8. Conditions (must close for Phase 21)

| ID | Condition | Blocking? |
|----|-----------|-----------|
| **W-C1** | Separate **implementation approval** for Phase 21 explicitly citing this gate | **Yes** |
| **W-C2** | Dual-control persistence design (new `treasury_mutations` tables **or** reuse CP approval store with `purpose=treasury`) documented in Phase 21 ADR before coding routes | **Yes** |
| **W-C3** | **TR-TW-060** reservation TTL / sweeper (or accepted risk record + force-release only) | **Yes** |
| **W-C4** | Automated tests: SoD, CSRF consume, idempotency, IDOR/404 tenancy, freeze, insufficient funds, duplicate key | **Yes** |
| **W-C5** | Console + API copy enforce `internal_acknowledgment`; reject custody-shaped funding kinds | **Yes** |
| **W-C6** | Audit-after-success + `audit_gap` behaviour ADR | **Yes** |
| **W-C7** | Record decision on **TR-TW-032** fleet freeze dual-control | No (record required) |
| **W-C8** | MFA recommended for `admin` before any **production** deployment that treats books as financial records of record | Production only |
| **W-C9** | Legal **C3** (money-transmitter posture) before marketing live fiat custody — unchanged from Phase 16 | Before custody / production fiat claims |
| **W-C10** | Apply remains hard-disabled; no sync routes in Phase 21 | **Yes** |

---

## 9. Mandatory actions before Phase 21

1. Obtain **explicit Phase 21 implementation approval** referencing **PASS WITH CONDITIONS**.  
2. Publish short ADRs for: mutation/approval store (**W-C2**), audit gap (**W-C6**), reservation sweeper (**W-C3**).  
3. Implement **only** the allowed surface in [TREASURY_WRITE_API_SPEC.md](TREASURY_WRITE_API_SPEC.md).  
4. Do **not** enable Apply, touch `aether-core`, add rails/custody, or alter accounting principles.  
5. Extend engine only if needed for allocation increase/unfreeze **without** changing double-entry rules.  
6. Keep read observatory and Apply-disabled banners intact; add write UX behind feature flag if desired.  
7. Close W-C4 test plan evidence before declaring Phase 21 done.

---

## 10. What Phase 20 did / did not create

| Created | Not created |
|---------|-------------|
| Four design documents in `Aether_docs/` | Treasury write routes |
| INV-TW*, TR-TW*, API shapes, gate verdict | Funding/allocation/reservation mutation code |
| Dual-control & failure rules | Console write UI |
| | Apply integration / protocol changes |
| | Rust / TypeScript implementation |

---

## 11. Gate checklist

| Criterion | Met? |
|-----------|------|
| Write authority model frozen | **Yes** |
| Threat model complete | **Yes** (OPEN items tracked) |
| API surface defined | **Yes** |
| Dual-control rules defined | **Yes** |
| Audit requirements defined | **Yes** |
| Failure behaviour documented | **Yes** |
| No mutation code created | **Yes** |
| Apply remains disabled | **Yes** |

---

## 12. Verdict statement

```text
GATE: TREASURY WRITE SECURITY (Phase 20)
DECISION: PASS WITH CONDITIONS
NEXT: Phase 21 implementation requires separate approval
APPLY: remains disabled
PROTOCOL: unchanged
ACCOUNTING MODEL: unchanged
```

**STOP.** Implementation requires separate approval.
