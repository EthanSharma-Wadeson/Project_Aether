import { useState } from "react";
import { Navigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, ReplayRecord } from "../api/client";
import { ApplyDisabledBanner, PageHeader, Section, StatusBadge } from "../components/ui";
import { DataTable } from "../components/DataTable";
import { canReconcile } from "../lib/roles";
import { useAuthStore } from "../store/auth";

function OpTable({
  title,
  rows,
  onAbort,
}: {
  title: string;
  rows: ReplayRecord[];
  onAbort?: (id: string) => void;
}) {
  return (
    <Section title={title}>
      <DataTable
        rows={rows}
        rowKey={(r) => r.operation_id}
        searchPlaceholder="Search operation_id…"
        searchFilter={(r, q) =>
          `${r.operation_id} ${r.status} ${r.dry_run_id}`.toLowerCase().includes(q)
        }
        columns={[
          {
            key: "id",
            header: "Operation",
            render: (r) => <span className="mono">{r.operation_id}</span>,
          },
          {
            key: "status",
            header: "Status",
            render: (r) => <StatusBadge status={r.status} />,
          },
          {
            key: "updated",
            header: "Updated",
            render: (r) => <span className="mono">{r.updated_at}</span>,
          },
          {
            key: "reason",
            header: "Terminal reason",
            render: (r) => r.terminal_reason ?? "—",
          },
          {
            key: "act",
            header: "Actions",
            render: (r) =>
              onAbort && !r.terminal ? (
                <button type="button" className="secondary" onClick={() => onAbort(r.operation_id)}>
                  Abort
                </button>
              ) : (
                "—"
              ),
          },
        ]}
        emptyMessage={`No ${title.toLowerCase()} operations.`}
      />
    </Section>
  );
}

export function ReconcilePage() {
  const role = useAuthStore((s) => s.role);
  const queryClient = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const [detailId, setDetailId] = useState("");
  const list = useQuery({
    queryKey: ["reconcile"],
    queryFn: api.reconcileList,
    enabled: canReconcile(role),
  });
  const detail = useQuery({
    queryKey: ["reconcile", detailId],
    queryFn: () => api.reconcileGet(detailId),
    enabled: canReconcile(role) && !!detailId,
  });

  const abort = useMutation({
    mutationFn: (operationId: string) => api.reconcileAbort(operationId, "admin_abort_from_console"),
    onSuccess: () => {
      setError(null);
      queryClient.invalidateQueries({ queryKey: ["reconcile"] });
    },
    onError: (e: Error) => setError(e.message),
  });

  if (!canReconcile(role)) {
    return <Navigate to="/dashboard" replace />;
  }

  return (
    <div>
      <PageHeader
        title="Reconcile"
        subtitle="Reserved / executing / stuck Apply operations — abort only; no PROTO-0 retry"
      />
      <ApplyDisabledBanner />
      {error && <p className="error-text">{error}</p>}

      {list.isLoading && <p>Loading reconcile scan…</p>}
      {list.error && <p className="error-text">{(list.error as Error).message}</p>}

      {list.data && (
        <>
          <p className="muted">
            Scanned at {list.data.scanned_at} · reserved timed out: {list.data.reserved_timed_out} ·
            executing timed out: {list.data.executing_timed_out} · apply_enabled=
            {String(list.data.apply_enabled)}
          </p>
          <OpTable title="Stuck" rows={list.data.stuck} onAbort={(id) => abort.mutate(id)} />
          <OpTable title="Executing" rows={list.data.executing} onAbort={(id) => abort.mutate(id)} />
          <OpTable title="Reserved" rows={list.data.reserved} />
        </>
      )}

      <Section title="Lookup operation">
        <div className="panel stack">
          <input
            className="mono"
            value={detailId}
            onChange={(e) => setDetailId(e.target.value)}
            placeholder="operation_id"
          />
          {detail.data && (
            <pre className="code-block">{JSON.stringify(detail.data, null, 2)}</pre>
          )}
        </div>
      </Section>
    </div>
  );
}
