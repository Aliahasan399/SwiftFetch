import { useRef, useState, type FormEvent } from "react";
import { api, asIpcError, IpcError } from "../ipc";
import type { Settings } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";
import { ErrorBanner, InfoBanner } from "./Feedback";
import { Icon } from "./Icon";
import { UpdatePanel } from "./UpdatePanel";
import { useTaskActions } from "./TaskActionProvider";
import "./Forms.css";

type NumericKey = "max_active_tasks" | "connections_per_task" | "global_speed_limit" | "default_task_speed_limit" | "max_retries" | "retention_days";
type BooleanKey = "verify_checksums" | "scan_with_antivirus" | "prevent_sleep" | "start_on_login" | "browser_integration" | "scheduler_enabled" | "shutdown_after_complete" | "keep_partial_on_failure" | "clipboard_capture" | "encrypt_saved_headers";
const reservedSettings = new Set<keyof Settings>(["verify_checksums"]);
const numbers: { key: NumericKey; label: string; min: number; max: number; hint: string }[] = [
  { key: "max_active_tasks", label: "Simultaneous downloads", min: 1, max: 16, hint: "Global queue slots, from 1 to 16." },
  { key: "connections_per_task", label: "Connections per download", min: 1, max: 16, hint: "1–16; range support is determined by the server." },
  { key: "global_speed_limit", label: "Global speed limit (B/s)", min: 0, max: Number.MAX_SAFE_INTEGER, hint: "Bytes per second across all tasks. 0 is unlimited." },
  { key: "default_task_speed_limit", label: "Default task speed limit (B/s)", min: 0, max: Number.MAX_SAFE_INTEGER, hint: "Applied to new downloads. 0 is unlimited." },
  { key: "max_retries", label: "Maximum retries", min: 0, max: 20, hint: "0–20. Applies to new downloads; the core retries only eligible failures." },
  { key: "retention_days", label: "History retention (days)", min: 0, max: 3650, hint: "Automatically remove old finished-task metadata. 0 keeps history. Downloaded files are preserved." },
];
const switches: { key: BooleanKey; label: string; hint: string }[] = [
  { key: "verify_checksums", label: "Verify checksums", hint: "Reserved: not enforced by this core. A checksum is verified whenever a task supplies one." },
  { key: "scan_with_antivirus", label: "Scan with antivirus", hint: "Request a scan before completion. Quoted paths and a scan deadline are handled by the core; readiness is not exposed." },
  { key: "keep_partial_on_failure", label: "Keep partial files after failure", hint: "When off, the core discards only this task's own working file on failure. Completed files and other files in the folder are never touched." },
  { key: "clipboard_capture", label: "Watch clipboard for download URLs", hint: "Off by default. When on, a copied HTTP/HTTPS URL opens the Add download form for confirmation; no download starts automatically." },
  { key: "scheduler_enabled", label: "Enable scheduler", hint: "Allow the core to evaluate configured queue windows." },
  { key: "prevent_sleep", label: "Prevent sleep during active transfers", hint: "Ask the OS to stay awake only while downloads are active." },
  { key: "start_on_login", label: "Start on Windows login", hint: "Register this application for the current Windows account. Saving reports a startup-registration failure." },
  { key: "browser_integration", label: "Enable browser integration", hint: "Opt-in native-host bridge for an existing/signed SwiftFetch.exe. Requires explicit per-user extension registration; package preview and browser-store QA are pending. Setup: browser-extension/README.md (local docs)." },
  { key: "shutdown_after_complete", label: "Shut down after completion", hint: "Request shutdown after a completed batch. A visible 60-second countdown lets you cancel; new queued work cancels the request." },
  { key: "encrypt_saved_headers", label: "Encrypt saved request headers", hint: "Off by default for existing queues. When on, newly saved request headers are encrypted with Windows DPAPI. Existing rows are not rewritten automatically; use the migration action in Queue recovery." },
];

