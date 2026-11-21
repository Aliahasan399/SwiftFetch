import { useCallback, useRef, useState, type FormEvent } from "react";
import { formatDays, weekDays } from "../format";
import { useResource } from "../hooks";
import { api, asIpcError, IpcError } from "../ipc";
import { downloadStore, useDownloads } from "../store";
import type { QueueDto, ScheduleDto } from "../types";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Dialog } from "../components/Dialog";
import { EmptyState } from "../components/EmptyState";
import { ErrorBanner, InfoBanner, LoadingState } from "../components/Feedback";
import { Icon } from "../components/Icon";
import { PAGE_SIZE, Pagination } from "../components/Pagination";
import { useTaskActions } from "../components/TaskActionProvider";
import "../components/Forms.css";

const afterLabels: Record<string, string> = { nothing: "No additional action", pause: "Pause queue", shutdown: "Request shutdown with a cancel countdown" };

function ScheduleEditor({ schedule, queues, onSave, onClose }: { schedule: ScheduleDto | null; queues: QueueDto[]; onSave: (value: ScheduleDto) => Promise<void>; onClose: () => void }) {
  const [draft, setDraft] = useState<ScheduleDto>(() => schedule ? { ...schedule } : { id: crypto.randomUUID(), queue_id: queues[0]?.id ?? "", days_mask: 127, start_local_time: "22:00", end_local_time: "06:00", action_after: "nothing", enabled: true });
  const [busy, setBusy] = useState(false);
  const [consent, setConsent] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const saving = useRef(false);
  function update<K extends keyof ScheduleDto>(key: K, value: ScheduleDto[K]) { setDraft((current) => ({ ...current, [key]: value })); }
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (saving.current) return;
    setError(null);
    if (!draft.queue_id || draft.days_mask < 1 || draft.days_mask > 127 || !/^([01]\d|2[0-3]):[0-5]\d$/.test(draft.start_local_time) || !/^([01]\d|2[0-3]):[0-5]\d$/.test(draft.end_local_time) || draft.start_local_time === draft.end_local_time) { setError(new IpcError({ code: "INVALID_SCHEDULE", message: "Choose a queue, at least one day, and distinct start/end times in HH:MM format.", retryable: false })); return; }
    if (draft.action_after === "shutdown" && !consent) { setError(new IpcError({ code: "CONFIRMATION_REQUIRED", message: "Confirm the shutdown action before saving this schedule.", retryable: false })); return; }
    saving.current = true; setBusy(true);
    try { await onSave(draft); onClose(); }
    catch (cause) { setError(asIpcError(cause)); }
    finally { saving.current = false; setBusy(false); }
  }
  return <Dialog title={schedule ? "Edit schedule" : "Create schedule"} description="Time windows use this computer's local time." onClose={onClose} busy={busy}>
    <form onSubmit={(event) => { void submit(event); }}><div className="dialog-body form-stack"><ErrorBanner error={error} /><fieldset className="fieldset-reset form-stack" disabled={busy}>
      <label className="field">Queue<select data-autofocus required value={draft.queue_id} onChange={(event) => update("queue_id", event.target.value)}><option value="" disabled>Choose a queue</option>{queues.map((queue) => <option key={queue.id} value={queue.id}>{queue.name}{!queue.enabled ? " (disabled)" : ""}</option>)}{draft.queue_id && !queues.some((queue) => queue.id === draft.queue_id) && <option value={draft.queue_id}>{draft.queue_id} (not currently listed)</option>}</select></label>
      <fieldset className="form-section"><legend>Days</legend><div className="day-picker">{weekDays.map((day, index) => <label className={(draft.days_mask & (1 << index)) !== 0 ? "day-selected" : ""} key={day}><input type="checkbox" checked={(draft.days_mask & (1 << index)) !== 0} onChange={(event) => update("days_mask", event.target.checked ? draft.days_mask | (1 << index) : draft.days_mask & ~(1 << index))} /><span>{day}</span></label>)}</div></fieldset>
      <div className="form-grid"><label className="field">Start time<input type="time" required step={60} value={draft.start_local_time} onChange={(event) => update("start_local_time", event.target.value)} /></label><label className="field">End time<input type="time" required step={60} value={draft.end_local_time} onChange={(event) => update("end_local_time", event.target.value)} /></label></div>
      {draft.end_local_time < draft.start_local_time && <InfoBanner>This window crosses midnight. Selected days identify the start day; the end falls on the following day.</InfoBanner>}
      <label className="field">After the window<select value={draft.action_after} onChange={(event) => { update("action_after", event.target.value); setConsent(false); }}>{Object.entries(afterLabels).map(([value, label]) => <option value={value} key={value}>{label}</option>)}{!(draft.action_after in afterLabels) && <option value={draft.action_after}>{draft.action_after} (core value)</option>}</select></label>
      <label className="check-field"><input type="checkbox" checked={draft.enabled} onChange={(event) => update("enabled", event.target.checked)} /><span>Enable this schedule</span></label>
      {draft.action_after === "shutdown" && <InfoBanner warning><label className="check-field"><input type="checkbox" required checked={consent} onChange={(event) => setConsent(event.target.checked)} /><span>I explicitly request shutdown after this window and understand that unsaved work could be lost. The core owns power actions.</span></label></InfoBanner>}
    </fieldset></div><footer className="dialog-footer"><button className="button" type="button" onClick={onClose} disabled={busy}>Cancel</button><button className="button button-primary" disabled={busy} type="submit">{busy ? "Saving…" : "Save schedule"}</button></footer></form>
  </Dialog>;
}

