# Enterprise Governance Console

Product UX for Aether Protocol v1.0. The console operates the Control Plane HTTP API as a **stable backend**. It does **not** modify `aether-core`, protocol semantics, or Apply enablement.

**Invariant:** `apply_enabled=false`. Live PROTO-0 mutation is impossible. The UI surfaces **Execution disabled.** on Apply-related surfaces.

---

## UI architecture

```
control_plane/frontend/
  src/
    App.tsx                 # Routes + auth gate
    main.tsx                # React Query + Router
    api/client.ts           # Typed same-origin API client + CSRF
    lib/roles.ts            # Persona → backend role mapping
    store/auth.ts           # JWT + CSRF session
    store/ui.ts             # Dark / light theme
    components/
      Layout.tsx            # Shell, nav, role-gated ops links
      DataTable.tsx         # Search + pagination
      ui.tsx                # Banner, metrics, badges, headers
    pages/                  # One screen per route
    index.css               # Design tokens (IBM Plex, dark/light)
```

**Stack:** React 18 · Vite · React Router · TanStack Query · Zustand · Recharts.

**Data flow:** Browser → same-origin `/api/*` and `/auth/*` → Control Plane. Mutations send `Authorization: Bearer` and `X-CSRF-Token`. Server enforces roles; the console only hides workflows.

---

## Navigation

| Section | Routes | Visibility |
|---------|--------|------------|
| Overview | `/dashboard` | All authenticated roles |
| Protocol | `/agents`, `/capabilities`, `/economic`, `/settlements`, `/reputation` | All |
| Governance | `/policies`, `/apply`, `/audit` | All (writes role-gated) |
| Operations | `/security`, `/reconcile`, `/admin` | Admin only |

Default post-login landing: **Dashboard**.

---

## Screens

1. **Login** — operator credentials; stores JWT + CSRF.
2. **Dashboard** — system health, agents, capabilities, policies, escrows, settlements, reputation, pending governance, audit timeline, security-tagged events, pending Apply/reconcile summary.
3. **Agents / Agent detail** — browse, identity, capabilities, reputation, escrow/settlement/audit history, freeze status.
4. **Capabilities / Escrow / Settlements / Reputation** — searchable observation tables (+ chart).
5. **Policies / Policy detail** — create, review, approve, cancel (reject), archive, version history, side-by-side diff. No Apply execution.
6. **Apply Workflows** — dry-run → approval (create/cancel) → prepare signature → execute (disabled) → operation status. Banner: Execution disabled.
7. **Audit Explorer** — filters (time, operator, agent, policy, request_id, operation_id, event type) + JSON/CSV export.
8. **Security Centre** — failed logins, invalid JWTs, CSRF, policy violations, signature failures, expired approvals, stuck ops, warnings (from audit + reconcile).
9. **Reconcile** — reserved / executing / stuck lists; admin abort; no PROTO-0 retry.
10. **System Admin** — session, roles, production/Apply status, reconcile snapshot, read-only config.

---

## Component hierarchy

```
App
└── Protected → Layout
    ├── ApplyDisabledBanner
    ├── Sidebar (nav + theme + sign-out)
    └── Outlet → Page
        ├── PageHeader
        ├── Metric / Section / StatusBadge
        └── DataTable | forms | charts
```

---

## Roles & personas

Backend roles: `admin` | `operator` | `viewer`.

| Persona | Maps to | Console workflows |
|---------|---------|-------------------|
| Administrator | `admin` | Full write, approve, execute attempt, reconcile, security, admin |
| Security Officer | `admin` | Security Centre + audit + approvals |
| Operator | `operator` | Policy drafts/submit, dry-run, prepare |
| Auditor | `viewer` | Read + Audit Explorer export |
| Viewer | `viewer` | Read-only observation |

---

## Accessibility

- Semantic landmarks (`nav`, `main`, `section`, `role="status"` on banners).
- Form controls labelled; search inputs have `aria-label`.
- Keyboard-usable buttons, links, selects; focusable tables via native controls.
- Theme contrast tokens for dark and light modes.
- Status communicated with text badges, not colour alone.

---

## Security model (console)

- JWT required for protected routes; invalid JWT → API 401.
- CSRF required on mutating requests.
- Role gates in UI are advisory; authorization is server-side.
- Console never sets `apply_enabled` or patches protocol crates.
- Execute UI may call `POST /api/apply` and must surface disabled outcomes (`APPLY_EXECUTION_DISABLED`).

---

## Future roadmap

1. Server-side audit query params (time range, agent, request_id) instead of client-only filter of latest window.
2. Dedicated Security event feed API (structured categories).
3. Operator provisioning UI (still server-managed today).
4. Session inventory / refresh-token revoke UI.
5. Richer policy diff (field-level JSON patch).
6. Optional SSO / IdP integration.
7. When Apply enablement gate opens: remove disabled banner only after production certification — never silently.

---

## Build

```bash
cd control_plane/frontend && npm install && npm run build
```
