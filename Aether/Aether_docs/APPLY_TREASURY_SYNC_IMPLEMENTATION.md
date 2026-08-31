# Apply ↔ Treasury Sync Implementation — Observation Layer

| Field | Value |
|-------|--------|
| **Document** | `APPLY_TREASURY_SYNC_IMPLEMENTATION.md` |
| **Phase** | 24 — Apply Treasury Sync Observation Layer |
| **Status** | **IMPLEMENTED (observation only)** |
| **Date** | 2026-08-01 |
| **Design** | [APPLY_TREASURY_SYNC_ARCHITECTURE.md](APPLY_TREASURY_SYNC_ARCHITECTURE.md), [APPLY_TREASURY_SYNC_SECURITY_GATE.md](APPLY_TREASURY_SYNC_SECURITY_GATE.md) |

---

## Scope

Phase 24 delivers **read-only** visibility between Treasury allocations and PROTO-0 capability grants.

| Allowed | Forbidden |
|---------|-----------|
| Drift detection & reporting | Enforcement / deny paths |
| Reconciliation status codes | Apply enablement (`apply_enabled` remains false) |
| Audit of observation reads | PROTO-0 mutation |
| Org-scoped queries | Treasury journal / balance mutation |
| | Automatic repair / heal |

---

## Architecture

```text
HTTP GET /api/sync/treasury-capabilities[/:agent_id]
        │
        ▼
SyncObservationService  (read-only)
        │
        ├── queries::load_snapshot
        │     ├── proto0::list_capabilities (CapabilityStore read)
        │     └── TreasuryEngine::list_allocations_for_organisation
        │
        ├── compare::compare_snapshot  (pure)
        │
        └── audit::record_observation  (CP audit_log only)
```

### Module layout

`control_plane/src/apply_treasury_sync/`

| File | Role |
|------|------|
| `models.rs` | `SyncStatus`, `DriftKind`, `DriftFinding`, `SyncObservationReport` |
| `queries.rs` | Snapshot loaders; active/expired helpers |
| `compare.rs` | Pure comparison engine |
| `service.rs` | Orchestration + report assembly |
| `audit.rs` | `observation.apply_treasury_sync` events |

### HTTP

| Method | Path | Auth |
|--------|------|------|
| GET | `/api/sync/treasury-capabilities` | JWT (viewer+) |
| GET | `/api/sync/treasury-capabilities/:agent_id` | JWT (viewer+) |

### Status codes

| Code | Meaning |
|------|---------|
| `SYNC_OK` | No findings |
| `DRIFT_DETECTED` | Missing allocation or missing capability |
| `REQUIRES_REVIEW` | Excess limit, expired/inactive allocation with active capability |

Overall status is the **max** severity across findings.

### Drift kinds

| Kind | Severity |
|------|----------|
| `capability_without_allocation` | `DRIFT_DETECTED` |
| `allocation_without_capability` | `DRIFT_DETECTED` |
| `capability_limit_exceeds_allocation` | `REQUIRES_REVIEW` |
| `expired_allocation_active_capability` | `REQUIRES_REVIEW` |
| `inactive_allocation_active_capability` | `REQUIRES_REVIEW` (closed/frozen) |

Join key: **agent_id** (+ asset when capability declares one). Capability “active” = **not revoked** (demo relative `valid_*` windows are not wall-clock-expired for observation).

---

## Security properties

| Property | How enforced |
|----------|----------------|
| Read-only | Service only calls list/get APIs; no write imports |
| No PROTO-0 calls that mutate | Uses `proto0::list_capabilities` only |
| No Apply execution | No Apply pipeline / enablement hooks |
| No treasury mutations | Engine query `list_allocations_for_organisation` only |
| No automatic repair | No heal/repair module; report flags `auto_repair: false` |
| Org isolation | Allocations filtered by adapter `organisation_id` |
| Audit | Every observation appends CP audit event |

Response always includes:

```json
{
  "observation_only": true,
  "enforcement": false,
  "auto_repair": false,
  "ledger_notice": "Observation only — no Apply execution, PROTO-0 mutation, treasury write, or auto-repair."
}
```

---

## Tests

`control_plane/tests/phase24_apply_treasury_sync_tests.rs`

| Test | Covers |
|------|--------|
| `missing_allocation_detected` | Cap without matching allocation |
| `excess_capability_requires_review` | `max_spend` > ceiling |
| `expired_allocation_with_active_capability` | Expired alloc + active cap |
| `matching_state_sync_ok` | Funded matching pair → `SYNC_OK` |
| `concurrent_reads_stable` | Parallel GETs |
| `cross_org_isolation` | Foreign-org allocation ignored |

Unit coverage in `compare.rs` (`matching_state_is_sync_ok`, `missing_allocation_detected`, `excess_capability_requires_review`).

Engine addition: `TreasuryEngine::list_allocations_for_organisation` (read-only).

---

## Remaining risks

1. **Asset plane mismatch** — Bootstrap PROTO caps use `AETHER_TEST`; demo treasury funds `GBP`. Org-wide observe will typically report drift until operators allocate matching assets.
2. **No enforcement** — Drift is visible but not blocking Apply or runtime spend (by design for Phase 24).
3. **Multi-cap / multi-alloc agents** — Pairing is heuristic (any compatible active pair); ambiguous sets may under- or over-report excess.
4. **Revoked capability lag** — Observation sees store state only; no link to pending Apply revoke intents.
5. **Time semantics** — Capability windows not wall-clock filtered; allocation expiry uses RFC3339 / status.

---

## Explicit non-goals (STOP)

- Do **not** enable Apply.
- Do **not** implement Hybrid B Apply-time backing checks here.
- Do **not** auto-heal PROTO-0 or Treasury.
- Do **not** mutate PROTO-0 from Treasury.

**Phase 24 complete — observation layer only.**
