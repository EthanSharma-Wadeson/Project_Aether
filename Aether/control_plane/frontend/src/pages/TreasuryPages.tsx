import { useEffect, useMemo, useState } from "react";
import { Link, Navigate, Route, Routes, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api, TreasuryNode } from "../api/client";
import { DataTable } from "../components/DataTable";
import { Metric, PageHeader, Section, StatusBadge } from "../components/ui";
import { canAdminister } from "../lib/roles";
import { useAuthStore } from "../store/auth";

function ObservationBanner() {
  return (
    <div className="banner warn" role="status">
      <strong>Observation only.</strong> No funding, custody, or treasury mutation via the console.
      Settlement posts here are treasury accounting truth — not PROTO-4 finality evidence.
    </div>
  );
}

function sumTotals(nodes: TreasuryNode[]) {
  let available = 0;
  let reserved = 0;
  let escrow = 0;
  for (const n of nodes) {
    for (const b of n.balances) {
      available += b.available_minor;
      reserved += b.reserved_minor;
      escrow += b.escrow_reserved_minor;
    }
  }
  return { available, reserved, escrow };
}

export function TreasuryDashboardPage() {
  const role = useAuthStore((s) => s.role);
  const overview = useQuery({ queryKey: ["treasury"], queryFn: api.treasuryOverview });
  if (overview.isLoading) return <p>Loading…</p>;
  if (overview.error) return <p className="error-text">{(overview.error as Error).message}</p>;

  const nodes = overview.data?.treasuries ?? [];
  const totals = overview.data?.totals?.length
    ? overview.data.totals.reduce(
        (acc, b) => ({
          available: acc.available + b.available_minor,
          reserved: acc.reserved + b.reserved_minor,
          escrow: acc.escrow + b.escrow_reserved_minor,
        }),
        { available: 0, reserved: 0, escrow: 0 },
      )
    : sumTotals(nodes);

  const allocQuery = useQuery({
    queryKey: ["treasury-alloc-count", nodes.map((n) => n.treasury_id).join(",")],
    queryFn: async () => {
      let n = 0;
      for (const t of nodes) {
        const r = await api.treasuryAllocations(t.treasury_id);
        n += r.allocations.length;
      }
      return n;
    },
    enabled: nodes.length > 0,
  });

  return (
    <div>
      <PageHeader
        title="Treasury Dashboard"
        subtitle={`Organisation ${overview.data?.organisation_id ?? "—"} · read-only observatory`}
      />
      <ObservationBanner />
      <div className="metric-grid" style={{ marginBottom: "1.25rem" }}>
        <Metric label="Available (minor)" value={totals.available} />
        <Metric label="Reserved (minor)" value={totals.reserved} />
        <Metric label="Escrow reserved" value={totals.escrow} />
        <Metric label="Allocations" value={allocQuery.data ?? "…"} />
        <Metric label="Treasury nodes" value={nodes.length} />
      </div>
      <Section title="Quick links">
        <div className="button-row">
          <Link to="/treasury/budgets">Budget Explorer →</Link>
          <Link to="/treasury/reservations">Reservation Timeline →</Link>
          <Link to="/treasury/journal">Journal Explorer →</Link>
          <Link to="/treasury/actions">Treasury Actions →</Link>
          {canAdminister(role) && <Link to="/treasury/security">Treasury Security →</Link>}
        </div>
      </Section>
    </div>
  );
}

