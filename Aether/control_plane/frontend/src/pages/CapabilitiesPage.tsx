import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { api, CapabilityView } from "../api/client";
import { DataTable } from "../components/DataTable";
import { PageHeader, StatusBadge } from "../components/ui";

export function CapabilitiesPage() {
  const { data, isLoading, error } = useQuery({
    queryKey: ["capabilities"],
    queryFn: api.capabilities,
  });
  if (isLoading) return <p>Loading…</p>;
  if (error) return <p className="error-text">Error: {(error as Error).message}</p>;

  return (
    <div>
      <PageHeader title="Capabilities" subtitle="PROTO-0 capability grants (read-only observation)" />
      <DataTable
        rows={data?.capabilities ?? []}
        rowKey={(c) => c.capability_id}
        searchPlaceholder="Search subject, issuer, actions…"
        searchFilter={(c: CapabilityView, q) =>
          `${c.subject_agent_id} ${c.issuer} ${c.actions.join(" ")}`.toLowerCase().includes(q)
        }
        columns={[
          {
            key: "subject",
            header: "Subject",
            render: (c) => (
              <Link className="mono" to={`/agents/${encodeURIComponent(c.subject_agent_id)}`}>
                {c.subject_agent_id.slice(0, 14)}…
              </Link>
            ),
          },
          {
            key: "issuer",
            header: "Issuer",
            render: (c) => <span className="mono">{c.issuer.slice(0, 14)}…</span>,
          },
          { key: "actions", header: "Actions", render: (c) => c.actions.join(", ") },
          { key: "spend", header: "Max spend", render: (c) => c.max_spend ?? "—" },
          {
            key: "rev",
            header: "Status",
            render: (c) => <StatusBadge status={c.revoked ? "revoked" : "active"} />,
          },
        ]}
      />
    </div>
  );
}
