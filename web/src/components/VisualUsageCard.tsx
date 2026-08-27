import { useState, useMemo } from "react";
import type { UsageSummary, SessionItem } from "../api";
import { translations } from "../i18n";

interface Props {
  usage: UsageSummary;
  t: typeof translations["zh-CN"];
  onOpenFolder?: (path: string) => void;
  onBackup?: () => void;
  onRestore?: () => void;
}

type Tab = "heatmap" | "trend" | "bars";

export function VisualUsageCard({ usage, t, onBackup, onRestore }: Props) {
  const [tab, setTab] = useState<Tab>("heatmap");
  const [showSessions, setShowSessions] = useState(false);
  const [search, setSearch] = useState("");

  const daily = usage.daily_usage || [];
  const sessions = usage.sessions || [];

  const filteredSessions = useMemo(() => {
    if (!search.trim()) return sessions;
    const q = search.toLowerCase();
    return sessions.filter(
      (s) =>
        s.session_id.toLowerCase().includes(q) ||
        (s.cwd && s.cwd.toLowerCase().includes(q)) ||
        (s.primary_model && s.primary_model.toLowerCase().includes(q))
    );
  }, [sessions, search]);

  const cachedRatio = usage.total_input_tokens > 0
    ? Math.round((usage.total_cached_input_tokens / usage.total_input_tokens) * 100)
    : 0;

  if (usage.turns_scanned === 0) {
    return (
      <div className="card glass-panel">
        <p className="card__title">{t.usageTitle}</p>
        <div className="empty">{t.noSessionsFound}</div>
      </div>
    );
  }

  return (
    <div className="card glass-panel">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
        <p className="card__title" style={{ margin: 0 }}>
          {t.usageTitle}{" "}
          <span className="tag">
            {usage.sessions_scanned} {t.sessionsTurnsCount.replace("{turns}", String(usage.turns_scanned))}
          </span>
        </p>
        <div style={{ display: "flex", gap: 4 }}>
          {onBackup && (
            <button className="btn btn--sm btn--glass" title={t.backupBtn} onClick={onBackup}>
              {t.backupBtn}
            </button>
          )}
          {onRestore && (
            <button className="btn btn--sm btn--glass" title={t.restoreBtn} onClick={onRestore}>
              {t.restoreBtn}
            </button>
          )}
        </div>
      </div>

      {/* Overview Stats */}
      <div className="stats-grid">
        <div className="stat-box">
          <div className="stat-box__label">{t.totalTokens}</div>
          <div className="stat-box__value">{formatNumber(usage.total_tokens)}</div>
        </div>
        <div className="stat-box">
          <div className="stat-box__label">{t.inputTokensCached.replace("{rate}", String(cachedRatio))}</div>
          <div className="stat-box__value">
            {formatNumber(usage.total_input_tokens)}{" "}
            <span style={{ fontSize: 11, color: "var(--fg-muted)" }}>
              ({formatNumber(usage.total_cached_input_tokens)})
            </span>
          </div>
        </div>
        <div className="stat-box">
          <div className="stat-box__label">{t.outputTokens}</div>
          <div className="stat-box__value">{formatNumber(usage.total_output_tokens)}</div>
        </div>
      </div>

      {/* Chart Tabs */}
      <div className="tab-group" style={{ marginTop: 12 }}>
        <button
          className={`tab-btn ${tab === "heatmap" ? "tab-btn--active" : ""}`}
          onClick={() => setTab("heatmap")}
        >
          {t.chartHeatmapTab}
        </button>
        <button
          className={`tab-btn ${tab === "trend" ? "tab-btn--active" : ""}`}
          onClick={() => setTab("trend")}
        >
          {t.chartTrendTab}
        </button>
        <button
          className={`tab-btn ${tab === "bars" ? "tab-btn--active" : ""}`}
          onClick={() => setTab("bars")}
        >
          {t.chartBarTab}
        </button>
      </div>

      {/* Visual Chart Content */}
      <div style={{ marginTop: 10 }}>
        {tab === "heatmap" && <HeatmapView daily={daily} t={t} />}
        {tab === "trend" && <TrendLineView daily={daily} />}
        {tab === "bars" && <ModelDistributionView models={usage.per_model} totalTokens={usage.total_tokens} />}
      </div>

      {/* Sessions Toggle Section */}
      <div style={{ marginTop: 14, paddingTop: 10, borderTop: "1px solid var(--border)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontSize: 12, fontWeight: 500, color: "var(--fg-muted)" }}>
            {t.sessionManagerTitle}
          </span>
          <button
            className="btn btn--sm btn--ghost"
            onClick={() => setShowSessions((v) => !v)}
          >
            {showSessions ? t.hideSessionsList : t.showSessionsList.replace("{count}", String(sessions.length))}
          </button>
        </div>

        {showSessions && (
          <div style={{ marginTop: 10 }}>
            <input
              type="text"
              className="search-input"
              placeholder={t.searchSessionsPlaceholder}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
            <div className="sessions-list">
              {filteredSessions.slice(0, 15).map((s) => (
                <SessionRow key={s.session_id} session={s} />
              ))}
              {filteredSessions.length === 0 && (
                <div className="empty" style={{ padding: 12 }}>
                  无匹配会话
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function HeatmapView({ daily, t }: { daily: Array<{ date: string; total_tokens: number }>; t: any }) {
  // Generate last 28 days
  const days: Array<{ date: string; tokens: number }> = [];
  const map = new Map<string, number>();
  for (const d of daily) {
    map.set(d.date, d.total_tokens);
  }

  const now = new Date();
  for (let i = 27; i >= 0; i--) {
    const dt = new Date(now.getTime() - i * 86400000);
    const dateStr = dt.toISOString().split("T")[0];
    days.push({
      date: dateStr,
      tokens: map.get(dateStr) || 0,
    });
  }

  const maxTokens = Math.max(1, ...days.map((d) => d.tokens));

  return (
    <div>
      <div style={{ fontSize: 11, color: "var(--fg-muted)", marginBottom: 6 }}>
        {t.last30Days}
      </div>
      <div className="heatmap-grid">
        {days.map((d) => {
          const intensity = d.tokens === 0 ? 0 : Math.min(4, Math.ceil((d.tokens / maxTokens) * 4));
          return (
            <div
              key={d.date}
              className={`heatmap-cell heatmap-cell--l${intensity}`}
              title={`${d.date}: ${formatNumber(d.tokens)} Tokens`}
            />
          );
        })}
      </div>
    </div>
  );
}

function TrendLineView({ daily }: { daily: Array<{ date: string; total_tokens: number; output_tokens: number }> }) {
  if (daily.length === 0) {
    return <div className="empty" style={{ padding: 12 }}>暂无近期趋势数据</div>;
  }
  const lastDays = daily.slice(-14);
  const maxVal = Math.max(1, ...lastDays.map((d) => d.total_tokens));
  const height = 90;
  const width = 320;
  const padding = 10;

  const points = lastDays.map((d, i) => {
    const x = padding + (i / Math.max(1, lastDays.length - 1)) * (width - 2 * padding);
    const y = height - padding - (d.total_tokens / maxVal) * (height - 2 * padding);
    return `${x},${y}`;
  }).join(" ");

  return (
    <div style={{ position: "relative", width: "100%", overflowX: "auto" }}>
      <svg width="100%" height={height} viewBox={`0 0 ${width} ${height}`} className="trend-svg">
        <defs>
          <linearGradient id="trendGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.35" />
            <stop offset="100%" stopColor="var(--accent)" stopOpacity="0.0" />
          </linearGradient>
        </defs>
        {lastDays.length > 1 && (
          <polygon
            points={`${padding},${height - padding} ${points} ${width - padding},${height - padding}`}
            fill="url(#trendGrad)"
          />
        )}
        <polyline
          fill="none"
          stroke="var(--accent)"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          points={points}
        />
        {lastDays.map((d, i) => {
          const x = padding + (i / Math.max(1, lastDays.length - 1)) * (width - 2 * padding);
          const y = height - padding - (d.total_tokens / maxVal) * (height - 2 * padding);
          return (
            <circle
              key={i}
              cx={x}
              cy={y}
              r="3"
              fill="var(--accent)"
              className="trend-point"
            >
              <title>{`${d.date}: ${formatNumber(d.total_tokens)} Tokens`}</title>
            </circle>
          );
        })}
      </svg>
    </div>
  );
}

function ModelDistributionView({ models, totalTokens }: { models: Array<{ model: string; total_tokens: number }>; totalTokens: number }) {
  if (models.length === 0) {
    return <div className="empty" style={{ padding: 12 }}>暂无模型分布数据</div>;
  }
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {models.slice(0, 5).map((m) => {
        const pct = totalTokens > 0 ? Math.round((m.total_tokens / totalTokens) * 100) : 0;
        return (
          <div key={m.model}>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, marginBottom: 2 }}>
              <span style={{ fontWeight: 500 }}>{m.model}</span>
              <span style={{ color: "var(--fg-muted)" }}>
                {formatNumber(m.total_tokens)} ({pct}%)
              </span>
            </div>
            <div className="gauge__bar" style={{ height: 6 }}>
              <div className="gauge__fill" style={{ width: `${pct}%`, background: "var(--accent)" }} />
            </div>
          </div>
        );
      })}
    </div>
  );
}

function SessionRow({ session }: { session: SessionItem }) {
  const shortId = session.session_id.slice(0, 8);
  const timeStr = new Date(session.last_updated_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const dateStr = new Date(session.last_updated_at).toLocaleDateString([], { month: "numeric", day: "numeric" });

  return (
    <div className="session-item">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontWeight: 600, fontSize: 12 }}>
          {session.cwd ? session.cwd.split(/[\\/]/).pop() || session.cwd : `Session ${shortId}`}
        </span>
        <span style={{ fontSize: 11, color: "var(--fg-muted)" }}>
          {dateStr} {timeStr}
        </span>
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--fg-muted)", marginTop: 2 }}>
        <span>{session.primary_model || "unknown"} · {session.turns_count} turns</span>
        <span style={{ fontWeight: 500, color: "var(--fg)" }}>{formatNumber(session.total_tokens)} Tok</span>
      </div>
    </div>
  );
}

function formatNumber(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`;
  return `${(n / 1_000_000).toFixed(2)}M`;
}
