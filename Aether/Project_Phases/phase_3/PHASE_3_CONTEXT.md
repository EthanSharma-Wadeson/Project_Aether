# Phase 3 — Control Plane Architecture

## Status

**Architecture design — not implementation**

Date: 2026-07-29

---

## Why the Control Plane Exists

Phases 1 and 2 built the protocol primitives for autonomous agent economics:

| Layer | What it does | Phase |
|-------|-------------|-------|
| PROTO-0 | Identity, capabilities, authorisation | 1 |
| PROTO-1 | Bilateral state channels | 1 |
| PROTO-2 | Escrow and deterministic settlement | 1 |
| PROTO-NET-0 | Authenticated transport | 2 |
| PROTO-4 | Settlement binding and finality evidence | 2 |
| PROTO-3 | Evidence-based reputation | 2 |

These layers run autonomously. No human interface exists.

Enterprises cannot govern agents they deploy. Researchers cannot observe agent behaviour. Developers cannot debug protocol interactions.

The Control Plane solves this: a human-facing interface that **observes and configures policy** without becoming a protocol authority.

---

## Phase 2 Completion Summary

| Component | Status |
|-----------|--------|
| PROTO-0 identity + capabilities | Implemented, security reviewed |
| PROTO-1 bilateral channels | Implemented, security reviewed |
| PROTO-2 escrow + settlement | Implemented, security reviewed |
| PROTO-NET-0 transport | Implemented, security reviewed |
| PROTO-4 settlement binding | Implemented, security reviewed (APPROVE WITH DOCUMENTED LIMITATIONS) |
| PROTO-3 reputation | Implemented, security reviewed (APPROVE WITH DOCUMENTED LIMITATIONS) |
| Enterprise Spend Control demonstrator | Complete (4/4 scenarios) |
| AETH economic architecture | Design only — no implementation |

All 311 protocol tests pass. The protocol stack is feature-complete for Phase 3 design.

---

## Problem Being Solved

Autonomous agents operate through signed protocol interactions. Humans need to:

1. **See** what agents are doing — tasks, spending, disputes, reputation
2. **Configure** what agents are allowed to do — spend limits, action scopes, expiry windows
3. **Investigate** when things go wrong — settlement failures, disputes, capability denials
4. **Govern** agent fleets — compliance visibility, audit trails, policy enforcement

Without a Control Plane, the only interface is raw protocol stores.

---

## Non-Goals

The Control Plane is **not**:

- A protocol authority layer (PROTO-0 remains sole authority)
- A marketplace or agent directory
- A social reputation system
- A governance DAO or voting mechanism
- A token incentive system
- A production cloud deployment architecture
- A frontend implementation

The Control Plane **does not**:

- Bypass capability checks
- Modify escrow balances directly
- Change settlement status
- Fabricate reputation events
- Override dispute outcomes
- Replace PROTO-0 authorisation

---

## Success Criteria

Phase 3 architecture is complete when:

1. All design documents exist and are internally consistent
2. Authority boundaries are explicit — Control Plane cannot escalate to protocol authority
3. Enterprise operator, researcher, and developer use cases are mapped to concrete views
4. Data flow (read paths and write paths) is documented
5. Threat model classifies risks as VALIDATED / OPEN
6. Implementation scope is frozen with acceptance criteria

**STOP** after architecture review. No Rust, no frontend, no APIs until explicit approval.

---

## Authority Invariant

```text
Human / Enterprise
        │
        ▼
  Control Plane (observe + configure policy)
        │
        ▼
  PROTO-0  →  capability issuance (the ONLY write path)
        │
        ▼
  PROTO-1/2/4/3/NET-0  (protocol execution)
```

The Control Plane's **only write path** to the protocol stack is through PROTO-0 capability issuance. It cannot write to escrow, settlement, channel, or reputation stores.

---

## Related

- [Phase 2 README](../phase_2/README.md)
- [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md)
- [CONTROL_PLANE_ARCHITECTURE.md](CONTROL_PLANE_ARCHITECTURE.md)
- [CONTROL_PLANE_THREAT_MODEL.md](CONTROL_PLANE_THREAT_MODEL.md)
- [CONTROL_PLANE_BUSINESS_CASE.md](CONTROL_PLANE_BUSINESS_CASE.md)
