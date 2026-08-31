# Control Plane Policy Threat Model — Milestone 2

## Status

**Design only — no write implementation authorised**

Date: 2026-07-29

Prerequisite: Milestone 1 Read-Only Observatory complete and security-approved.

Related: [CONTROL_PLANE_AUTHORITY_MODEL.md](CONTROL_PLANE_AUTHORITY_MODEL.md), [CONTROL_PLANE_WRITE_SECURITY.md](CONTROL_PLANE_WRITE_SECURITY.md), [../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md](../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md)

---

## 1. Scope

This threat model covers **Policy Management** — the first Control Plane write surface. It does not authorise implementation.

Milestone 2 introduces:

- Policy template create / update / archive (Control Plane database only)
- Policy apply → PROTO-0 capability issuance
- Capability revocation via PROTO-0
- Optional identity freeze via PROTO-0

Milestone 2 does **not** introduce:

- Escrow, settlement, or reputation mutation
- Marketplace, AETH, or governance DAO
- Direct protocol store writes outside PROTO-0 APIs

**Invariant:** A compromised Control Plane remains an application-layer incident. Protocol cryptographic guarantees stay enforced by `aether-core`.

---

## 2. Assets

| Asset | Location | Sensitivity | Why it matters |
|-------|----------|-------------|----------------|
| Policy definitions | CP SQLite `policy_templates` | High | Encode intended spend/action limits; wrong policy → over-privileged agents |
| Capability state | PROTO-0 `CapabilityStore` | Critical | Authorises agent economic and settlement actions |
| Operator identities | CP SQLite `operators` | High | Control who may request mutations |
| Signing authority | Server-held PROTO-0 key (Milestone 2) | Critical | Signs capability grants/revokes; compromise = enterprise-scoped capability fraud |
| Audit history | CP SQLite `audit_log` | High | Forensic record of governance actions; integrity required for compliance |
| Protocol state (read) | PROTO-0/2/3/4 stores | High | Truth source for observation; must not be forgeable via CP |
| Refresh / access tokens | Browser + CP DB | High | Session theft enables unauthorised operator actions |
| CSRF / anti-replay secrets | CP session / headers | Medium | Protect browser-originated write requests |

---

## 3. Threat Actors

| Actor | Capability | Motivation |
|-------|------------|------------|
| Compromised operator account | Valid JWT + role permissions | Credential theft, phishing |
| Stolen refresh token | Session renewal without password | Cookie theft, XSS, device compromise |
| Malicious insider | Legitimate `admin` / `operator` role | Fraud, sabotage, cover-up |
| Compromised frontend | Malicious JS in SPA origin | CSRF-like actions, token exfiltration |
| Compromised backend | Full CP process + optional signing key | Issue/revoke capabilities within key scope; alter CP DB |
| Network adversary | Replay, MITM (if no TLS) | Replay mutations; steal sessions |
| External attacker (unauthenticated) | Public login endpoint only | Brute force (mitigated M1); no write surface without auth |

---

## 4. Attack Scenarios and Mitigations

### 4.1 Unauthorised policy modification

**Scenario:** Attacker with `viewer` role or stolen low-privilege session creates/updates policy templates or applies policies.

| Mitigation | Layer |
|------------|-------|
| RBAC: only `operator` / `admin` may mutate policies | CP authZ |
| CSRF + Origin checks on all state-changing requests | CP write security |
| Policy templates alone have **no** protocol effect until apply | Authority model |
| Apply requires PROTO-0 validation + signing key | Protocol |

**Classification:** Mitigable — VALIDATED design; OPEN until implemented and tested.

---

### 4.2 Privilege escalation

**Scenario:** Viewer forges JWT role claim, or calls admin-only routes directly.

| Mitigation | Layer |
|------------|-------|
| Role taken only from verified JWT (`iss`/`aud`/`typ`) | M1 — VALIDATED |
| Router-level role middleware on write routes | CP authZ |
| Integration tests: viewer → write routes → 403 | Acceptance |
| PROTO-0 delegation ceiling even if admin is compromised | Protocol — VALIDATED |