export function TreasuryBudgetsPage() {
  const overview = useQuery({ queryKey: ["treasury"], queryFn: api.treasuryOverview });
  const nodes = overview.data?.treasuries ?? [];

  return (
    <div>
      <PageHeader
        title="Budget Explorer"
        subtitle="Organisation → department → project hierarchy (observation)"
      />
      <ObservationBanner />
      <DataTable
        rows={nodes}
        rowKey={(r) => r.treasury_id}
        searchPlaceholder="Search name, kind, status…"
        searchFilter={(r, q) =>
          `${r.name} ${r.kind} ${r.status} ${r.treasury_id}`.toLowerCase().includes(q)
        }
        columns={[
          {
            key: "name",
            header: "Name",
            render: (r) => <Link to={`/treasury/node/${r.treasury_id}`}>{r.name}</Link>,
          },
          { key: "kind", header: "Kind", render: (r) => r.kind },
          {
            key: "status",
            header: "Status",
            render: (r) => <StatusBadge status={r.status} />,
          },
          {
            key: "parent",
            header: "Parent",
            render: (r) => (
              <span className="mono">{r.parent_treasury_id?.slice(0, 8) ?? "—"}</span>
            ),
          },
          {
            key: "avail",
            header: "Available",
            render: (r) => r.balances.reduce((s, b) => s + b.available_minor, 0),
          },
        ]}
      />
    </div>
  );
}

export function TreasuryReservationsPage() {
  const overview = useQuery({ queryKey: ["treasury"], queryFn: api.treasuryOverview });
  const nodes = overview.data?.treasuries ?? [];
  const data = useQuery({
    queryKey: ["treasury-reservations-all"],
    queryFn: async () => {
      const rows = [];
      for (const t of nodes) {
        const r = await api.treasuryReservations(t.treasury_id);
        rows.push(
          ...r.reservations.map((x) => ({ ...x, treasury_name: t.name })),
        );
      }
      return rows;
    },
    enabled: nodes.length > 0,
  });

  return (
    <div>
      <PageHeader title="Reservation Timeline" subtitle="Active, consumed, and released holds" />
      <ObservationBanner />
      <DataTable
        rows={data.data ?? []}
        rowKey={(r) => r.reservation_id}
        searchPlaceholder="Search status, kind, escrow…"
        searchFilter={(r, q) =>
          `${r.status} ${r.kind} ${r.escrow_id ?? ""} ${r.treasury_name}`.toLowerCase().includes(q)
        }
        columns={[
          { key: "treasury", header: "Treasury", render: (r) => r.treasury_name },
          {
            key: "status",
            header: "Status",
            render: (r) => <StatusBadge status={r.status} />,
          },
          { key: "kind", header: "Kind", render: (r) => r.kind },
          { key: "amt", header: "Amount", render: (r) => r.amount_minor },
          { key: "asset", header: "Asset", render: (r) => r.asset_id },
          {
            key: "when",
            header: "Updated",
            render: (r) => <span className="mono">{r.updated_at}</span>,
          },
        ]}
      />
    </div>
  );
}

export function TreasuryJournalPage() {
  const overview = useQuery({ queryKey: ["treasury"], queryFn: api.treasuryOverview });
  const [treasuryId, setTreasuryId] = useState("");
  const nodes = overview.data?.treasuries ?? [];
  const selected = treasuryId || nodes[0]?.treasury_id || "";
  const journal = useQuery({
    queryKey: ["treasury-journal", selected],
    queryFn: () => api.treasuryJournal(selected),
    enabled: !!selected,
  });

  return (
    <div>
      <PageHeader title="Journal Explorer" subtitle="Immutable double-entry lines (read-only)" />
      <ObservationBanner />
      <div className="toolbar">
        <select
          value={selected}
          onChange={(e) => setTreasuryId(e.target.value)}
          aria-label="Treasury"
        >
          {nodes.map((n) => (
            <option key={n.treasury_id} value={n.treasury_id}>
              {n.name}
            </option>
          ))}
        </select>
      </div>
      <DataTable
        rows={journal.data?.entries ?? []}
        rowKey={(e) => e.entry_id}
        pageSize={20}
        searchPlaceholder="Filter event, account, request_id…"
        searchFilter={(e, q) =>
          `${e.event_type} ${e.account_code} ${e.request_id} ${e.asset_id}`
            .toLowerCase()
            .includes(q)
        }
        columns={[
          {
            key: "time",
            header: "Time",
            render: (e) => <span className="mono">{e.created_at}</span>,
          },
          { key: "event", header: "Event", render: (e) => e.event_type },
          { key: "acct", header: "Account", render: (e) => <span className="mono">{e.account_code}</span> },
          { key: "dr", header: "Debit", render: (e) => e.debit_minor },
          { key: "cr", header: "Credit", render: (e) => e.credit_minor },
          {
            key: "req",
            header: "Request",
            render: (e) => <span className="mono">{e.request_id.slice(0, 12)}</span>,
          },
        ]}
      />
    </div>
  );
}

