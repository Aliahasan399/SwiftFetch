import { useRef, useState } from "react";
import { formatBytes, formatNumber } from "../format";
import { useResource } from "../hooks";
import { api, asIpcError, type IpcError } from "../ipc";
import { resetTaskFilter } from "../store";
import type { RestoreResult } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";
import { ErrorBanner, InfoBanner } from "./Feedback";
import { Icon } from "./Icon";

/** Local queue backup, SQLite restore and redacted inventory import. Never sends a full backup to clipboard. */
export function RecoveryPanel() {
  const recovery = useResource(api.getRecoveryState);
  const [backupPath, setBackupPath] = useState<string | null>(null);
  const [inventory, setInventory] = useState<string | null>(null);
  const [restoreOpen, setRestoreOpen] = useState(false);
  const [restoreText, setRestoreText] = useState("");
  const [selectedBackup, setSelectedBackup] = useState<string | null>(null);
  const [sqlRestoreOpen, setSqlRestoreOpen] = useState(false);
  const [restartOpen, setRestartOpen] = useState(false);
  const [restoreResult, setRestoreResult] = useState<RestoreResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const [notice, setNotice] = useState("");
  const [encryptOpen, setEncryptOpen] = useState(false);
  const locked = useRef(false);

  async function migrateSecrets() {
    if (locked.current) return;
    locked.current = true; setBusy(true); setError(null);
    try {
      const result = await api.encryptSavedSecrets();
      setNotice(`${formatNumber(result.rows_rewritten)} saved header rows are now encrypted with Windows DPAPI. New tasks will be protected automatically.`);
      setEncryptOpen(false);
    } catch (cause) { setError(asIpcError(cause)); }
    finally { locked.current = false; setBusy(false); }
  }

  async function run(action: "backup" | "export") {
    if (locked.current) return;
    locked.current = true; setBusy(true); setError(null);
    try {
      if (action === "backup") {
        setBackupPath(await api.backupMetadata());
        // A new backup is written to the parent data folder; reload the list so
        // it can be selected for restore without a manual refresh.
        await recovery.reload();
      } else setInventory(await api.exportQueue());
    } catch (cause) { setError(asIpcError(cause)); }
    finally { locked.current = false; setBusy(false); }
  }

  async function restoreInventory() {
    if (locked.current) return;
    locked.current = true; setBusy(true); setError(null);
    try {
      const count = await api.importQueue(restoreText);
      setNotice(`${formatNumber(count)} queue rows imported. Incomplete tasks remain paused for manual start.`);
      setRestoreOpen(false); setRestoreText("");
      await resetTaskFilter();
    } catch (cause) { setError(asIpcError(cause)); }
    finally { locked.current = false; setBusy(false); }
  }

  async function restoreSqlite() {
    if (locked.current || !selectedBackup) return;
    locked.current = true; setBusy(true); setError(null);
    try {
      const result = await api.restoreMetadata(selectedBackup);
      setRestoreResult(result);
      setNotice(`${formatNumber(result.restored_tasks)} tasks restored from the local backup.`);
      setSqlRestoreOpen(false); setSelectedBackup(null);
      // The online path replaces the live database; re-read the full catalog.
      await resetTaskFilter();
    } catch (cause) { setError(asIpcError(cause)); }
    finally { locked.current = false; setBusy(false); }
  }

  async function restartNow() {
    if (locked.current) return;
    locked.current = true; setBusy(true); setError(null);
    try { await api.restartApplication(); }
    catch (cause) { setError(asIpcError(cause)); setRestartOpen(false); }
    finally { locked.current = false; setBusy(false); }
  }

  const recoveryData = recovery.data;
  return <section className="panel"><header className="panel-header"><div><h2>Queue recovery</h2><p>Local metadata backup, SQLite restore and a shareable queue inventory.</p></div><button className="button" disabled={recovery.loading} onClick={() => { void recovery.reload(); }}><Icon name="retry" size={17} />Refresh backups</button></header><div className="form-stack">
    <InfoBanner warning>A full metadata backup contains private URLs and headers. Keep it local and private. The shareable inventory strips URLs but keeps local paths, and cannot resume authenticated links. Restoring a backup replaces the current queue; a safety copy of the previous queue is kept automatically.</InfoBanner>
    <ErrorBanner error={recovery.error} onRetry={() => { void recovery.reload(); }} />
    <ErrorBanner error={error} onRetry={() => setError(null)} />
    {notice && <p role="status" className="field-hint">{notice}</p>}
    {recoveryData?.message && <InfoBanner warning>{recoveryData.message}</InfoBanner>}
    <div className="topbar-actions">
      <button className="button button-small" disabled={busy} onClick={() => { void run("backup"); }}>Back up metadata</button>
      <button className="button button-small" disabled={busy} onClick={() => { void run("export"); }}>Export queue inventory</button>
      <button className="button button-small" disabled={busy} onClick={() => { setError(null); setRestoreOpen(true); }}>Restore inventory</button>
      <button className="button button-small" disabled={busy} onClick={() => { setError(null); setEncryptOpen(true); }}>Encrypt saved headers</button>
    </div>
    {backupPath && <p className="break-anywhere mono" role="status">Backup saved locally: {backupPath}</p>}

    <section className="detail-section"><h3>Local SQLite backups</h3>
      {recoveryData?.data_directory && <p className="field-hint break-anywhere mono">Data folder: {recoveryData.data_directory}</p>}
      {recoveryData && recoveryData.backups.length === 0 && <p className="field-hint">No local queue backups are available yet. Use “Back up metadata” to create one.</p>}
      {recoveryData && recoveryData.backups.length > 0 && <div className="table-scroll" tabIndex={0} role="region" aria-label="Local SQLite backups"><table className="simple-table"><caption className="sr-only">Local SQLite backups</caption><thead><tr><th scope="col">Backup file</th><th scope="col">Size</th><th scope="col">Action</th></tr></thead><tbody>{recoveryData.backups.map((backup) => <tr key={backup.name}><td className="mono break-anywhere">{backup.name}</td><td>{formatBytes(backup.size_bytes)}</td><td><button className="button button-small" disabled={busy} onClick={() => { setError(null); setSelectedBackup(backup.name); setSqlRestoreOpen(true); }}>Restore…</button></td></tr>)}</tbody></table></div>}
      {restoreResult && <div className="detail-list" role="status"><div><dt>Restored tasks</dt><dd>{formatNumber(restoreResult.restored_tasks)}</dd></div><div><dt>Safety backup (kept locally)</dt><dd className="mono break-anywhere">{restoreResult.safety_backup}</dd></div><div><dt>Restart required</dt><dd>{restoreResult.restart_required ? "Yes — the application must restart to use the restored queue." : "No — the live queue was updated."}</dd></div></div>}
      {restoreResult?.restart_required && <div className="topbar-actions"><button className="button button-primary" disabled={busy} onClick={() => { setError(null); setRestartOpen(true); }}><Icon name="retry" size={16} />Restart now</button><p className="field-hint">Restart pauses active transfers first. You can also close and reopen SwiftFetch later.</p></div>}
    </section>

    {inventory !== null && <>
      <label className="field">Shareable inventory (URLs removed)<textarea className="mono" readOnly rows={6} value={inventory} aria-label="Shareable queue inventory" /></label>
      <button className="button button-small" onClick={() => {
        if (!navigator.clipboard) { setNotice("Clipboard unavailable. Select and copy the inventory manually."); return; }
        void navigator.clipboard.writeText(inventory).then(() => setNotice("Inventory copied. Review before sharing.")).catch(() => setNotice("Could not copy. Select and copy manually."));
      }}><Icon name="copy" size={16} />Copy inventory</button>
    </>}
  </div>
  {restoreOpen && <ConfirmDialog title="Restore queue inventory?" confirmLabel="Import inventory" danger busy={busy} error={error} onClose={() => { if (!busy) setRestoreOpen(false); }} onConfirm={() => { void restoreInventory(); }}>
    <p>Paste a previously exported inventory. Imported incomplete tasks stay paused and manual; no file is downloaded or deleted automatically.</p>
    <textarea className="mono" rows={6} value={restoreText} onChange={(event) => setRestoreText(event.target.value)} placeholder="Inventory JSON" aria-label="Inventory JSON to import" />
  </ConfirmDialog>}
  {sqlRestoreOpen && selectedBackup && <ConfirmDialog title="Replace the queue with this backup?" confirmLabel="Restore backup" danger busy={busy} error={error} onClose={() => { if (!busy) { setSqlRestoreOpen(false); setSelectedBackup(null); } }} onConfirm={() => { void restoreSqlite(); }}>
    <p>Selected local backup: <strong className="break-anywhere mono">{selectedBackup}</strong></p>
    <p>Active transfers will be paused. The current queue is safety-backed up before replacement. If this is an offline startup restore, the application will need a restart; you will see the result afterwards.</p>
  </ConfirmDialog>}
  {restartOpen && <ConfirmDialog title="Restart SwiftFetch now?" confirmLabel="Restart application" danger busy={busy} error={error} onClose={() => { if (!busy) setRestartOpen(false); }} onConfirm={() => { void restartNow(); }}>
    <p>Active transfers are paused before restart. Saved queue state and downloaded files are preserved.</p>
  </ConfirmDialog>}
  {encryptOpen && <ConfirmDialog title="Encrypt saved request headers?" confirmLabel="Encrypt now" danger busy={busy} error={error} onClose={() => { if (!busy) setEncryptOpen(false); }} onConfirm={() => { void migrateSecrets(); }}>
    <p>This enables at-rest protection for saved request headers using Windows DPAPI and rewrites existing rows in one transaction. If the encrypted format is unavailable, nothing changes and the current queue is preserved. Downloaded files are not touched.</p>
  </ConfirmDialog>}
  </section>;
}