**Classification:** Application escalation VALIDATED by M1 JWT design; protocol ceiling VALIDATED.

---

### 4.3 Replayed policy changes

**Scenario:** Attacker captures a legitimate `POST /api/policies/:id/apply` and replays it to re-issue capabilities.

| Mitigation | Layer |
|------------|-------|
| CSRF token single-use or short TTL for mutations | CP write security |
| Optional mutation nonce / idempotency key per apply | CP |
| PROTO-0 may reject duplicate grants by capability semantics | Protocol |
| Audit records each apply attempt (success/failure) | CP audit |
| TLS required for any non-localhost deployment | Deployment |

**Classification:** Design OPEN until CSRF + replay controls implemented.

---

### 4.4 Forged operator actions

**Scenario:** Attacker crafts requests appearing to come from an admin without valid session, or presents access token as refresh / wrong token type.

| Mitigation | Layer |
|------------|-------|
| JWT type separation (access ≠ refresh) — M1 | VALIDATED |
| Refresh rotation + reuse detection — M1 | VALIDATED |
| CSRF + Origin for cookie-authenticated browser writes | Design |
| Server-side signing: browser never holds PROTO-0 key | Authority model |
| Audit binds `operator_id` from verified JWT, not request body | CP |

**Classification:** Session forgery largely VALIDATED (M1); browser write forgery OPEN pending CSRF.

---

### 4.5 Audit tampering

**Scenario:** Malicious operator or DB admin alters/deletes audit rows to hide a grant or revoke.

| Mitigation | Layer |
|------------|-------|
| Append-only API (no update/delete endpoints) | CP |
| Payload hash + PROTO-0 tx reference on each write | CP audit |
| Hash-chained or external append-only store | OPEN — SEC-CP-03 pre-production |
| Protocol capability store remains independent evidence | Protocol |

**Classification:** Soft VALIDATED for MVP; strong tamper-evidence OPEN.

---

### 4.6 Accidental destructive actions

**Scenario:** Operator mis-clicks revoke/freeze or applies overly broad policy.

| Mitigation | Layer |
|------------|-------|
| Explicit confirmation step for revoke / freeze / apply | UX + API |
| Two-step policy flow: define template → separate apply | Authority model |
| Preview of capability constraints before apply | UX |
| Audit + undo path = issue narrowed replacement (not silent restore) | Process |
| Dual approval for freeze/revoke-identity | OPEN — post-MVP SEC-CP-04 |

**Classification:** Design required for M2; dual approval deferred.

---

### 4.7 Compromised backend / signing key

**Scenario:** Attacker gains CP process or `CP_KEY_PATH` material.

| Impact | Bound |
|--------|-------|
| Can issue/revoke within enterprise key delegation | PROTO-0 ceiling |
| Cannot forge escrow/settlement/reputation | Protocol stores |
| Cannot exceed parent capability scope | PROTO-0 narrowing |

| Mitigation | Status |
|------------|--------|
| Key never in DB, never to frontend, never logged | Required M2 |
| File permissions + env-only path | Required M2 |
| HSM / per-operator keys | OPEN — post-MVP |

**Classification:** Protocol ceiling VALIDATED; key custody OPEN for production hardening.

---

## 5. Residual Risks (Milestone 2)

| ID | Risk | Severity | Disposition |
|----|------|----------|-------------|
| M2-R01 | Shared enterprise signing key among all admins | High | Accept for M2; plan per-operator keys |
| M2-R02 | CP audit log mutable by DB admin | Medium | SEC-CP-03 pre-production |
| M2-R03 | CSRF not yet implemented | High | Blocker before any write route |
| M2-R04 | Accidental mass revoke without dual control | Medium | Confirmation required; dual approval later |
| M2-R05 | In-memory rate limiter only | Low | Redis for multi-instance |

---

## Freeze Statement

> This document defines threats for Policy Management. It does not authorise write routes, signing-key wiring, or policy engine code. Implementation requires explicit Milestone 2 approval after authority model, CSRF design, and signing approach gates pass.
