# PROTO-0 Security Review vs SECURITY_MODEL.md

## Status

**Phase 1 step 4 — complete (documentation alignment applied; no implementation changes)**

Sources reviewed:

- `Aether_docs/SECURITY_MODEL.md` ← updated from this review
- `Project_Phases/phase_1/PROTO_0_RESULTS.md`
- `Project_Phases/phase_1/PROTO_0_ACCEPTANCE_TESTS.md`
- Phase 0 invariants in `Project_Phases/phase_0/CONTEXT.md`
- PROTO-0 behaviour as implemented in `Aether/core/` (read-only)

Date: 2026-07-28

**Follow-up applied:** required `SECURITY_MODEL.md` sections were written into the security model. PROTO-1 remains unstarted pending explicit approval.

---

## Verdict

PROTO-0 **supports** the identity/capability portions of the security model and does **not** contradict Phase 0 invariants 1–7 or 9–12 within its declared local scope.

It does **not** validate payment, dispute, reputation, network, or settlement claims in `SECURITY_MODEL.md`. Those remain untested and must not be treated as evidenced by PROTO-0.

No blocking security contradiction was found that requires reverting PROTO-0 before review approval.

---

## 1. What PROTO-0 Actually Evidenced

Mapped to `SECURITY_MODEL.md` properties:

| Security property | PROTO-0 evidence | Strength |
|-------------------|------------------|----------|
| Authentication | Ed25519 envelope verify; invalid sig / context mismatch rejected (`P0-A09`, `P0-A10`) | Strong for local signed grants |
| Authorization | Capability envelope + parent narrowing + `authorise_action` gate (`P0-A01`–`A03`, `A11`, `A12`) | Strong for simulated economic gate |
| Integrity (settlement/channel) | Not tested | None |
| Accountability / slashability | Not tested | None |
| Availability / finality | Not tested | None |
| Privacy / selective disclosure | Commitments only; no private payload path exercised | Neutral / out of scope |
| Non-repudiation | Signatures bind grants; no dispute/attestation path | Partial |

Adversary classes exercised in-scope:

- **Malicious agent** — escalation, forged parent, constraint mutation
- **Compromised operational key** — only indirectly (valid signatures misuse still bounded by envelopes); full host-compromise recovery not tested
- **Network / Sybil / cartel / curious counterparty** — not exercised

---

## 2. Assumptions Introduced or Relied Upon by PROTO-0

These are now implicit trust assumptions of the reference verifier and should be made explicit in `SECURITY_MODEL.md` (or a linked Phase 1 note).

1. **Single operational key** is the sole authorising key for identity and capability grants in v0.
2. **Local shared state** — registry, capability store, and revocation map are coherent and honest for the verifier under test.
3. **Logical time `now` is supplied correctly** by the caller; PROTO-0 does not secure wall-clock time.
4. **Current-state validation only** — grants bound to prior `root_version` are invalid after authorised root update; historical validation is out of scope.
5. **Revocation is local** — no distributed gossip/CRL; a verifier without the revoke record may accept a revoked grant.
6. **Acting agent must be capability `subject`** — subject binding is part of the authorisation rule.
7. **Signature proves key control only** — already in Phase 0 invariant 5; PROTO-0 reinforces it (`P0-A12`).
8. **Canonical CBOR field order is protocol-critical** — encoding divergence is treated as failure.
9. **No settlement backend is in the trust path** for identity/capability correctness (supports invariant 10 for this prototype only).
10. **A passing test suite is not a global security proof** — already stated in Phase 1 policy; must remain in the security model’s development norms.

---

## 3. Contradictions and Tensions

### 3.1 Direct contradictions

None found between PROTO-0 behaviour and the letter of `SECURITY_MODEL.md` **within PROTO-0 scope**.

### 3.2 Scope / maturity tensions (require doc clarification, not code rollback)

| Tension | SECURITY_MODEL.md says | PROTO-0 does | Assessment |
|---------|------------------------|--------------|------------|
| Hierarchical keys + rapid rotation | Mitigate compromise via hierarchy + rotation | Single operational key; root update exists; **operational-key rotation not implemented/tested** | Gap vs mitigation language; not a behavioural contradiction yet |
| Emergency freeze for **root holders** | Root-holder freeze path | Identity freeze/revoke on registry entry; no distinct root/recovery key | Narrower than model wording |
| Capability checks prevent parties agreeing to violate principal limits | Payment/dispute section | Simulated `AuthoriseAction` only; no bilateral payment path | Consistent intent; unpaid evidence |
| Off-chain channel safety / watchtowers | Required for channel security | Out of scope | Deferred to PROTO-1+ |
| Slash for revoked/fraudulent state | Economic accountability | Revoke rejects action; no slash | Model ahead of implementation |
| Agents verify capabilities and proofs; do not trust free-text | Trust assumption | Enforced for capability grants | Aligned |
| Fail closed on unknown proof types | Verification security | Malformed/unknown critical objects reject; no attestation proof types yet | Compatible; incomplete coverage |

