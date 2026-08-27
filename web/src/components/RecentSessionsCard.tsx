import { useState } from "react";
import type { UsageSummary } from "../api";
import { translations } from "../i18n";

interface Props {
  usage: UsageSummary;
  t: typeof translations["zh-CN"];
  onRefresh?: () => void;
}

export function RecentSessionsCard({ usage, t, onRefresh }: Props) {
  const [refreshing, setRefreshing] = useState(false);
  const sessions = usage.sessions || [];

  async function handleRefresh() {
    if (!onRefresh) return;
    setRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setRefreshing(false);
    }
  }

  if (sessions.length === 0) {
    return null;
  }

  return (
    <div className="card glass-panel">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
        <p className="card__title" style={{ margin: 0 }}>
          {t.recentSessionsTitle}
        </p>
        {onRefresh && (
          <button
            className="app__refresh"
            style={{ width: 22, height: 22 }}
            title={t.refreshTitle}
            onClick={handleRefresh}
            disabled={refreshing}
          >
            {refreshing ? "…" : "↻"}
          </button>
        )}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {sessions.slice(0, 5).map((s) => {
          const folderName = s.cwd ? s.cwd.split(/[\\/]/).pop() || s.cwd : "workspace";
          const cost = s.estimated_cost_usd ? `$${Number(s.estimated_cost_usd).toFixed(4)}` : "$0.0000";

          return (
            <div key={s.session_id} className="recent-session-item">
              <div style={{ display: "flex", alignItems: "flex-start", gap: 8, minWidth: 0, flex: 1 }}>
                <span className="recent-session-dot" />
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div className="recent-session-title" title={s.title}>
                    {s.title}
                  </div>
                  <div className="recent-session-sub">
                    <span>{folderName}</span>
                    <span>·</span>
                    <span>{t.recentlyActive}</span>
                    <span>·</span>
                    <span>{formatNumber(s.total_tokens)} Tokens</span>
                  </div>
                </div>
              </div>
              <div className="recent-session-cost">
                {cost}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function formatNumber(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(2)}K`;
  return `${(n / 1_000_000).toFixed(2)}M`;
}
