import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api, AgentSummary } from "../api/client";
import { DataTable } from "../components/DataTable";
import { PageHeader, StatusBadge } from "../components/ui";

export function AgentsPage() {
  const { data, isLoading, error } = useQuery({
    queryKey: ["agents"],
    queryFn: api.agents,
  });

  if (isLoading) return <p>Loading agents…</p>;
  if (error) return <p className="error-text">Error: {(error as Error).message}</p>;

  const rows = data?.agents ?? [];

  return (
    <div>
      <PageHeader
        title="Agent Management"
        subtitle="Browse PROTO-0 identities, freeze status, and activity summaries"
      />
      <DataTable
        rows={rows}
        rowKey={(a) => a.agent_id}
        searchPlaceholder="Search agent id or status…"
        searchFilter={(a: AgentSummary, q) =>
          `${a.agent_id} ${a.status}`.toLowerCase().includes(q)
        }
        columns={[
          {
            key: "id",
            header: "Agent",
            render: (a) => (
              <Link className="mono" to={`/agents/${encodeURIComponent(a.agent_id)}`}>
                {a.agent_id.slice(0, 20)}…
              </Link>
            ),
          },
          {
            key: "status",
            header: "Freeze / status",
            render: (a) => <StatusBadge status={a.status} />,
          },
          {
            key: "caps",
            header: "Capabilities",
            render: (a) => a.active_capabilities,
          },
          { key: "esc", header: "Escrows", render: (a) => a.escrow_count },
          { key: "set", header: "Settlements", render: (a) => a.settlement_count },
          {
            key: "rep",
            header: "Reputation",
            render: (a) => (a.has_reputation_metrics ? "yes" : "no"),
          },
        ]}
      />
    </div>
  );
}
