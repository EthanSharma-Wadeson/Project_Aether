# Control Plane Security Model — Index & Signing Roadmap

## Status

**Living security index for Control Plane milestones**

Date: 2026-07-29

Canonical Milestone 1 application threat model and gates:

- [../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md](../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md)

Milestone 2 design companions:

- [CONTROL_PLANE_POLICY_THREAT_MODEL.md](CONTROL_PLANE_POLICY_THREAT_MODEL.md)
- [CONTROL_PLANE_AUTHORITY_MODEL.md](CONTROL_PLANE_AUTHORITY_MODEL.md)
- [CONTROL_PLANE_WRITE_SECURITY.md](CONTROL_PLANE_WRITE_SECURITY.md)

---

## Future Signing Architecture

The Control Plane must never treat the signer as protocol authority. Signing authenticates an already-authorized request; PROTO-0 remains the final validator of state transitions.

See also: [CONTROL_PLANE_AUTHORITY_MODEL.md § Signing Boundary](CONTROL_PLANE_AUTHORITY_MODEL.md).

### Evolution roadmap

```text
MVP
  single enterprise signing identity
  (server-held; behind Signer abstraction)

Production
  isolated signer service
  (CP calls signing service; key material never in API process)

Enterprise
  KMS / HSM-backed keys
  (custody, rotation, attestation)

High assurance
  multi-party approval
  (N-of-M before destructive or high-impact mutations)
```

| Stage | Key custody | Who can request sign | Typical use |
|-------|-------------|----------------------|-------------|
| **MVP** | Single enterprise identity on CP host (file/env), accessed only via `Signer` | RBAC `operator` / `admin` after CSRF | Grant / revoke for demo & early deploy |
| **Production** | Isolated signer service; CP holds no raw key | Same RBAC; network ACL to signer | Reduce blast radius of CP compromise |
| **Enterprise** | KMS/HSM-backed keys; audited unwrap | Same + org IAM binding | Compliance, rotation, hardware root |
| **High assurance** | HSM + policy engine for quorum | Multi-party approval workflow | Freeze, mass revoke, root changes |

### Non-negotiables across all stages

| Rule | Reason |
|------|--------|
| Browser never holds PROTO-0 signing keys | XSS / token theft must not yield protocol authority |
| Business logic depends on `Signer`, not raw keys | Enables KMS/HSM/service swap without redesign |
| Signer success ≠ protocol success | PROTO-0 may still reject |
| Every signer use is audited | `signer_identity` + action + outcome |
| No escrow/settlement/reputation signing via CP | Wrong authority domain |

### Milestone 2 implication

Implementation Phase 3 introduces the **Signer abstraction** before wiring MVP enterprise key material. Phase 4 is the first point at which a concrete signer may participate in PROTO-0 grant/revoke. Multi-party and HSM remain explicitly out of Milestone 2 scope.

---

## Related

- [SECURITY_MODEL.md](SECURITY_MODEL.md) — project-wide security model
- [../Project_Phases/phase_3/CONTROL_PLANE_MILESTONE_2_PLAN.md](../Project_Phases/phase_3/CONTROL_PLANE_MILESTONE_2_PLAN.md)
