# Admin MFA — Pilot Requirement & Roadmap

| Field | Value |
|-------|--------|
| **Document** | `TREASURY_ADMIN_MFA.md` |
| **Phase** | 22.5 |
| **Date** | 2026-08-01 |

---

## Boundary (implemented)

Config: `CP_ADMIN_MFA_REQUIRED` (default `false`).

When `true`, **admin** treasury mutating / ops routes require:

`X-Aether-MFA-Verified: true`

Enforced in:

- Dual-control create / approve / execute (admin actors)
- `/api/treasury/ops/sweep`, `/reconcile/run`, force-release

This is an **enforcement boundary**, not a built-in TOTP/WebAuthn implementation.

---

## Pilot requirement

Before any enterprise pilot that treats the ledger as a financial record of record:

1. Set `CP_ADMIN_MFA_REQUIRED=true`.  
2. Terminate MFA at the IdP / reverse proxy and inject the verified header (or map to future JWT `mfa_verified` claim).  
3. Do not rely on browser-only MFA without server check.

Lab may leave the flag `false`.

---

## Production roadmap (not Phase 22.5)

| Stage | Work |
|-------|------|
| Near-term | SSO/OIDC with ACR / `amr` claims → JWT `mfa_verified` |
| Mid-term | Step-up MFA for freeze / force-release / adjust |
| Later | HSM/KMS for Apply signer (separate from treasury ledger MFA) |

**Out of scope here:** HSM/KMS, embedding TOTP secrets in Control Plane.

---

## Related

`control_plane/src/treasury/ops/mfa.rs`
