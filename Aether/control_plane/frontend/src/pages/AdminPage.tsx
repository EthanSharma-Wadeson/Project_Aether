import { Navigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { ApplyDisabledBanner, Metric, PageHeader, Section, StatusBadge } from "../components/ui";
import { canAdminister, roleLabel } from "../lib/roles";
import { useAuthStore } from "../store/auth";

export function AdminPage() {
  const role = useAuthStore((s) => s.role);
  const token = useAuthStore((s) => s.token);
  const csrf = useAuthStore((s) => s.csrfToken);
  const me = useQuery({ queryKey: ["me"], queryFn: api.me });
  const system = useQuery({ queryKey: ["system"], queryFn: api.system });
  const reconcile = useQuery({
    queryKey: ["reconcile"],
    queryFn: api.reconcileList,
    retry: false,
  });

  if (!canAdminister(role)) {
    return <Navigate to="/dashboard" replace />;
  }

  return (
    <div>
      <PageHeader
        title="System Administration"
        subtitle="Configuration, session, production status — observation only; Apply remains disabled"
      />
      <ApplyDisabledBanner />

      <div className="metric-grid" style={{ marginBottom: "1.25rem" }}>
        <Metric label="System status" value={system.data?.status ?? "…"} />
        <Metric label="Read-only mode" value={String(system.data?.read_only_mode ?? true)} />
        <Metric label="Apply status" value="disabled" hint="apply_enabled=false" />
        <Metric label="Operator role" value={roleLabel(role)} />
      </div>

      <div className="grid-2">
        <Section title="Session">
          <div className="panel stack">
            <p>
              Username: <strong>{me.data?.username ?? "…"}</strong>
            </p>
            <p className="mono muted">operator_id: {me.data?.operator_id ?? "…"}</p>
            <p>
              Access token: <StatusBadge status={token ? "present" : "missing"} />
            </p>
            <p>
              CSRF token: <StatusBadge status={csrf ? "present" : "missing"} />
            </p>
            <p className="muted">
              Mutations require CSRF. Invalid JWTs return 401. Roles are enforced server-side.
            </p>
          </div>
        </Section>

        <Section title="Operators & roles">
          <div className="panel stack">
            <p className="muted">
              Backend roles exposed by the protocol API (no new roles invented by the console):
            </p>
            <ul>
              <li>
                <strong>admin</strong> — Administrator / Security Officer workflows
              </li>
              <li>
                <strong>operator</strong> — Operator workflows (write policies, prepare Apply)
              </li>
              <li>
                <strong>viewer</strong> — Auditor / Viewer read-only workflows
              </li>
            </ul>
            <p className="muted">Operator provisioning remains a server-side concern.</p>
          </div>
        </Section>
      </div>

      <Section title="Production status">
        <div className="panel stack">
          <p>
            Protocol observation index loaded at{" "}
            <span className="mono">{system.data?.loaded_at ?? "—"}</span>
          </p>
          <p>
            Agents {system.data?.protocol_agents_indexed ?? 0} · Capabilities{" "}
            {system.data?.protocol_capabilities_indexed ?? 0} · Escrows{" "}
            {system.data?.protocol_escrows_indexed ?? 0} · Settlements{" "}
            {system.data?.protocol_settlements_indexed ?? 0}
          </p>
          <p>
            Apply execution: <StatusBadge status="disabled" /> — production hard gate; console never
            enables Apply.
          </p>
        </div>
      </Section>

      <Section title="Startup validation / reconcile snapshot">
        <div className="panel stack">
          {reconcile.data ? (
            <>
              <p className="muted">Latest reconcile scan (admin): {reconcile.data.scanned_at}</p>
              <p>
                Reserved {reconcile.data.reserved.length} · Executing {reconcile.data.executing.length}{" "}
                · Stuck {reconcile.data.stuck.length}
              </p>
              <p>
                Timed out reserved: {reconcile.data.reserved_timed_out} · Timed out executing:{" "}
                {reconcile.data.executing_timed_out}
              </p>
              <pre className="code-block">
                {JSON.stringify(
                  {
                    apply_enabled: reconcile.data.apply_enabled,
                    protocol_mutated: reconcile.data.protocol_mutated,
                  },
                  null,
                  2,
                )}
              </pre>
            </>
          ) : (
            <p className="muted">Reconcile scan unavailable or still loading.</p>
          )}
        </div>
      </Section>

      <Section title="Configuration (read-only)">
        <div className="panel">
          <pre className="code-block">
            {JSON.stringify(
              {
                console: "Enterprise Governance Console",
                api_base: "same-origin",
                apply_enabled: false,
                read_only_protocol: true,
                csrf_required_on_mutations: true,
              },
              null,
              2,
            )}
          </pre>
        </div>
      </Section>
    </div>
  );
}
