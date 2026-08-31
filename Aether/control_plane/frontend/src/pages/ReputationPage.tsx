import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { api } from "../api/client";
import { PageHeader, Section } from "../components/ui";

export function ReputationPage() {
  const agents = useQuery({ queryKey: ["agents"], queryFn: api.agents });
  const [selected, setSelected] = useState<string>("");
  const agentId = selected || agents.data?.agents[0]?.agent_id || "";
  const reputation = useQuery({
    queryKey: ["reputation", agentId],
    queryFn: () => api.agentReputation(agentId),
    enabled: !!agentId,
  });

  if (agents.isLoading) return <p>Loading…</p>;

  const chartData =
    agents.data?.agents.map((a) => ({
      name: a.agent_id.slice(0, 8),
      escrows: a.escrow_count,
      capabilities: a.active_capabilities,
    })) ?? [];

  return (
    <div>
      <PageHeader
        title="Reputation"
        subtitle="PROTO-3 deterministic metrics — no rankings or global scores"
      />

      <Section title="Activity snapshot">
        <div className="panel" style={{ height: 280 }}>
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={chartData}>
              <XAxis dataKey="name" stroke="var(--muted)" />
              <YAxis stroke="var(--muted)" />
              <Tooltip />
              <Bar dataKey="escrows" fill="var(--accent-strong)" />
              <Bar dataKey="capabilities" fill="#4fd1c5" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </Section>

      <Section title="Agent metrics">
        <div className="panel stack">
          <label className="muted">Agent</label>
          <select
            value={agentId}
            onChange={(e) => setSelected(e.target.value)}
            aria-label="Select agent for reputation"
          >
            {(agents.data?.agents ?? []).map((a) => (
              <option key={a.agent_id} value={a.agent_id}>
                {a.agent_id.slice(0, 24)}…
              </option>
            ))}
          </select>
          {reputation.data ? (
            <>
              <p>Escrow completed: {reputation.data.metrics.escrow_completed}</p>
              <p>Settlement finalized: {reputation.data.metrics.settlement_finalized}</p>
              <p>Completion rate: {reputation.data.metrics.completion_rate ?? "—"}</p>
              <p>Settlement success rate: {reputation.data.metrics.settlement_success_rate ?? "—"}</p>
              <p className="muted">
                Events {reputation.data.events.length} · Evidence refs{" "}
                {reputation.data.evidence.length}
              </p>
            </>
          ) : (
            <p className="muted">No metrics for selected agent.</p>
          )}
        </div>
      </Section>
    </div>
  );
}
