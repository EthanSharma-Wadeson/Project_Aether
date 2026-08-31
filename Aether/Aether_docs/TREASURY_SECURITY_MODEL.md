# Treasury Security Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_SECURITY_MODEL.md` |
| **Phase** | 15 — Agent Treasury & Financial Architecture |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Related** | [TREASURY_ARCHITECTURE.md](TREASURY_ARCHITECTURE.md), [TREASURY_THREAT_MODEL.md](TREASURY_THREAT_MODEL.md), [CONTROL_PLANE_SECURITY_MODEL.md](CONTROL_PLANE_SECURITY_MODEL.md) |

---

## 1. Security objectives

| Objective | Description |
|-----------|-------------|
| **Ownership integrity** | Agents cannot obtain legal or ledger title to funds |
| **Authority integrity** | Spend only under allocation ∩ capability ∩ policy ∩ approval |
| **Attribution integrity** | Every movement reconstructible to org, treasury, policy, approval, capability, audit |
| **Non-repudiation** | Operators and systems cannot deny recorded funding/spend decisions |
| **Blast-radius control** | Compromises limited by freeze, caps, delegation depth, dual control |
| **Protocol boundary** | Treasury cannot bypass PROTO-0 validation or weaken Apply gates |

---

## 2. Trust boundaries

```text
┌──────── Human operators / IdP ────────┐
│                                       │
▼                                       ▼
Control Plane auth (JWT, CSRF, RBAC)   Finance approvers
│                                       │
▼                                       ▼
┌──────────── Treasury Plane ────────────┐
│  journals, budgets, allocations        │
│  regulatory hooks (extension points)   │
└────────────┬───────────────────────────┘
             │ capability mapping requests
             ▼
        PROTO-0 (authoritative agent authority)
             │
             ▼
        PROTO-2 / PROTO-4 (economic + settlement contracts)
             │
             ▼
        Custody / bank / ERP adapters (external trust)
```

**Critical:** External adapters are **untrusted for authority**. They may claim deposits/payments; Treasury accepts them only after reconciliation policy + attribution.

---

## 3. Controls by layer

### 3.1 Identity & access

- Reuse Control Plane roles (`admin`, `operator`, `viewer`) with **finer treasury permissions** in future (treasury_admin, finance_approver) — design only; not implemented.  
- Dual control for: deposit acknowledgment above threshold, budget increase above threshold, emergency unfreeze, recovery to external accounts.  
- MFA recommended for treasury_admin / finance_approver (enterprise deployment control).

### 3.2 Spending authority

- Compound checks before any spend intent.  
- Emergency stop: freeze budget + request PROTO-0 freeze/revoke (Apply path when enabled; until then operational runbooks only).  
- Delegation depth caps prevent infinite grant chains.  
- Daily / per-tx limits enforced even if capability `max_spend` is higher (least privilege of the intersection).

### 3.3 Accounting integrity

- Append-only journal; hash-chained batches (design target).  
- Idempotent posting.  
- Segregation: initiator ≠ sole approver for high-risk movements.  
- Export packages hashed for regulatory handoff.

### 3.4 Protocol / Apply

- No new mutation surface in Phase 15.  
- Future treasury→capability sync must use existing Apply security (validation, approval, replay, re-simulation).  
- `apply_enabled` remains false until independent gate.

### 3.5 Adapter security

- Least privilege API credentials to banks/ERP.  
- Signed webhooks with replay windows.  
- Allowlisted settlement destinations.  
- No agent-held custody credentials.

---

## 4. Regulatory extension points (not implementations)

Compliance engines are **pluggable hooks**, not core logic:

| Hook | When invoked | Expected effect |
|------|--------------|-----------------|
| **KYC** | Org onboarding; counterparty bind | Block treasury activation if fail |
| **AML** | Deposit, payout, high-velocity patterns | Hold to suspense; alert |
| **Sanctions** | Counterparty / merchant / geo check | Hard deny spend |
| **Tax** | Settlement post; export | Annotate journals; no auto-file required in core |
| **Reporting** | Period close; threshold events | Emit regulated extracts |
| **Jurisdiction rules** | Per-org residency profile | Enable/disable rails & assets |
| **Retention** | Journal/audit lifecycle | Legal hold vs purge policy |
| **Audit exports** | Regulator / auditor request | Packaged immutable extract |

Core treasury must remain operable with **no-op hooks** in lab deployments; production deployments bind real providers.

---

## 5. Emergency recovery

| Scenario | Designed response |
|----------|-------------------|
| Compromised operator | Revoke sessions; freeze treasuries; rotate credentials; dual-control recovery |
| Compromised signer (PROTO-0 path) | Freeze identities; revoke capabilities; halt allocations |
| Runaway agent | Emergency stop allocation + PROTO-0 freeze; cancel open approvals |
| Treasury exhaustion | Hard fail new reservations; alert; no overdraft unless explicit credit facility (out of scope) |
| Suspected double spend | Halt asset rail; reconcile journal vs custody; compensating entries |
| Lost custody adapter | Mark external balances unverified; block payouts; allow read-only audit |

Recovery **never** invents balances; it freezes, reconciles, then posts compensating journals.

---

## 6. Data classification

| Data | Class | Handling |
|------|-------|----------|
| Balances, journals | Confidential financial | Encrypt at rest (deployment); RBAC |
| Bank account numbers | Restricted | Vault / adapter-side; tokenize in CP |
| Agent spend patterns | Confidential | Need-to-know; auditor access |
| Regulatory holds | Restricted | Immutable reason codes |

---

## 7. Relationship to existing security docs

Extends, does not replace:

- `SECURITY_MODEL.md` (protocol)  
- `CONTROL_PLANE_SECURITY_MODEL.md`  
- `CONTROL_PLANE_WRITE_SECURITY.md`  
- Apply enablement / replay gates  

---

## 8. Freeze statement

Security objectives, trust boundaries, control layers, and regulatory hooks are **frozen** as design. Threat catalogue lives in [TREASURY_THREAT_MODEL.md](TREASURY_THREAT_MODEL.md).
