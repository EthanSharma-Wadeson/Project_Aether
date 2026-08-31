# Apply ↔ Treasury Sync Threat Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `APPLY_TREASURY_SYNC_THREAT_MODEL.md` |
| **Phase** | 23 — Apply ↔ Treasury Allocation Sync Design Gate |
| **Status** | **DESIGN FROZEN** — analysis only; no implementation authorised |
| **Date** | 2026-08-01 |
| **Related** | [APPLY_TREASURY_SYNC_ARCHITECTURE.md](APPLY_TREASURY_SYNC_ARCHITECTURE.md), [TREASURY_THREAT_MODEL.md](TREASURY_THREAT_MODEL.md), [TREASURY_WRITE_THREAT_MODEL.md](TREASURY_WRITE_THREAT_MODEL.md) |

---

## Executive Summary

Sync between PROTO-0 capabilities and Treasury allocations introduces **drift**, **race**, and **confused-deputy** risks. The Hybrid D+B model mitigates by never letting Treasury write protocol state, never letting Apply write journals, and enforcing intersection at spend-time plus backing checks at Apply-time.

Statuses: **MITIGATED (design)** · **OPEN** · **ACCEPTED** · **OUT OF SCOPE**

---

## 1. Assets

| Asset | Abuse impact |
|-------|--------------|
| PROTO-0 capability grants | Unfunded or unbounded agent authority |
| Treasury allocations / remaining | Overspend / liquidity lock |
| Apply approvals / execution_hash | Authorised but wrong-plane mutation |
| Effective-limit cache | Stale allow decisions |
| Sync correlation ids | Broken forensics |

---

## 2. Threat catalogue (TR-SYNC-*)

Likelihood / Impact: L / M / H / C

### TR-SYNC-001 — Capability / funding mismatch

| Field | Value |
|-------|--------|
| **Threat** | Agent holds capability without allocation (or reverse); operators believe spend is funded |
| **L** | H (today with Apply off + allocations) |
| **I** | H |
| **Mitigation** | Runtime intersection (D); Apply-time backing (B); drift reports (read-only C) |
| **Residual** | Lab intentional unfunded capabilities until enforced mode |
| **Status** | **MITIGATED (design)**; **OPEN** until spend-gate impl |

### TR-SYNC-002 — Replay causing double spend

| Field | Value |
|-------|--------|
| **Threat** | Replayed Apply grant or replayed reserve/settle doubles authority or books |
| **L** | M |
| **I** | C |
| **Mitigation** | Apply replay reserve; treasury idempotency keys; single-consume approvals; settlement binding uniqueness |
| **Residual** | Cross-plane replay of *paired* operations if correlation weak |
| **Status** | **MITIGATED (design)** — require shared `request_id` / idempotency scope in impl |

### TR-SYNC-003 — Stale allocation cache

| Field | Value |
|-------|--------|
| **Threat** | CP caches remaining balance; allows spend after exhaustion/freeze |
| **L** | M |
| **I** | H |
| **Mitigation** | INV-S06 — cache hint only; live read or short TTL attested snapshot; treasury wins on conflict |
| **Residual** | Clock skew on TTL snapshots |
| **Status** | **MITIGATED (design)** |

### TR-SYNC-004 — Privilege escalation through sync

| Field | Value |
|-------|--------|
| **Threat** | Sync service uses superuser path to raise capability or mint allocation |
| **L** | L |
| **I** | C |
| **Mitigation** | No sync writer to PROTO-0 except Apply; no sync writer to journal except engine APIs; RBAC on any future sync admin tools |
| **Residual** | Host compromise |
| **Status** | **MITIGATED (design)** |

### TR-SYNC-005 — Compromised operator

| Field | Value |
|-------|--------|
| **Threat** | Operator approves unfunded CapabilityGrant or inflates allocation then grants |
| **L** | M |
| **I** | H |
| **Mitigation** | Dual-control on treasury H ops + Apply approvals; MFA boundary; SoD requester ≠ approver |
| **Residual** | Colluding admins |
| **Status** | **MITIGATED (design)** / **ACCEPTED** collusion |

### TR-SYNC-006 — Race conditions

| Field | Value |
|-------|--------|
| **Threat** | Concurrent Apply grant vs allocation decrease; concurrent reserve vs revoke |
| **L** | M |
| **I** | H |
| **Mitigation** | Spend-time re-check; engine serialisation on reserve; Apply re-simulation; fail closed on conflict |
| **Residual** | TOCTOU windows — minimise with single decision transaction where possible |
| **Status** | **MITIGATED (design)**; **OPEN** until concurrency tests in sync impl |

### TR-SYNC-007 — Auto-heal bidirectional sync (related to Option C)

| Field | Value |
|-------|--------|
| **Threat** | Reconciler silently grants capabilities or posts journals to “fix” drift |
| **L** | L if forbidden |
| **I** | C |
| **Mitigation** | Forbid auto-heal mutations; drift = report + human/Apply/treasury governed fix |
| **Status** | **MITIGATED (design)** — Option C primary **rejected** |

### TR-SYNC-008 — Apply succeeds / treasury fails (split brain)

| Field | Value |
|-------|--------|
| **Threat** | Capability live without books updated (or opposite) |
| **L** | M after enablement |
| **I** | H |
| **Mitigation** | Ordering: prefer allocation ready **before** Apply grant; on Apply success + later accounting need, use compensating Apply revoke; never invent journal |
| **Status** | **MITIGATED (design)** |

### TR-SYNC-009 — Lab waiver abuse

| Field | Value |
|-------|--------|
| **Threat** | `lab_unfunded_waiver` used in production |
| **L** | M |
| **I** | H |
| **Mitigation** | Waiver flag forbidden when `is_production_mode`; audit; admin-only |
| **Status** | **MITIGATED (design)** |

---

## 3. Crosswalk

| Prior ID | Sync treatment |
|----------|----------------|
| TR-THR-050/051 | Covered by TR-SYNC-001 + intersection |
| TR-THR-053 | Reduced when B+D enforced; ACCEPTED while Apply off |
| TR-TW-003 | Allocation escalation still dual-control |
| Apply G1–G14 | Unchanged; sync adds preconditions, does not remove gates |

---

## 4. Residual risks (accepted for design freeze)

1. Apply-off lab will show capability/allocation drift until operators align manually.  
2. Colluding admins can still pass SoD on both planes.  
3. Host/DBA can edit SQLite outside API.  
4. Full TOCTOU elimination needs impl-phase transaction design.

---

## 5. Implementation gate requirements (future)

Before sync implementation ships:

- [ ] Spend-gate tests for TR-SYNC-001/003/006  
- [ ] Apply prepare rejects spend grant without allocation (non-waiver)  
- [ ] Production forbids lab waiver  
- [ ] No Treasury→PROTO-0 writer exists  
- [ ] Drift report is read-only  

---

## 6. Freeze statement

Threat IDs **TR-SYNC-001…009** are frozen for tracking. Status updates require version notes.

**No implementation is authorised by this document.**
