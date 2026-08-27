import type { QuotaSnapshot } from "../api";
import { translations } from "../i18n";

interface Props {
  quota: QuotaSnapshot;
  t: typeof translations["zh-CN"];
}

export function QuotaCard({ quota, t }: Props) {
  const primary = quota.primary;
  const usedPct = Math.round(primary.used_percent);
  const remainingPct = Math.max(0, 100 - usedPct);
  const cls =
    usedPct >= 90 ? "gauge__fill--danger" : usedPct >= 70 ? "gauge__fill--warn" : "";
  const remaining = primary.resets_at
    ? new Date(primary.resets_at)
    : null;

  // Quota Runway Estimation
  // Calculate burn rate based on used_percent vs time elapsed in the 5-hour (300m) primary window
  const windowMinutes = primary.window_minutes || 300;
  let runwayText = t.runwaySafe;
  let runoutEstimate: string | null = null;
  let burnRatePerHour = 0;

  if (remaining) {
    const diffMs = remaining.getTime() - Date.now();
    const remainingMinutes = Math.max(1, Math.round(diffMs / 60000));
    const elapsedMinutes = Math.max(1, windowMinutes - remainingMinutes);
    burnRatePerHour = (usedPct / elapsedMinutes) * 60;

    if (burnRatePerHour > 0 && remainingPct > 0) {
      const minutesToEmpty = (remainingPct / burnRatePerHour) * 60;
      if (minutesToEmpty < remainingMinutes) {
        runwayText = t.runwayCritical;
        const runoutDate = new Date(Date.now() + minutesToEmpty * 60000);
        runoutEstimate = runoutDate.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      }
    }
  }

  return (
    <div className="card glass-panel">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <p className="card__title" style={{ margin: 0 }}>
          {t.quotaTitle} {quota.plan ? `· ${quota.plan}` : ""}
        </p>
        <span className={`tag ${usedPct >= 90 ? "tag--danger" : usedPct >= 70 ? "tag--warn" : "tag--ok"}`}>
          {remainingPct}% {t.quotaRemaining}
        </span>
      </div>

      <div className="gauge" style={{ margin: "10px 0 12px" }}>
        <div className="gauge__bar">
          <div className={`gauge__fill ${cls}`} style={{ width: `${Math.min(100, usedPct)}%` }} />
        </div>
        <div className="gauge__value" style={{ fontWeight: 600 }}>
          {remainingPct}% <span style={{ fontSize: 11, fontWeight: "normal", color: "var(--fg-muted)" }}>({usedPct}% {t.quotaUsed})</span>
        </div>
      </div>

      <div className="kv">
        <span className="kv__k">{t.resetsIn}</span>
        <span className="kv__v">{remaining ? formatRelative(remaining) : "—"}</span>
      </div>

      {quota.secondary ? (
        <div className="kv">
          <span className="kv__k">{t.weeklyQuota}</span>
          <span className="kv__v">
            {Math.max(0, 100 - Math.round(quota.secondary.used_percent))}% {t.quotaRemaining} ({Math.round(quota.secondary.used_percent)}% {t.quotaUsed})
            {quota.secondary.resets_at
              ? ` · ${t.resetsIn} ${formatRelative(new Date(quota.secondary.resets_at))}`
              : ""}
          </span>
        </div>
      ) : null}

      {quota.additional_windows && quota.additional_windows.map((w, i) => (
        <div className="kv" key={i}>
          <span className="kv__k">{w.name}</span>
          <span className="kv__v">
            {Math.max(0, 100 - Math.round(w.window.used_percent))}% {t.quotaRemaining} ({Math.round(w.window.used_percent)}% {t.quotaUsed})
            {w.window.resets_at
              ? ` · ${t.resetsIn} ${formatRelative(new Date(w.window.resets_at))}`
              : ""}
          </span>
        </div>
      ))}

      {/* Quota Runway Estimation Section */}
      <div className="runway-box" style={{ marginTop: 10 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 2 }}>
          <span style={{ fontWeight: 600, fontSize: 11 }}>{t.runwayEstimateTitle}</span>
          <span style={{ fontSize: 11, color: usedPct >= 70 ? "var(--warn)" : "var(--ok)" }}>
            {runwayText}
          </span>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--fg-muted)" }}>
          <span>{t.runwayBurnRate}: ~{burnRatePerHour.toFixed(1)}%/h</span>
          {runoutEstimate && <span>{t.runwayRunout}: {runoutEstimate}</span>}
        </div>
      </div>

      {quota.credits_balance != null ? (
        <div className="kv" style={{ marginTop: 8 }}>
          <span className="kv__k">{t.creditsBalance}</span>
          <span className="kv__v" style={{ fontWeight: 600, color: "var(--accent)" }}>
            ${quota.credits_balance.toFixed(2)}
          </span>
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
