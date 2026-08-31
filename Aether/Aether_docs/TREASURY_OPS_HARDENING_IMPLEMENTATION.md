# Treasury Ops Hardening Implementation — Phase 22.5

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_OPS_HARDENING_IMPLEMENTATION.md` |
| **Phase** | 22.5 — Treasury Operational Remediation |
| **Date** | 2026-08-01 |
| **Gate input** | [TREASURY_OPERATIONAL_HARDENING_REVIEW.md](TREASURY_OPERATIONAL_HARDENING_REVIEW.md) |
| **Apply** | Remains disabled |
| **Protocol** | Unchanged |

---

## Summary

Closes Phase 22 mandatory conditions for **lab → pilot readiness path**:

1. Reservation TTL defaults + expiry enforcement + sweeper  
2. Stuck mutation / reservation reconciliation (audited)  
3. Backup/restore runbook + automated restore drill  
4. Ops metrics endpoint  
5. Admin MFA enforcement boundary (documented pilot requirement)

No Apply sync, rails, custody, or `aether-core` changes.

---

## Files created / updated

### Created

| Path |
|------|
| `control_plane/src/treasury/ops/{mod,sweeper,reconcile,metrics,mfa}.rs` |
| `control_plane/tests/phase22_5_ops_hardening_tests.rs` |
| `Aether_docs/TREASURY_BACKUP_RUNBOOK.md` |
| `Aether_docs/TREASURY_ADMIN_MFA.md` |
| `Aether_docs/TREASURY_OPS_HARDENING_IMPLEMENTATION.md` |

### Updated

| Path | Change |
|------|--------|
| `treasury` engine | Default/max TTL; settle rejects expired; `Stuck` status; expire/release/sweep APIs; ops counts |
| `control_plane` config | TTL / sweeper / MFA / executing-timeout env knobs |
| `routes/treasury.rs` | Ops metrics, reconcile, sweep, force-release |
| `lib.rs` | Startup scan + sweeper loop |

---

## Reservation lifecycle

| State | Meaning |
|-------|---------|
| `active` | Hold with funds reserved |
| `expired` | Past TTL; funds still held until release |
| `released` | Funds returned via journal |
| `consumed` | Settlement posted |
| `stuck` | Marked for attention; releasable |

Default TTL **900s**; max **86400s**. Omitting `expires_at` applies default. Settlement of time-expired actives is rejected.

Sweeper: `expire_and_release` → audits `reservation_expired` / `reservation_released`.

---

## Reconciliation

| API | Role |
|-----|------|
| `GET /api/treasury/ops/reconcile` | Scan stuck executing + due reservations |
| `POST /api/treasury/ops/reconcile/run` | Fix stuck mutations + optional sweep |
| `POST /api/treasury/ops/sweep` | Manual sweeper pass |
| `POST .../force-release` | Admin release with reason + journal |

No silent balance edits; every recovery audited.

---

## Monitoring

`GET /api/treasury/ops/metrics` (admin):

`reservation_age_max_secs`, `expired_reservations`, `due_active_reservations`, `stuck_reservations`, `stuck_operations`, `audit_gap_count`, `failed_mutations`, approval/execution latency averages.

---

## TR-TW-060

| Status | Detail |
|--------|--------|
| **MITIGATED (impl)** | TTL + sweeper + settle reject + audits + tests |

---

## Phase 22 conditions

| ID | Status |
|----|--------|
| OH-C1 sweeper/TTL | **Closed** |
| OH-C2 stuck executing reconcile | **Closed** |
| OH-C3 backup runbook + drill | **Closed** |
| OH-C4 metrics | **Closed** (alerting hooks still operator/log-level) |
| OH-C5 runbooks | **Closed** (backup + MFA docs; freeze runbook still in Phase 22 review) |
| OH-C6 MFA | **Boundary closed**; IdP wiring = pilot ops |
| OH-C8 P0 tests | **Closed** in `phase22_5_ops_hardening_tests` |

---

## Remaining risks / pilot blockers

- Alerting still log-based (no PagerDuty/webhook pack)  
- Fleet-wide freeze dual-control API not added (by design until needed)  
- Hash-chained journal still future  
- Real IdP MFA must be wired when `CP_ADMIN_MFA_REQUIRED=true`  
- Single-node SQLite limits  

**Enterprise pilot:** closer, but still requires IdP MFA + off-host backups + alert routing before “pilot ready” marketing.

---

## Readiness for Phase 23

Phase 23 may proceed as **Apply Final Enablement Gate** (docs) **or** further enterprise ops (alerting pack / fleet freeze) — **not** automatic Apply flip. Treasury remediation no longer blocks starting Apply gate *review*, but Apply enablement remains its own dual-control decision.

**STOP.**
