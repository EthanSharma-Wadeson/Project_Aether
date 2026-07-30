# Control Plane Apply — Required Test Plan (Pre-Implementation)

**Document type:** Test plan only  
**Status:** No Apply implementation; tests listed here are **required before Apply is enabled**  
**Date:** 2026-07-29  
**Related:** [CONTROL_PLANE_APPLY_SECURITY_MODEL.md](CONTROL_PLANE_APPLY_SECURITY_MODEL.md), [SIGNED_OPERATION_REPLAY_MODEL.md](SIGNED_OPERATION_REPLAY_MODEL.md)

---

## Scope

This plan covers tests that must exist **before** Apply routes or PROTO-0 mutations ship.

Do **not** implement Apply to satisfy this plan yet. Where a test needs Apply behaviour, mark it as **blocked on Apply coding** and implement it in the same change set as Apply — not earlier via stub mutation paths.

Configuration and execution-hash unit tests may ship in the design/hardening phase.

---

## Security

| ID | Test | Expected | Phase |
|----|------|----------|-------|
| S1 | Replayed signed operation rejected | Second Apply with same `operation_id` → reject; protocol unchanged | Apply coding |
| S2 | Expired approval rejected | Apply after approval TTL → reject | Apply coding |
| S3 | Modified policy invalidates approval | Edit/version bump after approval → Apply reject | Apply coding |
| S4 | Modified dry-run invalidates execution | `dry_run_id` / `execution_hash` mismatch → reject | Apply coding |
| S5 | Missing signer rejected | Apply / dry-run without signer → fail closed | Dry-run exists; Apply coding for Apply path |
| S6 | Dry-run signature cannot be reused for Apply | Present dry-run `signature_hex` as Apply proof → reject | Apply coding |
| S7 | Expired signed operation rejected | `expires_at` in past → reject | Apply coding |

---

## Concurrency

| ID | Test | Expected | Phase |
|----|------|----------|-------|
| C1 | Concurrent approvals (different versions) | Only latest valid version+hash can Apply; stale rejected | Apply coding |
| C2 | Stale policy version | Dry-run then policy change then Apply → reject | Apply coding |
| C3 | Duplicate requests | Idempotent rejection / no double mutation | Apply coding |
| C4 | Cancelled requests | Cancelled approval cannot Apply | Apply coding |
| C5 | Concurrent update vs submit (policy lifecycle) | Status-guarded UPDATE; one wins | Pre-Apply (policy) |
| C6 | Concurrent dual-approve | At most one approve succeeds | Pre-Apply (policy) |

---

## Configuration

| ID | Test | Expected | Phase |
|----|------|----------|-------|
| P1 | Default password refusal | Production mode + default `admin` password → boot fail | **Implemented** (`production_config_check`) |
| P2 | Missing origin refusal | Production + empty `CP_ALLOWED_ORIGINS` → fail | **Implemented** |
| P3 | Missing signer refusal | Production + no `CP_SIGNER_SEED_HEX` → fail | **Implemented** |
| P4 | Insecure cookie refusal | Production + `CP_SECURE_COOKIES` false → fail | **Implemented** |
| P5 | Weak / short JWT secret refusal | Production + weak secret → fail | **Implemented** |
| P6 | Identity-derived signer forbidden in prod | No seed → fail | **Implemented** (via P3) |

---

## Execution Hash

| ID | Test | Expected | Phase |
|----|------|----------|-------|
| H1 | Hash stable for identical binding | Same inputs → same `execution_hash` | **Implemented** (unit) |
| H2 | Hash changes with policy data / version / target / intent | Different binding → different hash | **Implemented** (unit) |
| H3 | Dry-run report includes `execution_hash` + `dry_run_id` | Fields present in API/audit | Hardening / dry-run |

---

## Regression Gates (Always)

- `cargo test -p aether-control-plane`
- `cargo test -p aether-core`
- `cargo clippy -p aether-control-plane --all-targets -- -D warnings`
- Grep / architectural check: no Control Plane callers of PROTO-0 grant/revoke mutation APIs

---

## Exit Criteria for Apply Coding Start

All of:

1. P1–P6 green in CI  
2. Apply security model + replay model frozen and approved  
3. H1–H3 green  
4. This plan accepted; S* and C* scheduled into Apply implementation PR checklist  
5. Separate Apply design approval recorded
