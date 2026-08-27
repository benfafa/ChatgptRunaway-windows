import { useState } from "react";
import type { ApiCostSummary } from "../api";
import { translations } from "../i18n";

interface Props {
  cost: ApiCostSummary;
  t: typeof translations["zh-CN"];
}

export function CostCard({ cost, t }: Props) {
  const [expanded, setExpanded] = useState(false);
  if (cost.turns_priced + cost.turns_unknown === 0) {
    return null;
  }
  const usd = formatUsd(cost.estimated_usd);
  return (
    <div className="card glass-panel">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <p className="card__title" style={{ margin: 0 }}>
          {t.apiCostTitle}
        </p>
        <span className="tag">
          {t.pricedTurns.replace("{priced}", String(cost.turns_priced)).replace("{unknown}", String(cost.turns_unknown))}
        </span>
      </div>

      <div className="kv">
        <span className="kv__k">{t.estimatedTotal}</span>
        <span className="kv__v" style={{ fontWeight: 700, fontSize: 14, color: "var(--accent)" }}>{usd}</span>
      </div>
      <div className="kv">
        <span className="kv__k">输入 (含缓存)</span>
        <span className="kv__v">
          {formatNumber(cost.total_uncached_input_tokens)} (
          {formatNumber(cost.total_cached_input_tokens)})
        </span>
      </div>
      <div className="kv">
        <span className="kv__k">输出</span>
        <span className="kv__v">{formatNumber(cost.total_output_tokens)}</span>
      </div>
      {cost.per_model.slice(0, expanded ? 20 : 4).map((m) => (
        <div className="kv" key={m.raw_model}>
          <span className="kv__k">
            {m.raw_model}
            {!m.priced ? <span className="tag tag--warn" style={{ marginLeft: 6 }}>{t.unknownModel}</span> : null}
          </span>
          <span className="kv__v">
            {m.priced ? formatUsd(m.estimated_usd) : "—"}
          </span>
        </div>
      ))}
      {cost.per_model.length > 4 ? (
        <button
          className="btn btn--ghost"
          onClick={() => setExpanded((v) => !v)}
          style={{ marginTop: 6 }}
        >
          {expanded ? t.showFewer : t.showMore.replace("{count}", String(cost.per_model.length))}
        </button>
      ) : null}
      <div className="kv" style={{ marginTop: 6, color: "var(--fg-subtle)", fontSize: 11 }}>
        <span className="kv__k">{t.pricingVersion}</span>
        <span className="kv__v">{cost.pricing_version}</span>
      </div>
    </div>
  );
}

function formatUsd(s: string): string {
  const n = Number(s);
  if (!Number.isFinite(n)) return s;
  if (n < 0.01) return `$${n.toFixed(4)}`;
  return `$${n.toFixed(2)}`;
}

function formatNumber(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`;
  return `${(n / 1_000_000).toFixed(2)}M`;
}
