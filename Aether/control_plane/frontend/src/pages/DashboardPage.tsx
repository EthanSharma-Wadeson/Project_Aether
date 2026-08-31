import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { api } from "../api/client";
import { Metric, PageHeader, Section, StatusBadge } from "../components/ui";
import { useAuthStore } from "../store/auth";
import { canReconcile } from "../lib/roles";

export function DashboardPage() {
  const role = useAuthStore((s) => s.role);
  const system = useQuery({ queryKey: ["system"], queryFn: api.system });
  const agents = useQuery({ queryKey: ["agents"], queryFn: api.agents });
  const caps = useQuery({ queryKey: ["capabilities"], queryFn: api.capabilities });
  const policies = useQuery({ queryKey: ["policies"], queryFn: api.policies });
  const escrows = useQuery({ queryKey: ["escrows"], queryFn: api.escrows });
  const settlements = useQuery({ queryKey: ["settlements"], queryFn: api.settlements });
  const audit = useQuery({ queryKey: ["audit"], queryFn: api.audit });
  const reconcile = useQuery({
    queryKey: ["reconcile"],
    queryFn: api.reconcileList,
    enabled: canReconcile(role),
    retry: false,
  });

  const pendingPolicies =
    policies.data?.policies.filter((p) => p.status === "pending_review" || p.status === "draft")
      .length ?? 0;
  const frozen = agents.data?.agents.filter((a) => a.status.toLowerCase().includes("frozen"))
    .length ?? 0;
  const recentAudit = (audit.data?.events ?? []).slice(0, 8);
  const securityEvents = (audit.data?.events ?? [])
    .filter((e) =>
      /csrf|jwt|login|fail|forbidden|signature|expired|stuck|invalid/i.test(e.action),
    )
    .slice(0, 6);

  return (
    <div>
      <PageHeader
        title="Dashboard"
        subtitle="Enterprise governance overview · protocol observation + Apply-disabled controls"
      />

      <div className="metric-grid" style={{ marginBottom: "1.25rem" }}>
        <Metric
          label="System"
          value={system.data?.status ?? "…"}
          hint={system.data?.read_only_mode ? "Observation mode" : undefined}
        />
        <Metric label="Agents" value={system.data?.protocol_agents_indexed ?? "…"} />
        <Metric
          label="Capabilities"
          value={system.data?.protocol_capabilities_indexed ?? caps.data?.capabilities.length ?? "…"}
        />
        <Metric label="Policies" value={policies.data?.policies.length ?? "…"} hint={`${pendingPolicies} pending`} />
        <Metric label="Escrows" value={system.data?.protocol_escrows_indexed ?? escrows.data?.escrows.length ?? "…"} />
        <Metric
          label="Settlements"
          value={system.data?.protocol_settlements_indexed ?? settlements.data?.settlements.length ?? "…"}
        />
        <Metric label="Reputation events" value={system.data?.reputation_events ?? "…"} />
        <Metric label="Frozen agents" value={frozen} />
      </div>

      <div className="grid-2">
        <Section title="Pending Apply / Governance">
          <div className="panel stack">
            <p className="muted">
              Live Apply execution is disabled. Pending policy work and reconcile queues still surface here.
            </p>
            <p>
              Policy drafts/submitted: <strong>{pendingPolicies}</strong>
            </p>
            {canReconcile(role) && reconcile.data && (
              <>
                <p>
                  Reserved: <strong>{reconcile.data.reserved.length}</strong> · Executing:{" "}
                  <strong>{reconcile.data.executing.length}</strong> · Stuck:{" "}
                  <strong>{reconcile.data.stuck.length}</strong>
                </p>
                <Link to="/reconcile">Open reconcile →</Link>
              </>
            )}
            <Link to="/apply">Open Apply workflows →</Link>
          </div>
        </Section>

        <Section title="Recent security events">
          <div className="panel">
            {securityEvents.length === 0 ? (
              <p className="muted">No matching security-tagged audit rows in the latest window.</p>
            ) : (
              <ul className="stack" style={{ margin: 0, paddingLeft: "1.1rem" }}>
                {securityEvents.map((e) => (
                  <li key={e.id}>
                    <span className="mono">{e.action}</span>
                    <div className="muted">{e.created_at}</div>
                  </li>
                ))}
              </ul>
            )}
            <div className="button-row">
              <Link to="/security">Security Centre →</Link>
              <Link to="/audit">Audit Explorer →</Link>
            </div>
          </div>
        </Section>
      </div>

      <Section title="Audit timeline">
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Time</th>
                <th>Action</th>
                <th>Target</th>
              </tr>
            </thead>
            <tbody>
              {recentAudit.map((e) => (
                <tr key={e.id}>
                  <td className="mono">{e.created_at}</td>
                  <td>{e.action}</td>
                  <td className="mono">{e.target ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Section>

      <Section title="Agents snapshot">
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Agent</th>
                <th>Status</th>
                <th>Caps</th>
                <th>Escrows</th>
              </tr>
            </thead>
            <tbody>
              {(agents.data?.agents ?? []).slice(0, 8).map((a) => (
                <tr key={a.agent_id}>
                  <td>
                    <Link className="mono" to={`/agents/${encodeURIComponent(a.agent_id)}`}>
                      {a.agent_id.slice(0, 18)}…
                    </Link>
                  </td>
                  <td>
                    <StatusBadge status={a.status} />
                  </td>
                  <td>{a.active_capabilities}</td>
                  <td>{a.escrow_count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Section>
    </div>
  );
}
