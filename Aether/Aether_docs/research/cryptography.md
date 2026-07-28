# research/cryptography.md

## Focus

Cryptographic building blocks for identity, capabilities, payments, attestations, and selective disclosure.

## Topics

### Signatures & key agility

- Primary signature suite for agent messages
- Multi-key identity commitments and rotation proofs
- Delegation chains with bounded depth and efficient verification

### Capabilities

- Encoding of constraints (spend, rate, counterparty, method)
- Macaroons / UCAN / object-capability comparisons
- Revocation: CRLs, short expiries, status lists, or accumulator-based schemes

### Channel authentication

- Efficient signing of high-frequency updates
- Optional aggregated or precomputed session keys
- Binding channel keys to capability envelopes

### Attestation & execution integrity

| Family | Research questions |
|--------|-------------------|
| Receipts / hashing | What is sufficient for v1 escrow release? |
| TEEs | Attestation verification, side channels, vendor trust |
| ZK / ZKML | Cost, latency, circuit stability for agent tasks |
| Optimistic + fraud proofs | Challenge games for nondeterministic LLM outputs |

### Selective disclosure

- Commitments to metadata and reputation thresholds
- Predicate proofs (“score ≥ θ”, “bonded ≥ β”)
- Avoiding overdisclosure in attestation payloads

### Privacy-enhancing networking (optional stretch)

- Whether encrypted transport / mix routing is in-protocol or layered above

## Working Hypotheses

1. v1 should standardize **message auth + capabilities + receipt/escrow**, with proof backends pluggable.
2. Nondeterministic LLM outputs need careful “what is being proven” definitions — bit-identical replay is often the wrong goal.
3. Short-lived capabilities beat complex long-lived authority with weak revocation.

## Exit Criteria

- Chosen crypto suite + agility policy
- Capability verification complexity budget for agents
- Attestation type registry design (versioned proof kinds)

## Links

- [IDENTITY_SYSTEM.md](../IDENTITY_SYSTEM.md)
- [PRIVACY_MODEL.md](../PRIVACY_MODEL.md)
- [SECURITY_MODEL.md](../SECURITY_MODEL.md)
- [unanswered_questions.md](unanswered_questions.md)
