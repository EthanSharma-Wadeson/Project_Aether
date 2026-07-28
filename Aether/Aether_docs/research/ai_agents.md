# research/ai_agents.md

## Focus

How autonomous agents behave as economic actors — lifecycle, incentives, failure modes — and what that implies for Aether primitives.

## Topics

### Lifecycle economics

- Cold start: how new agents earn first trust  
- Burst spawn: thousands of short-lived workers  
- Retirement: clean channel close vs. abandonment  
- Principal–agent problems inside a single operator’s fleet  

### Decision loops

- Agents budgeting fees, bonds, and dispute risk  
- Counterparty selection via reputation queries  
- When to demand stronger attestation types  

### Misaligned & adversarial behavior

- Goal misspecification leading to harmful optimization  
- Prompt-level attacks are **application** concerns; protocol still must bound spend and require proofs  
- Collusion, wash work, and fake volume for reputation  

### Nondeterminism

- LLMs and tools are often nondeterministic  
- Protocol should define **claim types** (output hash, metric thresholds, human-equivalent rubrics) rather than assuming reproducible bits  
- Escrow conditions must be machine-checkable  

### Multi-agent markets

- Agents hiring agents (recursive composition)  
- Reputation cascading and systemic risk  
- Discovery without centralized stores (research + product boundary)  

## Working Hypotheses

1. Capability ceilings matter more than “aligned model” assumptions for protocol safety.  
2. Reputation without bonded economic outcomes will be farmed.  
3. Micro-payments change agent architecture: continuous metering beats large opaque invoices.  
4. Principals will operate fleets; identity systems must support delegation without identity smash-and-grab.  

## Metrics to Study

- Task completion under adversarial counterparties  
- Profitability of wash-reputation strategies under proposed scorers  
- Latency budgets for typical “hire → work → attest → pay” loops  

## Exit Criteria

- Reference agent lifecycle mapped to protocol messages  
- Attack playbook for reputation/payment abuse with mitigations  
- Guidance for attestation types suitable for nondeterministic workers  

## Links

- [AI_AGENT_MODEL.md](../AI_AGENT_MODEL.md)
- [REPUTATION_SYSTEM.md](../REPUTATION_SYSTEM.md)
- [ECONOMIC_MODEL.md](../ECONOMIC_MODEL.md)
- [../experiments/hypotheses.md](../experiments/hypotheses.md)
