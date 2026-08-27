import { useState } from "react";
import { open } from "@tauri-apps/plugin-shell";
import { api } from "../api";

type Mode = "oauth" | "paste";

interface Props {
  onClose: () => void;
  /** Called with the parsed Codex auth when paste mode succeeds. */
  onSubmitPaste: (auth: unknown, label: string | null) => void;
  /** Called after OAuth login completes (the account is already in the
   *  library; the parent should refresh). */
  onOAuthComplete: () => void;
}

export function AddAccountDialog({ onClose, onSubmitPaste, onOAuthComplete }: Props) {
  const [mode, setMode] = useState<Mode>("oauth");
  return (
    <div className="dialog-backdrop" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <h2>Add Codex account</h2>
        <p>
          Sign in with ChatGPT, or paste an existing{" "}
          <code>~/.codex/auth.json</code> file.
        </p>
        <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
          <button
            className={`btn ${mode === "oauth" ? "" : "btn--ghost"}`}
            onClick={() => setMode("oauth")}
            disabled={mode === "oauth"}
          >
            Sign in with ChatGPT
          </button>
          <button
            className={`btn ${mode === "paste" ? "" : "btn--ghost"}`}
            onClick={() => setMode("paste")}
            disabled={mode === "paste"}
          >
            Paste auth.json
          </button>
        </div>
        {mode === "oauth" ? (
          <OAuthFlow onClose={onClose} onComplete={onOAuthComplete} />
        ) : (
          <PasteFlow onClose={onClose} onSubmit={onSubmitPaste} />
        )}
      </div>
    </div>
  );
}

function OAuthFlow({
  onClose,
  onComplete,
}: {
  onClose: () => void;
  onComplete: () => void;
}) {
  const [phase, setPhase] = useState<"idle" | "opening" | "waiting" | "done" | "error">("idle");
  const [error, setError] = useState<string | null>(null);

  async function start() {
    setError(null);
    setPhase("opening");
    try {
      const session = await api.oauthStart();
      // Open the system browser. On Windows this is the default browser
      // because Tauri 2's `shell.open` resolves to the OS handler.
      await open(session.auth_url);
      setPhase("waiting");
      // Hand off to the backend: it binds the callback server, waits for
      // the browser to redirect, and exchanges the code for tokens.
      await api.oauthFinish(
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
      <p style={{ marginBottom: 12 }}>
        Click below to open ChatGPT sign-in in your browser. After you finish,
        this window will refresh with the new account ready to use.
      </p>
      {error ? (
        <div style={{ color: "var(--danger)", fontSize: 12, marginBottom: 8 }}>
          {error}
        </div>
      ) : null}
      <div className="dialog__actions">
        <button className="btn btn--ghost" onClick={onClose}>
          Cancel
        </button>
        <button
          className="btn"
          onClick={start}
          disabled={phase === "opening" || phase === "waiting" || phase === "done"}
        >
          {phase === "idle" && "Open ChatGPT sign-in"}
          {phase === "opening" && "Preparing…"}
          {phase === "waiting" && "Waiting for sign-in…"}
          {phase === "done" && "Done"}
          {phase === "error" && "Retry"}
        </button>
      </div>
    </div>
  );
}

function PasteFlow({
  onClose,
  onSubmit,
}: {
  onClose: () => void;
  onSubmit: (auth: unknown, label: string | null) => void;
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
      setError(`Invalid JSON: ${(e as Error).message}`);
      return;
    }
    if (typeof parsed !== "object" || parsed == null) {
      setError("Expected a JSON object");
      return;
    }
    onSubmit(parsed, label.trim() || null);
  }

  return (
    <div>
      <div className="field">
        <label htmlFor="label">Label (optional)</label>
        <input
          id="label"
          type="text"
          placeholder="personal · work · plus"
          value={label}
          onChange={(e) => setLabel(e.target.value)}
        />
      </div>
      <div className="field">
        <label htmlFor="auth">auth.json</label>
        <textarea
          id="auth"
          spellCheck={false}
          placeholder='{ "auth_mode": "chatgpt", "tokens": { "access_token": "…", "refresh_token": "…" } }'
          value={pasted}
          onChange={(e) => setPasted(e.target.value)}
        />
      </div>
      {error ? (
        <div style={{ color: "var(--danger)", fontSize: 12 }}>{error}</div>
      ) : null}
      <div className="dialog__actions">
        <button className="btn btn--ghost" onClick={onClose}>
          Cancel
        </button>
        <button className="btn" onClick={submit} disabled={!pasted.trim()}>
          Add
        </button>
      </div>
    </div>
  );
}
