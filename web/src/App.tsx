import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  api,
  type AccountIndex,
  type QuotaSnapshot,
  type ResetCreditsSnapshot,
  type UsageSummary,
  type AccountRow,
  type ApiCostSummary,
  type SessionIndexHealth,
} from "./api";
import { useLanguage } from "./i18n";
import { AccountsCard } from "./components/AccountsCard";
import { QuotaCard } from "./components/QuotaCard";
import { ResetCreditsCard } from "./components/ResetCreditsCard";
import { SessionRepairCard } from "./components/SessionRepairCard";
import { RecentSessionsCard } from "./components/RecentSessionsCard";
import { VisualUsageCard } from "./components/VisualUsageCard";
import { CostCard } from "./components/CostCard";
import { AddAccountDialog } from "./components/AddAccountDialog";

export default function App() {
  const { toggle, t } = useLanguage();
  const [accounts, setAccounts] = useState<AccountIndex | null>(null);
  const [quota, setQuota] = useState<QuotaSnapshot | null>(null);
  const [reset, setReset] = useState<ResetCreditsSnapshot | null>(null);
  const [usage, setUsage] = useState<UsageSummary | null>(null);
  const [cost, setCost] = useState<ApiCostSummary | null>(null);
  const [health, setHealth] = useState<SessionIndexHealth | null>(null);
  const [showAdd, setShowAdd] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
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

  useEffect(() => {
    if (toast) {
      const timer = setTimeout(() => setToast(null), 4000);
      return () => clearTimeout(timer);
    }
  }, [toast]);

  async function refresh() {
    setRefreshing(true);
    setError(null);
    try {
      const [idx, usage, cost, healthData] = await Promise.all([
        api.listAccounts(),
        api.scanLocalSessions(),
        api.computeApiCost().catch((e) => {
          console.warn("cost calc failed", e);
          return null;
        }),
        api.checkSessionIndexHealth().catch(() => null),
      ]);
      setAccounts(idx);
      setUsage(usage);
      setCost(cost);
      setHealth(healthData);

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
      // Update dynamic tray icon to match current primary quota
      if (q?.primary) {
        api.applyTrayIconFor(q.primary.used_percent).catch(() => {});
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleRepairIndex() {
    try {
      const res = await api.repairSessionIndex();
      setToast(t.sessionRepairSuccess.replace("{count}", String(res.repaired_count)));
      await refresh();
    } catch (e) {
      setError(`修复失败: ${e}`);
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
    if (!confirm(`确定要移除账号 "${row.label}" 吗？`)) {
      return;
    }
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

  async function handleBackup() {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "选择会话备份目标文件夹 (Select Backup Folder)",
      });
      if (selected && typeof selected === "string") {
        const res = await api.backupSessions(selected);
        setToast(t.backupSuccess.replace("{count}", String(res.files_copied)).replace("{path}", res.target_dir));
      }
    } catch (e) {
      setError(`备份失败: ${e}`);
    }
  }

  async function handleRestore() {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "选择包含 .jsonl 会话的备份文件夹 (Select Restore Folder)",
      });
      if (selected && typeof selected === "string") {
        const res = await api.restoreSessions(selected);
        setToast(t.restoreSuccess.replace("{count}", String(res.files_restored)));
        await refresh();
      }
    } catch (e) {
      setError(`还原失败: ${e}`);
    }
  }

  return (
    <div className="app">
      <header className="app__header">
        <div className="app__title">
          <span className="app__title-dot" />
          <span>{t.appTitle}</span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <button
            className="btn btn--sm btn--glass"
            onClick={toggle}
            title="Switch Language / 切换语言"
            style={{ fontWeight: 700, padding: "2px 6px" }}
          >
            {t.langToggle}
          </button>
          <button
            className="app__refresh"
            aria-label={t.refreshTitle}
            title={t.refreshTitle}
            onClick={() => refresh()}
            disabled={refreshing}
          >
            {refreshing ? "…" : "↻"}
          </button>
        </div>
      </header>

      <main className="app__body">
        {error ? (
          <div className="card glass-panel" style={{ color: "var(--danger)", marginBottom: 8, fontSize: 12 }}>
            {error}
          </div>
        ) : null}

        {toast ? (
          <div className="card glass-panel" style={{ color: "var(--ok)", marginBottom: 8, fontSize: 12, border: "1px solid var(--ok)" }}>
            {toast}
          </div>
        ) : null}

        {accounts ? (
          <AccountsCard
            accounts={accounts}
            t={t}
            onActivate={onActivate}
            onDelete={onDelete}
            onAddClick={() => setShowAdd(true)}
          />
        ) : (
          <div className="empty">{t.refreshing}</div>
        )}

        {quota ? <QuotaCard quota={quota} t={t} /> : null}
        {reset ? <ResetCreditsCard reset={reset} t={t} /> : null}

        {/* Session Index Health & Repair Card */}
        {health ? (
          <SessionRepairCard
            health={health}
            t={t}
            onRepair={handleRepairIndex}
            onRefresh={refresh}
          />
        ) : null}

        {/* Recent Sessions with Title, Tokens & Estimated Cost */}
        {usage && usage.sessions && usage.sessions.length > 0 ? (
          <RecentSessionsCard
            usage={usage}
            t={t}
            onRefresh={refresh}
          />
        ) : null}

        {usage ? (
          <VisualUsageCard
            usage={usage}
            t={t}
            onBackup={handleBackup}
            onRestore={handleRestore}
          />
        ) : null}

        {cost ? <CostCard cost={cost} t={t} /> : null}
      </main>

      <footer className="app__footer">
        <span>{t.localOnlyFooter}</span>
      </footer>

      {showAdd ? (
        <AddAccountDialog
          onClose={() => setShowAdd(false)}
          onSubmitPaste={onAdd}
          onOAuthComplete={() => {
            setShowAdd(false);
            refresh();
          }}
          t={t}
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

function decodeJwtSub(token: string | undefined): string | null {
  if (!token) return null;
  try {
    const parts = token.split(".");
    if (parts.length < 2) return null;
    const payload = JSON.parse(atob(parts[1]));
    return payload.sub ?? payload.user_id ?? null;
  } catch {
    return null;
  }
}
