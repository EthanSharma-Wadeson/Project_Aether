# Control Plane Authority Model — Milestone 2

## Status

**Phase 2 policy lifecycle implemented (CP-local only) — no protocol apply authorised**

Date: 2026-07-29

Related: [CONTROL_PLANE_POLICY_THREAT_MODEL.md](CONTROL_PLANE_POLICY_THREAT_MODEL.md), [CONTROL_PLANE_WRITE_SECURITY.md](CONTROL_PLANE_WRITE_SECURITY.md), [../Project_Phases/phase_3/CONTROL_PLANE_ARCHITECTURE.md](../Project_Phases/phase_3/CONTROL_PLANE_ARCHITECTURE.md)

---

## 1. Purpose

Define **who can write**, **what can be written**, and **how protocol authority remains protected** when the Control Plane gains Policy Management.

The Control Plane must never become an alternate authority layer.

---

## 2. Two Modes of Operation

### 2.1 Observer (Milestone 1 — current)

```text
Protocol state (PROTO-0 / 2 / 3 / 4)
        │
        ▼  read-only adapters
Control Plane
        │
        ▼
Human visibility
```

Properties:

- No PROTO-0 mutations
- No policy engine
- No signing keys
- Dashboard failure does not affect protocol operation

### 2.2 Authorised mutation (Milestone 2 — proposed)

```text
Operator
   │
   ▼  authenticated + authorised action
Control Plane (policy engine + audit)
   │
   ▼  PROTO-0 API only (grant / revoke / freeze)
Protocol validation (signature, chain, narrowing, root binding)
   │
   ▼
Capability / identity state change
```

Properties:

- Control Plane **requests** mutations; PROTO-0 **decides**
- Operators never write escrow, settlement, channel, or reputation stores
- Every mutation produces audit evidence
- Loss of Control Plane does not roll back protocol state

---

## 3. Explicit Invariants

| # | Invariant |
|---|-----------|
| I1 | Control Plane does **not** bypass protocol rules |
| I2 | Policy writes that affect agents **must** go through PROTO-0 validation |
| I3 | Operators cannot directly mutate protocol stores |
| I4 | Every mutation requires authenticated operator identity + audit evidence |
| I5 | Policy templates in CP DB have **no** protocol effect until apply |
| I6 | Soft finality ≠ hard finality remains observationally preserved (unchanged) |

---

## 4. What May Be Written

### 4.1 Control Plane–local (no protocol effect)

| Object | Operation | Protocol impact |
|--------|-----------|-----------------|
| Policy template | Create / update / archive | None |
| Operator preferences | Update | None |
| Audit log | Append only | None |

### 4.2 Protocol-affecting (PROTO-0 only)

| Operation | PROTO-0 API | Notes |
|-----------|-------------|-------|
| Apply policy to agent | `grant_capability` | Template → capability constraints |
| Revoke capability | `revoke_capability` | Requires confirmation |
| Freeze identity | `freeze_identity` | Destructive; confirmation + audit |

### 4.3 Forbidden forever (Milestone 2 and beyond unless re-approved)

| Operation | Why |
|-----------|-----|
| Mutate escrow / settlement / reputation / channel | Different protocol authorities |
| Forge agent signatures | Cryptographic impossibility by design |
| Expand beyond enterprise key delegation | PROTO-0 narrowing |
| Silent capability grant without audit | Governance requirement |

---

## 5. Operator Authorization Options

### Option A — Role-based permissions (RBAC)

Roles (proposed for Milestone 2):

| Role | Read | Policy templates | Apply / revoke | Freeze identity |
|------|------|------------------|----------------|-----------------|
| `viewer` | Yes | No | No | No |
| `operator` | Yes | Yes | Yes | No |
| `admin` | Yes | Yes | Yes | Yes |

| Dimension | Assessment |
|-----------|------------|
| Security | Adequate for single-tenant MVP; compromised admin = full CP write surface within key scope |
| Complexity | Low — extends Milestone 1 JWT roles |
| Enterprise suitability | High for first governance console; maps to existing IAM mental models |

### Option B — Cryptographic operator approval

Each mutation is a signed request:

```text
operator identity + operator signing key → signed mutation request → CP verifies → PROTO-0
```

| Dimension | Assessment |
|-----------|------------|
| Security | Strong non-repudiation; binds human key to each action |
| Complexity | High — key custody, UX for signing, recovery, HSM path |
| Enterprise suitability | High long-term; heavy for first write milestone |

### Option C — Multi-party approval

Destructive or high-impact actions require N-of-M operators.

| Dimension | Assessment |
|-----------|------------|
| Security | Best for freeze / mass revoke |
| Complexity | High — workflow, timeouts, deadlocks |
| Enterprise suitability | High for regulated environments; premature for M2 demo |

---

## 6. Recommendation

### Selected approach: **Hybrid A + constrained B**

