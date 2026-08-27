import { useState } from "react";
import type { ApiCostSummary } from "../api";

interface Props {
  cost: ApiCostSummary;
}

export function CostCard({ cost }: Props) {
  const [expanded, setExpanded] = useState(false);
  if (cost.turns_priced + cost.turns_unknown === 0) {
    return null;
  }
  const usd = formatUsd(cost.estimated_usd);
  return (
    <div className="card">
      <p className="card__title">
        API-equivalent cost{" "}
        <span className="tag">
          {cost.turns_priced} priced · {cost.turns_unknown} unknown
        </span>
      </p>
      <div className="kv">
        <span className="kv__k">Estimated total</span>
        <span className="kv__v">{usd}</span>
      </div>
      <div className="kv">
        <span className="kv__k">Tokens (input cached)</span>
        <span className="kv__v">
          {formatNumber(cost.total_uncached_input_tokens)} (
          {formatNumber(cost.total_cached_input_tokens)})
        </span>
      </div>
      <div className="kv">
        <span className="kv__k">Output</span>
        <span className="kv__v">{formatNumber(cost.total_output_tokens)}</span>
      </div>
      {cost.per_model.slice(0, expanded ? 20 : 4).map((m) => (
        <div className="kv" key={m.raw_model}>
          <span className="kv__k">
            {m.raw_model}
            {!m.priced ? <span className="tag tag--warn" style={{ marginLeft: 6 }}>unknown model</span> : null}
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
          {expanded ? "Show fewer" : `Show all ${cost.per_model.length}`}
        </button>
      ) : null}
      <div className="kv" style={{ marginTop: 6, color: "var(--fg-subtle)" }}>
        <span className="kv__k">Pricing version</span>
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
