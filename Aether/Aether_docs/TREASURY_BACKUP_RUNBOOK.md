# Treasury Backup & Restore Runbook

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_BACKUP_RUNBOOK.md` |
| **Phase** | 22.5 — Treasury Operational Remediation |
| **Date** | 2026-08-01 |
| **Scope** | Control Plane SQLite + `aether-treasury` SQLite (lab / single-node) |

---

## 1. What to back up

| Database | Default path | Contents |
|----------|--------------|----------|
| Control Plane | `CP_DB_PATH` (e.g. `./control_plane.db`) | Operators, audit, mutation requests/approvals, policies, Apply metadata |
| Treasury | `CP_TREASURY_DB_PATH` or `{CP_DB_PATH}.treasury.db` | Journal, balances, allocations, reservations |

Always back up **both** files together (same timestamp). Include `-wal` / `-shm` if present, or checkpoint first.

**Not in scope:** protocol in-memory state, Apply live mutation (disabled), external custody.

---

## 2. Backup procedure (lab)

1. Stop or quiesce writers if possible (preferred).  
2. Checkpoint WAL (optional):
   ```bash
   sqlite3 "$CP_DB_PATH" "PRAGMA wal_checkpoint(FULL);"
   sqlite3 "$TREASURY_DB" "PRAGMA wal_checkpoint(FULL);"
   ```
3. Copy with timestamp:
   ```bash
   TS=$(date -u +%Y%m%dT%H%M%SZ)
   mkdir -p backups/$TS
   cp -a "$CP_DB_PATH" "$CP_DB_PATH"-wal "$CP_DB_PATH"-shm backups/$TS/ 2>/dev/null || true
   cp -a "$TREASURY_DB" "$TREASURY_DB"-wal "$TREASURY_DB"-shm backups/$TS/ 2>/dev/null || true
   ```
4. Record `sha256` of each file in `backups/$TS/MANIFEST.txt`.  
5. Store off-host for pilot deployments.

---

## 3. Restore procedure

1. Stop Control Plane process.  
2. Replace live DB files with backup copies (both CP + treasury).  
3. Start Control Plane.  
4. Verify integrity (below).  
5. Run admin reconcile scan: `GET /api/treasury/ops/reconcile`.  
6. If due reservations appear, run `POST /api/treasury/ops/sweep` (CSRF + admin; MFA header if `CP_ADMIN_MFA_REQUIRED`).

**Never** hand-edit journal rows or `account_balances`.

---

## 4. Integrity verification

1. Treasury security / metrics: `GET /api/treasury/ops/metrics`  
2. Engine replay (ops tooling / tests): `replay_balances(organisation_id)` must leave balances unchanged.  
3. Spot-check: pick a `request_id` from mutation timeline → journal batch exists and balances.  
4. CP audit_log still readable for the same window.

Automated evidence: `cargo test --test phase22_5_ops_hardening_tests backup_restore_drill_preserves_journal_integrity`.

---

## 5. Recovery testing (drill)

| Step | Action | Pass criteria |
|------|--------|---------------|
| 1 | Create funded treasury + journal post | Fund succeeds |
| 2 | Copy treasury DB to restore path | Files present |
| 3 | Open restored engine | Connect OK |
| 4 | Read available balance | Matches pre-backup |
| 5 | `replay_balances` | No drift |

Repeat after every significant schema migration.

---

## 6. Incident notes

- Prefer restore of **paired** CP+treasury backups from the same `TS`.  
- After restore, sweeper may release reservations that expired while offline — expected and audited.  
- Apply remains disabled; no protocol restore required for treasury ledger recovery.

---

## Related

- [TREASURY_OPERATIONAL_HARDENING_REVIEW.md](TREASURY_OPERATIONAL_HARDENING_REVIEW.md)  
- [TREASURY_OPS_HARDENING_IMPLEMENTATION.md](TREASURY_OPS_HARDENING_IMPLEMENTATION.md)
