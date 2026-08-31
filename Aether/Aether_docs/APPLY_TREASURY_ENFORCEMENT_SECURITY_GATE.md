# Apply ↔ Treasury Enforcement Security Gate — Phase 25 Verdict

| Field | Value |
|-------|--------|
| **Document** | `APPLY_TREASURY_ENFORCEMENT_SECURITY_GATE.md` |
| **Phase** | 25 — Capability Treasury Enforcement Design Gate |
| **Date** | 2026-08-01 |
| **Scope** | Design & security review of post-drift enforcement |
| **Non-goals** | Code, Apply enablement, PROTO-0 mutations, treasury write changes |
| **Companion** | [APPLY_TREASURY_ENFORCEMENT_MODEL.md](APPLY_TREASURY_ENFORCEMENT_MODEL.md) |

---

## Executive Summary

After Phase 24 drift detection, Aether must **deny unsafe spend at runtime** and **remediate only through human-governed paths** (treasury write façade and/or Apply). Automatic capability mutation is rejected.

### Gate decision

# **PASS WITH CONDITIONS**

| Option | Result |
|--------|--------|
| PASS | — |
| **PASS WITH CONDITIONS** | **Selected** |
| FAIL | — |

**Meaning:** Enforcement model is sound and consistent with Hybrid D+B and INV-S01…S08. **Implementation is not authorised** until conditions below are closed in a separate phase. **Apply enablement is not authorised by this gate.**

---

## 1. Recommended enforcement model

**Layered E2 + E1 + E3(proposal-only)**

| Layer | Decision |
|-------|----------|
| **E2 Runtime denial without mutation** | **Required** continuous control |
| **E1 Human approval workflow** | **Required** for remediation |
| **E3 Capability reduction proposal** | **Allowed** as non-executing draft → Apply |
| **E4 Automatic capability mutation** | **Forbidden** |

Severity: **INFO / WARNING / CRITICAL** with max-escalation; CRITICAL defaults to DENY + dual-control remediation.

Lifecycle: **Detection → Decision → Approval → Execution → Audit** with full correlation ids.

---

## 2. Security invariants

| ID | Invariant |
|----|-----------|
| **INV-E01** | No silent permission changes |
| **INV-E02** | No automatic money movement from enforcement |
| **INV-E03** | No bypassing PROTO-0 (Apply-only protocol mutation) |
| **INV-E04** | Complete audit trail across lifecycle stages |
| **INV-E05** | Recoverability preferred over silent heal |
| **INV-E06** | Fail closed on authority/read unavailability |
| **INV-E07** | No silent CRITICAL downgrade |
| **INV-E08** | Enforcement never sets `apply_enabled=true` |

Plus inherited: **INV-S01…S08**.

---

## 3. Option security / ops / compliance verdicts

| Option | Security | Ops | Compliance | Gate |
|--------|----------|-----|------------|------|
| E1 Human workflow | Pass | Accept latency | Pass | **Adopt** |
| E2 Runtime deny | Pass | Clear DENY codes needed | Pass | **Adopt** |
| E3 Reduction proposal | Pass if non-executing | Pass | Pass | **Adopt (proposal-only)** |
| E4 Auto mutation | **Fail** | Opaque | **Fail** | **Reject** |

---

## 4. Implementation blockers

| ID | Blocker | Severity |
|----|---------|----------|
| **E-C1** | Separate **implementation approval** for spend-gate (E2) + case workflow (E1) | Blocking |
| **E-C2** | Specify spend-gate placement (CP library vs agent runtime) and DENY reason taxonomy | Blocking |
| **E-C3** | Map Phase 24 findings → INFO/WARNING/CRITICAL in code/config (policy table) | Blocking |
| **E-C4** | Bind enforcement to **remaining**, not ceiling-only (exhaustion) | Blocking |
| **E-C5** | Case store + SLA + dual-control rules for CRITICAL | Blocking for E1 |
| **E-C6** | E3 proposals must reuse Apply approval/replay gates — no side door | Blocking for E3 execute |
| **E-C7** | Production ban on `lab_unfunded_waiver` | Blocking for prod |
| **E-C8** | TOCTOU / concurrent allocation+capability change test plan | Blocking |
| **E-C9** | UX: never offer one-click “auto-heal capability” | Blocking |
| **E-C10** | Asset mismatch handling (`AETHER_TEST` vs funded assets) documented in DENY codes | Blocking for honest ops |

Non-blocking residuals: hash-chained audit, HSM, paging integrations.

---

## 5. Enforcement vs Apply enablement

| Question | Decision |
|----------|----------|
| Should **runtime enforcement (E2)** happen **before** Apply enablement? | **Yes** — preferred. Protects spend paths while Apply remains off. |
| Should **capability remediation execution (E3→Apply)** happen **before** Apply enablement? | **No** — proposals only; execution requires Apply enablement for revoke/reduce. |
| Should enforcement design wait until after Apply is on? | **No** — delaying E2 leaves drift visible but spend possibly unconstrained in product paths. |
| Does this gate enable Apply? | **No.** |

```text
Observation (24) ──► Enforcement design (25) ──PASS WITH CONDITIONS──►
    E2 + E1 implementation (Apply still off) ──►
        Apply enablement (separate gate; freeze then spend) ──►
            E3 proposals may execute via Apply
```

**Normative:** Implement and prove **E2 before** production Apply enablement for spend CapabilityGrant. E1 can land in parallel. E3 execute only after Apply path exists.

---

## 6. Conditions checklist

| Criterion | Met? |
|-----------|------|
| Post-drift options evaluated | **Yes** |
| Recommended model selected | **Yes** (E2+E1+E3 proposal) |
| E4 rejected with rationale | **Yes** |
| Severity model defined | **Yes** |
| Lifecycle defined | **Yes** |
| Security requirements INV-E01…E08 | **Yes** |
| Apply sequencing answered | **Yes** |
| Implementation blockers listed | **Yes** |
| Code / Apply / PROTO-0 / treasury writes in this phase | **No** (correct) |

---

## 7. Residual risks (accepted until impl)

1. Phase 24 observation alone does not stop spend — **E2 not yet built**.
2. Without Apply, overscoped grants can only be **contained** (DENY + treasury freeze), not **removed** from PROTO-0.
3. Multi-cap/multi-alloc heuristics may mis-pair — enforcement must prefer fail-closed.
4. Operators may confuse DENY with “system broken” — needs clear reason codes and console copy.

---

## 8. Freeze statement

Enforcement recommendation, invariants INV-E01…E08, severity model, and Apply sequencing are **frozen**.

**This gate does not authorise implementation, Apply enablement, PROTO-0 mutation, or treasury write changes.**

**STOP.**
