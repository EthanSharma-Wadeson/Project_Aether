import { useQuery } from "@tanstack/react-query";
import { api, EscrowView } from "../api/client";
import { DataTable } from "../components/DataTable";
import { PageHeader, StatusBadge } from "../components/ui";

export function EconomicPage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["escrows"], queryFn: api.escrows });
  if (isLoading) return <p>Loading…</p>;
  if (error) return <p className="error-text">Error: {(error as Error).message}</p>;

  return (
    <div>
      <PageHeader title="Escrow Activity" subtitle="PROTO-2 escrow lifecycle (read-only)" />
      <DataTable
        rows={data?.escrows ?? []}
        rowKey={(e) => e.escrow_id}
        searchPlaceholder="Search escrow, payer, provider, status…"
        searchFilter={(e: EscrowView, q) =>
          `${e.escrow_id} ${e.payer} ${e.provider} ${e.status}`.toLowerCase().includes(q)
        }
        columns={[
          {
            key: "id",
            header: "Escrow",
            render: (e) => <span className="mono">{e.escrow_id.slice(0, 14)}…</span>,
          },
          {
            key: "status",
            header: "Status",
            render: (e) => <StatusBadge status={e.status} />,
          },
          {
            key: "payer",
            header: "Payer",
            render: (e) => <span className="mono">{e.payer.slice(0, 10)}…</span>,
          },
          {
            key: "provider",
            header: "Provider",
            render: (e) => <span className="mono">{e.provider.slice(0, 10)}…</span>,
          },
          { key: "amt", header: "Amount", render: (e) => e.principal_amount },
          {
            key: "fin",
            header: "Soft / Hard",
            render: (e) => `${String(e.soft_finality)} / ${String(e.hard_finality)}`,
          },
        ]}
      />
    </div>
  );
}
