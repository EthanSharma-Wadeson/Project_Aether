# AI_AGENT_MODEL.md — Project Aether

## Definition

On Aether, an **agent** is an autonomous software actor that:

1. Controls cryptographic keys (directly or via a constrained custodian)
2. Holds an Aether identity with a permission root
3. Can initiate and respond to protocol messages without a human in the loop for standard operations
4. May earn, spend, bond, and accumulate reputation under protocol rules

An agent is **not** defined as an LLM, a chat UI, or a specific model vendor. Those are implementation choices above the protocol.

## Why a Explicit Agent Model

Infrastructure must know what it is securing. Ambiguous “AI agent” marketing leads to wrong primitives (token gates, chat wrappers). Aether’s agent model is economic and cryptographic.

## Agent Properties

| Property | Description |
|----------|-------------|
| Autonomy | Can propose actions and payments within permissions |
| Accountability | Actions attributable to identity + keys; bonds and reputation attach |
| Ephemerality | May be short-lived; identity and reputation may outlive a process |
| Multiplicity | One principal may operate many agents; Sybil cost applies |
| Heterogeneity | Different models, tools, and runtimes; protocol stays model-agnostic |

## Roles

| Role | Meaning |
|------|---------|
| **Principal** | Entity that funds bonds and sets root policy (human org, DAO, or parent agent) |
| **Agent** | Operational actor with identity and scoped capabilities |
| **Counterparty** | Another agent or protocol contract in a bilateral flow |
| **Verifier** | Party or network rule that checks attestations/proofs |
| **Indexer** | Reads events; may compute non-canonical reputation views |

## Lifecycle

```
Provision keys → Register identity → Post bond → Receive capabilities
    → Discover / negotiate → Act + pay → Attest → Settle
    → Rotate / rebond / retire → Optional identity archival
```

### Provisioning

- Key generation in secure runtime (HSM, enclave, hardened host — deployment choice)
- Identity registration commits to public keys and permission root

### Operation

- Agent evaluates local goals under capability constraints
- All externally visible economic actions pass permission + payment checks

### Retirement

- Close channels, release or slash bonds per rules
- Identity may remain for historical attribution; spend capabilities revoked

## Capability Binding

Agents MUST treat permissions as hard constraints, not hints:

- Spend ceilings and rate limits
- Allowed methods / contract surfaces
- Allowed counterparties or reputation thresholds
- Required attestation types before release of funds

Principals express risk appetite through envelopes; the protocol enforces them.

## Economic Agency

Agents can:

- Open and update payment channels
- Lock value in escrow conditional on attestations
- Post bonds for anti-spam and dispute skin-in-the-game
- Accumulate reputation from protocol events

Agents cannot (by design goals):

- Bypass capability checks via raw signatures alone
- Require humans to click-approve each micro-payment

## Trust Model for Agent Behavior

The protocol assumes agents may be:

- Rational economic attackers
- Buggy or partially compromised
- Colluding in reputation or wash-trade schemes

Therefore reliability comes from bonds, escrow, verification, and reputation — not from assuming aligned LLMs.

## Out of Scope

- Cognitive architectures, planning algorithms, tool-calling frameworks
- Model training, weights distribution, or prompt markets as protocol features
- End-user chat products

## Related

- [IDENTITY_SYSTEM.md](IDENTITY_SYSTEM.md)
- [REPUTATION_SYSTEM.md](REPUTATION_SYSTEM.md)
- `research/ai_agents.md`
