import type { UsageSummary } from "../api";

interface Props {
  usage: UsageSummary;
}

export function UsageCard({ usage }: Props) {
  if (usage.turns_scanned === 0) {
    return (
      <div className="card">
        <p className="card__title">Local usage</p>
        <div className="empty">No Codex session logs found yet.</div>
      </div>
    );
  }
  return (
    <div className="card">
      <p className="card__title">
        Local usage <span className="tag">{usage.sessions_scanned} sessions · {usage.turns_scanned} turns</span>
      </p>
      <div className="kv">
        <span className="kv__k">Total tokens</span>
        <span className="kv__v">{formatNumber(usage.total_tokens)}</span>
      </div>
      <div className="kv">
        <span className="kv__k">Input (cached)</span>
        <span className="kv__v">
          {formatNumber(usage.total_input_tokens)} ({formatNumber(usage.total_cached_input_tokens)})
        </span>
      </div>
      <div className="kv">
        <span className="kv__k">Output</span>
        <span className="kv__v">{formatNumber(usage.total_output_tokens)}</span>
      </div>
      {usage.per_model.slice(0, 4).map((m) => (
        <div className="kv" key={m.model}>
          <span className="kv__k">{m.model}</span>
          <span className="kv__v">{formatNumber(m.total_tokens)}</span>
        </div>
      ))}
    </div>
  );
}

function formatNumber(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`;
  return `${(n / 1_000_000).toFixed(2)}M`;
}
