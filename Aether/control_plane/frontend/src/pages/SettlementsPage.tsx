import { useQuery } from "@tanstack/react-query";
import { api, SettlementView } from "../api/client";
import { DataTable } from "../components/DataTable";
import { PageHeader, StatusBadge } from "../components/ui";

export function SettlementsPage() {
  const { data, isLoading, error } = useQuery({
    queryKey: ["settlements"],
    queryFn: api.settlements,
  });
  if (isLoading) return <p>Loading…</p>;
  if (error) return <p className="error-text">Error: {(error as Error).message}</p>;

  return (
    <div>
      <PageHeader
        title="Settlement Activity"
        subtitle="PROTO-4 bindings — soft finality ≠ hard finality"
      />
      <DataTable
        rows={data?.settlements ?? []}
        rowKey={(s) => s.binding_id}
        searchPlaceholder="Search binding, escrow, status…"
        searchFilter={(s: SettlementView, q) =>
          `${s.binding_id} ${s.escrow_id} ${s.settlement_status} ${s.external_settlement_ref ?? ""}`
            .toLowerCase()
            .includes(q)
        }
        columns={[
          {
            key: "id",
            header: "Binding",
            render: (s) => <span className="mono">{s.binding_id.slice(0, 14)}…</span>,
          },
          {
            key: "status",
            header: "Status",
            render: (s) => <StatusBadge status={s.settlement_status} />,
          },
          {
            key: "esc",
            header: "Escrow",
            render: (s) => <span className="mono">{s.escrow_id.slice(0, 12)}…</span>,
          },
          {
            key: "ext",
            header: "External ref",
            render: (s) => <span className="mono">{s.external_settlement_ref ?? "—"}</span>,
          },
          { key: "soft", header: "Soft", render: (s) => String(s.soft_finality) },
          { key: "hard", header: "Hard", render: (s) => String(s.hard_finality) },
        ]}
      />
    </div>
  );
}
