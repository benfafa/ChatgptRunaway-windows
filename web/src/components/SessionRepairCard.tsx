import { useState, useEffect } from "react";
import type { SessionIndexHealth } from "../api";
import { translations } from "../i18n";

interface Props {
  health: SessionIndexHealth | null;
  t: typeof translations["zh-CN"];
  onRepair: () => Promise<void>;
  onRefresh: () => Promise<void>;
}

export function SessionRepairCard({ health, t, onRepair, onRefresh }: Props) {
  const [repairing, setRepairing] = useState(false);
  const [justRepaired, setJustRepaired] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    if (justRepaired) {
      const timer = setTimeout(() => setJustRepaired(false), 5000);
      return () => clearTimeout(timer);
    }
  }, [justRepaired]);

  if (!health) return null;

  const hasIssues = health.missing_count > 0 || health.orphan_count > 0 || health.duplicate_count > 0;

  async function handleRepair() {
    setRepairing(true);
    try {
      await onRepair();
      setJustRepaired(true);
    } finally {
      setRepairing(false);
    }
  }

  async function handleRefresh() {
    setRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setRefreshing(false);
    }
  }

  return (
    <div className="card glass-panel">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <p className="card__title" style={{ margin: 0 }}>
          {t.sessionRepairTitle}
        </p>
        <button
          className="app__refresh"
          style={{ width: 22, height: 22 }}
          title={t.refreshTitle}
          onClick={handleRefresh}
          disabled={refreshing || repairing}
        >
          {refreshing ? "…" : "↻"}
        </button>
      </div>

      <div style={{ fontSize: 12, color: "var(--fg-muted)", marginBottom: 10 }}>
        {justRepaired || !hasIssues ? (
          <span style={{ color: "var(--ok)", fontWeight: 600 }}>
            ✓ 索引状态正常（已同步 {health.total_files} 个会话）
          </span>
        ) : (
          <span>
            {t.sessionRepairStatus
              .replace("{missing}", String(health.missing_count))
              .replace("{orphan}", String(health.orphan_count))
              .replace("{duplicate}", String(health.duplicate_count))}
          </span>
        )}
      </div>

      <button
        className="btn btn--glass"
        style={{
          width: "100%",
          padding: "8px 12px",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          gap: 6,
          fontWeight: 600,
          background: justRepaired || !hasIssues ? "rgba(16, 185, 129, 0.08)" : "rgba(255, 90, 54, 0.08)",
          borderColor: justRepaired || !hasIssues ? "var(--ok)" : "var(--accent)",
          color: justRepaired || !hasIssues ? "var(--ok)" : "var(--fg)",
        }}
        onClick={handleRepair}
        disabled={repairing || justRepaired || !hasIssues}
      >
        <span>{justRepaired || !hasIssues ? "✓" : "🏥"}</span>
        <span>
          {repairing
            ? t.sessionRepairing
            : justRepaired || !hasIssues
            ? "索引已是最新状态"
            : t.sessionRepairAction}
        </span>
      </button>
    </div>
  );
}
