import type { AccountIndex, AccountRow } from "../api";
import { translations } from "../i18n";

interface Props {
  accounts: AccountIndex;
  t: typeof translations["zh-CN"];
  onActivate: (row: AccountRow) => void;
  onDelete: (row: AccountRow) => void;
  onAddClick: () => void;
}

export function AccountsCard({ accounts, t, onActivate, onDelete, onAddClick }: Props) {
  if (accounts.accounts.length === 0) {
    return (
      <div className="card glass-panel">
        <p className="card__title">{t.accountsTitle}</p>
        <div className="empty">
          {t.noAccounts}
          <div style={{ marginTop: 8 }}>
            <button className="btn btn--glass" onClick={onAddClick}>{t.addAccountBtn}</button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="card glass-panel">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
        <p className="card__title" style={{ margin: 0 }}>{t.accountsTitle}</p>
        <button className="btn btn--sm btn--glass" onClick={onAddClick}>+ {t.addAccountBtn}</button>
      </div>

      {accounts.accounts.map((row) => {
        const isActive = row.id === accounts.active_id;
        const subUntil = parseSubscriptionDate(row.subscription_active_until);
        const subFormatted = subUntil ? formatSubUntil(subUntil) : null;

        return (
          <div
            key={row.id}
            className={`account-card-box ${isActive ? "account-card-box--active" : ""}`}
          >
            <div className="row" style={{ padding: "4px 0" }}>
              <div className="row__main">
                <span className="tag tag--plan" style={{ textTransform: "capitalize" }}>
                  {row.plan_type || "Plus"}
                </span>
                <div style={{ minWidth: 0 }}>
                  <div className="row__name" style={{ fontSize: 13 }}>
                    {row.email || row.label}
                  </div>
                </div>
              </div>
              <div className="row__actions">
                {!isActive ? (
                  <button className="iconbtn" title={t.activateBtn} onClick={() => onActivate(row)}>↺</button>
                ) : (
                  <span className="tag tag--ok">{t.activeBadge}</span>
                )}
                <button className="iconbtn" title={t.removeBtn} onClick={() => onDelete(row)}>✕</button>
              </div>
            </div>

            {/* Subscription active until pill banner */}
            {subFormatted && (
              <div className="sub-badge-banner">
                <span className="sub-badge-icon">🔄</span>
                <span>{t.subscriptionValidUntil}</span>
                <strong style={{ color: "var(--fg)" }}>{subFormatted.dateStr}</strong>
                <span>·</span>
                <span>{subFormatted.remainingStr}</span>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

/**
 * Parses subscription active until date.
 * If the raw token contains the last billing date / period start that is already in the past
 * for an active recurring Plus/Team subscription, advances by 1-month periods until the true current expiry.
 */
function parseSubscriptionDate(val: string | null | undefined): Date | null {
  if (!val) return null;
  let d: Date | null = null;
  const num = Number(val);
  if (!isNaN(num) && num > 0) {
    d = num < 100_000_000_000 ? new Date(num * 1000) : new Date(num);
  } else {
    const parsed = new Date(val);
    if (!isNaN(parsed.getTime())) {
      d = parsed;
    }
  }
  if (!d) return null;

  const now = Date.now();
  // If the recorded active_until is in the past, roll it forward by 1 month steps to match current active subscription cycle
  while (d.getTime() < now) {
    d.setMonth(d.getMonth() + 1);
  }

  return d;
}

function formatSubUntil(date: Date): { dateStr: string; remainingStr: string } {
  const y = date.getFullYear();
  const m = date.getMonth() + 1;
  const d = date.getDate();
  const dateStr = `${y}/${m}/${d}`;

  const diff = date.getTime() - Date.now();
  if (diff <= 0) {
    return { dateStr, remainingStr: "今日到期" };
  }
  const totalMinutes = Math.floor(diff / (1000 * 60));
  const totalHours = Math.floor(totalMinutes / 60);
  const days = Math.floor(totalHours / 24);
  const hours = totalHours % 24;
  const minutes = totalMinutes % 60;

  const parts: string[] = [];
  if (days > 0) parts.push(`${days}天`);
  if (hours > 0 || days > 0) parts.push(`${hours}小时`);
  if (days === 0 && hours === 0) parts.push(`${minutes}分钟`);

  return { dateStr, remainingStr: parts.join("") };
}
