# IDENTITY_SYSTEM.md — Project Aether

## Purpose

Provide cryptographic, non-sovereign identifiers for autonomous agents — portable across applications, enforceable with permissions, and independent of legal-person KYC by default.

## Design Goals

- **Non-sovereign:** Identity is not issued by a nation-state or single platform as root of trust
- **Cryptographic:** Control proven by keys; transfer/delegation is explicit and auditable
- **Programmable:** Permission roots and capabilities bind to identity
- **Machine-native:** Registration, rotation, and presentation are API operations
- **Stable under ephemerality:** Process death does not destroy attributable identity history

## Non-Goals

- Replacing legal identity for regulated human finance
- Social usernames as consensus-critical identity
- Mandatory doxxing of agent operators

## Identity Object (Logical)

```text
AgentIdentity {
  id: AgentId,                    // commitment / derived ID
  schema_version: u32,
  public_keys: [KeyedPurpose],    // auth, encryption, etc.
  permission_root: Commitment,    // root of capability tree
  metadata_commitment: Optional,  // selective disclosure friendly
  registered_at: ConsensusTime,
  status: Active | Frozen | Revoked
}
```

`AgentId` is derived from canonical public material so that IDs are collision-resistant and transferable only via protocol rules — not by “knowing a handle.”

## Key Hierarchy

| Key class | Role |
|-----------|------|
| Root / recovery | Rarely used; rotation and high-impact revocation |
| Operational | Day-to-day authentication of protocol messages |
| Capability-scoped | Delegated keys limited by envelopes |
| Channel keys | Optional bilateral keys for payment update volume |

Compromise of operational keys MUST be containable via rotation and capability revocation without necessarily burning historical reputation attribution (policy TBD).

## Delegation

Principals delegate to agents; agents may sub-delegate within `delegation_depth` and envelope constraints.

- Delegations are signed, expirable, and revocable
- Verifiers check the full chain to permission root
- Delegation does not equal identity transfer

## Registration Flow

1. Generate key hierarchy  
2. Commit to permission root and optional metadata  
3. Submit `IdentityRegister` with anti-spam bond / fee  
4. Network includes identity in canonical state  
5. Agent may publish discovery hints off-protocol (DHT, directory) keyed by `AgentId`

## Presentation

When acting, an agent presents:

- `AgentId`
- Signature under an authorized key
- Capability proof satisfying the action
- Optional selectively disclosed attributes (see [PRIVACY_MODEL.md](PRIVACY_MODEL.md))

## Relationship to Legal Identity

Aether identity is **orthogonal** to KYC. Bridges to regulated identity may exist as optional attestations (e.g. “this agent is bonded by licensed entity X”) without making legal ID the root.

## Sybil Considerations

Identity creation is easy by design; **economic and reputation costs** make Sybil swarms expensive. Registration may require minimum bond (see [ECONOMIC_MODEL.md](ECONOMIC_MODEL.md)).

## Cross-Chain / Multi-Backend Portability

Aether identity is independent of any single settlement address. Settlement accounts bind to an Agent ID via authorised `SettlementBinding` capabilities (backend, account, key, scope, expiry). See [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) §12.

## Open Questions

- Exact ID derivation and multi-key commitment scheme
- Recovery UX for principals without undermining agent autonomy
- Cross-chain / cross-domain identity portability format → **SettlementBinding model decided; exact wire format TBD**
- Whether display names exist only in off-chain discovery layers

## Related

- [AI_AGENT_MODEL.md](AI_AGENT_MODEL.md)
- [PROTOCOL_DESIGN.md](PROTOCOL_DESIGN.md)
- [SECURITY_MODEL.md](SECURITY_MODEL.md)
