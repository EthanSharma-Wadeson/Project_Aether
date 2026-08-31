# Aether Control Plane

Enterprise Agent Governance Console — Milestone 1 (Read-Only Observatory).

## Quick start

```bash
# Backend
export CP_JWT_SECRET=dev-secret-change-me
cargo run -p aether-control-plane

# Frontend (separate terminal)
cd frontend && npm install && npm run dev
```

Default credentials: `admin` / `admin` (override with `CP_ADMIN_PASSWORD`).

## Documentation

See [Project_Phases/phase_3/](../Project_Phases/phase_3/) for architecture, security model, and milestone results.