**Phase for Milestone 2 implementation (when approved):**

1. **RBAC (Option A)** gates who may call write APIs (`viewer` / `operator` / `admin`).
2. **Enterprise PROTO-0 signing key (constrained Option B)** is held by the Control Plane server (file path / env), used only for PROTO-0 grant/revoke/freeze.
3. Browser sessions never receive the PROTO-0 key.
4. **Option C (multi-party)** deferred to post-MVP for freeze / identity revoke.

### Why this hybrid

| Concern | How addressed |
|---------|---------------|
| Enterprise readiness | RBAC matches operator expectations |
| Protocol integrity | All agent-affecting writes still pass PROTO-0 |
| Non-repudiation (CP layer) | JWT operator id + audit payload hash |
| Non-repudiation (protocol layer) | PROTO-0 signatures under enterprise key |
| Complexity budget | Avoids per-operator crypto UX in first write ship |

### Explicit non-goals for first M2 implementation

- Per-operator PROTO-0 keys (SEC-CP-02)
- Dual approval workflows (SEC-CP-04)
- Hardware-backed signing (HSM)
- Direct private-key types in route handlers (use Signer abstraction)

---

## 7. Signing Boundary

**The signer is not the authority.**

The signer only authenticates an already-authorized request. It does not decide policy, does not grant RBAC, and does not override PROTO-0 validation.

### Authority chain

```text
Operator identity
        │
        ▼
RBAC authorization
        │
        ▼
Policy validation   (template constraints, lifecycle: approved?)
        │
        ▼
Signer approval     (authenticates the request under enterprise identity)
        │
        ▼
Protocol validation (PROTO-0 — final authority)
        │
        ▼
State transition    (or reject)
```

### Implications

| Layer | Decides |
|-------|---------|
| Operator identity | Who is acting (JWT) |
| RBAC | Whether CP allows the action class |
| Policy validation | Whether the template/apply request is well-formed and approved |
| Signer | Cryptographic authenticity of the enterprise-backed request |
| PROTO-0 | Whether the capability/identity transition is legal |

If PROTO-0 rejects, the mutation fails even if RBAC and Signer succeeded. The Control Plane records the failure; it does not force the transition.

### Abstraction requirement

Control Plane business logic depends on a `Signer` interface (`sign_request`, `verify_request`, `audit_signature_use`), not on raw private key material. MVP may back the interface with a server-held enterprise signer; production may swap KMS/HSM/signing service without changing the authority chain.

---

## 8. Mutation Audit Requirements

Audit must be **append-only from the application perspective** (no update/delete APIs for audit rows).

Every future mutation attempt (success or failure) must record:

| Field | Purpose |
|-------|---------|
| `operator_id` | Verified JWT subject |
| `role` | Role claimed at time of action |
| `action` | e.g. `policy.create`, `policy.approve`, `policy.apply`, `capability.revoke` |
| `target` | Target entity (policy id, agent id, capability id) |
| `requested_at` | When the operator request was received |
| `approved_at` | When CP authorized the attempt (RBAC + CSRF + policy checks); null if rejected early |
| `signer_identity` | Signer backend id / key id used (never raw key material) |
| `protocol_result` | `success` / `rejected` / `not_applicable` (CP-local writes) |
| `failure_reason` | Human-safe reject reason (authZ, CSRF, PROTO-0 error) |
| `request_id` | Correlation id for identity tracking |
| `payload_hash` | SHA-256 of canonical request body |

Protocol-affecting mutations additionally record a protocol reference (e.g. capability id) when PROTO-0 succeeds.

Template-only writes use `protocol_result = not_applicable` and may omit signer usage until Phase 4.

---

## 9. Policy Lifecycle (Phase 2 — implemented)

```text
Definition (Draft)
        │
        ▼
Review (PendingReview)
        │
        ▼
Approval (Approved)
        │
        ▼
(Future) Apply   ← Phase 4 only; not implemented
```

| Stage | CP effect | Protocol effect |
|-------|-----------|-----------------|
| Draft / update | CP DB row + version history | **None** |
| Submit for review | Status → `pending_review` | **None** |
| Approve / reject | Status → `approved` / `rejected` | **None** |
| Archive | Status → `archived` | **None** |
| Apply (future) | Request PROTO-0 grant | PROTO-0 decides |

**Explicit:** Approved policies are **not** active until a future controlled application phase. Approval is a governance gate on a Control Plane template — it does **not** grant capabilities, revoke capabilities, or mutate agents.

Separation of duties: the creating/submitting operator cannot approve their own template.

---

## Freeze Statement

> Observer mode remains the default for protocol state. Phase 2 adds CP-local policy templates only. Authorised protocol mutation remains a deliberate, audited path through RBAC → policy validation → Signer → PROTO-0 (Phases 3–4). The signer authenticates; the protocol decides. Approved templates are not protocol-active.
