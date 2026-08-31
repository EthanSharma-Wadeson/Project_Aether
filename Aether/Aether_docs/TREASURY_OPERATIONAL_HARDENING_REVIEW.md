# Treasury Operational Hardening Review — Phase 22

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_OPERATIONAL_HARDENING_REVIEW.md` |
| **Phase** | 22 — Treasury Operational Hardening Gate |
| **Status** | **REVIEW ONLY** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Inputs** | Phase 19–21 treasury pack; [TREASURY_WRITE_IMPLEMENTATION.md](TREASURY_WRITE_IMPLEMENTATION.md); [TREASURY_WRITE_THREAT_MODEL.md](TREASURY_WRITE_THREAT_MODEL.md); [V1_RELEASE_CERTIFICATION.md](V1_RELEASE_CERTIFICATION.md) |
| **Protocol impact** | **None** |
| **Apply impact** | **None** — remains disabled |
| **Code impact** | **None** — design / ops / security review only |

---

## Executive Summary

Phase 21 shipped the first **internal ledger governance** mutation surface (allocations, reservations, settlement posts, adjustments, freeze/unfreeze) behind JWT, RBAC, CSRF, SoD, and idempotency. Accounting truth remains in `aether-treasury` (append-only journal). There is still **no** external money movement, custody, Apply sync, or PROTO-0 coupling.

This phase asks whether that surface is **safe under failure, concurrency, recovery, and operational stress** for a future enterprise pilot.

| Area | Verdict |
|------|---------|
| Mutation security (authZ / SoD / CSRF) | **Adequate for lab** — Phase 21 tests cover core SoD/CSRF/idempotency |
| Reservation lifecycle | **Incomplete** — TTL field exists; **no sweeper**; TR-TW-060 still open in practice |
| Freeze model | **Acceptable with conditions** — emergency solo freeze OK; fleet-wide needs dual-control design |
| Audit integrity | **Reconstructable, not tamper-evident** — hash chain designed only |
| Ops readiness (backup / metrics / runbooks) | **Not enterprise-ready** — same OPEN residuals as V1 cert |
| Enterprise pilot | **NOT READY** until mandatory conditions below close |

### Gate decision

# **PASS WITH CONDITIONS**

| Option | Result |
|--------|--------|
| PASS | — |
| **PASS WITH CONDITIONS** | **Selected** |
| FAIL | — |

**Meaning:** Architecture and Phase 21 controls are coherent enough that **operational hardening may proceed under a separate implementation phase**. This gate does **not** authorise Apply, PROTO-0 spend, rails, or production fiat claims. Enterprise pilot remains blocked until reservation sweeper, backup/restore runbooks, and minimum observability land.

**STOP:** No code in Phase 22.

---

## 1. Reservation Lifecycle Review

### 1.1 Current behaviour (as implemented)

| Stage | Behaviour today |
|-------|-----------------|
| **Create** | `reserve` / `escrow_reserve` via single-shot CP write; engine locks treasury row; idempotent by key; decreases allocation `remaining_minor` |
| **Expiry field** | Optional `expires_at` stored on reservation |
| **Expiry enforcement** | **None automatic** — expired-but-`active` rows remain holds |
| **Release** | Explicit `release_reservation` restores available + allocation remaining |
| **Settlement consume** | `settlement_post` marks reservation `consumed` + expense journal |
| **Abandoned / stuck** | Visible as `aged_active_reservations` in security metrics; **no auto-release** |
| **Force recovery** | Designed in Phase 20 (`force_release` dual-control); **not fully productised as dedicated sweeper** |

### 1.2 TTL policy (design freeze)

| Parameter | Design default | Notes |
|-----------|----------------|-------|
| `TW_RESERVATION_DEFAULT_TTL` | **15 minutes** for operator-created budget holds | Must be set on create when omitted (future impl) |
| `TW_RESERVATION_MAX_TTL` | **24 hours** | Hard reject above max |
| Escrow-linked holds | Align to escrow policy when PROTO-2 evidence exists; until then same max TTL | No protocol mutation |
| Allocation expiry | Existing `expire_allocation_if_needed` on reserve path | Does not release open reservations |

**Normative rule (design):** An `active` reservation with `expires_at ≤ now` is **logically expired** and **MUST NOT** be settlement-posted; sweeper or dual-control force-release MUST release it.

### 1.3 Sweeper behaviour (design — not implemented)

```text
Periodic job (CP or treasury worker):
  1. SELECT active reservations WHERE expires_at IS NOT NULL AND expires_at <= now()
     OR (expires_at IS NULL AND created_at <= now() - max_orphan_age)
  2. For each: engine.release_reservation(id, request_id=sweeper-*, idempotency_key=sweep:{id}:{expires_at})
  3. Append treasury_mutation_audit + CP audit: treasury.mutation.sweep_release
  4. Never settlement_post from sweeper
  5. Fail-closed on engine error; retry with same idempotency key
