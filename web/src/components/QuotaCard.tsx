import type { QuotaSnapshot } from "../api";

interface Props {
  quota: QuotaSnapshot;
}

export function QuotaCard({ quota }: Props) {
  const primary = quota.primary;
  const pct = Math.round(primary.used_percent);
  const cls =
    pct >= 90 ? "gauge__fill--danger" : pct >= 70 ? "gauge__fill--warn" : "";
  const remaining = primary.resets_at
    ? new Date(primary.resets_at)
    : null;

  return (
    <div className="card">
      <p className="card__title">Quota {quota.plan ? `· ${quota.plan}` : ""}</p>
      <div className="gauge">
        <div className="gauge__bar">
          <div className={`gauge__fill ${cls}`} style={{ width: `${Math.min(100, pct)}%` }} />
        </div>
        <div className="gauge__value">{pct}%</div>
      </div>
      <div className="kv">
        <span className="kv__k">Resets in</span>
        <span className="kv__v">{remaining ? formatRelative(remaining) : "—"}</span>
      </div>
      {quota.secondary ? (
        <div className="kv">
          <span className="kv__k">Weekly</span>
          <span className="kv__v">
            {Math.round(quota.secondary.used_percent)}%
            {quota.secondary.resets_at
              ? ` · resets ${formatRelative(new Date(quota.secondary.resets_at))}`
              : ""}
          </span>
        </div>
      ) : null}
      {quota.credits_balance != null ? (
        <div className="kv">
          <span className="kv__k">Credits balance</span>
          <span className="kv__v">${quota.credits_balance.toFixed(2)}</span>
        </div>
      ) : null}
    </div>
  );
}

function formatRelative(date: Date): string {
  const diff = date.getTime() - Date.now();
  if (diff <= 0) return "now";
  const minutes = Math.round(diff / 60_000);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${hours}h`;
  const days = Math.round(hours / 24);
  return `${days}d`;
}
