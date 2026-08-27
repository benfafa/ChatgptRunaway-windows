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
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <p className="card__title" style={{ margin: 0 }}>{t.accountsTitle}</p>
        <button className="btn btn--sm btn--glass" onClick={onAddClick}>+ {t.addAccountBtn}</button>
      </div>

      {accounts.accounts.map((row) => (
        <div
          key={row.id}
          className={`row ${row.id === accounts.active_id ? "row--active" : ""}`}
        >
          <div className="row__main">
            <div className="row__avatar">{initials(row.label)}</div>
            <div style={{ minWidth: 0 }}>
              <div className="row__name">{row.label}</div>
              <div className="row__sub">
                {row.email ?? row.account_id ?? row.id}
                {row.requires_reauth ? (
                  <span className="tag tag--danger" style={{ marginLeft: 6 }}>{t.reauthRequired}</span>
                ) : null}
              </div>
            </div>
          </div>
          <div className="row__actions">
            {row.id !== accounts.active_id ? (
              <button className="iconbtn" title={t.activateBtn} onClick={() => onActivate(row)}>↺</button>
            ) : (
              <span className="tag tag--ok">{t.activeBadge}</span>
            )}
            <button className="iconbtn" title={t.removeBtn} onClick={() => onDelete(row)}>✕</button>
          </div>
        </div>
      ))}
    </div>
  );
}

function initials(label: string): string {
  const parts = label.split(/[@\s._-]+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[1][0]).toUpperCase();
}
