import type { AccountIndex, AccountRow } from "../api";

interface Props {
  accounts: AccountIndex;
  onActivate: (row: AccountRow) => void;
  onDelete: (row: AccountRow) => void;
  onAddClick: () => void;
}

export function AccountsCard({ accounts, onActivate, onDelete, onAddClick }: Props) {
  if (accounts.accounts.length === 0) {
    return (
      <div className="card">
        <p className="card__title">Accounts</p>
        <div className="empty">
          No Codex accounts in the library.
          <div style={{ marginTop: 8 }}>
            <button className="btn" onClick={onAddClick}>Add account</button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <p className="card__title">Accounts</p>
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
                  <span className="tag tag--danger" style={{ marginLeft: 6 }}>re-auth required</span>
                ) : null}
              </div>
            </div>
          </div>
          <div className="row__actions">
            {row.id !== accounts.active_id ? (
              <button className="iconbtn" title="Activate" onClick={() => onActivate(row)}>↺</button>
            ) : (
              <span className="tag tag--ok">active</span>
            )}
            <button className="iconbtn" title="Remove from library" onClick={() => onDelete(row)}>✕</button>
          </div>
        </div>
      ))}
      <div style={{ marginTop: 8 }}>
        <button className="btn btn--ghost" onClick={onAddClick}>+ Add account</button>
      </div>
    </div>
  );
}

function initials(label: string): string {
  const parts = label.split(/[@\s._-]+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[1][0]).toUpperCase();
}
