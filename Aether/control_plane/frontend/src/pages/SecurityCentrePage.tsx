import { useMemo, useState } from "react";
import { Navigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api, AuditEvent } from "../api/client";
import { DataTable } from "../components/DataTable";
import { Metric, PageHeader, Section, StatusBadge } from "../components/ui";
import { canViewSecurityCentre } from "../lib/roles";
import { useAuthStore } from "../store/auth";

function matchesSecurity(e: AuditEvent): boolean {
  return /csrf|jwt|login|fail|forbidden|signature|expired|stuck|invalid|unauthorized|policy.?viol/i.test(
    `${e.action} ${e.target ?? ""} ${JSON.stringify(e.metadata ?? {})}`,
  );
}

function categorize(e: AuditEvent): string {
  const hay = `${e.action} ${e.target ?? ""}`.toLowerCase();
  if (hay.includes("login") && /fail|invalid|denied/.test(hay)) return "failed_login";
  if (hay.includes("jwt") || hay.includes("token")) return "invalid_jwt";
  if (hay.includes("csrf")) return "csrf_failure";
  if (hay.includes("policy") && /viol|reject|deny/.test(hay)) return "policy_violation";
  if (hay.includes("signature") || hay.includes("sign")) return "signature_failure";
  if (hay.includes("expired") && hay.includes("approv")) return "expired_approval";
  if (hay.includes("stuck")) return "stuck_operation";
  if (hay.includes("warn")) return "system_warning";
  return "security_event";
}

export function SecurityCentrePage() {
  const role = useAuthStore((s) => s.role);
  const audit = useQuery({ queryKey: ["audit"], queryFn: api.audit });
  const reconcile = useQuery({
    queryKey: ["reconcile"],
    queryFn: api.reconcileList,
    retry: false,
  });
  const [filter, setFilter] = useState("all");

  if (!canViewSecurityCentre(role)) {
    return <Navigate to="/dashboard" replace />;
  }

  const events = useMemo(
    () => (audit.data?.events ?? []).filter(matchesSecurity),
    [audit.data],
  );

  const counts = useMemo(() => {
    const c: Record<string, number> = {};
    for (const e of events) {
      const k = categorize(e);
      c[k] = (c[k] ?? 0) + 1;
    }
    return c;
  }, [events]);

  const filtered =
    filter === "all" ? events : events.filter((e) => categorize(e) === filter);

  return (
    <div>
      <PageHeader
        title="Security Centre"
        subtitle="Failed auth, CSRF, policy, signature, and stuck-operation signals from audit + reconcile"
      />

      <div className="metric-grid" style={{ marginBottom: "1.25rem" }}>
        <Metric label="Failed logins" value={counts.failed_login ?? 0} />
        <Metric label="Invalid JWTs" value={counts.invalid_jwt ?? 0} />
        <Metric label="CSRF failures" value={counts.csrf_failure ?? 0} />
        <Metric label="Policy violations" value={counts.policy_violation ?? 0} />
        <Metric label="Signature failures" value={counts.signature_failure ?? 0} />
        <Metric label="Expired approvals" value={counts.expired_approval ?? 0} />
        <Metric label="Stuck ops" value={reconcile.data?.stuck.length ?? counts.stuck_operation ?? 0} />
        <Metric label="Warnings" value={counts.system_warning ?? 0} />
      </div>

      <Section title="Security event stream">
        <div className="toolbar">
          <select value={filter} onChange={(e) => setFilter(e.target.value)} aria-label="Filter category">
            <option value="all">All categories</option>
            <option value="failed_login">Failed logins</option>
            <option value="invalid_jwt">Invalid JWTs</option>
            <option value="csrf_failure">CSRF failures</option>
            <option value="policy_violation">Policy violations</option>
            <option value="signature_failure">Signature failures</option>
            <option value="expired_approval">Expired approvals</option>
            <option value="stuck_operation">Stuck operations</option>
            <option value="system_warning">System warnings</option>
          </select>
        </div>
        <DataTable
          rows={filtered}
          rowKey={(e) => e.id}
          searchPlaceholder="Search action, target, operator…"
          searchFilter={(e, q) =>
            `${e.action} ${e.target ?? ""} ${e.operator_id ?? ""} ${e.created_at}`
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
              key: "cat",
              header: "Category",
              render: (e) => <StatusBadge status={categorize(e)} />,
            },
            { key: "action", header: "Action", render: (e) => e.action },
            {
              key: "target",
              header: "Target",
              render: (e) => <span className="mono">{e.target ?? "—"}</span>,
            },
            {
              key: "op",
              header: "Operator",
              render: (e) => <span className="mono">{e.operator_id?.slice(0, 10) ?? "—"}</span>,
            },
          ]}
          emptyMessage="No security-tagged events in the latest audit window."
        />
      </Section>
    </div>
  );
}