export function SchedulerView() {
  const revision = useDownloads().catalog_revision;
  const load = useCallback(() => { void revision; return api.listSchedules(); }, [revision]);
  const schedules = useResource(load);
  const queues = useResource(api.listQueues);
  const settings = useResource(api.getSettings);
  const actions = useTaskActions();
  const [editor, setEditor] = useState<{ schedule: ScheduleDto | null } | null>(null);
  const [deleting, setDeleting] = useState<ScheduleDto | null>(null);
  const [enabling, setEnabling] = useState<ScheduleDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const mutating = useRef(false);
  const [page, setPage] = useState(1);
  const rows = schedules.data ?? [];
  const safePage = Math.min(page, Math.max(1, Math.ceil(rows.length / PAGE_SIZE)));
  async function save(value: ScheduleDto) {
    await api.saveSchedule(value);
    const confirmed = await api.listSchedules();
    const saved = confirmed.find((entry) => entry.id === value.id);
    if (!saved || (Object.keys(value) as (keyof ScheduleDto)[]).some((key) => value[key] !== saved[key])) throw new IpcError({ code: "READBACK_FAILED", message: "The schedule was not returned as saved. Reload schedules before trying again.", retryable: true });
    schedules.replace(confirmed); downloadStore.bumpCatalog();
    actions.notify("Schedule saved and confirmed with the core.");
  }
  async function toggle(value: ScheduleDto) {
    if (mutating.current) return;
    mutating.current = true; setBusy(true); setError(null);
    try { await save({ ...value, enabled: !value.enabled }); setEnabling(null); }
    catch (cause) { setError(asIpcError(cause)); }
    finally { mutating.current = false; setBusy(false); }
  }
  async function remove() {
    if (!deleting || mutating.current) return;
    mutating.current = true; setBusy(true); setError(null);
    try {
      await api.deleteSchedule(deleting.id);
      const confirmed = await api.listSchedules();
      if (confirmed.some((entry) => entry.id === deleting.id)) throw new IpcError({ code: "READBACK_FAILED", message: "The schedule is still present after deletion. Reload before trying again.", retryable: true });
      schedules.replace(confirmed); downloadStore.bumpCatalog(); setDeleting(null);
      actions.notify("Schedule removed. Downloads and files were not removed.");
    } catch (cause) { setError(asIpcError(cause)); }
    finally { mutating.current = false; setBusy(false); }
  }
  return <div className="view-stack">
    {settings.data && !settings.data.scheduler_enabled && <InfoBanner warning>Scheduling is disabled in Settings. Windows can be saved here but will not take effect until the scheduler is enabled.</InfoBanner>}
    <InfoBanner>Queue windows use local time. Overnight windows continue into the next day. A configured shutdown starts a visible 60-second countdown only after a window has actually ended; you can cancel it.</InfoBanner>
    <ErrorBanner error={schedules.error} onRetry={() => { void schedules.reload(); }} /><ErrorBanner error={queues.error} onRetry={() => { void queues.reload(); }} /><ErrorBanner error={settings.error} onRetry={() => { void settings.reload(); }} /><ErrorBanner error={error} />
    <section className="panel"><header className="panel-header"><div><h2>Scheduled windows</h2><p>Use quiet hours without giving up control.</p></div><button className="button button-primary" disabled={!queues.data?.length || busy} onClick={() => setEditor({ schedule: null })}><Icon name="plus" size={18} />Create schedule</button></header>
      {schedules.loading && !schedules.data ? <LoadingState /> : rows.length === 0 ? <EmptyState icon="clock" title={schedules.error ? "Schedules unavailable" : "No schedules yet"} description="Create a window for an existing queue. Settings and the queue must both be enabled." /> : <div className="schedule-list">{rows.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE).map((schedule) => <article className="schedule-card" key={schedule.id}><span className="section-icon"><Icon name="clock" /></span><div className="schedule-body"><h3>{queues.data?.find((queue) => queue.id === schedule.queue_id)?.name ?? schedule.queue_id}</h3><strong className="schedule-time">{schedule.start_local_time} <span>to</span> {schedule.end_local_time}{schedule.end_local_time < schedule.start_local_time && <small> next day</small>}</strong><p>{formatDays(schedule.days_mask)} · {afterLabels[schedule.action_after] ?? schedule.action_after}</p></div><div className="schedule-controls"><label className="check-field"><input type="checkbox" role="switch" checked={schedule.enabled} disabled={busy} aria-label={`Enable schedule ${schedule.start_local_time} for ${schedule.queue_id}`} onChange={() => { if (!schedule.enabled && schedule.action_after === "shutdown") { setError(null); setEnabling(schedule); } else void toggle(schedule); }} /><span>{schedule.enabled ? "Enabled" : "Disabled"}</span></label><button className="icon-button" disabled={busy} aria-label={`Edit schedule ${schedule.start_local_time} for ${schedule.queue_id}`} onClick={() => setEditor({ schedule })}><Icon name="edit" size={17} /></button><button className="icon-button danger-text" disabled={busy} aria-label={`Delete schedule ${schedule.start_local_time} for ${schedule.queue_id}`} onClick={() => { setError(null); setDeleting(schedule); }}><Icon name="trash" size={17} /></button></div></article>)}</div>}
      {rows.length > 0 && <Pagination page={safePage} total={rows.length} onChange={setPage} />}
    </section>
    {editor && <ScheduleEditor schedule={editor.schedule} queues={queues.data ?? []} onSave={save} onClose={() => setEditor(null)} />}
    {deleting && <ConfirmDialog title="Delete this schedule?" confirmLabel="Delete schedule" danger busy={busy} error={error} onClose={() => { if (!busy) setDeleting(null); }} onConfirm={() => { void remove(); }}><p>Remove the {deleting.start_local_time}–{deleting.end_local_time} window? Tasks and files stay in place. Without this window the queue follows the core's remaining schedule rules.</p></ConfirmDialog>}
    {enabling && <ConfirmDialog title="Enable a shutdown schedule?" confirmLabel="Enable shutdown schedule" danger busy={busy} error={error} onClose={() => { if (!busy) setEnabling(null); }} onConfirm={() => { void toggle(enabling); }}><p>This schedule requests a system shutdown after its window. Save other work before enabling it. Actual power behavior is owned by the core.</p></ConfirmDialog>}
  </div>;
}
