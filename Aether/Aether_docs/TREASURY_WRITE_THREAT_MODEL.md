# Treasury Write Threat Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_WRITE_THREAT_MODEL.md` |
| **Phase** | 20 — Treasury Write Security Gate |
| **Status** | **DESIGN FROZEN** — analysis only; no implementation authorised |
| **Date** | 2026-08-01 |
| **Related** | [TREASURY_WRITE_SECURITY_MODEL.md](TREASURY_WRITE_SECURITY_MODEL.md), [TREASURY_THREAT_MODEL.md](TREASURY_THREAT_MODEL.md), [TREASURY_CONTROL_PLANE_ARCHITECTURE.md](TREASURY_CONTROL_PLANE_ARCHITECTURE.md) |

---

## Executive Summary

This catalogue covers threats introduced or amplified by **exposing treasury mutations** through the Control Plane. It extends Phase 15 IDs (**TR-THR-***) with write-path IDs (**TR-TW-***).

Statuses:

| Status | Meaning |
|--------|---------|
| **MITIGATED (design)** | Controls specified in write security model / API spec |
| **OPEN** | Must be closed by Phase 21 design ADR or accepted with record |
| **ACCEPTED** | Residual risk acknowledged for lab / Apply-off posture |
| **OUT OF SCOPE** | Custody/rails/Apply — tracked but not Phase 21 |

---

## 1. Assets (write-path)

| Asset | Impact if abused |
|-------|------------------|
| Journal posting authority via CP | Fraudulent funding / expense recognition |
| Allocation ceilings | Over-authority to agents (books) |
| Reservations | Lock or drain available liquidity |
| Dual-control approvals | Bypass SoD → insider fraud |
| Idempotency / replay store | Double post or stuck deny |
| Org tenancy binding | Cross-tenant funding theft / disclosure |
| CP audit completeness | Undetectable fraud |
| Freeze control | Availability DoS / cover for theft |

---

## 2. Adversaries

| Class | Goal via write APIs |
|-------|---------------------|
| Compromised operator | Inflate allocations; self-deal reservations |
| Malicious administrator | Solo fund + suppress evidence |
| External attacker (stolen JWT) | CSRF-less or token replay mutations |
| Colluding operator+admin | Dual-control theatre with shared intent |
| Confused deputy (buggy adapter) | Wrong org engine calls |
| Automated client bugs | Duplicate posts, race releases |
| Runaway agent | **N/A directly** — agents have no treasury write API; risk is via future spend path |

---

## 3. Threat catalogue (TR-TW-*)

Likelihood / Impact: **L** Low · **M** Medium · **H** High · **C** Critical  

### 3.1 Unauthorised funding & allocation escalation

| ID | Threat | L | I | Mitigation | Residual | Status |
|----|--------|---|---|------------|----------|--------|
| **TR-TW-001** | Unauthorised funding via forged/stolen session | M | C | JWT+CSRF+Origin; dual-control always on fund; session revoke | MFA not enforced in CP lab | **MITIGATED (design)** |
| **TR-TW-002** | Operator solo-funds by skipping approval | M | C | Server rejects fund without valid `approval_id`; SoD check | Misconfigured threshold=0 bypass if coded wrong — test mandatory | **MITIGATED (design)** |
| **TR-TW-003** | Allocation escalation beyond org appetite | M | H | Dual-control above threshold; audit; org aggregate caps (Phase 15) | Caps not fully productised | **MITIGATED (design)** / **OPEN** on aggregate enforcement ADR |
| **TR-TW-004** | “Funding” marketed as bank deposit without rails | M | H | INV-TW14; API `funding_kind=internal_acknowledgment` | Operator confusion | **MITIGATED (design)** |

### 3.2 Replay, duplicate posting, races

| ID | Threat | L | I | Mitigation | Residual | Status |
|----|--------|---|---|------------|----------|--------|
| **TR-TW-010** | Replay of successful fund/reserve HTTP | M | H | Idempotency key + engine unique constraint; CSRF consume on execute | Client key collision across ops — require key scope | **MITIGATED (design)** |
| **TR-TW-011** | Duplicate journal from double-click / retry | H | H | Return original batch on duplicate key | — | **MITIGATED (design)** |
| **TR-TW-012** | Reservation race double-spends available | M | H | Engine serialisation per treasury; atomic reserve | Cross-process SQLite limits under load | **MITIGATED (design)** |
| **TR-TW-013** | Release + settle race | M | H | Reservation status machine; only Active consumable | — | **MITIGATED (design)** |
| **TR-TW-014** | Re-execute consumed approval | M | C | Approval single-consume in CP txn before/with engine call ordering ADR | — | **MITIGATED (design)** |
| **TR-TW-015** | Chargeback/refund double restore | M | H | Engine serialise; suspense; link prior settlement | TR-THR-033 carry-forward | **MITIGATED (design)** / test **OPEN** until Phase 21 proves |

### 3.3 Tenant leakage & confused deputy

| ID | Threat | L | I | Mitigation | Residual | Status |
|----|--------|---|---|------------|----------|--------|
| **TR-TW-020** | Cross-tenant fund by guessing treasury_id | M | C | Adapter binds `CP_TREASURY_ORG_ID`; mismatch → 404 | Single-org deployments only | **MITIGATED (design)** |
| **TR-TW-021** | Adapter uses engine pool as superuser across orgs | L | C | INV-TW06; no org parameter from client trusted alone | Multi-tenant later raises risk | **MITIGATED (design)** |
| **TR-TW-022** | Viewer triggers mutation via CSRF | M | H | Viewer RBAC deny; CSRF still required for operator | — | **MITIGATED (design)** |

### 3.4 Insider & compromised privileged users