export function TreasurySecurityPage() {
  const role = useAuthStore((s) => s.role);
  const overview = useQuery({ queryKey: ["treasury"], queryFn: api.treasuryOverview });
  const nodes = overview.data?.treasuries ?? [];
  const [treasuryId, setTreasuryId] = useState("");
  const selected = treasuryId || nodes[0]?.treasury_id || "";
  const security = useQuery({
    queryKey: ["treasury-security", selected],
    queryFn: () => api.treasurySecurity(selected),
    enabled: !!selected && canAdminister(role),
    retry: false,
  });
  const audit = useQuery({ queryKey: ["audit"], queryFn: api.audit });

  if (!canAdminister(role)) {
    return <Navigate to="/treasury" replace />;
  }

  const treasuryEvents = useMemo(
    () =>
      (audit.data?.events ?? []).filter((e) => e.action.includes("treasury")).slice(0, 20),
    [audit.data],
  );

  return (
    <div>
      <PageHeader
        title="Treasury Security"
        subtitle="Frozen nodes, aged reservations, adjustments — admin only"
      />
      <ObservationBanner />
      <div className="toolbar">
        <select value={selected} onChange={(e) => setTreasuryId(e.target.value)}>
          {nodes.map((n) => (
            <option key={n.treasury_id} value={n.treasury_id}>
              {n.name}
            </option>
          ))}
        </select>
      </div>
      {security.data && (
        <div className="metric-grid" style={{ marginBottom: "1.25rem" }}>
          <Metric label="Status" value={security.data.treasury_status} />
          <Metric label="Frozen/expired allocs" value={security.data.frozen_or_expired_allocations} />
          <Metric label="Active reservations" value={security.data.active_reservations} />
          <Metric label="Aged active" value={security.data.aged_active_reservations} />
          <Metric label="Adjustments" value={security.data.adjustments} />
          <Metric label="Chargebacks" value={security.data.chargebacks} />
        </div>
      )}
      <Section title="Related audit events">
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
              {treasuryEvents.map((e) => (
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
    </div>
  );
}

export function TreasuryNodePage() {
  const { id = "" } = useParams();
  const detail = useQuery({
    queryKey: ["treasury-node", id],
    queryFn: () => api.treasury(id),
    enabled: !!id,
  });
  const allocs = useQuery({
    queryKey: ["treasury-node-alloc", id],
    queryFn: () => api.treasuryAllocations(id),
    enabled: !!id,
  });

  if (detail.isLoading) return <p>Loading…</p>;
  if (detail.error) return <p className="error-text">{(detail.error as Error).message}</p>;
  const t = detail.data?.treasury;

  return (
    <div>
      <p>
        <Link to="/treasury/budgets">← Budgets</Link>
      </p>
      <PageHeader title={t?.name ?? "Treasury"} subtitle={t?.kind} />
      <ObservationBanner />
      <Section title="Balances">
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Asset</th>
                <th>Available</th>
                <th>Reserved</th>
                <th>Escrow</th>
              </tr>
            </thead>
            <tbody>
              {(t?.balances ?? []).map((b) => (
                <tr key={b.asset_id}>
                  <td>{b.asset_id}</td>
                  <td>{b.available_minor}</td>
                  <td>{b.reserved_minor}</td>
                  <td>{b.escrow_reserved_minor}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Section>
      <Section title="Agent allocations">
        <DataTable
          rows={allocs.data?.allocations ?? []}
          rowKey={(a) => a.allocation_id}
          columns={[
            { key: "agent", header: "Agent", render: (a) => <span className="mono">{a.agent_id}</span> },
            {
              key: "status",
              header: "Status",
              render: (a) => <StatusBadge status={a.status} />,
            },
            { key: "ceil", header: "Ceiling", render: (a) => a.ceiling_minor },
            { key: "rem", header: "Remaining", render: (a) => a.remaining_minor },
            { key: "asset", header: "Asset", render: (a) => a.asset_id },
          ]}
        />
      </Section>
    </div>
  );
}

export function TreasuryRoutes() {
  return (
    <Routes>
      <Route index element={<TreasuryDashboardPage />} />
      <Route path="budgets" element={<TreasuryBudgetsPage />} />
      <Route path="reservations" element={<TreasuryReservationsPage />} />
      <Route path="journal" element={<TreasuryJournalPage />} />
      <Route path="actions" element={<TreasuryActionsPage />} />
      <Route path="security" element={<TreasurySecurityPage />} />
      <Route path="node/:id" element={<TreasuryNodePage />} />
    </Routes>
  );
}

function LedgerBanner() {
  return (
    <div className="banner warn" role="status">
      <strong>Internal ledger governance only.</strong> No external money movement, custody, banking,
      or payment rails. Apply remains disabled.
    </div>
  );
}

export function TreasuryActionsPage() {
  const role = useAuthStore((s) => s.role);
  const overview = useQuery({ queryKey: ["treasury"], queryFn: api.treasuryOverview });
  const pending = useQuery({
    queryKey: ["treasury-mutations", "pending"],
    queryFn: () => api.treasuryMutations("pending"),
  });
  const approved = useQuery({
    queryKey: ["treasury-mutations", "approved"],
    queryFn: () => api.treasuryMutations("approved"),
  });
  const history = useQuery({
    queryKey: ["treasury-mutations", "all"],
    queryFn: () => api.treasuryMutations(),
  });
  const timeline = useQuery({
    queryKey: ["treasury-mutation-timeline"],
    queryFn: api.treasuryMutationTimeline,
  });

  const [treasuryId, setTreasuryId] = useState("");
  const [agentId, setAgentId] = useState("agent-demo");
  const [ceiling, setCeiling] = useState("1000");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const nodes = overview.data?.treasuries ?? [];
  useEffect(() => {
    if (!treasuryId && nodes[0]) setTreasuryId(nodes[0].treasury_id);
  }, [nodes, treasuryId]);

  async function refresh() {
    await Promise.all([
      pending.refetch(),
      approved.refetch(),
      history.refetch(),
      timeline.refetch(),
    ]);
  }

  async function createAllocationRequest() {
    setError(null);
    setMessage(null);
    try {
      await api.csrfToken();
      const res = await api.treasuryCreateMutation({
        operation: "allocation_create",
        idempotency_key: `ui-alloc-${Date.now()}`,
        payload: {
          treasury_id: treasuryId,
          agent_id: agentId,
          asset_id: "GBP",
          ceiling_minor: Number(ceiling),
          initial_minor: 0,
        },
      });
      setMessage(`Request ${res.mutation_id} created (${res.status})`);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "request failed");
    }
  }

  async function approve(id: string) {
    setError(null);
    try {
      await api.csrfToken();
      await api.treasuryApproveMutation(id, "approve");
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "approve failed");
    }
  }

  async function execute(id: string) {
    setError(null);
    try {
      await api.csrfToken();
      const res = await api.treasuryExecuteMutation(id, `ui-exec-${Date.now()}`);
      setMessage(`Executed ${res.mutation_id ?? id}: ${res.outcome}`);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "execute failed");
    }
  }

  async function freezeSelected() {
    setError(null);
    try {
      await api.csrfToken();
      await api.treasuryFreeze(treasuryId, `ui-freeze-${Date.now()}`, "console emergency freeze");
      setMessage(`Frozen ${treasuryId}`);
      await overview.refetch();
    } catch (e) {
      setError(e instanceof Error ? e.message : "freeze failed");
    }
  }

  async function requestUnfreeze() {
    setError(null);
    try {
      await api.csrfToken();
      const res = await api.treasuryCreateMutation({
        operation: "unfreeze",
        idempotency_key: `ui-unfreeze-${Date.now()}`,
        payload: { treasury_id: treasuryId },
      });
      setMessage(`Unfreeze request ${res.mutation_id}`);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "unfreeze request failed");
    }
  }

  return (
    <div>
      <PageHeader
        title="Treasury Actions"
        subtitle="Internal ledger governance — dual-control for high-risk changes"
      />
      <LedgerBanner />
      {message && <p className="muted">{message}</p>}
      {error && <p className="error-text">{error}</p>}

      <Section title="Create allocation request">
        <div className="form-row" style={{ gap: "0.75rem", flexWrap: "wrap" }}>
          <label>
            Treasury{" "}
            <select value={treasuryId} onChange={(e) => setTreasuryId(e.target.value)}>
              {nodes.map((n) => (
                <option key={n.treasury_id} value={n.treasury_id}>
                  {n.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            Agent{" "}
            <input value={agentId} onChange={(e) => setAgentId(e.target.value)} />
          </label>
          <label>
            Ceiling (minor){" "}
            <input value={ceiling} onChange={(e) => setCeiling(e.target.value)} />
          </label>
          <button type="button" onClick={createAllocationRequest} disabled={!treasuryId}>
            Submit request
          </button>
          {canAdminister(role) && (
            <button type="button" onClick={freezeSelected} disabled={!treasuryId}>
              Emergency freeze
            </button>
          )}
          <button type="button" onClick={requestUnfreeze} disabled={!treasuryId}>
            Request unfreeze
          </button>
        </div>
      </Section>

      <Section title="Approval queue (pending)">
        <DataTable
          rows={pending.data?.mutations ?? []}
          columns={[
            { key: "id", header: "Mutation", render: (m) => m.mutation_id.slice(0, 8) },
            { key: "op", header: "Operation", render: (m) => m.operation },
            { key: "by", header: "Requester", render: (m) => m.requested_by.slice(0, 8) },
            {
              key: "act",
              header: "Actions",
              render: (m) =>
                canAdminister(role) ? (
                  <button type="button" onClick={() => approve(m.mutation_id)}>
                    Approve
                  </button>
                ) : (
                  <span className="muted">admin only</span>
                ),
            },
          ]}
        />
      </Section>

      <Section title="Ready to execute (approved)">
        <DataTable
          rows={approved.data?.mutations ?? []}
          columns={[
            { key: "id", header: "Mutation", render: (m) => m.mutation_id.slice(0, 8) },
            { key: "op", header: "Operation", render: (m) => m.operation },
            {
              key: "act",
              header: "Actions",
              render: (m) =>
                canAdminister(role) ? (
                  <button type="button" onClick={() => execute(m.mutation_id)}>
                    Execute
                  </button>
                ) : (
                  <span className="muted">admin only</span>
                ),
            },
          ]}
        />
      </Section>

      <Section title="Approval history">
        <DataTable
          rows={history.data?.mutations ?? []}
          columns={[
            { key: "id", header: "Mutation", render: (m) => m.mutation_id.slice(0, 8) },
            { key: "op", header: "Operation", render: (m) => m.operation },
            {
              key: "st",
              header: "Status",
              render: (m) => <StatusBadge status={m.status} />,
            },
            { key: "out", header: "Outcome", render: (m) => m.outcome ?? "—" },
            { key: "j", header: "Journal", render: (m) => m.journal_batch_id?.slice(0, 8) ?? "—" },
          ]}
        />
      </Section>

      <Section title="Mutation timeline">
        <DataTable
          rows={timeline.data?.events ?? []}
          columns={[
            { key: "t", header: "Time", render: (e) => e.created_at },
            { key: "a", header: "Actor", render: (e) => e.actor.slice(0, 8) },
            { key: "op", header: "Operation", render: (e) => e.operation },
            { key: "o", header: "Outcome", render: (e) => e.outcome },
            { key: "j", header: "Journal", render: (e) => e.journal_reference?.slice(0, 8) ?? "—" },
          ]}
        />
      </Section>
    </div>
  );
}
