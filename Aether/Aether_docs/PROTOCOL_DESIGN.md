# PROTOCOL_DESIGN.md — Project Aether

## Purpose

Specify how Aether components speak: message types, state machines, identifiers, and verification rules. This document is the bridge between [ARCHITECTURE.md](ARCHITECTURE.md) and implementation.

**Status:** Design draft. Fields marked `TBD` await research lock.

## Design Goals

- Deterministic, machine-parseable messages (no human-oriented encoding as primary)
- Explicit versioning and capability negotiation
- Fail-closed permission checks
- Cheap authentication of high-frequency updates; expensive consensus only when required

## Identifier & Encoding Conventions

| Item | Convention (draft) |
|------|---------------------|
| Agent ID | Cryptographic commitment to public key material + identity schema version |
| Message ID | Hash of canonical serialization |
| Serialization | Canonical binary or deterministic CBOR/JSON subset — TBD |
| Time | Logical timestamps + wall-clock bounds for expiry; agents must not rely on UI clocks alone |
| Signatures | Modern signatures (e.g. Ed25519 / equivalent); scheme agility via typed envelopes |

## Core Message Families

### Identity

- `IdentityRegister` — publish agent ID, keys, permission root commitment
- `IdentityRotate` — rotate keys under existing ID policy
- `IdentityDelegate` — issue scoped capability to another key/agent
- `IdentityRevoke` — revoke delegation or mark key compromised

### Permissions

- `CapabilityGrant` — scoped allowances (methods, limits, expiry)
- `CapabilityProve` — present capability + proof of validity for an action
- `CapabilityRevoke`

Permission evaluation is mandatory before payment, settlement, and attested action acceptance.

### Payments

- `ChannelOpen` — participants, assets, initial deposits, dispute window
- `ChannelUpdate` — signed off-chain balance/state update
- `ChannelSettle` — submit latest state for on-protocol finalization
- `ChannelDispute` — challenge with supporting evidence within window

### Attestation & Verification

- `AttestationSubmit` — claim about work/outcome with proof payload
- `AttestationVerify` — verification result (local or network-acknowledged)
- `DisputeOpen` / `DisputeResolve` — challenge attested claims tied to escrow

### Reputation

- `ReputationCommit` — checkpoint of scored events or aggregate
- `ReputationQuery` — request machine-readable score vector (may be local index)

## State Machines (Summary)

### Channel lifecycle

```
Open → Active ⇄ Updating → Closing → Settled
                ↘ Disputed → Settled / Slashed
```

### Identity lifecycle

```
Unregistered → Registered → Active
                    ↘ Rotating → Active
                    ↘ Frozen / Revoked (policy)
```

### Escrowed task (application-facing, protocol-supported)

```
Offer → Accepted → InProgress → Attested → Released
                              ↘ Disputed → Resolved
```

## Permission Envelope (Logical Schema)

`CapabilityV0` is the unsigned semantic authority object. The authoritative signature belongs to the transport envelope that carries it.

```text
CapabilityV0 {
  issuer: AgentId,
  subject: KeyOrAgentId,
  actions: [ActionSelector],
  constraints: {
    max_spend, asset, counterparties, rate_limit,
    valid_after, valid_before, require_attestation_type
  },
  delegation_depth: u32,
  parent_capability_id: Optional<CapabilityId>
}

CapabilityGrant = SignedMessage<CapabilityV0>
```

The protocol must not treat a duplicated inline capability-signature field as separately authoritative. Trust in capability semantics begins only after reconstructing the canonical signed body and validating the enclosing signed message.

Agents MUST reject actions that fail envelope checks even if signatures on payments are valid.

## Verification Pipeline

1. Authenticate message signatures  
2. Check identity status (not revoked/frozen)  
3. Evaluate capability envelope  
4. Validate economic preconditions (bond, channel state, fees)  
5. Verify attestation/proof if required  
6. Apply state transition; emit events for reputation indexers  

## Consensus Touchpoints

Only these (draft) require global ordering / finality:

- Identity registration and high-impact revocation
- Channel open/settle/dispute resolution
- Bond lock/slash/release
- Reputation checkpoints that are protocol-canonical (if any)

High-frequency `ChannelUpdate` and local attestation verification stay off the hot consensus path where possible.

## Versioning & Compatibility

- Every envelope carries `protocol_version` and `feature_flags`
- Unknown critical fields → reject; unknown optional fields → ignore per version policy
- Breaking changes require version bump and dual-run window

## Explicit Non-Goals

- Defining LLM prompts or agent frameworks
- Mandating a single smart-contract VM in v1
- Human-readable transaction narratives as consensus-critical data

## Open Protocol Questions

See `research/unanswered_questions.md` and subsystem docs for:

- Exact serialization and crypto suite
- Dispute window parameters vs. latency SLOs
- Whether reputation aggregates are consensus-canonical or indexer-derived
