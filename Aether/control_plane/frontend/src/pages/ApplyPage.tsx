import { FormEvent, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, DryRunReport } from "../api/client";
import { ApplyDisabledBanner, PageHeader, Section, StatusBadge } from "../components/ui";
import { useAuthStore } from "../store/auth";
import {
  canExecuteApply,
  canPrepareApply,
  canApprovePolicies,
} from "../lib/roles";

type WorkflowState = {
  policyId: string;
  dryRun?: DryRunReport;
  approvalId?: string;
  operationId?: string;
  payloadHash?: string;
  lastExecute?: string;
  lastStatus?: string;
};

export function ApplyPage() {
  const role = useAuthStore((s) => s.role);
  const queryClient = useQueryClient();
  const policies = useQuery({ queryKey: ["policies"], queryFn: api.policies });
  const [policyId, setPolicyId] = useState("");
  const [state, setState] = useState<WorkflowState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [statusOp, setStatusOp] = useState("");

  const approved = (policies.data?.policies ?? []).filter((p) => p.status === "approved");

  const dryRun = useMutation({
    mutationFn: async () => {
      if (!policyId) throw new Error("Select an approved policy");
      return api.dryRunPolicy(policyId);
    },
    onSuccess: (res) => {
      setError(null);
      setState({
        policyId,
        dryRun: res.report,
      });
    },
    onError: (e: Error) => setError(e.message),
  });

  const approve = useMutation({
    mutationFn: async () => {
      if (!state?.dryRun) throw new Error("Run dry-run first");
      return api.createApplyApproval({
        dry_run_id: state.dryRun.dry_run_id,
        execution_hash: state.dryRun.execution_hash,
        policy_id: state.dryRun.policy_id,
        policy_version: state.dryRun.policy_version,
        operation_intent: state.dryRun.predicted_protocol_operation,
      });
    },
    onSuccess: (res) => {
      setError(null);
      setState((s) => (s ? { ...s, approvalId: res.approval.approval_id } : s));
    },
    onError: (e: Error) => setError(e.message),
  });

  const cancelApproval = useMutation({
    mutationFn: async () => {
      if (!state?.approvalId) throw new Error("No approval to cancel");
      return api.cancelApplyApproval(state.approvalId);
    },
    onSuccess: () => {
      setError(null);
      setState((s) => (s ? { ...s, approvalId: undefined } : s));
    },
    onError: (e: Error) => setError(e.message),
  });

  const prepare = useMutation({
    mutationFn: async () => {
      if (!state?.dryRun || !state.approvalId) throw new Error("Approval required");
      return api.prepareApply({
        approval_id: state.approvalId,
        dry_run_id: state.dryRun.dry_run_id,
        execution_hash: state.dryRun.execution_hash,
        policy_id: state.dryRun.policy_id,
        policy_version: state.dryRun.policy_version,
        operation_intent: state.dryRun.predicted_protocol_operation,
        target_agent: undefined,
      });
    },
    onSuccess: (res) => {
      setError(null);
      setState((s) =>
        s
          ? {
              ...s,
              operationId: res.operation_id,
              payloadHash: res.payload_hash,
            }
          : s,
      );
      setStatusOp(res.operation_id);
    },
    onError: (e: Error) => setError(e.message),
  });

  const execute = useMutation({
    mutationFn: async () => {
      if (!state?.dryRun || !state.approvalId || !state.operationId) {
        throw new Error("Prepare an operation first");
      }
      if (!confirm) throw new Error("confirm must be true");
      return api.executeApply({
        operation_id: state.operationId,
        approval_id: state.approvalId,
        dry_run_id: state.dryRun.dry_run_id,
        execution_hash: state.dryRun.execution_hash,
        confirm: true,
      });
    },
    onSuccess: (res) => {
      setError(null);
      setState((s) =>
        s
          ? {
              ...s,
              lastExecute: res.outcome_code,
              lastStatus: res.phase,
            }
          : s,
      );
      queryClient.invalidateQueries({ queryKey: ["audit"] });
    },
    onError: (e: Error) => setError(e.message),
  });

  const status = useMutation({
    mutationFn: async () => {
      if (!statusOp) throw new Error("Enter operation_id");
      return api.applyOperation(statusOp);
    },
    onError: (e: Error) => setError(e.message),
  });

  return (
    <div>
      <PageHeader
        title="Apply Workflows"
        subtitle="Dry-run → attestation → approval → prepare → status. Live execution remains disabled."
      />
      <ApplyDisabledBanner />

      {error && <p className="error-text">{error}</p>}

      <Section title="1. Dry run">
        <div className="panel stack">
          <label className="muted">Approved policy</label>
          <select value={policyId} onChange={(e) => setPolicyId(e.target.value)}>
            <option value="">Select…</option>
            {approved.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} (v{p.version})
              </option>
            ))}
          </select>
          <button
            type="button"
            disabled={!policyId || dryRun.isPending || !canPrepareApply(role)}
            onClick={() => dryRun.mutate()}
          >
            Run dry-run
          </button>
          {state?.dryRun && (
            <div className="stack">
              <p>
                Executable: <StatusBadge status={String(state.dryRun.executable)} /> · Intent:{" "}
                <span className="mono">{state.dryRun.predicted_protocol_operation}</span>
              </p>
              <p className="mono muted">dry_run_id: {state.dryRun.dry_run_id}</p>
              <p className="mono muted">execution_hash: {state.dryRun.execution_hash}</p>
            </div>
          )}
        </div>
      </Section>

      <Section title="2. Approval">
        <div className="panel stack">
          <p className="muted">Admin creates Apply approval bound to the dry-run attestation.</p>
          <div className="button-row">
            <button
              type="button"
              disabled={!state?.dryRun || approve.isPending || !canApprovePolicies(role)}
              onClick={() => approve.mutate()}
            >
              Create approval
            </button>
            <button
              type="button"
              className="secondary"
              disabled={!state?.approvalId || cancelApproval.isPending || !canApprovePolicies(role)}
              onClick={() => cancelApproval.mutate()}
            >
              Cancel approval
            </button>
          </div>
          {state?.approvalId && (
            <p className="mono success-text">approval_id: {state.approvalId}</p>
          )}
        </div>
      </Section>

      <Section title="3. Prepare signature">
        <div className="panel stack">
          <button
            type="button"
            disabled={!state?.approvalId || prepare.isPending || !canPrepareApply(role)}
            onClick={() => prepare.mutate()}
          >
            Prepare signature
          </button>
          {state?.operationId && (
            <>
              <p className="mono">operation_id: {state.operationId}</p>
              <p className="mono muted">payload_hash: {state.payloadHash}</p>
            </>
          )}
        </div>
      </Section>

      <Section title="4. Execute (disabled)">
        <div className="panel stack">
          <label className="chip-row">
            <input
              type="checkbox"
              checked={confirm}
              onChange={(e) => setConfirm(e.target.checked)}
            />
            confirm = true (required)
          </label>
          <button
            type="button"
            disabled={
              !state?.operationId || !confirm || execute.isPending || !canExecuteApply(role)
            }
            onClick={() => execute.mutate()}
          >
            Submit execute
          </button>
          {state?.lastExecute && (
            <p>
              Outcome: <StatusBadge status={state.lastExecute} /> · phase {state.lastStatus}
            </p>
          )}
          <p className="muted">
            Expected while disabled: <span className="mono">APPLY_EXECUTION_DISABLED</span>, no
            protocol mutation.
          </p>
        </div>
      </Section>

      <Section title="5. Operation status">
        <form
          className="panel stack"
          onSubmit={(e: FormEvent) => {
            e.preventDefault();
            status.mutate();
          }}
        >
          <input
            className="mono"
            value={statusOp}
            onChange={(e) => setStatusOp(e.target.value)}
            placeholder="operation_id"
          />
          <button type="submit" disabled={status.isPending}>
            Lookup
          </button>
          {status.data && (
            <pre className="code-block">{JSON.stringify(status.data, null, 2)}</pre>
          )}
        </form>
      </Section>
    </div>
  );
}
