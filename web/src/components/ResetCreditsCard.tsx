import type { ResetCreditsSnapshot } from "../api";

interface Props {
  reset: ResetCreditsSnapshot;
}

export function ResetCreditsCard({ reset }: Props) {
  if (reset.credits.length === 0) {
    return null;
  }
  return (
    <div className="card">
      <p className="card__title">
        Reset credits <span className="tag">{reset.available_count} available</span>
      </p>
      {reset.credits.slice(0, 4).map((c, i) => (
        <div className="kv" key={i}>
          <span className="kv__k">{c.id ?? c.status}</span>
          <span className="kv__v">
            <span className={`tag ${c.status === "available" ? "tag--ok" : "tag--warn"}`}>
              {c.status}
            </span>
            {c.remaining_seconds > 0 ? ` · ${formatRemaining(c.remaining_seconds)}` : ""}
          </span>
        </div>
      ))}
    </div>
  );
}

function formatRemaining(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${hours}h`;
  return `${Math.round(hours / 24)}d`;
}