| ID | Threat | L | I | Mitigation | Residual | Status |
|----|--------|---|---|------------|----------|--------|
| **TR-TW-030** | Malicious admin self-approves | M | C | INV-TW08 requester ≠ approver | Two colluding admins | **ACCEPTED** (collusion) with audit forensics |
| **TR-TW-031** | Compromised operator account | M | H | Dual-control on H; L ops capped by allocation; freeze | MFA optional | **MITIGATED (design)** |
| **TR-TW-032** | Compromised admin freezes all treasuries (DoS) | M | H | Audit; unfreeze dual-control; monitoring | Fleet-wide dual-control still **OPEN** (TR-THR-041) | **OPEN** |
| **TR-TW-033** | Admin deletes/suppresses CP audit after fraud | L | C | Append-only audit posture; no delete API; export hash target | Not hash-chained yet | **ACCEPTED** residual (align V1 cert) |

### 3.5 Audit manipulation & divergence

| ID | Threat | L | I | Mitigation | Residual | Status |
|----|--------|---|---|------------|----------|--------|
| **TR-TW-040** | Engine succeeds, CP audit missing | M | H | Audit-after-success + `audit_gap` event; reconcile by `request_id`/`batch_id` | Brief window | **MITIGATED (design)** |
| **TR-TW-041** | CP claims success without engine post | L | C | Response `journal_batch_id` only from engine result; no CP-fabricated balances | — | **MITIGATED (design)** |
| **TR-TW-042** | Accounting divergence (cache vs journal) | M | M | No authoritative CP cache; reads hit engine (Phase 19) | Future cache TR-CP-007 | **MITIGATED (design)** |
| **TR-TW-043** | Journal line update/delete via API | L | C | No such API; engine `ImmutabilityViolation` | Direct DB access by host admin | **ACCEPTED** (host trust) |

### 3.6 Approval & session integrity

| ID | Threat | L | I | Mitigation | Residual | Status |
|----|--------|---|---|------------|----------|--------|
| **TR-TW-050** | Stale approval after payload change | M | H | Bind `payload_hash`; reject mismatch | — | **MITIGATED (design)** |
| **TR-TW-051** | Expired approval execute | M | H | TTL check at execute | Clock skew | **MITIGATED (design)** |
| **TR-TW-052** | CSRF missing on mutation (recent console bug class) | H | H | Mandatory CSRF; tests; clear session on 401 | Operator disables CSRF in misconfig | **MITIGATED (design)** |
| **TR-TW-053** | Stale JWT missing `iss` used for writes | M | H | Verify claims; frontend clear invalid tokens | — | **MITIGATED (design)** |

### 3.7 Reservation / freeze operational abuse

| ID | Threat | L | I | Mitigation | Residual | Status |
|----|--------|---|---|------------|----------|--------|
| **TR-TW-060** | Fake reservations block recovery (TR-THR-003) | M | H | TTL sweeper + admin force-release dual-control | Sweeper not implemented | **OPEN** (mandatory Phase 21 ADR) |
| **TR-TW-061** | Unfreeze without investigation | M | H | Dual-control unfreeze + reason code | — | **MITIGATED (design)** |
| **TR-TW-062** | Settlement post without real PROTO-4 finality | M | M | API documents `evidence_ref` optional; ops policy | Apply-off / sim world | **ACCEPTED** (lab) INV-TW14-adjacent |

### 3.8 Out-of-scope amplifiers (tracked)

| ID | Threat | Status |
|----|--------|--------|
| **TR-TW-070** | Custody adapter credential theft enabling fake funding | **OUT OF SCOPE** (TR-THR-062) until custody phase |
| **TR-TW-071** | Apply sync grants capability without allocation | **OUT OF SCOPE** until Apply + sync gate |
| **TR-TW-072** | Payment rail double payout | **OUT OF SCOPE** |

---

## 4. Crosswalk to Phase 15 threats

| Phase 15 | Write-path treatment |
|----------|----------------------|
| TR-THR-001 Budget abuse | TR-TW-003 + dual-control |
| TR-THR-003 Fake reservations | **TR-TW-060 OPEN** — Phase 21 sweeper ADR required |
| TR-THR-020 Self-approval | TR-TW-002 / TR-TW-030 |
| TR-THR-030 Duplicate payments | TR-TW-010/011 (journal posts) |
| TR-THR-033 Refund race | TR-TW-015 |
| TR-THR-041 Freeze storm | TR-TW-032 OPEN |
| TR-THR-053 Apply-off pretend funds | **ACCEPTED**; internal funding labelled; no protocol mutate |
| TR-THR-062 Custody creds | OUT OF SCOPE for write gate |

---

## 5. Residual risks (accepted for this gate)

1. **Colluding dual admins** can pass SoD — mitigated by audit forensics, not cryptography.  
2. **Host/DBA** can edit SQLite outside API — deployment trust boundary.  
3. **Lab funding** is book-entry only — must not be sold as settled cash.  
4. **No HSM / no MFA mandate** in CP lab — enterprise deployments should add MFA for `admin` before production treasury writes.  
5. **Hash-chained journal / audit** still a target — append-only + tests are the Phase 21 bar.

---

## 6. Phase 21 threat closure checklist

Before shipping write APIs:

- [ ] Close **TR-TW-060** with reservation TTL/sweeper design + tests  
- [ ] Record accept/reject for **TR-TW-032** fleet freeze dual-control  
- [ ] Prove TR-TW-010/011/014/020/002 in automated tests  
- [ ] Document audit-gap ADR (TR-TW-040)  
- [ ] Confirm INV-TW14 labelling in API + console copy  

---

## 7. Freeze statement

Threat IDs **TR-TW-001…072** are frozen for tracking. Implementation reviews update **Status** with version notes only.

**No implementation is authorised by this document.**
