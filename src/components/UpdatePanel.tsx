import { useCallback, useEffect, useRef, useState } from "react";
import { formatBytes } from "../format";
import { useT } from "../i18n";
import { api, asIpcError, type IpcError } from "../ipc";
import type { UpdateState } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";
import { ErrorBanner, InfoBanner, LoadingState } from "./Feedback";
import { Icon } from "./Icon";
import "./Forms.css";

type Pending = "install" | "rollback" | null;

/** Reads publisher-supplied update state. This UI never fetches a remote URL. */
export function UpdatePanel() {
  const t = useT();
  const [state, setState] = useState<UpdateState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<IpcError | null>(null);
  const [pending, setPending] = useState<Pending>(null);
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);

  const load = useCallback(async () => {
    try {
      const next = await api.getUpdateState();
      if (alive.current) setState(next);
    } catch (cause) { if (alive.current) setError(asIpcError(cause)); }
    finally { if (alive.current) setLoading(false); }
  }, []);

  useEffect(() => {
    alive.current = true;
    void load();
    return () => { alive.current = false; };
  }, [load]);

  // Poll only while the core reports an active transfer, never when idle.
  useEffect(() => {
    if (!state || !["checking", "downloading", "installing"].includes(state.phase)) return;
    const timer = setInterval(() => { void load(); }, 1500);
    return () => clearInterval(timer);
  }, [state?.phase, load]);

  async function run(action: "check" | "install" | "rollback") {
    if (busy) return;
    setBusy(true); setError(null);
    try {
      const next = action === "check" ? await api.checkForUpdates() : action === "install" ? await api.installUpdate() : await api.rollbackUpdate();
      if (alive.current) setState(next);
      if (action !== "check") setPending(null);
    } catch (cause) { if (alive.current) setError(asIpcError(cause)); }
    finally { setBusy(false); }
  }

  const configured = state?.configured && state.phase !== "not_configured";
  return <section className="panel settings-section"><header><span className="section-icon"><Icon name="activity" /></span><div><h2>{t("update.title")}</h2><p>Publisher-supplied update status; no automatic background download.</p></div></header><div className="form-stack">
    <ErrorBanner error={error} onRetry={() => { void load(); }} />
    {loading && !state && <LoadingState />}
    {state && !configured && <InfoBanner>{t("update.notConfigured")}</InfoBanner>}
    {state && configured && <>
      <p>Current version: <strong>{state.current_version}</strong>{state.available_version && <> · Available: <strong>{state.available_version}</strong></>}</p>
      <p role="status">Status: {state.phase}{state.total_bytes ? ` · ${formatBytes(state.downloaded_bytes)} of ${formatBytes(state.total_bytes)}` : ""}</p>
      {state.notes && <p>{state.notes}</p>}
      {state.error && <InfoBanner warning>{state.error}</InfoBanner>}
    </>}
    {configured && <div className="topbar-actions">
      <button type="button" className="button button-small" disabled={busy} onClick={() => { void run("check"); }}>{t("update.check")}</button>
      <button type="button" className="button button-small" disabled={busy || (state?.phase !== "ready" && state?.phase !== "available")} onClick={() => setPending("install")}>{t("update.install")}</button>
      <button type="button" className="button button-small" disabled={busy || !state?.rollback_available} onClick={() => setPending("rollback")}>{t("update.rollback")}</button>
    </div>}
  </div>
  {pending && <ConfirmDialog title={pending === "install" ? "Install update now?" : "Roll back to the previous version?"} confirmLabel={pending === "install" ? "Install update" : "Roll back"} danger busy={busy} onClose={() => { if (!busy) setPending(null); }} onConfirm={() => { void run(pending); }}>
    <p>Active transfers must pause before the application files are replaced. Save your work and confirm when ready.</p>
    <p>No downloaded file or queue state is deleted by this action.</p>
  </ConfirmDialog>}
  </section>;
}
