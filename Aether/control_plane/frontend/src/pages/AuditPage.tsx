import { useMemo, useState } from "react";
import { api, AuditEvent } from "../api/client";
import { useQuery } from "@tanstack/react-query";
import { DataTable } from "../components/DataTable";
import { PageHeader } from "../components/ui";

function metaText(e: AuditEvent): string {
  try {
    return JSON.stringify(e.metadata ?? {});
  } catch {
    return "";
  }
}

function download(filename: string, content: string, type: string) {
  const blob = new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export function AuditPage() {
  const audit = useQuery({ queryKey: ["audit"], queryFn: api.audit });
  const [timeFrom, setTimeFrom] = useState("");
  const [timeTo, setTimeTo] = useState("");
  const [operator, setOperator] = useState("");
  const [agent, setAgent] = useState("");
  const [policy, setPolicy] = useState("");
  const [requestId, setRequestId] = useState("");
  const [operationId, setOperationId] = useState("");
  const [eventType, setEventType] = useState("");

  const filtered = useMemo(() => {
    let rows = audit.data?.events ?? [];
    if (timeFrom) rows = rows.filter((e) => e.created_at >= timeFrom);
    if (timeTo) rows = rows.filter((e) => e.created_at <= timeTo);
    if (operator.trim()) {
      const q = operator.trim().toLowerCase();
      rows = rows.filter((e) => (e.operator_id ?? "").toLowerCase().includes(q));
    }
    if (agent.trim()) {
      const q = agent.trim().toLowerCase();
      rows = rows.filter((e) => `${e.target ?? ""} ${metaText(e)}`.toLowerCase().includes(q));
    }
    if (policy.trim()) {
      const q = policy.trim().toLowerCase();
      rows = rows.filter(
        (e) =>
          e.action.toLowerCase().includes("policy") &&
          `${e.target ?? ""} ${metaText(e)}`.toLowerCase().includes(q),
      );
    }
    if (requestId.trim()) {
      const q = requestId.trim().toLowerCase();
      rows = rows.filter((e) => metaText(e).toLowerCase().includes(q));
    }
    if (operationId.trim()) {
      const q = operationId.trim().toLowerCase();
      rows = rows.filter((e) => `${e.target ?? ""} ${metaText(e)}`.toLowerCase().includes(q));
    }
    if (eventType.trim()) {
      const q = eventType.trim().toLowerCase();
      rows = rows.filter((e) => e.action.toLowerCase().includes(q));
    }
    return rows;
  }, [
    audit.data,
    timeFrom,
    timeTo,
    operator,
    agent,
    policy,
    requestId,
    operationId,
    eventType,
  ]);

  function exportJson() {
    download(
      `aether-audit-${Date.now()}.json`,
      JSON.stringify({ exported_at: new Date().toISOString(), events: filtered }, null, 2),
      "application/json",
    );
  }

  function exportCsv() {
    const header = ["id", "created_at", "operator_id", "action", "target", "metadata"];
    const lines = [
      header.join(","),
      ...filtered.map((e) =>
        [
          e.id,
          e.created_at,
          e.operator_id ?? "",
          e.action,
          e.target ?? "",
          JSON.stringify(e.metadata ?? {}),
        ]
          .map((cell) => `"${String(cell).replace(/"/g, '""')}"`)
          .join(","),
      ),
    ];
    download(`aether-audit-${Date.now()}.csv`, lines.join("\n"), "text/csv");
  }

  if (audit.isLoading) return <p>Loading…</p>;
  if (audit.error) return <p className="error-text">Error: {(audit.error as Error).message}</p>;

  return (
    <div>
      <PageHeader
        title="Audit Explorer"
        subtitle="Searchable enterprise audit stream with JSON/CSV export"
        actions={
          <div className="button-row" style={{ marginTop: 0 }}>
            <button type="button" className="secondary" onClick={exportCsv}>
              Export CSV
            </button>
            <button type="button" className="secondary" onClick={exportJson}>
              Export JSON
            </button>
          </div>
        }
      />

      <div className="panel" style={{ marginBottom: "1rem" }}>
        <div className="grid-3" style={{ gap: "0.75rem" }}>
          <div className="stack">
            <label className="muted">Time from (ISO)</label>
            <input className="mono" value={timeFrom} onChange={(e) => setTimeFrom(e.target.value)} />
          </div>
          <div className="stack">
            <label className="muted">Time to (ISO)</label>
            <input className="mono" value={timeTo} onChange={(e) => setTimeTo(e.target.value)} />
          </div>
          <div className="stack">
            <label className="muted">Event type</label>
            <input value={eventType} onChange={(e) => setEventType(e.target.value)} placeholder="e.g. policy" />
          </div>
          <div className="stack">
            <label className="muted">Operator</label>
            <input value={operator} onChange={(e) => setOperator(e.target.value)} />
          </div>
          <div className="stack">
            <label className="muted">Agent</label>
            <input value={agent} onChange={(e) => setAgent(e.target.value)} />
          </div>
          <div className="stack">
            <label className="muted">Policy</label>
            <input value={policy} onChange={(e) => setPolicy(e.target.value)} />
          </div>
          <div className="stack">
            <label className="muted">request_id</label>
            <input className="mono" value={requestId} onChange={(e) => setRequestId(e.target.value)} />
          </div>
          <div className="stack">
            <label className="muted">operation_id</label>
            <input
              className="mono"
              value={operationId}
              onChange={(e) => setOperationId(e.target.value)}
            />
          </div>
        </div>
      </div>

      <DataTable
        rows={filtered}
        rowKey={(e) => e.id}
        pageSize={20}
        searchPlaceholder="Quick search…"
        searchFilter={(e, q) =>
          `${e.action} ${e.target ?? ""} ${e.operator_id ?? ""} ${metaText(e)}`
            .toLowerCase()
            .includes(q)
        }
        columns={[
          {
            key: "time",
            header: "Time",
            render: (e) => <span className="mono">{e.created_at}</span>,
          },
          {
            key: "op",
            header: "Operator",
            render: (e) => <span className="mono">{e.operator_id?.slice(0, 12) ?? "—"}</span>,
          },
          { key: "action", header: "Event type", render: (e) => e.action },
          {
            key: "target",
            header: "Target",
            render: (e) => <span className="mono">{e.target ?? "—"}</span>,
          },
          {
            key: "meta",
            header: "Metadata",
            render: (e) => (
              <span className="mono muted">{metaText(e).slice(0, 80)}</span>
            ),
          },
        ]}
      />
    </div>
  );
}
