# roadmap/phase_2.md

## Phase 2 — Networked Settlement & Economic Safety

**Theme:** Move from simulators to networked settlement; introduce bonds/fees that make spam and cheap fraud unprofitable; harden disputes and watchtower assumptions.

**Entry:** Phase 1 exit criteria met.

## Objectives

1. Implement channel open/settle/dispute against chosen settlement backend(s)  
2. Ship bonding + fee metering for registration and disputes  
3. Run PROTO-3–PROTO-4 (reputation adversary tests; backend comparison → selection)  
4. Define attestation type registry (receipts required; TEE/ZK optional slots)  
5. Light-client / inclusion proof path for agents verifying settlements  

## Scope In

- Production-shaped channel endpoints (still developer/agent-facing APIs)  
- Bond lock/slash/release flows  
- Watchtower or principal-mirror reference design  
- Reputation scorer v1 with published attack ROI results  
- Selective disclosure spike if reputation thresholds are required for payments  
- Interop notes for external assets as bonds (if any)  

## Scope Out

- Mass retail consumer wallet UX  
- Complex governance token politics  
- Guaranteed ZK proving for arbitrary ML  
- Fully decentralized discovery at global scale (may remain app-layer)  

## Exit Criteria

- [ ] Headless agents settle micro-payments with measured soft/hard finality  
- [ ] Dispute path recovers funds under crash tests  
- [ ] H1 and H5 supported or design revised with evidence  
- [ ] H3 mitigations documented; scorer parameters justified  
- [ ] Economic parameter sheet (bonds/fees/windows) versioned  
- [ ] Security review of payment + bond paths  

## Key Risks

| Risk | Mitigation |
|------|------------|
| Backend fee spikes break agent loops | Batching, prepaid tickets, multi-backend option |
| Watchtower missing → silent fund loss | Default mirrors; documented offline assumptions |
| Reputation farming | Bonded evidence only; adversary tests |

## Exit Artifacts

- Testnet or equivalent public agent endpoints  
- Parameter sheet v1  
- Updated WHITEPAPER + ECONOMIC_MODEL reflecting reality  

## Next

→ [phase_3.md](phase_3.md)