### 3.3 Invariant review (Phase 0)

| Invariant | PROTO-0 status |
|-----------|----------------|
| 1 Envelope limits | Evidenced |
| 2 Delegation ≤ parent | Evidenced |
| 3 Expiry/revoke | Evidenced |
| 4 No settlement from identity alone | Evidenced as identity-only bypass rejection |
| 5 Signature ≠ truth | Evidenced |
| 6 History across key rotation | **Partially** — AgentId stable across permission-root update; **operational-key rotation not evidenced** |
| 7 Deterministic transitions | Evidenced locally |
| 8 Dispute reproducibility | Out of scope |
| 9 Commitment ≠ disclosure | Not stressed; no private evidence path |
| 10 Backend failure isolation | Vacuously held (no backend) |
| 11 Capability before economic auth | Evidenced |
| 12 Child bounded by parent | Evidenced |

**Important:** Do not treat invariant 6 as fully validated by PROTO-0.

---

## 4. Required SECURITY_MODEL.md Updates

Recommended documentation updates (not implementation changes):

### 4.1 Add a “Validated by PROTO-0” subsection

State clearly what is evidenced:

- signed capability grants
- fail-closed authorisation before simulated economic action
- non-escalating delegation
- expiry, explicit revoke, identity freeze/revoke
- stale permission-root rejection
- identity-alone and secondary-signature bypass rejection

State clearly what is **not** evidenced:

- channels, disputes, watchtowers
- settlement backends
- reputation anti-wash
- distributed revocation
- operational-key rotation / key hierarchy
- privacy selective-disclosure flows

### 4.2 Narrow or stage the Key Risk Mitigations language

Update mitigations to distinguish **v0 / PROTO-0** from later stages:

- v0: single operational key + capability envelopes + identity freeze/revoke + root-version invalidation
- later: hierarchical keys, rapid operational-key rotation, hardware custody, distributed revocation

### 4.3 Add explicit trust assumptions from PROTO-0

Add under Trust Assumptions:

- verifiers share coherent local revocation/root state, or fail closed when state is missing
- logical time is an external input with its own integrity requirements
- current-state authority validation is the default; historical validation is a separate mode
- capability subject binding is mandatory for action authorisation

### 4.4 Clarify signature semantics in Security Properties / Verification

Make invariant 5 first-class in `SECURITY_MODEL.md`:

> A valid signature authenticates key control and message integrity under a domain-separated context. It does not authenticate output correctness, task quality, or policy compliance by itself.

### 4.5 Add a Capability / Identity Authorization section

`SECURITY_MODEL.md` currently folds this into generic Authorization and Payment sections. PROTO-0 shows it deserves its own short section covering:

- verification order (authenticate → identity status → root binding → capability semantics → economic gate)
- child-narrowing rule
- revoke / freeze / stale-root fail-closed behaviour

### 4.6 Record open security questions raised by PROTO-0

Add to Open Questions:

- How is revocation distributed so verifiers cannot miss a revoke?
- What is the authoritative clock / logical-time source for expiry?
- When is historical (non-current-root) capability validation allowed?
- When must operational-key rotation be mandatory vs optional?
- Formal verification targets for capability narrowing and authorise pipeline (already partially listed)

---

## 5. Risks Discovered (No Code Change Required Now)

1. **Revocation completeness risk** — local revoke store implies false accepts under partitioned verifier state.
2. **Time integrity risk** — expiry security collapses if `now` is attacker-controlled.
3. **Key-hierarchy gap** — compromised operational key can still issue grants until freeze/revoke/root update; blast radius is envelope-limited but not hierarchy-limited.
4. **Invariant 6 overclaim risk** — root-version continuity ≠ operational-key rotation evidence.
5. **Subject-binding policy** — current rule is strict; multi-key / delegated-subject presentation may need explicit security-model language before PROTO-1 payment flows.

None of these are PROTO-0 test failures. They are documentation and Phase 1+ design risks.

---

## 6. Recommendation on Phase 1 Gate

**PROTO-0 is approved for security-review purposes** as an identity/capability primitive.

Documentation contingencies from this review are now applied in `SECURITY_MODEL.md`:

1. Validated / Not Yet Validated sections
2. PROTO-0 trust assumptions
3. signature semantics invariant
4. capability authorization rules
5. open questions for revoke distribution, clock source, historical validation, and key rotation

Carry-forward rules remain:

- do not treat PROTO-0 as evidence for payment/channel/dispute security
- keep revocation distribution, time semantics, and operational-key rotation as PROTO-1 / Phase 1 review triggers

**PROTO-1 is unblocked from a documentation/security-alignment perspective**, but must not start until explicitly approved.
