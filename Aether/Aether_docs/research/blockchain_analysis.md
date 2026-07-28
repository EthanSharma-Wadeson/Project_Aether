# research/blockchain_analysis.md

## Focus

Analyze existing blockchain and settlement designs for fitness as agent infrastructure — what to adopt, wrap, or avoid.

## Evaluation Rubric

For each system/class, score:

- Latency to soft and hard finality  
- Fee predictability for automated agents  
- Native support for channels / streaming value  
- Identity beyond addresses  
- Programmability of escrow and disputes  
- Operational complexity for headless agents  
- Speculative culture risk (does using it force token narrative?)  

## Classes Under Review

### Monolithic L1s

High throughput claims vs. actual fee variance and state bloat. Suitability as settlement certificates vs. primary UX.

### Optimistic & ZK rollups

Settlement latency (challenge periods) vs. ZK prove times. Cost of many tiny on-chain events — pressure toward batching and channels.

### Appchains / Cosmos-style zones

Sovereignty and custom message types vs. fragmented security and liquidity.

### Bitcoin & Lightning-class networks

Battle-tested channels; scripting limits; agent identity integration gaps; operational watchtower lessons.

### Account abstraction & session keys (EVM)

Useful for reducing HITL signing — still often framed as human wallet UX; capability richness varies.

### Intent-based and solver networks

Interesting for agent negotiation; may inform discovery/matching without being full Aether.

## Questions to Answer Empirically

1. Cost of N channel settles/day under candidate bases  
2. Time-to-hard-finality distributions (p50/p95)  
3. Failure modes when agent bursts collide with fee spikes  
4. Light client / proof verification cost on agent hardware profiles  

## Adoption Posture (Draft)

- Prefer **primitives-first design** that can settle onto more than one backend if needed  
- Reject “we are an L2” as a substitute for agent identity/reputation/payment design  
- Document honest soft vs hard finality whenever marketing latency  

## Exit Criteria

- Shortlist of settlement backends for phase 1 prototypes  
- Explicit reject list with reasons  
- Interop note: identity portable across settlement targets?  

## Links

- [CONSENSUS_DESIGN.md](../CONSENSUS_DESIGN.md)
- [PAYMENT_LAYER.md](../PAYMENT_LAYER.md)
- [COMPETITOR_ANALYSIS.md](../COMPETITOR_ANALYSIS.md)
- [../experiments/prototypes.md](../experiments/prototypes.md)
