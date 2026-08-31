import { FormEvent, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";
import { ApplyDisabledBanner, PageHeader, StatusBadge } from "../components/ui";
import { canApprovePolicies, canWritePolicies } from "../lib/roles";
import { useAuthStore } from "../store/auth";

export function PolicyDetailPage() {
  const { id = "" } = useParams();
  const role = useAuthStore((s) => s.role);
  const queryClient = useQueryClient();
  const detail = useQuery({
    queryKey: ["policy", id],
    queryFn: () => api.policy(id),
    enabled: !!id,
  });

  const policy = detail.data?.policy;
  const canEdit =
    canWritePolicies(role) &&
    policy &&
    (policy.status === "draft" || policy.status === "rejected");
  const canSubmit = canEdit && policy?.status === "draft";
  const canReview = canApprovePolicies(role) && policy?.status === "pending_review";
  const canArchive =
    canApprovePolicies(role) &&
    policy &&
    (policy.status === "approved" || policy.status === "rejected");

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [policyType, setPolicyType] = useState("");
  const [policyData, setPolicyData] = useState("");
  const [rejectReason, setRejectReason] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!policy) return;
    setName(policy.name);
    setDescription(policy.description);
    setPolicyType(policy.policy_type);
    setPolicyData(JSON.stringify(policy.policy_data, null, 2));
  }, [policy]);

  function invalidate() {
    queryClient.invalidateQueries({ queryKey: ["policy", id] });
    queryClient.invalidateQueries({ queryKey: ["policies"] });
  }

  const save = useMutation({
    mutationFn: async () => {
      const data = JSON.parse(policyData);
      return api.updatePolicy(id, {
        name,
        description,
        policy_type: policyType,
        policy_data: data,
      });
    },
    onSuccess: () => {
      invalidate();
      setError(null);
    },
    onError: (err: Error) => setError(err.message),
  });

  const submit = useMutation({
    mutationFn: () => api.submitPolicy(id),
    onSuccess: invalidate,
    onError: (err: Error) => setError(err.message),
  });
  const approve = useMutation({
    mutationFn: () => api.approvePolicy(id),
    onSuccess: invalidate,
    onError: (err: Error) => setError(err.message),
  });
  const reject = useMutation({
    mutationFn: () => api.rejectPolicy(id, rejectReason || undefined),
    onSuccess: invalidate,
    onError: (err: Error) => setError(err.message),
  });
  const archive = useMutation({
    mutationFn: () => api.archivePolicy(id),
    onSuccess: invalidate,
    onError: (err: Error) => setError(err.message),
  });

  if (detail.isLoading) return <p>Loading…</p>;
  if (detail.error) return <p className="error-text">Error: {(detail.error as Error).message}</p>;
  if (!policy) return <p>Not found</p>;

  const versions = detail.data?.versions ?? [];
  const prev = versions.length > 1 ? versions[1] : null;

  return (
    <div>
      <p>
        <Link to="/policies">← Policies</Link>
      </p>
      <PageHeader
        title={policy.name}
        subtitle={`v${policy.version} · hash ${policy.hash.slice(0, 12)}…`}
        actions={<StatusBadge status={policy.status} />}
      />
      <ApplyDisabledBanner />
      {detail.data?.note && <p className="muted">{detail.data.note}</p>}
      {error && <p className="error-text">{error}</p>}

      <div className="grid-2">
        <div className="panel stack">
          <h3>{canEdit ? "Review / edit" : "Policy definition"}</h3>
          {canEdit ? (
            <form
              className="stack"
              onSubmit={(e: FormEvent) => {
                e.preventDefault();
                try {
                  JSON.parse(policyData);
                  save.mutate();
                } catch {
                  setError("policy_data must be valid JSON");
                }
              }}
            >
              <label className="muted">Name</label>
              <input value={name} onChange={(e) => setName(e.target.value)} />
              <label className="muted">Description</label>
              <input value={description} onChange={(e) => setDescription(e.target.value)} />
              <label className="muted">Type</label>
              <input value={policyType} onChange={(e) => setPolicyType(e.target.value)} />
              <label className="muted">Data (JSON)</label>
              <textarea rows={10} value={policyData} onChange={(e) => setPolicyData(e.target.value)} />
              <div className="button-row">
                <button type="submit" disabled={save.isPending}>
                  Save new version
                </button>
                {canSubmit && (
                  <button type="button" onClick={() => submit.mutate()} disabled={submit.isPending}>
                    Submit for review
                  </button>
                )}
              </div>
            </form>
          ) : (
            <pre className="code-block">{JSON.stringify(policy.policy_data, null, 2)}</pre>
          )}

          {canReview && (
            <div className="stack">
              <h3>Approve / cancel</h3>
              <p className="muted">
                Approval records governance intent only. Cancel maps to reject — never applies to
                protocol.
              </p>
              <label className="muted">Cancel / reject reason</label>
              <input value={rejectReason} onChange={(e) => setRejectReason(e.target.value)} />
              <div className="button-row">
                <button type="button" onClick={() => approve.mutate()} disabled={approve.isPending}>
                  Approve policy
                </button>
                <button type="button" className="secondary" onClick={() => reject.mutate()}>
                  Cancel policy
                </button>
              </div>
            </div>
          )}

          {canArchive && (
            <div className="button-row">
              <button type="button" className="secondary" onClick={() => archive.mutate()}>
                Archive policy
              </button>
            </div>
          )}

          {policy.status === "approved" && (
            <p className="muted">
              Next: use <Link to="/apply">Apply Workflows</Link> for dry-run / prepare (execution
              disabled).
            </p>
          )}
        </div>

        <div className="stack">
          <div className="panel">
            <h3>Version history</h3>
            <table>
              <thead>
                <tr>
                  <th>Ver</th>
                  <th>Action</th>
                  <th>Status</th>
                  <th>When</th>
                </tr>
              </thead>
              <tbody>
                {versions.map((v) => (
                  <tr key={v.id}>
                    <td>v{v.version}</td>
                    <td>{v.change_action}</td>
                    <td>
                      <StatusBadge status={v.status} />
                    </td>
                    <td className="mono">{v.changed_at}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {prev && (
            <div className="panel">
              <h3>Diff viewer</h3>
              <p className="muted">
                v{prev.version} → v{policy.version}
              </p>
              <div className="grid-2">
                <div>
                  <p className="muted">Previous</p>
                  <pre className="code-block">{JSON.stringify(prev.policy_data, null, 2)}</pre>
                </div>
                <div>
                  <p className="muted">Current</p>
                  <pre className="code-block">{JSON.stringify(policy.policy_data, null, 2)}</pre>
                </div>
              </div>
            </div>
          )}

          <div className="panel">
            <h3>Audit history</h3>
            <table>
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {(detail.data?.audit_events ?? []).map((e) => (
                  <tr key={e.id}>
                    <td className="mono">{e.created_at}</td>
                    <td>{e.action}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
