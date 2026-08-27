import { useState } from "react";
import { open } from "@tauri-apps/plugin-shell";
import { api } from "../api";
import { translations } from "../i18n";

type Mode = "oauth" | "paste";

interface Props {
  onClose: () => void;
  onSubmitPaste: (auth: unknown, label: string | null) => void;
  onOAuthComplete: () => void;
  t: typeof translations["zh-CN"];
}

export function AddAccountDialog({ onClose, onSubmitPaste, onOAuthComplete, t }: Props) {
  const [mode, setMode] = useState<Mode>("oauth");
  return (
    <div className="dialog-backdrop" onClick={onClose}>
      <div className="dialog glass-modal" onClick={(e) => e.stopPropagation()}>
        <h2>{t.dialogAddTitle}</h2>
        <p style={{ color: "var(--fg-muted)", fontSize: 12, marginTop: 4 }}>
          {t.dialogAddSubtitle}
        </p>
        <div style={{ display: "flex", gap: 6, marginBottom: 12, marginTop: 12 }}>
          <button
            className={`btn ${mode === "oauth" ? "" : "btn--ghost"}`}
            onClick={() => setMode("oauth")}
          >
            {t.dialogOAuthTab}
          </button>
          <button
            className={`btn ${mode === "paste" ? "" : "btn--ghost"}`}
            onClick={() => setMode("paste")}
          >
            {t.dialogPasteTab}
          </button>
        </div>
        {mode === "oauth" ? (
          <OAuthFlow onClose={onClose} onComplete={onOAuthComplete} t={t} />
        ) : (
          <PasteFlow onClose={onClose} onSubmit={onSubmitPaste} t={t} />
        )}
      </div>
    </div>
  );
}

function OAuthFlow({
  onClose,
  onComplete,
  t,
}: {
  onClose: () => void;
  onComplete: () => void;
  t: typeof translations["zh-CN"];
}) {
  const [phase, setPhase] = useState<"idle" | "opening" | "waiting" | "done" | "error">("idle");
  const [error, setError] = useState<string | null>(null);

  async function start() {
    setError(null);
    setPhase("opening");
    try {
      const session = await api.oauthStart();
      await open(session.auth_url);
      setPhase("waiting");
      await api.oauthFinish(
        session.port,
        session.login_id,
        session.code_verifier,
        session.state,
      );
      setPhase("done");
      onComplete();
    } catch (e) {
      setPhase("error");
      setError(String(e));
    }
  }

  return (
    <div>
      <p style={{ marginBottom: 12, fontSize: 12, color: "var(--fg-muted)" }}>
        {t.dialogOAuthPrompt}
      </p>
      {error ? (
        <div style={{ color: "var(--danger)", fontSize: 12, marginBottom: 8 }}>
          {error}
        </div>
      ) : null}
      <div className="dialog__actions">
        <button className="btn btn--ghost" onClick={onClose}>
          {t.dialogCancel}
        </button>
        <button
          className="btn"
          onClick={start}
          disabled={phase === "opening" || phase === "waiting" || phase === "done"}
        >
          {phase === "idle" && t.dialogOAuthStartBtn}
          {phase === "opening" && t.dialogOAuthOpening}
          {phase === "waiting" && t.dialogOAuthWaiting}
          {phase === "done" && t.dialogOAuthDone}
          {phase === "error" && t.dialogOAuthRetry}
        </button>
      </div>
    </div>
  );
}

function PasteFlow({
  onClose,
  onSubmit,
  t,
}: {
  onClose: () => void;
  onSubmit: (auth: unknown, label: string | null) => void;
  t: typeof translations["zh-CN"];
}) {
  const [pasted, setPasted] = useState("");
  const [label, setLabel] = useState("");
  const [error, setError] = useState<string | null>(null);

  function submit() {
    setError(null);
    let parsed: any;
    try {
      parsed = JSON.parse(pasted);
    } catch (e) {
      setError(`JSON 格式无效: ${(e as Error).message}`);
      return;
    }
    if (typeof parsed !== "object" || parsed == null) {
      setError("需提供合法的 JSON 对象");
      return;
    }
    onSubmit(parsed, label.trim() || null);
  }

  return (
    <div>
      <div className="field">
        <label htmlFor="label">{t.dialogLabelField}</label>
        <input
          id="label"
          type="text"
          placeholder={t.dialogLabelPlaceholder}
          value={label}
          onChange={(e) => setLabel(e.target.value)}
        />
      </div>
      <div className="field">
        <label htmlFor="auth">auth.json</label>
        <textarea
          id="auth"
          spellCheck={false}
          placeholder={t.dialogAuthPlaceholder}
          value={pasted}
          onChange={(e) => setPasted(e.target.value)}
        />
      </div>
      {error ? (
        <div style={{ color: "var(--danger)", fontSize: 12 }}>{error}</div>
      ) : null}
      <div className="dialog__actions">
        <button className="btn btn--ghost" onClick={onClose}>
          {t.dialogCancel}
        </button>
        <button className="btn" onClick={submit} disabled={!pasted.trim()}>
          {t.dialogSubmitAdd}
        </button>
      </div>
    </div>
  );
}
