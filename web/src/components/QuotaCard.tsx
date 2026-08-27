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
  
  // Highlighting Remaining (Hero level)
  const remainingCls =
    remainingPct <= 10 ? "hero-gauge__fill--danger" : remainingPct <= 30 ? "hero-gauge__fill--warn" : "";
  const remaining = primary.resets_at
    ? new Date(primary.resets_at)
    : null;

  // Quota Runway Estimation
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
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <p className="card__title" style={{ margin: 0 }}>
          {t.quotaTitle} {quota.plan ? `· ${quota.plan}` : ""}
        </p>
        <span className={`tag ${remainingPct <= 10 ? "tag--danger" : remainingPct <= 30 ? "tag--warn" : "tag--ok"}`}>
          {remainingPct <= 10 ? "⚠️ 配额偏紧" : remainingPct <= 30 ? "⚡ 剩余适中" : "✨ 额度充沛"}
        </span>
      </div>

      {/* HERO SECTION: Highlighting Remaining Quota */}
      <div className="hero-quota">
        <div className="hero-quota__left">
          <div className="hero-quota__number">{remainingPct}%</div>
          <div className="hero-quota__label">{t.quotaRemaining}可用额度</div>
        </div>
        <div style={{ textAlign: "right" }}>
          <div style={{ fontSize: 13, fontWeight: 700, color: "var(--fg-muted)" }}>
            已用 {usedPct}%
          </div>
          <div className="hero-quota__sub">
            {t.resetsIn} {remaining ? formatRelative(remaining) : "—"}
          </div>
        </div>
      </div>

      {/* Hero Remaining Progress Gauge (Green = Remaining) */}
      <div className="hero-gauge" style={{ marginBottom: 12 }}>
        <div className={`hero-gauge__fill ${remainingCls}`} style={{ width: `${Math.min(100, remainingPct)}%` }} />
      </div>

      {/* Weekly Quota (Remaining Highlighted) */}
      {quota.secondary ? (
        <div className="kv" style={{ padding: "6px 0", borderTop: "1px solid var(--border)" }}>
          <span className="kv__k">{t.weeklyQuota}</span>
          <span className="kv__v" style={{ color: "var(--fg)" }}>
            <strong style={{ color: "var(--ok)", fontSize: 13 }}>
              {Math.max(0, 100 - Math.round(quota.secondary.used_percent))}% {t.quotaRemaining}
            </strong>{" "}
            <span style={{ fontSize: 11, color: "var(--fg-muted)", fontWeight: "normal" }}>
              (已用 {Math.round(quota.secondary.used_percent)}%)
            </span>
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
            <strong style={{ color: "var(--ok)" }}>
              {Math.max(0, 100 - Math.round(w.window.used_percent))}% {t.quotaRemaining}
            </strong>{" "}
            <span style={{ fontSize: 11, color: "var(--fg-muted)", fontWeight: "normal" }}>
              (已用 {Math.round(w.window.used_percent)}%)
            </span>
          </span>
        </div>
      ))}

      {/* Quota Runway Estimation Section */}
      <div className="runway-box" style={{ marginTop: 10 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 3 }}>
          <span style={{ fontWeight: 700, fontSize: 11.5, color: "var(--fg)" }}>💡 {t.runwayEstimateTitle}</span>
          <span style={{ fontSize: 11, fontWeight: 600, color: remainingPct <= 20 ? "var(--warn)" : "var(--ok)" }}>
            {runwayText}
          </span>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--fg-muted)" }}>
          <span>{t.runwayBurnRate}: ~{burnRatePerHour.toFixed(1)}%/h</span>
          {runoutEstimate && <span>{t.runwayRunout}: <strong style={{ color: "var(--danger)" }}>{runoutEstimate}</strong></span>}
        </div>
      </div>

      {quota.credits_balance != null ? (
        <div className="kv" style={{ marginTop: 8, paddingTop: 4 }}>
          <span className="kv__k">{t.creditsBalance}</span>
          <span className="kv__v" style={{ fontWeight: 700, color: "var(--accent)" }}>
            ${quota.credits_balance.toFixed(2)}
          </span>
        </div>
      ) : null}
    </div>
  );
}

function formatRelative(date: Date): string {
  const diff = date.getTime() - Date.now();
  if (diff <= 0) return "即将重置 (now)";
  const totalSeconds = Math.floor(diff / 1000);
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);

  const parts: string[] = [];
  if (days > 0) parts.push(`${days}天`);
  if (hours > 0 || days > 0) parts.push(`${hours}小时`);
  parts.push(`${minutes}分钟`);

  return parts.join(" ");
}