export function SettingsForm({ settings, onSaved }: { settings: Settings; onSaved: (settings: Settings) => void }) {
  const [draft, setDraft] = useState<Settings>({ ...settings });
  const [baseline, setBaseline] = useState<Settings>({ ...settings });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const [confirm, setConfirm] = useState(false);
  const saving = useRef(false);
  const actions = useTaskActions();
  const dirty = JSON.stringify(draft) !== JSON.stringify(baseline);
  function update<K extends keyof Settings>(key: K, value: Settings[K]) { setDraft((current) => ({ ...current, [key]: value })); }

  async function save() {
    if (saving.current) return;
    saving.current = true; setBusy(true); setError(null);
    try {
      await api.saveSettings(draft);
      const confirmed = await api.getSettings();
      const differs = (Object.keys(draft) as (keyof Settings)[]).some((key) => draft[key] !== confirmed[key]);
      setDraft({ ...confirmed }); setBaseline({ ...confirmed }); onSaved(confirmed); setConfirm(false);
      actions.notify(differs ? "Settings saved. The core normalized some values; the confirmed values are now shown." : "Settings saved and read back from the desktop core.");
    } catch (cause) { setError(asIpcError(cause)); }
    finally { saving.current = false; setBusy(false); }
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setError(null);
    if (numbers.some((field) => !Number.isSafeInteger(draft[field.key]) || draft[field.key] < field.min || draft[field.key] > field.max)) { setError(new IpcError({ code: "INVALID_SETTINGS", message: "Enter integer limits within the bounds shown in the form.", retryable: false })); return; }
    if (!draft.destination_root.trim()) { setError(new IpcError({ code: "INVALID_DESTINATION", message: "A default destination folder is required.", retryable: false })); return; }
    if (draft.proxy) {
      try {
        const proxy = new URL(draft.proxy);
        if (!["http:", "https:"].includes(proxy.protocol) || !proxy.hostname || proxy.username || proxy.password || proxy.search || proxy.hash) throw new Error("Invalid proxy");
      } catch { setError(new IpcError({ code: "INVALID_PROXY", message: "Use an HTTP or HTTPS proxy URL without credentials, a query, or a fragment.", retryable: false })); return; }
    }
    const risky = (draft.overwrite_policy !== baseline.overwrite_policy && draft.overwrite_policy === "replace") || (draft.start_on_login && !baseline.start_on_login) || (draft.shutdown_after_complete && !baseline.shutdown_after_complete) || (!draft.keep_partial_on_failure && baseline.keep_partial_on_failure) || (draft.scan_with_antivirus && (!baseline.scan_with_antivirus || draft.scanner_command !== baseline.scanner_command));
    if (risky) setConfirm(true); else void save();
  }

  return <>
    <form className="settings-form" onSubmit={submit}>
      <ErrorBanner error={error} />
      <fieldset className="fieldset-reset form-stack" disabled={busy}>
        <section className="panel settings-section"><header><span className="section-icon"><Icon name="folder" /></span><div><h2>Files & appearance</h2><p>Destination, collision behavior, and your workspace theme.</p></div></header><div className="form-grid">
          <label className="field full-width">Default destination folder<input required value={draft.destination_root} onChange={(event) => update("destination_root", event.target.value)} /><span className="field-hint">An explicit task destination or category rule may override this folder.</span></label>
          <label className="field">If a file already exists<select value={draft.overwrite_policy} onChange={(event) => update("overwrite_policy", event.target.value as Settings["overwrite_policy"])}><option value="rename">Rename the new file (recommended)</option><option value="skip">Skip existing file</option><option value="replace">Replace existing file</option><option value="ask">Ask me in task details</option></select></label>
          <label className="field">Theme<select value={draft.theme} onChange={(event) => update("theme", event.target.value)}><option value="dark">Dark</option><option value="light">Light</option><option value="system">Follow system</option>{!["dark", "light", "system"].includes(draft.theme) && <option value={draft.theme}>{draft.theme} (core value)</option>}</select></label>
          <label className="field">Language<select value={draft.locale || "en"} onChange={(event) => update("locale", event.target.value)}><option value="en">English</option><option value="bn">বাংলা (Bengali)</option>{!["en", "bn"].includes(draft.locale) && <option value={draft.locale}>{draft.locale} (core value)</option>}</select><span className="field-hint">Persisted as a locale key. Only listed strings are translated; other text stays in English.</span></label>
        </div>{(draft.overwrite_policy === "ask" || draft.overwrite_policy === "replace") && <InfoBanner warning>Replace overwrites an existing file and requires confirmation when adding a download. Ask retains the new file and offers explicit choices in task details. Rename creates a unique sibling name.</InfoBanner>}</section>
        <section className="panel settings-section"><header><span className="section-icon"><Icon name="activity" /></span><div><h2>Connections & limits</h2><p>Bounded concurrency and bandwidth, enforced by the core.</p></div></header><div className="form-grid">{numbers.map((field) => <label className="field" key={field.key}>{field.label}<input type="number" required disabled={reservedSettings.has(field.key)} step={1} min={field.min} max={field.max} value={draft[field.key]} onChange={(event) => update(field.key, Number(event.target.value))} /><span className="field-hint">{field.hint}</span></label>)}</div></section>
        <section className="panel settings-section"><header><span className="section-icon"><Icon name="shield" /></span><div><h2>Safety, scheduling & integration</h2><p>Unavailable preferences are disabled. Review each setting before enabling it.</p></div></header><div className="settings-switches">{switches.map((field) => <label className="settings-switch" key={field.key}><span><strong>{field.label}</strong><small>{field.hint}</small></span><input type="checkbox" role="switch" disabled={reservedSettings.has(field.key)} checked={draft[field.key]} onChange={(event) => update(field.key, event.target.checked)} aria-label={field.label} /></label>)}</div></section>
        <section className="panel settings-section"><header><span className="section-icon"><Icon name="settings" /></span><div><h2>Advanced configuration</h2><p>Use trusted local tools. Secrets never belong in a proxy URL.</p></div></header><div className="form-stack">
          <label className="field">Scanner command<input value={draft.scanner_command} autoComplete="off" spellCheck={false} onChange={(event) => update("scanner_command", event.target.value)} placeholder="Trusted scanner command supported by your core" /><span className="field-hint">The UI does not execute commands. The core handles quoted paths and a scan deadline; readiness is not exposed.</span></label>
          {draft.scan_with_antivirus && !draft.scanner_command && <InfoBanner warning>No scanner command is configured. Antivirus may be unavailable; the core does not expose a scanner readiness check. Do not treat an unreported scan as a clean result.</InfoBanner>}
          <label className="field">Proxy URL <span className="optional">Optional</span><input type="text" autoComplete="off" spellCheck={false} value={draft.proxy} onChange={(event) => update("proxy", event.target.value)} placeholder="http://proxy.example:8080" /><span className="field-hint">HTTP or HTTPS, without credentials. Applies to new connections after saving. Leave empty for direct connections.</span></label>
          {draft.browser_integration && <InfoBanner><strong>Browser bridge is enabled.</strong> For Chrome/Edge, load the development extension and register the native host with the exact extension ID shown at chrome://extensions; Firefox support is development-only and unregistered. If a download handoff fails, the browser download is left intact and you can paste the link into Add download. Setup: browser-extension/README.md; reversible cleanup: browser-extension/tools/unregister-host.ps1.</InfoBanner>}
        </div></section>
      </fieldset>
      <UpdatePanel />
      <footer className="settings-footer"><span className="muted" role="status">{busy ? "Saving and checking core settings…" : dirty ? "You have unsaved changes" : "Showing settings confirmed by the core"}</span><div><button type="button" className="button" disabled={busy || !dirty} onClick={() => { setDraft({ ...baseline }); setError(null); }}>Discard changes</button><button className="button button-primary" type="submit" disabled={busy || !dirty}>{busy ? "Saving…" : "Save settings"}</button></div></footer>
    </form>
    {confirm && <ConfirmDialog title="Apply sensitive settings?" confirmLabel="Save these settings" danger busy={busy} error={error} onClose={() => { if (!busy) setConfirm(false); }} onConfirm={() => { void save(); }}><p>Review these policies before applying them:</p><ul>{draft.overwrite_policy === "replace" && <li>Existing files may be overwritten.</li>}{draft.start_on_login && <li>SwiftFetch will be registered to start when you sign in.</li>}{draft.shutdown_after_complete && <li>The computer can shut down after a completed batch. Save other work first.</li>}{!draft.keep_partial_on_failure && <li>Failed tasks may discard only their own partial files.</li>}{draft.scan_with_antivirus && <li>The configured scanner will run before completion. Use only a trusted scanner.</li>}</ul></ConfirmDialog>}
  </>;
}
