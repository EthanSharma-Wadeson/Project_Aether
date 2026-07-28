# 04 — Agents, Identity & Permissions

**Key takeaway:** An Aether agent is an autonomous software actor with keys and a permission envelope — not an LLM. Identity is cryptographic and non-sovereign. Permissions are hard constraints enforced by the protocol, not suggestions.

---

## What is an agent on Aether?

An **agent** is software that:

1. Controls cryptographic keys (directly or via constrained custodian)
2. Holds an Aether identity with a permission root
3. Can send/receive protocol messages without human approval for standard ops
4. May earn, spend, bond, and accumulate reputation under protocol rules

**Not an agent:** An LLM, chat UI, or specific model vendor. Those live above the protocol.

---

## Agent properties

| Property | Meaning |
|----------|---------|
| Autonomy | Can propose actions and payments within permissions |
| Accountability | Actions tied to identity + keys; bonds and reputation attach |
| Ephemerality | May be short-lived; identity history can outlive the process |
| Multiplicity | One principal may operate many agents; Sybil cost applies |
| Heterogeneity | Different models, tools, runtimes — protocol stays model-agnostic |

---

## Roles in the system

| Role | Who |
|------|-----|
| **Principal** | Funds bonds and sets root policy (human org, DAO, or parent agent) |
| **Agent** | Operational actor with identity and scoped capabilities |
| **Counterparty** | Another agent in a bilateral flow |
| **Verifier** | Party or network rule that checks attestations/proofs |
| **Indexer** | Reads events; may compute non-canonical reputation views |

---

## Agent lifecycle

```
Provision keys → Register identity → Post bond → Receive capabilities
    → Discover / negotiate → Act + pay → Attest → Settle
    → Rotate / rebond / retire
```

### Provisioning
- Keys generated in secure runtime (HSM, enclave, hardened host)
- Identity registration commits to public keys and permission root

### Operation
- Agent pursues local goals under capability constraints
- All economic actions pass permission + payment checks

### Retirement
- Close channels, release or slash bonds per rules
- Identity may remain for historical attribution; spend capabilities revoked

---

## Identity system

### Design goals

| Goal | Meaning |
|------|---------|
| Non-sovereign | Not issued by nation-state or single platform |
| Cryptographic | Control proven by keys; delegation is explicit and auditable |
| Programmable | Permission roots and capabilities bind to identity |
| Machine-native | Registration, rotation, presentation are API operations |
| Stable under ephemerality | Process death doesn't destroy attributable history |

### Identity object (conceptual fields)

| Field | Purpose |
|-------|---------|
| Agent ID | Derived from public key material — collision-resistant |
| Public keys | Auth, encryption, channel keys (by purpose) |
| Permission root | Root of the capability tree |
| Metadata commitment | Optional, supports selective disclosure |
| Status | Active, Frozen, or Revoked |

### Key hierarchy

| Key class | When used |
|-----------|-----------|
| Root / recovery | Rarely — rotation and high-impact revocation |
| Operational | Day-to-day protocol message authentication |
| Capability-scoped | Delegated keys limited by envelopes |
| Channel keys | Optional bilateral keys for high-volume payment updates |

**Containment:** Compromise of operational keys should be fixable via rotation without burning historical reputation.

---

## Delegation

- Principals delegate to agents; agents may sub-delegate within depth limits
- Delegations are signed, expirable, and revocable
- Verifiers check the full chain back to permission root
- Delegation ≠ identity transfer

---

## Registration flow

1. Generate key hierarchy
2. Commit to permission root and optional metadata
3. Submit identity registration with anti-spam bond/fee
4. Network includes identity in canonical state
5. Agent may publish discovery hints off-protocol (keyed by Agent ID)

---

## When acting, an agent presents

- Agent ID
- Signature under an authorized key
- Capability proof satisfying the action
- Optional selectively disclosed attributes

---

## Relationship to legal identity

Aether identity is **orthogonal** to KYC. Optional bridges may exist (e.g. "bonded by licensed entity X") without making legal ID the root.

---

## Permission envelopes (capabilities)

Permissions are **hard constraints**, not hints:

| Constraint type | Example |
|-----------------|---------|
| Spend ceilings | Max 0.01 per transaction |
| Rate limits | Max 100 payments/hour |
| Allowed methods | Only specific contract surfaces |
| Allowed counterparties | Only agents above reputation threshold |
| Time windows | Valid only between dates |
| Required attestations | Funds release only after proof type X |

### What agents can do economically
- Open and update payment channels
- Lock value in escrow conditional on attestations
- Post bonds for anti-spam and dispute skin-in-the-game
- Accumulate reputation from protocol events

### What agents cannot do (by design)
- Bypass capability checks via raw signatures alone
- Require humans to click-approve each micro-payment

---

## Sybil considerations

Identity creation is **easy by design**. Economic and reputation costs make Sybil swarms expensive:

- Registration may require minimum bond
- New IDs start "cold" — no trust history
- Reputation gated by stake-weighted evidence

---

## Trust model for agent behavior

The protocol assumes agents may be:

- Rational economic attackers
- Buggy or partially compromised
- Colluding in reputation or wash-trade schemes

**Therefore:** Reliability comes from bonds, escrow, verification, and reputation — not from assuming aligned LLMs.

**Next:** [05-payments-and-escrow.md](05-payments-and-escrow.md)
