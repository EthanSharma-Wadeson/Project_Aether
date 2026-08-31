# Agent Identity Model — Design Freeze

| Field | Value |
|-------|--------|
| **Document** | `AGENT_IDENTITY_MODEL.md` |
| **Phase** | 27 — Agent Runtime Integration Gate |
| **Status** | **DESIGN FROZEN** — no implementation authorised |
| **Date** | 2026-08-01 |
| **Companion** | [AGENT_RUNTIME_ARCHITECTURE.md](AGENT_RUNTIME_ARCHITECTURE.md), PROTO-0 identity registry |

---

## 1. What is an agent?

An **agent** in Aether is a **registered governed principal** that may request actions under:

- PROTO-0 identity + capability constraints (protocol authority)  
- Organisation treasury **allocation** (financial authority — non-owning)  
- Control Plane policy + E2 enforcement (governance)

An agent is **not**:

- An LLM vendor account  
- A chat session alone  
- A human operator login  
- A wallet or payment instrument  
- “Whatever the model claims in a prompt”

```text
Human / org (accountable party)
        │ registers & funds
        ▼
Agent principal (agent_id)     ← Aether identity
        │ uses
        ▼
Model provider instance        ← interchangeable inference backend
```

---

## 2. How Aether identifies an external AI instance

Identification is **layered**. The model process is bound to a registered principal; the vendor is not the principal.

| Layer | Identifier | Purpose |
|-------|------------|---------|
| **L1 Agent principal** | PROTO-0 `agent_id` (stable) | Authority subject for caps / freeze / reputation |
| **L2 Organisation** | `organisation_id` | Tenancy; treasury ownership |
| **L3 Runtime binding** | `agent_runtime_credential` (future) | Authn of the workload calling CP/tools |
| **L4 Session** | `session_id` | One conversation / loop instance |
| **L5 Provider** | `provider` + optional `model_id` | Observability only — **not** authority |
| **L6 Operator** | Human JWT (Console) | Governance — distinct from agent credential |

**Rule:** Enforcement and treasury checks key off **L1 + L2**, never off “Claude” or “Gemini” alone.

---

## 3. Are agents API keys? Signed identities? Workloads? Human-owned?

**All of the following, in a strict hierarchy — not “API key = agent”.**

| Concept | Role | Verdict |
|---------|------|---------|
| **Human-owned / org-owned principals** | Org registers agent; humans remain accountable | **Required** |
| **Signed protocol identities** | PROTO-0 identity bundles / registry entries | **Source of protocol truth** |
| **Registered workloads** | Runtime that holds agent credentials and calls E2/tools | **Required for production runtime** |
| **API keys** | (a) Provider inference keys (org secret); (b) optional agent→CP credentials | **Credentials only — never the agent itself** |

### 3.1 Recommended binding model

```text
Organisation
  └── AgentRegistration
        ├── agent_id              (PROTO-0)
        ├── display_name
        ├── owner_operator_ids[]  (accountable humans)
        ├── allowed_providers[]   (claude|gemini|openai|local|enterprise)
        ├── runtime_auth_method   (mTLS | signed JWT | workload identity)
        └── status                (active|suspended)  // CP ops; freeze still PROTO-0
```

| Auth method (future) | Use |
|----------------------|-----|
| **Workload identity / mTLS** | Preferred for enterprise agents |
| **Short-lived signed agent JWT** | Bound to `agent_id` + org; not operator role |
| **Provider API key** | **Only** for calling Anthropic/Google/OpenAI — stored in org secret manager; **never** accepted as proof of `agent_id` |

### 3.2 Explicit non-identities

| Thing | May authenticate as agent? |
|-------|----------------------------|
| Raw Anthropic/Gemini/OpenAI API key | **No** |
| Prompt text (“I am agent X”) | **No** |
| Operator Console JWT | **No** (different plane — governance) |
| Stolen tool result containing agent_id | **No** without L3 credential |

---

## 4. Impersonation & compromise model

| Threat | Control (design) |
|--------|------------------|
| Prompt claims false agent_id | Ignore; identity from L3 credential only |
| Compromised provider API key | Rotate vendor key; does not grant CP agent role; monitor spend on inference only |
| Compromised agent runtime credential | Revoke L3; PROTO-0 freeze agent; treasury freeze allocation |
| Compromised operator JWT | Existing CP auth controls; cannot mint protocol caps without Apply |

---

## 5. Multi-provider, one principal

```text
agent_id = agent-research-01
  ├── session on Claude 4  → same caps / £1 allocation
  └── session on Gemini    → same caps / £1 allocation
```

Provider switch is an ops choice. **Authority does not fork per vendor.**

---

## 6. Relationship to treasury & “wallet”

| Question | Answer |
|----------|--------|
| Does the agent own money? | **Never** |
| What does the agent have? | A **spending allocation** (authorisation over org balances) |
| Who holds title? | **Organisation** via Treasury |

See [AGENT_RUNTIME_ARCHITECTURE.md](AGENT_RUNTIME_ARCHITECTURE.md) § agent wallet principle and TREASURY_ARCHITECTURE ownership rules.

---

## 7. Freeze statement

Agent = registered PROTO-0 principal + org tenancy + runtime credential; providers and API keys are **not** agents. Implementation of L3 credentials is a future phase.

**This document does not authorise implementation.**
