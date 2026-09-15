# Project Aether

Enterprise AI agent governance and control plane.

**Models propose. Aether decides.** Agents never own money.

Aether is infrastructure for governing what registered agents may do inside an organisation: identity, sessions, tool permissions, treasury observation, and fail-closed enforcement. It is **not** a retail bank, payment institution, custodian, or production financial service.

## Status

**Lab / research.** Apply is disabled by default. There are no live payment rails in the lab build. Do not treat this repository as production-ready financial software.

## Core invariants

| Invariant | Meaning |
|-----------|---------|
| Apply disabled by default | `CP_APPLY_ENABLED` defaults to off; no money-moving Apply path in normal lab runs |
| No real payment rails in lab | Settlement and spend paths are simulated / ledger-internal; no banking or chain clients |
| Provider ≠ identity | External model adapters (e.g. Claude lab) propose intents only; agent identity comes from trusted control-plane state |
| Fail-closed enforcement | Ambiguous, unauthorised, or errored requests deny; enforcement does not invent authority or funds |
| Agents never hold title | Org treasury → allocation → caps; agents operate under delegated authority only |

## Repository layout

```text
Project_Aether/
├── Aether/
│   ├── control_plane/     # HTTP API + enterprise console (Axum + frontend)
│   ├── treasury/          # Internal treasury domain (ledger, allocations)
│   ├── core/              # Protocol / identity / escrow primitives
│   ├── demo/              # Enterprise demo harness
│   ├── Aether_docs/       # Design, security, and architecture docs
│   ├── Project_Phases/    # Phased milestone notes and gates
│   ├── schemas/           # Versioned schemas
│   ├── experiments/       # Local research experiments
│   └── laws_regulations/  # Regulatory reference material
└── Aether_Notes/          # Working notes
```

## Quick start — control plane

Requires Rust (stable) and Node.js for the console UI.

```bash
# Backend
cd Aether/control_plane
export CP_JWT_SECRET=dev-secret-change-me
cargo run -p aether-control-plane

# Frontend (separate terminal)
cd Aether/control_plane/frontend
npm install && npm run dev
```

Default console credentials: `admin` / `admin` (override with `CP_ADMIN_PASSWORD`).

See [Aether/control_plane/README.md](Aether/control_plane/README.md) for details.

### Lab Claude (optional)

External provider adapters are **lab-only**. To exercise the Anthropic/Claude lab path:

```bash
export AETHER_LAB_ANTHROPIC_API_KEY=...   # preferred
# or: export AETHER_LAB_PROVIDER_API_KEY=...
# optional: AETHER_LAB_ANTHROPIC_MODEL, AETHER_LAB_ANTHROPIC_API_URL
```

Provider output is treated as untrusted proposals only. API keys must resolve via secret refs — never embed raw keys in request bodies. Production mode refuses external adapters.

Details: [EXTERNAL_PROVIDER_LAB_IMPLEMENTATION.md](Aether/Aether_docs/EXTERNAL_PROVIDER_LAB_IMPLEMENTATION.md).

## Key documentation

| Doc | Topic |
|-----|--------|
| [VISION.md](Aether/Aether_docs/VISION.md) | North star and design invariants |
| [REGULATORY_POSITIONING.md](Aether/Aether_docs/REGULATORY_POSITIONING.md) | Governance control plane vs financial-service claims |
| [EXTERNAL_PROVIDER_LAB_IMPLEMENTATION.md](Aether/Aether_docs/EXTERNAL_PROVIDER_LAB_IMPLEMENTATION.md) | Lab Claude adapter and secret handling |
| [ARCHITECTURE.md](Aether/Aether_docs/ARCHITECTURE.md) | System architecture |
| [SECURITY_MODEL.md](Aether/Aether_docs/SECURITY_MODEL.md) | Security model overview |
| [Project_Phases/](Aether/Project_Phases/) | Milestone plans, results, and gates |

## Safety notes

- Do not commit `.env`, credentials, or local `*.db` files (see `.gitignore`).
- Lab secrets stay in environment variables; they are never written to audit metadata.
- This codebase is for research and enterprise-governance prototyping only.
