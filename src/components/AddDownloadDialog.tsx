import { useEffect, useRef, useState, type FormEvent } from "react";
import { PRIORITY_MAX, PRIORITY_MIN } from "../actions";
import { validateDownloadUrl, validateFilename } from "../format";
import { useResource } from "../hooks";
import { useT } from "../i18n";
import { api, asIpcError, IpcError } from "../ipc";
import { downloadStore } from "../store";
import type { AddDownloadRequest } from "../types";
import { Dialog } from "./Dialog";
import { ErrorBanner, InfoBanner } from "./Feedback";
import { Icon } from "./Icon";
import { useTaskActions } from "./TaskActionProvider";
import "./Forms.css";

export function AddDownloadDialog({ onClose, initialUrl = "" }: { onClose: () => void; initialUrl?: string }) {
  const settings = useResource(api.getSettings);
  const categories = useResource(api.listCategories);
  const queues = useResource(api.listQueues);
  const actions = useTaskActions();
  const t = useT();
  const [url, setUrl] = useState(() => (validateDownloadUrl(initialUrl) === null ? initialUrl.trim() : ""));
  const [pasteError, setPasteError] = useState<string | null>(null);

  async function pasteUrl() {
    setPasteError(null);
    try {
      const value = await api.readClipboardUrl();
      if (!value) { setPasteError("The clipboard does not contain a URL. Copy a permitted HTTP or HTTPS link, then try again."); return; }
      const invalid = validateDownloadUrl(value);
      if (invalid) { setPasteError(invalid); return; }
      setUrl(value.trim());
    } catch (cause) { setPasteError(asIpcError(cause).message); }
  }
  const [name, setName] = useState("");
  const [destination, setDestination] = useState("");
  const [queueId, setQueueId] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [priority, setPriority] = useState(50);
  const [connections, setConnections] = useState(4);
  const [speedLimit, setSpeedLimit] = useState(0);
  const [startPolicy, setStartPolicy] = useState("immediate");
  const [algorithm, setAlgorithm] = useState("");
  const [checksum, setChecksum] = useState("");
  const [overwriteConsent, setOverwriteConsent] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const [validation, setValidation] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [createdId, setCreatedId] = useState<string | null>(null);
  const initialized = useRef(false);
  const submitting = useRef(false);
  const touchedConnections = useRef(false);
  const touchedSpeedLimit = useRef(false);
  const overwrites = settings.data?.overwrite_policy === "replace";

  useEffect(() => {
    if (settings.data && !initialized.current) {
      initialized.current = true;
      if (!touchedConnections.current) setConnections(settings.data.connections_per_task);
      if (!touchedSpeedLimit.current) setSpeedLimit(settings.data.default_task_speed_limit);
    }
  }, [settings.data]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting.current) return;
    setValidation(null);
    setError(null);
    if (!createdId) {
      const invalid = validateDownloadUrl(url) ?? validateFilename(name);
      if (invalid) { setValidation(invalid); return; }
      if (!settings.data) { setError(new IpcError({ code: "SETTINGS_UNAVAILABLE", message: "Load the core settings before adding a task so the overwrite policy is known.", retryable: true })); return; }
      if (![priority, connections, speedLimit].every(Number.isSafeInteger) || priority < PRIORITY_MIN || priority > PRIORITY_MAX || connections < 1 || connections > 16 || speedLimit < 0) { setValidation(`Use integer limits: priority ${PRIORITY_MIN}–${PRIORITY_MAX}, connections 1–16, and a nonnegative speed limit.`); return; }
      const lengths: Record<string, number> = { sha256: 64, sha512: 128, sha1: 40, md5: 32 };
      if (algorithm && (!/^[a-fA-F0-9]+$/.test(checksum.trim()) || checksum.trim().length !== lengths[algorithm])) { setValidation(`Enter a ${lengths[algorithm]}-character hexadecimal ${algorithm.toUpperCase()} checksum from a trusted source.`); return; }
      if (overwrites && !overwriteConsent) { setValidation("Confirm the current overwrite policy, or change it to Rename in Settings."); return; }
    }
    submitting.current = true;
    setBusy(true);
    try {
      let id = createdId;
      if (!id) {
        const request: AddDownloadRequest = {
          url: url.trim(), suggested_name: name || undefined,
          destination_directory: destination.trim() || undefined,
          queue_id: queueId || undefined, category_id: categoryId || undefined,
          priority, connections, speed_limit: speedLimit, start_policy: startPolicy,
          headers: [], checksum: algorithm ? { algorithm, value: checksum.trim() } : undefined,
        };
        const added = await api.addDownload(request);
        id = added.id;
        setCreatedId(id);
        downloadStore.upsert(added);
      }
      const revision = downloadStore.getRevision();
      const confirmed = await api.getTask(id);
      downloadStore.upsert(confirmed, revision);
      actions.notify("Download added and confirmed with the desktop core.");
      onClose();
    } catch (cause) {
      setError(asIpcError(cause));
    } finally {
      submitting.current = false;
      setBusy(false);
    }
  }

  return <Dialog title="Add download" description="A permitted link. Your destination. The core handles the rest." onClose={onClose} busy={busy} wide>
    <form onSubmit={(event) => { void submit(event); }}>
      <div className="dialog-body form-stack">
        <InfoBanner>Browser handoff is opt-in and needs per-user extension registration. If it is not set up, paste the permitted URL here. A failed request keeps your input; nothing is sent to a cloud service.</InfoBanner>
        <ErrorBanner error={settings.error} onRetry={() => { void settings.reload(); }} />
        <ErrorBanner error={categories.error} onRetry={() => { void categories.reload(); }} />
        <ErrorBanner error={queues.error} onRetry={() => { void queues.reload(); }} />
        {createdId && <InfoBanner warning>The core accepted this download, but read-back may still be pending. Confirm the existing task below; do not submit the URL again.</InfoBanner>}
        <ErrorBanner error={error} />
        {validation && <p className="field-error" role="alert" id="add-validation">{validation}</p>}
        <fieldset className="fieldset-reset form-stack" disabled={busy || createdId !== null}>
          <label className="field">Download URL <input data-autofocus type="text" inputMode="url" autoComplete="off" spellCheck={false} required value={url} onChange={(event) => { const value = event.target.value; setUrl(value); setPasteError(null); if (validation && validateDownloadUrl(value) === null) setValidation(null); }} placeholder="https://example.com/download.zip" aria-describedby={validation ? "add-validation" : pasteError ? "paste-error" : "url-help"} aria-invalid={Boolean(validation && validateDownloadUrl(url))} /><span className="field-hint" id="url-help">HTTP or HTTPS only. Credentials embedded in URLs are not accepted.</span><button type="button" className="button button-small" disabled={busy || createdId !== null} onClick={() => { void pasteUrl(); }}><Icon name="copy" size={15} />{t("dialog.paste")}</button>{pasteError && <span className="field-error" id="paste-error" role="alert">{pasteError}</span>}</label>
          <div className="form-grid"><label className="field">Suggested filename <span className="optional">Optional</span><input value={name} onChange={(event) => setName(event.target.value)} placeholder="Use the server's filename" /></label><label className="field">Destination folder <span className="optional">Optional</span><input value={destination} onChange={(event) => setDestination(event.target.value)} placeholder={settings.data?.destination_root ?? "Use the core's default folder"} /><span className="field-hint">Enter a full folder path. The core validates it.</span></label></div>
          <div className="form-grid"><label className="field">Category<select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}><option value="">Automatic / default</option>{categories.data?.map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}</select></label><label className="field">Queue<select value={queueId} onChange={(event) => setQueueId(event.target.value)}><option value="">Default queue</option>{queues.data?.map((queue) => <option key={queue.id} value={queue.id}>{queue.name}{!queue.enabled ? " (disabled)" : ""}</option>)}</select></label></div>
          <fieldset className="form-section"><legend>Transfer control</legend><div className="form-grid form-grid-three">
            <label className="field">Priority<input type="number" required min={PRIORITY_MIN} max={PRIORITY_MAX} step={1} value={priority} onChange={(event) => setPriority(Number(event.target.value))} /><span className="field-hint">{PRIORITY_MIN}–{PRIORITY_MAX}; higher priorities start first.</span></label>
            <label className="field">Connections<input type="number" required min={1} max={16} step={1} value={connections} onChange={(event) => { touchedConnections.current = true; setConnections(Number(event.target.value)); }} /><span className="field-hint">1 is single-stream mode.</span></label>
            <label className="field">Speed limit (B/s)<input type="number" required min={0} max={Number.MAX_SAFE_INTEGER} step={1} value={speedLimit} onChange={(event) => { touchedSpeedLimit.current = true; setSpeedLimit(Number(event.target.value)); }} /><span className="field-hint">Bytes per second; 0 is unlimited.</span></label>
          </div><label className="field">When to start<select value={startPolicy} onChange={(event) => setStartPolicy(event.target.value)}><option value="immediate">As soon as a queue slot is available</option><option value="manual">Manually — keep paused until I start it</option><option value="scheduled">During this queue's scheduled window</option></select></label>{startPolicy === "scheduled" && <p className="field-hint">Configure the queue's window in Scheduler. Scheduling must also be enabled in Settings.</p>}</fieldset>
          <fieldset className="form-section"><legend>Integrity check <span className="optional">Optional</span></legend><div className="form-grid checksum-grid"><label className="field">Algorithm<select value={algorithm} onChange={(event) => { setAlgorithm(event.target.value); setChecksum(""); }}><option value="">No expected checksum</option><option value="sha256">SHA-256 (recommended)</option><option value="sha512">SHA-512</option><option value="sha1">SHA-1 (legacy)</option><option value="md5">MD5 (legacy)</option></select></label><label className="field">Expected checksum<input className="mono" value={checksum} disabled={!algorithm} required={Boolean(algorithm)} autoComplete="off" spellCheck={false} onChange={(event) => setChecksum(event.target.value)} placeholder="Trusted hexadecimal digest" /></label></div><p className="field-hint">A hash checks integrity, not whether the file is safe to execute. MD5 and SHA-1 are not collision-resistant.</p></fieldset>
          {overwrites && <InfoBanner warning><label className="check-field"><input type="checkbox" required checked={overwriteConsent} onChange={(event) => setOverwriteConsent(event.target.checked)} /><span>I accept replacing an existing file at this destination. Choose Rename in Settings to preserve existing files.</span></label></InfoBanner>}
        </fieldset>
        <p className="privacy-note"><Icon name="shield" size={16} />No credentials are collected here. Downloads are never opened automatically.</p>
      </div>
      <footer className="dialog-footer"><button type="button" className="button" onClick={onClose} disabled={busy}>{createdId ? "Close" : "Cancel"}</button><button className="button button-primary" type="submit" disabled={busy || settings.loading}><Icon name="plus" size={18} />{busy ? "Waiting for core…" : createdId ? "Confirm added task" : "Add download"}</button></footer>
    </form>
  </Dialog>;
}
