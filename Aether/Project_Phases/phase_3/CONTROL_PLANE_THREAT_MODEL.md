# Control Plane Threat Model

## Status

**Architecture-phase threat analysis — extends [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)**

The Control Plane introduces a new attack surface: a human-facing application layer that reads protocol state and writes to PROTO-0 capability issuance. This document classifies threats against the Control Plane and its integration with the Aether protocol stack.

References:

- [CONTROL_PLANE_ARCHITECTURE.md](CONTROL_PLANE_ARCHITECTURE.md)
- [PHASE_3_CONTEXT.md](PHASE_3_CONTEXT.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
- [PROTO_3_THREAT_MODEL.md](../phase_2/PROTO_3_THREAT_MODEL.md)

---

## 1. Assets

| Asset | Why it matters |
|-------|----------------|
| Operator signing keys | Control capability issuance for entire agent fleet |
| Policy templates | Define agent authority boundaries |
| CP authentication credentials | Access to all dashboard operations |
| Protocol store indexes | Derived data; if poisoned, misleads operators |
| Audit log integrity | Forensic trail of all policy changes |
| Agent privacy | Enterprise operational data visible through CP |

---

## 2. Adversaries

| Class | Goal |
|-------|------|
| **Compromised operator** | Abuse signing keys to over-grant capabilities or exfiltrate data |
| **External attacker** | Compromise CP to gain protocol-level authority |
| **Malicious insider** | Use legitimate CP access for unauthorized policy changes |
| **Supply chain attacker** | Inject malicious dependencies into CP application |
| **Data exfiltrator** | Extract agent economic data, counterparty graphs, or reputation metrics |
| **Rogue agent** | Exploit CP to obtain escalated capabilities |

---

## 3. Threat Catalogue

### 3.1 Authentication and Access

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| CP-THR-001 | Stolen CP authentication credentials grant full dashboard access | **OPEN** | MFA enforcement; session expiry; RBAC with least privilege |
| CP-THR-002 | Leaked API keys allow programmatic access to CP | **OPEN** | Key rotation; short-lived tokens; audit log monitoring |
| CP-THR-003 | Session hijacking via XSS or CSRF | **OPEN** | Standard web security controls; CSP headers; CSRF tokens |

### 3.2 Operator Key Compromise

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| CP-THR-010 | Compromised operator signing key allows unrestricted capability issuance | **OPEN** | HSM/enclave key storage; key rotation; delegation narrowing limits blast radius |
| CP-THR-011 | Operator grants capabilities exceeding their own delegation scope | **VALIDATED** (design) | PROTO-0 delegation narrowing enforced — children cannot expand parent authority |
| CP-THR-012 | Operator revokes critical agent capabilities causing service disruption | **OPEN** | Revocation requires confirmation; audit trail; multi-approval for fleet-wide revoke |
| CP-THR-013 | Operator signing key used to create new identities for unauthorized agents | **OPEN** | Key management; separation of identity creation from capability issuance |

### 3.3 Policy Manipulation

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| CP-THR-020 | Malicious policy template with excessive spend limits | **OPEN** | Policy review workflow; upper bounds from parent capability; audit log |
| CP-THR-021 | Policy applied to wrong agent set | **OPEN** | Confirmation step; dry-run preview; audit log |
| CP-THR-022 | Shadow policy bypasses CP approval workflow | **VALIDATED** (design) | All capability issuance goes through PROTO-0; CP is not the only issuance path but audit log captures CP operations |
| CP-THR-023 | Deleted/modified audit log hides policy changes | **OPEN** | Append-only audit store; separate audit key; external audit sink |

### 3.4 Data Leakage

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| CP-THR-030 | CP indexes expose counterparty graphs to unauthorized users | **OPEN** | RBAC; enterprise local-only mode; aggregate-only views for restricted roles |
| CP-THR-031 | Research Lab export includes private escrow details | **OPEN** | Export access controls; redaction rules; data classification |
| CP-THR-032 | CP API responses leak signing keys or private material | **VALIDATED** (design) | CP reads public data from protocol stores; signing keys never stored in CP DB; key operations use HSM |
| CP-THR-033 | Cross-tenant data leakage in multi-tenant deployment | **NOT APPLICABLE** (v0) | v0 is single-tenant enterprise only |

### 3.5 Protocol Authority Escalation

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| CP-THR-040 | CP component gains direct write access to EscrowStore | **VALIDATED** (design) | `pub(crate)` on store internals; CP is a separate process; no code path from CP to store mutation |
| CP-THR-041 | CP fabricates reputation events to boost/damage agent scores | **VALIDATED** (design) | `ReputationStore.append()` requires verified `event_id` + evidence refs; CP has no evidence fabrication path |
| CP-THR-042 | CP modifies settlement finality status | **VALIDATED** (design) | `promote_verified_hard_settlement` is `pub(crate)`; only `finalize_settlement` can set hard flag |
| CP-THR-043 | CP creates capability that bypasses PROTO-0 verification | **VALIDATED** (design) | All capabilities verified by `authorise_action` — signature, chain, scope, expiry |
| CP-THR-044 | CP issues capability to agent not under its delegation tree | **VALIDATED** (design) | PROTO-0 subject binding + delegation depth enforcement |

### 3.6 Index Integrity

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| CP-THR-050 | Poisoned CP index shows incorrect agent metrics to operator | **OPEN** | Indexes are derived and rebuildable from protocol stores; spot-check verification |
| CP-THR-051 | Stale index data leads to incorrect policy decisions | **OPEN** | Index freshness indicators; staleness alerts; manual refresh capability |
| CP-THR-052 | CP index diverges from protocol store after partial failure | **OPEN** | Full rebuild capability; consistency checks; event sequence tracking |

### 3.7 Availability

| ID | Threat | Status | Mitigation |
|----|--------|--------|------------|
| CP-THR-060 | CP downtime prevents policy changes but agents continue operating | **VALIDATED** (design) | Protocol operates independently; CP loss = visibility loss, not protocol failure |
| CP-THR-061 | CP downtime during emergency prevents agent freeze | **OPEN** | Direct PROTO-0 CLI access as backup; emergency key procedures |
| CP-THR-062 | DoS against CP API prevents operator access | **OPEN** | Rate limiting; DDoS protection; local CLI fallback |

---

## 4. VALIDATED vs OPEN Summary

### VALIDATED (design)

- CP cannot escalate to protocol authority (PROTO-0 enforces all capability rules regardless of caller)
- CP cannot modify escrow, settlement, or reputation stores (`pub(crate)` visibility)
- CP cannot fabricate reputation events (deterministic event_id + evidence requirement)
- CP cannot issue capabilities beyond its own delegation scope (PROTO-0 delegation narrowing)
- CP cannot bypass settlement finality rules (`promote_verified_hard_settlement` is crate-private)
- CP loss does not affect protocol operation (protocol layers are self-contained)

### OPEN (requires implementation controls)

- Operator key compromise allows capability issuance within delegation scope
- CP authentication compromise grants dashboard access
- Audit log integrity depends on implementation (append-only store, external sink)
- Index integrity depends on correct implementation of derived views
- Data leakage controls depend on RBAC implementation
- Emergency access during CP downtime requires out-of-band procedures

### NOT APPLICABLE (v0)

- Multi-tenant data isolation (v0 is single-tenant)

---

## 5. Security Invariants

1. **INV-CP-01:** No code path from Control Plane to `EscrowStore`, `SettlementStore`, or `ChannelStore` mutation.
2. **INV-CP-02:** Every capability issued via CP goes through the same `grant_capability` → `authorise_action` path as any other caller.
3. **INV-CP-03:** CP signing keys are delegated capabilities, not root keys. Blast radius bounded by delegation scope.
4. **INV-CP-04:** Loss of CP does not halt, degrade, or modify protocol operation.
5. **INV-CP-05:** Audit log records all CP write operations before they execute.

---

## 6. Residual Risk Statement

The Control Plane introduces **application-layer risk** without introducing **protocol-layer risk**. The most significant residual risks are:

1. **Operator key compromise** — bounded by delegation narrowing, but within that scope an attacker can issue/revoke capabilities
2. **Data visibility** — any user with CP access can see economic activity, counterparty relationships, and reputation data within their tenant
3. **Audit log tampering** — if the audit store is compromised, forensic accountability is lost

These risks are **standard enterprise application security risks** and can be mitigated with standard controls (HSM, MFA, RBAC, external audit sinks). They do not represent novel protocol-level threats.

---

## Freeze Statement

> Control Plane threats are classified. VALIDATED items are enforced by protocol design. OPEN items require implementation-time security controls. No protocol authority leaks exist in the architecture.
