# roadmap/phase_1.md

## Phase 1 — Primitives & Threat Foundations

**Theme:** Make the smallest honest stack real: identity, capabilities, channels, receipts, and a written threat model. No speculative token launch as a milestone.

## Objectives

1. Freeze mission and non-goals ([CONTEXT.md](../CONTEXT.md))  
2. Specify v0 message types and state machines ([PROTOCOL_DESIGN.md](../PROTOCOL_DESIGN.md))  
3. Deliver prototypes PROTO-0–PROTO-2 (identity/capabilities, channel sim, escrow+receipts)  
4. Record security and prototype findings needed to frame Phase 2 settlement work  
5. Publish security model v0 and open-question list hygiene  

## Scope In

- Agent identity register/rotate/delegate (logical + reference impl)  
- Capability envelopes with spend/rate/counterparty limits  
- Bilateral channel state machine (sim only; no live backend integration)  
- Receipt-based escrow flow  
- Non-canonical reputation indexer experiments (not consensus-canonical yet)  
- Documentation suite consistency  

## Scope Out

- Native token generation events  
- ZKML production proving  
- Full discovery marketplace product  
- Human chat/agent UI as protocol deliverable  
- Multiparty channel mesh at scale  

## Exit Criteria

- [ ] H2 demonstrated in tests (capabilities contain loss)  
- [ ] Channel lifecycle including dispute path simulated with metrics  
- [ ] Escrow+receipt loop completes headlessly  
- [ ] `unanswered_questions.md` updated; critical path questions owned  
- [ ] SECURITY_MODEL reviewed against PROTO-0–PROTO-2 findings  

## Key Risks

| Risk | Mitigation |
|------|------------|
| Scope creep into app/agent framework | Enforce non-goals |
| Premature tokenomics | Phase gate: infra metrics first |
| Soft/hard finality confusion | Explicit API fields + H6 experiments |

## Exit Artifacts

- Spec freeze notes for identity, capabilities, channels  
- Prototype metrics reports  
- Phase 2 engineering plan  

## Next

→ [phase_2.md](phase_2.md)
