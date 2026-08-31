# aether-treasury — Phase 17 Foundation

Internal Treasury domain: hierarchy, asset registry, allocations, reservations, and an append-only double-entry journal.

**Not included:** HTTP, frontend, custody, payments, FX, ERP, Apply, `aether-core`.

## Tables

- `asset_types`
- `treasuries`
- `allocations`
- `reservations`
- `journal_batches` / `journal_entries`
- `account_balances` (projections; rebuildable via replay)

## Run tests

```bash
cd treasury && cargo test
```
