# Treasury Threat Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_THREAT_MODEL.md` |
| **Phase** | 15 — Agent Treasury & Financial Architecture |
| **Status** | **DESIGN FROZEN** — architecture-phase analysis; no implementation authorised |
| **Date** | 2026-08-01 |
| **Related** | [TREASURY_SECURITY_MODEL.md](TREASURY_SECURITY_MODEL.md), [TREASURY_ARCHITECTURE.md](TREASURY_ARCHITECTURE.md) |

---

## 1. Assets

| Asset | Impact if abused |
|-------|------------------|
| Organisation treasury balances | Direct financial loss |
| Budget / allocation state | Overspend, stealth draining |
| Spending authority mappings | Bypass of enterprise policy |
| Journal / audit integrity | Undetectable fraud, failed forensics |
| Custody adapter credentials | External fund theft |
| Operator / approver sessions | Insider or account-takeover fraud |
| PROTO-0 signing path (future sync) | Unbounded capability → unbounded spend |
| Merchant / counterparty allowlists | Payment diversion |

---

## 2. Adversaries

| Class | Goal |
|-------|------|
| Runaway / compromised agent | Maximise spend or exfiltrate via merchants |
| Malicious insider (operator) | Inflate budgets, self-approve, suppress audit |
| Compromised finance approver | Authorise fraudulent payouts |
| External attacker | Steal sessions, forge deposits, replay payouts |
| Colluding counterparty | Duplicate invoices / escrow abuse |
| Supply-chain attacker | Malicious dependency in treasury services |
| Curious auditor misuse | Privacy violation (secondary) |

---

## 3. Threat catalogue

Statuses: **OPEN** (must be addressed in implementation design), **MITIGATED (design)** (controls specified in architecture/security model), **ACCEPTED** (residual risk acknowledged).

### 3.1 Budget & allocation abuse

| ID | Threat | Status | Mitigation (design) |
|----|--------|--------|---------------------|
| TR-THR-001 | Budget abuse — operator allocates beyond organisational policy | **MITIGATED (design)** | Dual control above thresholds; policy ceilings; audit |
| TR-THR-002 | Temporary funding never expires | **MITIGATED (design)** | TTL + auto-recover unused |
| TR-THR-003 | Decrease/recover blocked while “fake” reservations held | **OPEN** | Reservation timeouts; reconcile jobs; admin break-glass with audit |
| TR-THR-004 | Hidden multi-budget aggregation exceeds org risk appetite | **MITIGATED (design)** | Org-level aggregate caps across budgets |

### 3.2 Runaway agents

| ID | Threat | Status | Mitigation (design) |
|----|--------|--------|---------------------|
| TR-THR-010 | Runaway agent spends until treasury empty | **MITIGATED (design)** | Daily/tx caps ∩ max_spend ∩ emergency stop |
| TR-THR-011 | Agent opens many small escrows under per-tx limit | **OPEN** | Velocity limits; open-escrow count caps |
| TR-THR-012 | Agent switches assets to bypass single-asset cap | **MITIGATED (design)** | Per-asset caps + optional org FX/aggregate risk limit |

### 3.3 Insider fraud

| ID | Threat | Status | Mitigation (design) |
|----|--------|--------|---------------------|
| TR-THR-020 | Insider fraud — self-approval of funding | **MITIGATED (design)** | Segregation of duties; dual control |
| TR-THR-021 | Insider suppresses or deletes journal lines | **MITIGATED (design)** | Append-only + hash chain target; no delete API |
| TR-THR-022 | Insider widens merchant allowlist then pays self | **OPEN** | Change-control on allowlists; delayed activation; monitoring |

### 3.4 Payment integrity

| ID | Threat | Status | Mitigation (design) |
|----|--------|--------|---------------------|
| TR-THR-030 | Duplicate payments | **MITIGATED (design)** | Idempotency keys; settlement binding uniqueness |
| TR-THR-031 | Replay of signed spend / payout intents | **MITIGATED (design)** | Nonces, request_id uniqueness, replay store patterns (align Apply) |
| TR-THR-032 | Double spending same reserved funds | **MITIGATED (design)** | Atomic reserve; single-consumer reservation |
| TR-THR-033 | Chargeback / refund race drains available twice | **OPEN** | Suspense accounts; serialised refund posting |

### 3.5 Exhaustion & availability

| ID | Threat | Status | Mitigation (design) |
|----|--------|--------|---------------------|
| TR-THR-040 | Treasury exhaustion DoS against critical agents | **ACCEPTED** / ops | Reserves; priority budgets; monitoring (not infinite overdraft) |
| TR-THR-041 | Freeze storm by compromised security officer | **OPEN** | Dual control for fleet-wide freeze; audit; time-boxed freezes |

### 3.6 Policy & protocol bypass

| ID | Threat | Status | Mitigation (design) |
|----|--------|--------|---------------------|
| TR-THR-050 | Policy bypass — treasury allows spend without capability | **MITIGATED (design)** | Capability check mandatory for agent-initiated spend |
| TR-THR-051 | Capability max_spend raised without treasury allocation | **MITIGATED (design)** | Intersection rule; allocation is ceiling |
| TR-THR-052 | Direct protocol/store write to invent balances | **MITIGATED (design)** | No protocol ownership of org GL; CP RBAC; Apply gates |
| TR-THR-053 | Apply disabled → operators “manually” pretend funds moved | **ACCEPTED** | Runbooks + audit; no silent protocol mutation |

### 3.7 Compromise scenarios

| ID | Threat | Status | Mitigation (design) |
|----|--------|--------|---------------------|
| TR-THR-060 | Compromised operator | **MITIGATED (design)** | Session revoke, MFA, dual control, freeze |
| TR-THR-061 | Compromised signer | **MITIGATED (design)** | Key custody (HSM path from CP security), freeze identities |
| TR-THR-062 | Compromised custody adapter credentials | **OPEN** | Vault, rotation, allowlisted payouts, anomaly detection |
| TR-THR-063 | Emergency recovery abused to exfiltrate | **MITIGATED (design)** | Dual control recovery; destination allowlist; delayed payout |

### 3.8 Regulatory / privacy

| ID | Threat | Status | Mitigation (design) |
|----|--------|--------|---------------------|
| TR-THR-070 | Sanctions evasion via unlisted counterparty | **MITIGATED (design)** | Sanctions hook hard-deny extension point |
| TR-THR-071 | Retention wipe destroys legal evidence | **MITIGATED (design)** | Legal hold extension; append-only |
| TR-THR-072 | Over-collection of personal data in journals | **OPEN** | Data minimisation guidelines per jurisdiction |

---

## 4. Residual risks (accepted for design freeze)

1. External custody providers can fail or lie until reconciliation completes.  
2. Soft finality will always lag hard cash movement — books must tolerate timing gaps.  
3. Without Apply enabled, automated capability sync is unavailable; operational process risk remains.  
4. Velocity/allowlist controls need product metrics not fully specified here (see open questions).

---

## 5. Implementation gate requirements

Before any treasury implementation ships:

- [ ] Address all **OPEN** threats with concrete designs or accepted risk records  
- [ ] Prove append-only journal + idempotency in tests  
- [ ] Prove agents cannot hold title in data model  
- [ ] Prove intersection of allocation ∩ capability  
- [ ] No protocol semantic changes without separate RFC  

---

## 6. Freeze statement

Threat catalogue IDs **TR-THR-001…072** are frozen for design tracking. Status updates belong in future implementation security reviews — not silent doc drift without version notes.