```

| Property | Requirement |
|----------|-------------|
| Schedule | Configurable; lab default every 60s |
| Concurrency | One sweeper lease per deployment (SQLite file lock / advisory) |
| Authority | System actor `treasury-sweeper` (not a human JWT); audited |
| SoD | N/A for TTL expiry; **force** release of non-expired still dual-control |

### 1.4 Stuck reservation handling

| Class | Detection | Handling |
|-------|-----------|----------|
| TTL expired, still active | Sweeper + security metrics | Auto-release |
| No TTL, age > 24h | Metrics + sweeper orphan policy | Auto-release **or** alert + dual-control force |
| Active during treasury freeze | Engine rejects new reserves; existing holds remain until release/settle | Ops: prefer release before unfreeze |
| Engine crash mid-reserve | SQLite txn rollback | No partial journal (foundation tests) |
| CP success / missing audit | `audit_gap` design (Phase 20 W-C6) | Reconcile by `request_id` / journal batch |

### 1.5 Manual recovery process (runbook outline)

1. Confirm via Console journal + reservations + security aged count.  
2. Prefer normal `release` (operator) if hold is known-safe.  
3. If disputed / blocked: dual-control **force_release** mutation with reason.  
4. Never delete journal lines; never edit `account_balances` by hand.  
5. Record incident id in approval reason + audit metadata.  
6. If balances diverge: `replay_balances` / integrity check (engine) before further writes.

### 1.6 TR-TW-060 resolution

| Field | Value |
|-------|--------|
| **Threat** | Fake / abandoned reservations block recovery and lock liquidity (TR-THR-003 / TR-TW-060) |
| **Status after Phase 22** | **DESIGN CLOSED** — TTL + sweeper + force-release + orphan policy specified |
| **Implementation status** | **OPEN** — requires separate hardening implementation phase |
| **Acceptance until impl** | Lab-only: operators manually release; aged metrics warn; **not** enterprise-pilot ready |

---

## 2. Freeze Model Review

### 2.1 Current controls

| Action | Actor | Control |
|--------|-------|---------|
| Freeze single treasury | Admin | Solo emergency (single-shot) |
| Unfreeze | Operator/Admin request → Admin approve ≠ requester → Admin execute | Dual-control |
| Effect | Engine rejects fund/allocate/reserve on frozen node | Journal not wiped |

### 2.2 Decisions (frozen for ops design)

| Question | Decision |
|----------|----------|
| Emergency freeze remain single actor? | **Yes** — blast-radius stop must not wait for second human |
| Permanent / long-lived freeze require approval? | **Yes for retention beyond `TW_FREEZE_SOLO_MAX` (design: 24h)** — after that, dual-control **confirm_freeze** or auto-page security; unfreeze path already dual-control |
| Fleet-wide freeze (all treasuries / org) require dual-control? | **Yes** — closes TR-TW-032 / TR-THR-041; **not** exposed as solo admin API |

### 2.3 Freeze authority matrix

| Scope | Initiate | Approve | Execute | Notes |
|-------|----------|---------|---------|-------|
| Single treasury emergency freeze | Admin | — | Admin solo | Audited `treasury.mutation.freeze` |
| Single treasury freeze confirm (past 24h) | Admin A | Admin B ≠ A | Admin | Future op `freeze_confirm` |
| Fleet / org freeze | Admin A | Admin B ≠ A | Admin | Future op `fleet_freeze`; rate-limit |
| Unfreeze any scope | Operator or Admin | Admin ≠ requester | Admin | Existing Phase 21 path |
| Viewer | — | — | — | Read only |

### 2.4 Recovery workflow

1. Freeze → investigate (journal, mutation timeline, security metrics).  
2. Release or settle open reservations as appropriate (may require unfreeze first if engine blocks — **design note:** release/settle on frozen treasury should remain allowed for recovery; validate in hardening impl).  
3. Dual-control unfreeze with written reason.  
4. Post-incident: export audit package; rotate credentials if compromise suspected.

### 2.5 Audit requirements

Every freeze/unfreeze **MUST** record: `request_id`, actor, treasury_id, reason, prior status, new status, approval_id (unfreeze), timestamp. Fleet freeze **MUST** list affected treasury ids (or org scope token).

---

## 3. Audit Integrity Review

### 3.1 Current state

| Layer | Property |
|-------|----------|
| Treasury journal | Append-only; updates/deletes rejected (`ImmutabilityViolation`) |
| Balances | Projections of journal |
| CP `treasury_mutation_audit` | Append-only rows; reconstructible timeline API |
| CP `audit_log` | Parallel governance trail |
| Hash chain | **Not implemented** (by design this phase) |
| External attestation | **Not implemented** |

### 3.2 Future requirements (design only)

| Capability | Design target | Phase |
|------------|---------------|-------|
| **Hash chaining** | Per-batch `prev_hash` / `batch_hash` over canonical line bytes; org-scoped chain | Future integrity phase |
| **Tamper evidence** | Verify chain on startup / on-demand; alert on break | Same |
| **Export packages** | Signed zip: journal slice + mutation audit + CP audit by `request_id` range; include manifest hash | Ops hardening impl |
| **Audit verification** | CLI/API `POST /api/treasury/integrity/verify` (read-only) | Future |
| **WORM / external log** | Optional ship to SIEM; not required for lab | Enterprise |

**Phase 22 explicitly does not implement hash chaining.**

### 3.3 Reconstruction guarantee (lab bar)

Given `request_id` or `mutation_id`, operator can recover: requester, approver, payload hash, outcome, and `journal_batch_id` (when posted). Residual: CP audit gap if process dies after engine commit (see Failure Model).

---

## 4. Failure Model Review

| # | Failure scenario | Current mitigation | Residual risk | Required improvement |
|---|------------------|--------------------|---------------|----------------------|
| F1 | DB failure mid engine txn | SQLite rollback; HTTP error | Brief unavailable | Metrics + 503 alerting |
| F2 | Transaction rollback after validation | No journal partials (foundation tests) | — | Keep regression tests |
| F3 | Duplicate execution / retry | Idempotency keys + mutation store | Client key misuse | Document key scope; reject empty keys (done) |
| F4 | Network interrupt after engine commit, before client ACK | Client retries → duplicate success | Operator confusion | Return `duplicate: true`; runbook |
| F5 | Control Plane restart | CP SQLite durable; in-flight `executing` may stick | Stuck mutation status | Startup scan: `executing` → `failed` or reconcile vs journal (design) |
| F6 | Treasury engine / DB restart | WAL durability | File corruption | Backup/restore; integrity check |
| F7 | Approval expiry | Execute rejected; status failed | Orphan approved rows | Sweeper for expired approvals |
| F8 | Concurrent mutations | Engine treasury row lock; CP conditional updates | SQLite write contention | Load test; optional queue |
| F9 | Concurrent reservations | Engine serialisation; overdraw tests | High concurrency latency | Document limits |
| F10 | Concurrent settlement vs release | Status CAS on reservation | Rare race → conflict error | Explicit tests (gap) |
| F11 | Partial audit failure (engine OK, audit insert fails) | Designed `audit_gap`; not fully ADR-hardened | Forensic hole | Compensating event + alert (W-C6) |
| F12 | Freeze during open reservations | Holds remain | Liquidity stuck | Recovery allowlist for release on frozen |
| F13 | Host edits SQLite | None (trust host) | Silent fraud | File perms + hash chain later + backups |

---

## 5. Operational Readiness

### 5.1 Enterprise pilot checklist

| Capability | Status | Notes |
|------------|--------|-------|
| Backups (CP DB + treasury DB) | **OPEN** | Operator file copy only; no tested job |
| Restore testing | **OPEN** | No documented restore drill |
| Metrics | **Partial** | Security aged counts; no SLO/RED metrics pack |
| Alerting | **OPEN** | No alert rules for audit_gap, aged reservations, freeze storms |
| Runbooks | **Partial** | Outlines in this doc; not published ops pack |
| Incident response | **OPEN** | Need RACI + compromise playbook |
| Operator procedures | **Partial** | Console Actions UI exists; dual-control training needed |
| Deployment model | **Lab** | Single-node SQLite; not multi-instance HA |

### 5.2 Required before enterprise pilot (blocking)

1. Reservation TTL defaults + sweeper (TR-TW-060 impl).  
2. Backup + restore runbook with evidence of successful restore drill.  
3. Minimum metrics: mutation success/fail, aged reservations, freeze events, audit_gap count.  
4. Alert hooks (even if webhook/log-based) for aged reservations and audit_gap.  
5. Published freeze/unfreeze and stuck-reservation runbooks.  
6. MFA for `admin` in any deployment treating ledger as financial record of record.  

### 5.3 Explicitly not required for lab continuation

HSM, Postgres, multi-tenant, Apply enablement, custody adapters, hash-chained journal.

---

## 6. Security Review

| Topic | Assessment | Disposition |
|-------|------------|-------------|
| **MFA** | Not enforced in CP lab | **Required** before enterprise pilot (admin); recommended for operator |
| **HSM/KMS** | Signer still seed-hex; treasury writes don’t use PROTO-0 signer | Roadmap unchanged; **not** blocking ledger-only lab |
| **Signer security** | Apply signer irrelevant to treasury posts | Keep Apply disabled; no new signing surface for treasury |
| **Admin compromise** | Solo freeze + colluding dual-control residual | Session revoke, freeze, credential rotate; fleet freeze dual-control |
| **Database access** | Host DBA can edit files | OS ACLs; separate volumes; future hash chain |
| **Privilege escalation** | Server RBAC on write routes; viewer blocked | Keep regression tests |
| **Audit tampering** | Append-only app layer; no chain | Design in §3; ACCEPTED for lab |
| **Tenant isolation** | Deployment-bound org; 404 cross-org | Adequate until multi-tenant phase |

---

## 7. Testing Review

### 7.1 Covered (Phase 21 / foundation)

- RBAC / SoD / CSRF / idempotency / dual-control execute  
- Freeze / unfreeze dual-control  
- Concurrent approval  
- Cross-org 404  
- Engine: concurrent reservations, rollback on insufficient, journal immutability  

### 7.2 Missing tests (mandatory for hardening impl)

| Gap | Priority |
|-----|----------|
| Reservation expiry enforcement (reject settle on expired) | **P0** |
| Sweeper auto-release + idempotent re-run | **P0** |
| Crash recovery: CP restart with `executing` mutation | **P0** |
| Concurrent settlement vs release | **P0** |
| Freeze/unfreeze races with in-flight reserve | **P1** |
| Release allowed while frozen (recovery policy) | **P1** |
| Audit reconstruction golden test (request → journal) | **P1** (partial today) |
| Backup restore drill (scripted) | **P1** |
| Fleet freeze SoD (when API exists) | **P2** |
| Approval expiry sweeper | **P2** |

---

## 8. Remaining Risks

| ID | Risk | Severity | Treatment |
|----|------|----------|-----------|
| R-OP-01 | Abandoned reservations lock books (TR-TW-060) | High | Sweeper impl |
| R-OP-02 | No backup/restore evidence | High | Runbook + drill |
| R-OP-03 | Audit gap after commit | Medium | Compensating event + alert |
| R-OP-04 | Solo admin freeze storm / fleet DoS | Medium | Fleet dual-control; rate limits |
| R-OP-05 | No MFA on admin | Medium | Pilot prerequisite |
| R-OP-06 | SQLite single-writer under load | Medium | Document limits; Postgres later |
| R-OP-07 | Hash-chain absent | Medium | ACCEPTED lab; future integrity phase |
| R-OP-08 | Misread ledger posts as real money | High (comms) | Keep INV-TW14 labelling |
| R-OP-09 | Apply accidentally enabled later without sync | High | Keep Apply gate independent |

---

## 9. Conditions (must close before calling treasury “enterprise-ready”)

| ID | Condition | Blocks pilot? |
|----|-----------|---------------|
| **OH-C1** | Implement reservation TTL defaults + sweeper per §1 (closes TR-TW-060 impl) | **Yes** |
| **OH-C2** | Startup reconcile for stuck `executing` mutations vs journal | **Yes** |
| **OH-C3** | Backup + restore runbook with recorded drill | **Yes** |
| **OH-C4** | Metrics + alert for aged reservations and audit_gap | **Yes** |
| **OH-C5** | Publish freeze / stuck-reservation incident runbooks | **Yes** |
| **OH-C6** | MFA for admin in pilot deployments | **Yes** (pilot) |
| **OH-C7** | Fleet-wide freeze dual-control design implemented or API absent | **Yes** if fleet API added |
| **OH-C8** | P0 tests in §7.2 green | **Yes** |
| **OH-C9** | Apply remains disabled; no PROTO-0 treasury coupling | Ongoing |
| **OH-C10** | Hash chain remains optional (design only until integrity phase) | No |

---

## 10. Mandatory actions before Phase 23

Phase 23 (suggested): **Treasury Operational Hardening Implementation** — code + runbooks only for OH-C1–C5 + C8; still no Apply, rails, or protocol changes.

Before approving Phase 23:

1. Explicit implementation approval citing this **PASS WITH CONDITIONS**.  
2. Scope locked to: sweeper, TTL defaults, stuck-mutation reconcile, metrics/alerts stubs, runbooks, P0 tests.  
3. Confirm **non-goals**: Apply enablement, PROTO-0 spend, custody, hash-chain implementation, multi-tenant Postgres.  
4. Update TR-TW-060 status to **MITIGATED (impl)** only after sweeper ships with tests.

---

## 11. Gate checklist

| Criterion | Met? |
|-----------|------|
| No code changes in Phase 22 | **Yes** |
| No Apply / protocol changes | **Yes** |
| Reservation lifecycle defined | **Yes** |
| TR-TW-060 design resolution | **Yes** (impl still open) |
| Freeze model defined | **Yes** |
| Operational risks documented | **Yes** |
| Security gaps documented | **Yes** |
| Enterprise readiness assessed | **Yes** — **NOT READY** until conditions |

---

## 12. Verdict statement

```text
GATE: TREASURY OPERATIONAL HARDENING (Phase 22)
DECISION: PASS WITH CONDITIONS
ENTERPRISE PILOT: NOT READY
NEXT: Phase 23 hardening implementation (separate approval)
APPLY: remains disabled
PROTOCOL: unchanged
LEDGER: internal acknowledgment only
```

**STOP.** Implementation requires separate approval.
