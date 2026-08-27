import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type AccountIndex, type QuotaSnapshot, type ResetCreditsSnapshot, type UsageSummary, type AccountRow, type ApiCostSummary } from "./api";
import { AccountsCard } from "./components/AccountsCard";
import { QuotaCard } from "./components/QuotaCard";
import { ResetCreditsCard } from "./components/ResetCreditsCard";
import { UsageCard } from "./components/UsageCard";
import { CostCard } from "./components/CostCard";
import { AddAccountDialog } from "./components/AddAccountDialog";

export default function App() {
  const [accounts, setAccounts] = useState<AccountIndex | null>(null);
  const [quota, setQuota] = useState<QuotaSnapshot | null>(null);
  const [reset, setReset] = useState<ResetCreditsSnapshot | null>(null);
  const [usage, setUsage] = useState<UsageSummary | null>(null);
  const [cost, setCost] = useState<ApiCostSummary | null>(null);
  const [showAdd, setShowAdd] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    refresh().catch((e) => setError(String(e)));

    const unlistenPromise = listen("refresh-requested", () => {
      refresh().catch((e) => setError(String(e)));
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  async function refresh() {
    setRefreshing(true);
    setError(null);
    try {
      const [idx, usage, cost] = await Promise.all([
        api.listAccounts(),
        api.scanLocalSessions(),
        api.computeApiCost().catch((e) => {
          console.warn("cost calc failed", e);
          return null;
        }),
      ]);
      setAccounts(idx);
      setUsage(usage);
      setCost(cost);
      if (idx.active_id) {
        await fetchFor(idx.active_id);
      } else {
        setQuota(null);
        setReset(null);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(false);
    }
  }

  async function fetchFor(id: string) {
    try {
      const [q, r] = await Promise.all([
        api.fetchQuota(id),
        api.fetchResetCredits(id).catch(() => null),
      ]);
      setQuota(q);
      setReset(r);
      // fetch_quota already pushed the icon, but call again to be explicit
      // (and to update if we ever support a different rounding).
      await api.applyTrayIconFor(q.primary.used_percent).catch(() => {});
    } catch (e) {
      setError(String(e));
    }
  }

  async function onActivate(row: AccountRow) {
    setRefreshing(true);
    try {
      await api.activateAccount(row.id);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(false);
    }
  }

  async function onDelete(row: AccountRow) {
    if (!confirm(`Delete account "${row.label}"? This only removes it from the Codex Runway library; the official auth.json is not touched.`)) return;
    setRefreshing(true);
    try {
      await api.deleteAccount(row.id);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(false);
    }
  }

  async function onAdd(auth: unknown, label: string | null) {
    setRefreshing(true);
    try {
      const id = pickAccountId(auth);
      await api.upsertAccount(id, label, auth);
      setShowAdd(false);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(false);
    }
  }

  const active = accounts?.accounts.find((a) => a.id === accounts.active_id);

  return (
    <div className="app">
      <header className="app__header">
        <div className="app__title">
          <span className="app__title-dot" />
          Codex Runway
          {active ? <span className="tag">{active.label}</span> : null}
        </div>
        <button
          className="app__refresh"
          aria-label="Refresh"
          onClick={() => refresh()}
          disabled={refreshing}
        >
          {refreshing ? "…" : "↻"}
        </button>
      </header>

      <main className="app__body">
        {error ? <div className="empty" style={{ color: "var(--danger)" }}>{error}</div> : null}

        {accounts ? (
          <AccountsCard
            accounts={accounts}
            onActivate={onActivate}
            onDelete={onDelete}
            onAddClick={() => setShowAdd(true)}
          />
        ) : (
          <div className="empty">Loading…</div>
        )}

        {quota ? <QuotaCard quota={quota} /> : null}
        {reset ? <ResetCreditsCard reset={reset} /> : null}
        {usage ? <UsageCard usage={usage} /> : null}
        {cost ? <CostCard cost={cost} /> : null}
      </main>

      <footer className="app__footer">
        <span>Local-only • no tokens uploaded</span>
      </footer>

      {showAdd ? (
        <AddAccountDialog
          onClose={() => setShowAdd(false)}
          onSubmitPaste={onAdd}
          onOAuthComplete={() => {
            setShowAdd(false);
            refresh();
          }}
        />
      ) : null}
    </div>
  );
}

function pickAccountId(auth: any): string {
  const accountId = auth?.tokens?.account_id;
  const subject = decodeJwtSub(auth?.tokens?.id_token);
  if (accountId) return String(accountId);
  if (subject) return String(subject);
  return "imported";
}

function decodeJwtSub(jwt: string | undefined): string | null {
  if (!jwt) return null;
  const parts = jwt.split(".");
  if (parts.length < 2) return null;
  try {
    const padded = parts[1].replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - (parts[1].length % 4)) % 4);
    const json = atob(padded);
    const obj = JSON.parse(json);
    return typeof obj.sub === "string" ? obj.sub : null;
  } catch {
    return null;
  }
}
