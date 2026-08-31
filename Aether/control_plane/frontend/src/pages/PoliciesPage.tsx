import { FormEvent, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, PolicyTemplate } from "../api/client";
import { DataTable } from "../components/DataTable";
import { ApplyDisabledBanner, PageHeader, StatusBadge } from "../components/ui";
import { canWritePolicies } from "../lib/roles";
import { useAuthStore } from "../store/auth";

export function PoliciesPage() {
  const role = useAuthStore((s) => s.role);
  const queryClient = useQueryClient();
  const policies = useQuery({ queryKey: ["policies"], queryFn: api.policies });
  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [policyType, setPolicyType] = useState("capability_constraints");
  const [policyData, setPolicyData] = useState('{\n  "max_spend": 1000,\n  "actions": ["transfer"]\n}');
  const [error, setError] = useState<string | null>(null);

  const canWrite = canWritePolicies(role);

  const create = useMutation({
    mutationFn: async () => {
      let data: unknown;
      try {
        data = JSON.parse(policyData);
      } catch {
        throw new Error("policy_data must be valid JSON");
      }
      return api.createPolicy({
        name,
        description,
        policy_type: policyType,
        policy_data: data,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["policies"] });
      setShowCreate(false);
      setName("");
      setError(null);
    },
    onError: (err: Error) => setError(err.message),
  });

  const rows = useMemo(() => policies.data?.policies ?? [], [policies.data]);

  if (policies.isLoading) return <p>Loading…</p>;
  if (policies.error) return <p className="error-text">Error: {(policies.error as Error).message}</p>;

  return (
    <div>
      <PageHeader
        title="Policy Management"
        subtitle="Create, review, approve, cancel (reject), and archive templates — no Apply execution"
        actions={
          canWrite ? (
            <button type="button" onClick={() => setShowCreate((v) => !v)}>
              {showCreate ? "Close" : "Create policy"}
            </button>
          ) : undefined
        }
      />
      <ApplyDisabledBanner />

      {showCreate && (
        <form
          className="panel stack"
          style={{ marginBottom: "1rem" }}
          onSubmit={(e: FormEvent) => {
            e.preventDefault();
            create.mutate();
          }}
        >
          <h3>Create draft</h3>
          <label className="muted">Name</label>
          <input value={name} onChange={(e) => setName(e.target.value)} required />
          <label className="muted">Description</label>
          <input value={description} onChange={(e) => setDescription(e.target.value)} />
          <label className="muted">Policy type</label>
          <input value={policyType} onChange={(e) => setPolicyType(e.target.value)} required />
          <label className="muted">Policy data (JSON)</label>
          <textarea rows={8} value={policyData} onChange={(e) => setPolicyData(e.target.value)} />
          {error && <p className="error-text">{error}</p>}
          <button type="submit" disabled={create.isPending}>
            Save draft
          </button>
        </form>
      )}

      <DataTable
        rows={rows}
        rowKey={(p) => p.id}
        searchPlaceholder="Search name, status, owner…"
        searchFilter={(p: PolicyTemplate, q) =>
          `${p.name} ${p.status} ${p.created_by_username ?? ""} ${p.policy_type}`
            .toLowerCase()
            .includes(q)
        }
        columns={[
          {
            key: "name",
            header: "Name",
            render: (p) => <Link to={`/policies/${p.id}`}>{p.name}</Link>,
          },
          {
            key: "status",
            header: "Status",
            render: (p) => <StatusBadge status={p.status} />,
          },
          { key: "ver", header: "Version", render: (p) => `v${p.version}` },
          {
            key: "owner",
            header: "Owner",
            render: (p) => p.created_by_username ?? p.created_by.slice(0, 8),
          },
          {
            key: "updated",
            header: "Updated",
            render: (p) => <span className="mono">{p.updated_at}</span>,
          },
        ]}
      />
    </div>
  );
}
