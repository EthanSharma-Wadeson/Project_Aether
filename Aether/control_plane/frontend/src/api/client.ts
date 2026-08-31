import { useAuthStore } from "../store/auth";

const API_BASE = "";

function csrfHeader(): HeadersInit {
  const csrf = useAuthStore.getState().csrfToken;
  return csrf ? { "X-CSRF-Token": csrf } : {};
}

async function request<T>(path: string, init?: RequestInit & { mutate?: boolean }): Promise<T> {
  const token = useAuthStore.getState().token;
  const headers = new Headers(init?.headers);
  headers.set("Content-Type", "application/json");
  // Do not attach a stale Bearer token to login/refresh.
  const isAuthForm = path.startsWith("/auth/login") || path.startsWith("/auth/refresh");
  if (token && !isAuthForm) {
    headers.set("Authorization", `Bearer ${token}`);
  }
  if (init?.mutate) {
    Object.entries(csrfHeader()).forEach(([k, v]) => headers.set(k, v));
  }
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers,
    credentials: "same-origin",
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    const message =
      body.error ?? body.code ?? body.outcome_code ?? body.message ?? "request failed";
    const text = typeof message === "string" ? message : "request failed";
    // Stale/invalid session — clear and force re-login (except on the login call itself).
    if (
      !isAuthForm &&
      (response.status === 401 ||
        /invalid access token|jwt error|missing field 'iss'|missing authorization/i.test(text))
    ) {
      useAuthStore.getState().clear();
      if (typeof window !== "undefined" && !window.location.pathname.startsWith("/login")) {
        window.location.assign("/login");
      }
    }
    throw new Error(text);
  }
  if (response.status === 204) {
    return undefined as T;
  }
  return response.json() as Promise<T>;
}

