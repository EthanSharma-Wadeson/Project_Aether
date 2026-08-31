import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { Metric, PageHeader, Section, StatusBadge } from "../components/ui";

export function AgentDetailPage() {
  const { id = "" } = useParams();
  const decoded = decodeURIComponent(id);
  const agent = useQuery({
    queryKey: ["agent", decoded],
    queryFn: () => api.agent(decoded),
    enabled: !!decoded,
  });
  const caps = useQuery({
    queryKey: ["agent-caps", decoded],
    queryFn: () => api.agentCapabilities(decoded),
    enabled: !!decoded,
  });
  const escrows = useQuery({
    queryKey: ["agent-escrows", decoded],
    queryFn: () => api.agentEscrows(decoded),
    enabled: !!decoded,
  });
  const settlements = useQuery({
    queryKey: ["agent-settlements", decoded],
    queryFn: () => api.agentSettlements(decoded),
    enabled: !!decoded,
  });
  const reputation = useQuery({
    queryKey: ["agent-reputation", decoded],
    queryFn: () => api.agentReputation(decoded),
    enabled: !!decoded,
  });
  const events = useQuery({
    queryKey: ["agent-events", decoded],
    queryFn: () => api.agentEvents(decoded),
    enabled: !!decoded,
  });

  if (agent.isLoading) return <p>Loading agent…</p>;
  if (agent.error) return <p className="error-text">Error: {(agent.error as Error).message}</p>;

  return (
    <div>
      <p>
        <Link to="/agents">← Agents</Link>
      </p>
      <PageHeader
        title="Agent detail"
        subtitle="Identity, capabilities, reputation, escrow, settlement, and audit history"
      />

      <div className="metric-grid" style={{ marginBottom: "1.25rem" }}>
        <Metric label="Status" value={agent.data?.status ?? "—"} />
        <Metric label="Capabilities" value={agent.data?.activity_summary.active_capabilities ?? 0} />
        <Metric label="Escrows" value={agent.data?.activity_summary.escrows ?? 0} />
        <Metric label="Settlements" value={agent.data?.activity_summary.settlements ?? 0} />
        <Metric
          label="Reputation events"
          value={agent.data?.activity_summary.reputation_events ?? 0}
        />
        <Metric label="Root version" value={agent.data?.root_version ?? "—"} />
      </div>

      <Section title="Identity">
        <div className="panel stack">
          <p className="mono">{agent.data?.agent_id}</p>
          <p>
            Freeze / status: <StatusBadge status={agent.data?.status ?? "unknown"} />
          </p>
          <p className="muted">
            Operational public key:{" "}
            <span className="mono">{agent.data?.operational_public_key?.slice(0, 48)}…</span>
          </p>
          <p className="muted">Read at {agent.data?.read_at}</p>
        </div>
      </Section>

      <div className="grid-2">
        <Section title="Capabilities">
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Actions</th>
                  <th>Max spend</th>
                  <th>Revoked</th>
                </tr>
              </thead>
              <tbody>
                {(caps.data?.capabilities ?? []).map((c) => (
                  <tr key={c.capability_id}>
                    <td>{c.actions.join(", ")}</td>
                    <td>{c.max_spend ?? "—"}</td>
                    <td>
                      <StatusBadge status={c.revoked ? "revoked" : "active"} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Section>

        <Section title="Reputation">
          <div className="panel stack">
            {reputation.data ? (
              <>
                <p>Escrow completed: {reputation.data.metrics.escrow_completed}</p>
                <p>Settlement finalized: {reputation.data.metrics.settlement_finalized}</p>
                <p>
                  Completion rate: {reputation.data.metrics.completion_rate ?? "—"} · Settlement
                  success: {reputation.data.metrics.settlement_success_rate ?? "—"}
                </p>
                <p className="muted">
                  Events {reputation.data.events.length} · Evidence {reputation.data.evidence.length}
                </p>
              </>
            ) : (
              <p className="muted">No metrics</p>
            )}
          </div>
        </Section>
      </div>

      <div className="grid-2">
        <Section title="Escrow history">
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Status</th>
                  <th>Amount</th>
                  <th>Finality</th>
                </tr>
              </thead>
              <tbody>
                {(escrows.data?.escrows ?? []).map((e) => (
                  <tr key={e.escrow_id}>
                    <td>
                      <StatusBadge status={e.status} />
                    </td>
                    <td>{e.principal_amount}</td>
                    <td>
                      soft={String(e.soft_finality)} / hard={String(e.hard_finality)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Section>

        <Section title="Settlement history">
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Status</th>
                  <th>External ref</th>
                  <th>Finality</th>
                </tr>
              </thead>
              <tbody>
                {(settlements.data?.settlements ?? []).map((s) => (
                  <tr key={s.binding_id}>
                    <td>
                      <StatusBadge status={s.settlement_status} />
                    </td>
                    <td className="mono">{s.external_settlement_ref ?? "—"}</td>
                    <td>
                      soft={String(s.soft_finality)} / hard={String(s.hard_finality)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Section>
      </div>

      <Section title="Audit history">
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Source</th>
                <th>Type</th>
                <th>Summary</th>
              </tr>
            </thead>
            <tbody>
              {(events.data?.events ?? []).map((e, idx) => (
                <tr key={idx}>
                  <td>{e.source}</td>
                  <td>{e.event_type}</td>
                  <td>{e.summary}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Section>
    </div>
  );
}
