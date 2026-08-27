import type { ResetCreditsSnapshot } from "../api";
import { translations } from "../i18n";

interface Props {
  reset: ResetCreditsSnapshot;
  t: typeof translations["zh-CN"];
}

export function ResetCreditsCard({ reset, t }: Props) {
  if (reset.credits.length === 0 && reset.available_count === 0) {
    return null;
  }
  return (
    <div className="card glass-panel">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <p className="card__title" style={{ margin: 0 }}>
          {t.resetCreditsTitle}
        </p>
        <span className={`tag ${reset.available_count > 0 ? "tag--ok" : "tag--warn"}`}>
          {reset.available_count} {t.availableCreditsTag}
        </span>
      </div>

      <div className="kv" style={{ marginBottom: 6 }}>
        <span className="kv__k">{t.todayResetStatus}</span>
        <span className="kv__v" style={{ fontWeight: 500, color: reset.available_count > 0 ? "var(--ok)" : "var(--fg-muted)" }}>
          {reset.available_count > 0 ? `✓ ${t.todayResetReady}` : "—"}
        </span>
      </div>

      {reset.credits.slice(0, 4).map((c, i) => (
        <div className="kv" key={i}>
          <span className="kv__k">{c.id ? c.id.slice(0, 12) + "…" : (c.status === "available" ? t.creditStatusAvailable : t.creditStatusUsed)}</span>
          <span className="kv__v">
            <span className={`tag ${c.status === "available" ? "tag--ok" : "tag--warn"}`}>
              {c.status === "available" ? t.creditStatusAvailable : c.status}
            </span>
            {c.remaining_seconds > 0 ? ` · ${t.creditExpiresIn} ${formatRemaining(c.remaining_seconds)}` : ""}
          </span>
        </div>
      ))}
    </div>
  );
}

function formatRemaining(seconds: number): string {
  if (seconds <= 0) return "已到期";
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  const parts: string[] = [];
  if (days > 0) parts.push(`${days}天`);
  if (hours > 0 || days > 0) parts.push(`${hours}小时`);
  parts.push(`${minutes}分钟`);

  return parts.join(" ");
}