export const api = {
  login: (username: string, password: string) =>
    request<{ access_token: string; role: string; csrf_token: string }>("/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    }),
  me: () => request<{ operator_id: string; username: string; role: string }>("/api/me"),
  csrfToken: () =>
    request<{ csrf_token: string }>("/api/csrf-token").then(async (r) => {
      useAuthStore.getState().setCsrf(r.csrf_token);
      return r;
    }),
  agents: () => request<{ agents: AgentSummary[] }>("/api/agents"),
  agent: (id: string) => request<AgentDetail>(`/api/agents/${encodeURIComponent(id)}`),
  agentCapabilities: (id: string) =>
    request<{ capabilities: CapabilityView[] }>(
      `/api/agents/${encodeURIComponent(id)}/capabilities`,
    ),
  agentEscrows: (id: string) =>
    request<{ escrows: EscrowView[] }>(`/api/agents/${encodeURIComponent(id)}/escrows`),
  agentSettlements: (id: string) =>
    request<{ settlements: SettlementView[] }>(
      `/api/agents/${encodeURIComponent(id)}/settlements`,
    ),
  agentReputation: (id: string) =>
    request<ReputationResponse>(`/api/agents/${encodeURIComponent(id)}/reputation`),
  agentEvents: (id: string) =>
    request<{ events: TimelineEvent[] }>(`/api/agents/${encodeURIComponent(id)}/events`),
  capabilities: () => request<{ capabilities: CapabilityView[] }>("/api/capabilities"),
  escrows: () => request<{ escrows: EscrowView[] }>("/api/escrows"),
  settlements: () => request<{ settlements: SettlementView[] }>("/api/settlements"),
  audit: () => request<{ events: AuditEvent[] }>("/api/audit"),
  system: () => request<SystemHealth>("/api/system"),
  policies: () => request<{ policies: PolicyTemplate[]; note?: string }>("/api/policies"),
  policy: (id: string) =>
    request<{
      policy: PolicyTemplate;
      versions: PolicyVersion[];
      audit_events: AuditEvent[];
      protocol_active: boolean;
      note?: string;
    }>(`/api/policies/${id}`),
  createPolicy: (body: CreatePolicyBody) =>
    request<{ policy: PolicyTemplate }>("/api/policies", {
      method: "POST",
      body: JSON.stringify(body),
      mutate: true,
    }),
  updatePolicy: (id: string, body: UpdatePolicyBody) =>
    request<{ policy: PolicyTemplate }>(`/api/policies/${id}`, {
      method: "PUT",
      body: JSON.stringify(body),
      mutate: true,
    }),
  submitPolicy: (id: string) =>
    request<{ policy: PolicyTemplate }>(`/api/policies/${id}/submit`, {
      method: "POST",
      body: "{}",
      mutate: true,
    }),
  approvePolicy: (id: string) =>
    request<{ policy: PolicyTemplate }>(`/api/policies/${id}/approve`, {
      method: "POST",
      body: "{}",
      mutate: true,
    }),
  rejectPolicy: (id: string, reason?: string) =>
    request<{ policy: PolicyTemplate }>(`/api/policies/${id}/reject`, {
      method: "POST",
      body: JSON.stringify({ reason }),
      mutate: true,
    }),
  archivePolicy: (id: string) =>
    request<{ policy: PolicyTemplate }>(`/api/policies/${id}/archive`, {
      method: "POST",
      body: "{}",
      mutate: true,
    }),
  dryRunPolicy: (id: string) =>
    request<{
      report: DryRunReport;
      protocol_mutated: boolean;
      apply_enabled: boolean;
      note?: string;
    }>(`/api/policies/${id}/dry-run`, {
      method: "POST",
      body: "{}",
      mutate: true,
    }),

  // Apply surface (execution disabled)
  createApplyApproval: (body: CreateApprovalBody) =>
    request<{ approval: ApplyApproval; apply_enabled: boolean; protocol_mutated: boolean }>(
      "/api/apply-approvals",
      { method: "POST", body: JSON.stringify(body), mutate: true },
    ),
  getApplyApproval: (id: string) =>
    request<{ approval: ApplyApproval; apply_enabled: boolean }>(`/api/apply-approvals/${id}`),
  cancelApplyApproval: (id: string) =>
    request<{ approval: ApplyApproval }>(`/api/apply-approvals/${id}`, {
      method: "DELETE",
      mutate: true,
    }),
  prepareApply: (body: PrepareApplyBody) =>
    request<PrepareApplyResponse>("/api/apply/prepare", {
      method: "POST",
      body: JSON.stringify(body),
      mutate: true,
    }),
  executeApply: (body: ExecuteApplyBody) =>
    request<ExecuteApplyResponse>("/api/apply", {
      method: "POST",
      body: JSON.stringify(body),
      mutate: true,
    }),
  applyOperation: (operationId: string) =>
    request<ApplyOperationStatus>(`/api/apply/operations/${encodeURIComponent(operationId)}`),
  reconcileList: () => request<ReconcileListResponse>("/api/apply/reconcile"),
  reconcileGet: (operationId: string) =>
    request<{ operation: ReplayRecord; apply_enabled: boolean }>(
      `/api/apply/reconcile/${encodeURIComponent(operationId)}`,
    ),
  reconcileAbort: (operationId: string, reason?: string) =>
    request<{ operation: ReplayRecord; protocol_mutated: boolean }>(
      `/api/apply/reconcile/${encodeURIComponent(operationId)}/abort`,
      {
        method: "POST",
        body: JSON.stringify({ reason }),
        mutate: true,
      },
    ),

  // Treasury observation (read-only)
  treasuryOverview: () => request<TreasuryOverview>("/api/treasury"),
  treasury: (id: string) =>
    request<{ treasury: TreasuryNode; note?: string }>(
      `/api/treasury/${encodeURIComponent(id)}`,
    ),
  treasuryAllocations: (id: string) =>
    request<{ allocations: TreasuryAllocation[]; read_only: boolean }>(
      `/api/treasury/${encodeURIComponent(id)}/allocations`,
    ),
  treasuryReservations: (id: string) =>
    request<{ reservations: TreasuryReservation[]; read_only: boolean; note?: string }>(
      `/api/treasury/${encodeURIComponent(id)}/reservations`,
    ),
  treasuryJournal: (id: string) =>
    request<{ entries: TreasuryJournalEntry[]; immutable: boolean; read_only: boolean }>(
      `/api/treasury/${encodeURIComponent(id)}/journal`,
    ),
  treasurySettlements: (id: string) =>
    request<{
      posts: TreasurySettlementPost[];
      read_only: boolean;
      label: string;
      note?: string;
    }>(`/api/treasury/${encodeURIComponent(id)}/settlements`),
  treasurySecurity: (id: string) =>
    request<TreasurySecurityView>(`/api/treasury/${encodeURIComponent(id)}/security`),

  // Phase 21 — internal ledger governance writes (no external money movement)
  treasuryMutations: (status?: string) =>
    request<{ mutations: TreasuryMutation[]; ledger_notice: string }>(
      status ? `/api/treasury/mutations?status=${encodeURIComponent(status)}` : "/api/treasury/mutations",
    ),
  treasuryMutationTimeline: () =>
    request<{ events: TreasuryMutationAuditEvent[]; ledger_notice: string }>(
      "/api/treasury/mutations/timeline",
    ),
  treasuryCreateMutation: (body: {
    operation: string;
    payload: Record<string, unknown>;
    idempotency_key: string;
    target?: string;
  }) =>
    request<TreasuryMutation>("/api/treasury/mutations", {
      method: "POST",
      body: JSON.stringify(body),
      mutate: true,
    }),
  treasuryApproveMutation: (id: string, decision: "approve" | "reject", reason?: string) =>
    request<TreasuryMutation>(`/api/treasury/mutations/${encodeURIComponent(id)}/approve`, {
      method: "POST",
      body: JSON.stringify({ decision, reason }),
      mutate: true,
    }),
  treasuryExecuteMutation: (id: string, idempotency_key: string) =>
    request<TreasuryExecuteResult>(`/api/treasury/mutations/${encodeURIComponent(id)}/execute`, {
      method: "POST",
      body: JSON.stringify({ confirm: true, idempotency_key }),
      mutate: true,
    }),
  treasuryCancelMutation: (id: string) =>
    request<TreasuryMutation>(`/api/treasury/mutations/${encodeURIComponent(id)}/cancel`, {
      method: "POST",
      body: JSON.stringify({}),
      mutate: true,
    }),
  treasuryFreeze: (id: string, idempotency_key: string, reason?: string) =>
    request<TreasuryExecuteResult>(`/api/treasury/${encodeURIComponent(id)}/freeze`, {
      method: "POST",
      body: JSON.stringify({ confirm: true, idempotency_key, reason }),
      mutate: true,
    }),
  treasuryCreateReservation: (
    treasuryId: string,
    body: {
      allocation_id: string;
      asset_id: string;
      amount_minor: number;
      idempotency_key: string;
    },
  ) =>
    request<TreasuryExecuteResult>(
      `/api/treasury/${encodeURIComponent(treasuryId)}/reservations`,
      {
        method: "POST",
        body: JSON.stringify({ ...body, confirm: true }),
        mutate: true,
      },
    ),
};

export type CreateApprovalBody = {
  dry_run_id: string;
  execution_hash: string;
  policy_id: string;
  policy_version: number;
  operation_intent: string;
};

export type PrepareApplyBody = {
  approval_id: string;
  dry_run_id: string;
  execution_hash: string;
  policy_id: string;
  policy_version: number;
  operation_intent: string;
  operation_id?: string;
  target_agent?: string;
};

export type ExecuteApplyBody = {
  operation_id: string;
  approval_id: string;
  dry_run_id: string;
  execution_hash: string;
  confirm: boolean;
};

export type ApplyApproval = {
  approval_id: string;
  dry_run_id: string;
  execution_hash: string;
  policy_id: string;
  policy_version: number;
  operation_intent: string;
  status: string;
  approved_by: string;
  expires_at: string;
  request_id: string;
};

export type PrepareApplyResponse = {
  operation_id: string;
  payload_hash: string;
  execution_hash: string;
  approval_id: string;
  dry_run_id: string;
  policy_id: string;
  policy_version: number;
  operation_intent: string;
  signature: Record<string, unknown>;
  apply_enabled: boolean;
  protocol_mutated: boolean;
  note?: string;
};

export type ExecuteApplyResponse = {
  outcome_code: string;
  code: string;
  apply_enabled: boolean;
  protocol_mutated: boolean;
  protocol_unchanged: boolean;
  operation_id: string;
  phase: string;
  request_id: string;
  replay?: ReplayRecord;
  note?: string;
};

export type ReplayRecord = {
  operation_id: string;
  execution_hash: string;
  dry_run_id: string;
  status: string;
  created_at: string;
  updated_at: string;
  reserved_at: string;
  finalised_at: string | null;
  terminal_reason: string | null;
  audit_reference: string | null;
  terminal: boolean;
};

export type ApplyOperationStatus = {
  operation_id: string;
  status: string;
  signed_operation: Record<string, unknown> | null;
  replay: ReplayRecord | null;
  terminal: boolean;
  failure_reason: string | null;
  apply_enabled: boolean;
  protocol_mutated: boolean;
};

export type ReconcileListResponse = {
  scanned_at: string;
  reserved: ReplayRecord[];
  executing: ReplayRecord[];
  stuck: ReplayRecord[];
  reserved_timed_out: number;
  executing_timed_out: number;
  apply_enabled: boolean;
  protocol_mutated: boolean;
};

export type PolicyTemplate = {
  id: string;
  name: string;
  description: string;
  target_agent_id: string | null;
  policy_type: string;
  policy_data: unknown;
  status: string;
  created_by: string;
  created_by_username: string | null;
  created_at: string;
  updated_at: string;
  submitted_by: string | null;
  submitted_at: string | null;
  approved_by: string | null;
  approved_at: string | null;
  rejected_by: string | null;
  rejected_at: string | null;
  rejection_reason: string | null;
  version: number;
  hash: string;
};

export type PolicyVersion = {
  id: string;
  policy_id: string;
  version: number;
  name: string;
  status: string;
  hash: string;
  changed_by: string;
  changed_at: string;
  change_action: string;
  policy_data: unknown;
};

export type CreatePolicyBody = {
  name: string;
  description?: string;
  target_agent_id?: string;
  policy_type: string;
  policy_data: unknown;
};

export type UpdatePolicyBody = {
  name?: string;
  description?: string;
  target_agent_id?: string;
  policy_type?: string;
  policy_data?: unknown;
};

export type DryRunReport = {
  executable: boolean;
  validation_results: { name: string; passed: boolean; detail: string }[];
  predicted_protocol_operation: string;
  warnings: string[];
  blocking_errors: string[];
  execution_mode: string;
  dry_run_id: string;
  operation_id: string;
  policy_id: string;
  policy_version: number;
  execution_hash: string;
  signer_identity: string | null;
  signature_hex: string | null;
  simulation: { executable: boolean; reason?: string | null };
  note: string;
};

export type AgentSummary = {
  agent_id: string;
  status: string;
  registered_at: number;
  active_capabilities: number;
  escrow_count: number;
  settlement_count: number;
  has_reputation_metrics: boolean;
};

export type AgentDetail = {
  agent_id: string;
  status: string;
  registered_at: number;
  operational_public_key: string;
  root_version: number;
  activity_summary: {
    active_capabilities: number;
    escrows: number;
    settlements: number;
    reputation_events: number;
  };
  read_at: string;
};

export type CapabilityView = {
  capability_id: string;
  issuer: string;
  subject_agent_id: string;
  actions: string[];
  max_spend: number | null;
  revoked: boolean;
};

export type EscrowView = {
  escrow_id: string;
  status: string;
  payer: string;
  provider: string;
  principal_amount: number;
  soft_finality: boolean;
  hard_finality: boolean;
};

export type SettlementView = {
  binding_id: string;
  escrow_id: string;
  settlement_status: string;
  external_settlement_ref: string | null;
  soft_finality: boolean;
  hard_finality: boolean;
};

export type ReputationResponse = {
  metrics: {
    escrow_completed: number;
    settlement_finalized: number;
    completion_rate: number | null;
    settlement_success_rate: number | null;
  };
  events: { event_type: string; logical_time: number }[];
  evidence: { ref_type: string; commitment: string }[];
};

export type TimelineEvent = {
  source: string;
  event_type: string;
  summary: string;
  timestamp: number;
};

export type AuditEvent = {
  id: string;
  operator_id?: string | null;
  action: string;
  target: string | null;
  metadata?: unknown;
  created_at: string;
};

export type SystemHealth = {
  status: string;
  read_only_mode: boolean;
  protocol_agents_indexed: number;
  protocol_capabilities_indexed?: number;
  protocol_escrows_indexed?: number;
  protocol_settlements_indexed?: number;
  reputation_events?: number;
  loaded_at?: string;
};

export type TreasuryBalance = {
  asset_id: string;
  available_minor: number;
  reserved_minor: number;
  escrow_reserved_minor: number;
};

export type TreasuryNode = {
  treasury_id: string;
  organisation_id: string;
  parent_treasury_id: string | null;
  kind: string;
  name: string;
  status: string;
  created_at: string;
  balances: TreasuryBalance[];
};

export type TreasuryOverview = {
  organisation_id: string;
  treasuries: TreasuryNode[];
  totals: TreasuryBalance[];
  observation_only: boolean;
  note?: string;
};

export type TreasuryAllocation = {
  allocation_id: string;
  treasury_id: string;
  agent_id: string;
  asset_id: string;
  ceiling_minor: number;
  remaining_minor: number;
  status: string;
  expires_at: string | null;
  created_at: string;
};

export type TreasuryReservation = {
  reservation_id: string;
  treasury_id: string;
  allocation_id: string | null;
  asset_id: string;
  amount_minor: number;
  kind: string;
  status: string;
  escrow_id: string | null;
  created_at: string;
  updated_at: string;
  read_only: boolean;
};

export type TreasuryJournalEntry = {
  entry_id: string;
  batch_id: string;
  event_type: string;
  treasury_id: string | null;
  allocation_id: string | null;
  reservation_id: string | null;
  asset_id: string;
  account_code: string;
  debit_minor: number;
  credit_minor: number;
  request_id: string;
  created_at: string;
  immutable: boolean;
};

export type TreasurySettlementPost = {
  entry_id: string;
  batch_id: string;
  treasury_id: string | null;
  reservation_id: string | null;
  asset_id: string;
  amount_minor: number;
  created_at: string;
  request_id: string;
  accounting_truth: string;
  note: string;
};

export type TreasurySecurityView = {
  treasury_id: string;
  organisation_id: string;
  treasury_status: string;
  frozen_treasury: boolean;
  frozen_or_expired_allocations: number;
  active_reservations: number;
  consumed_reservations: number;
  released_reservations: number;
  aged_active_reservations: number;
  settlement_posts: number;
  adjustments: number;
  chargebacks: number;
  as_of: string;
  related_audit_hint?: string;
};

export type TreasuryMutation = {
  mutation_id: string;
  status: string;
  operation: string;
  payload_hash: string;
  requested_by: string;
  approved_by: string | null;
  approval_id: string | null;
  expires_at: string;
  request_id: string;
  journal_batch_id: string | null;
  outcome: string | null;
  duplicate: boolean;
  ledger_notice: string;
};

export type TreasuryExecuteResult = {
  request_id: string;
  outcome: string;
  operation: string;
  journal_batch_id: string | null;
  mutation_id: string | null;
  resource: Record<string, unknown>;
  approval_id: string | null;
  duplicate: boolean;
  ledger_notice: string;
};

export type TreasuryMutationAuditEvent = {
  id: string;
  mutation_id: string | null;
  request_id: string;
  actor: string;
  operation: string;
  target: string | null;
  approval_state: string | null;
  journal_reference: string | null;
  outcome: string;
  created_at: string;
};
