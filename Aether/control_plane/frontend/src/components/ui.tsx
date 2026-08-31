export function StatusBadge({ status }: { status: string }) {
  const s = status.toLowerCase();
  let tone = "neutral";
  if (["active", "ok", "approved", "executed", "valid", "executable"].some((x) => s.includes(x))) {
    tone = "ok";
  } else if (["pending", "submitted", "reserved", "executing", "prepared"].some((x) => s.includes(x))) {
    tone = "warn";
  } else if (
    ["frozen", "rejected", "failed", "expired", "stuck", "cancelled", "revoked", "error"].some((x) =>
      s.includes(x),
    )
  ) {
    tone = "bad";
  }
  return <span className={`badge tone-${tone}`}>{status}</span>;
}

export function ApplyDisabledBanner() {
  return (
    <div className="banner warn" role="status">
      <strong>Execution disabled.</strong> Apply live mutation is off (`apply_enabled=false`).
      Dry-run, approval, prepare, status, and reconcile remain available for governance workflows.
    </div>
  );
}

export function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
}) {
  return (
    <div className="page-header">
      <div>
        <h2>{title}</h2>
        {subtitle && <p className="muted">{subtitle}</p>}
      </div>
      {actions}
    </div>
  );
}

export function Metric({ label, value, hint }: { label: string; value: string | number; hint?: string }) {
  return (
    <div className="metric">
      <div className="metric-label">{label}</div>
      <div className="metric-value">{value}</div>
      {hint && <div className="muted">{hint}</div>}
    </div>
  );
}

export function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="section">
      <h3 className="section-title">{title}</h3>
      {children}
    </section>
  );
}
