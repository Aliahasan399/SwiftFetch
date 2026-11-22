import { useEffect, useRef, useState } from "react";
import { formatDate, formatNumber, isActive, isTerminal, statusLabels } from "./format";
import { applyDocumentLocale, LocaleProvider, normalizeLocale, translate, type Locale } from "./i18n";
import { api, asIpcError, hasDesktopCore, listenCore, subscribeCore, type IpcError } from "./ipc";
import { downloadStore, refreshTasks, useDownloads } from "./store";
import { useResource } from "./hooks";
import type { PendingConfirmation, PowerRequest, RecoveryEvent, Settings, ViewId } from "./types";
import { validateDownloadUrl } from "./format";
import { bootDesktop } from "./desktopBoot";
import { AddDownloadDialog } from "./components/AddDownloadDialog";
import { BrowserDownloadConfirmation } from "./components/BrowserDownloadConfirmation";
import { ErrorBanner, InfoBanner } from "./components/Feedback";
import { Icon, type IconName } from "./components/Icon";
import { TaskActionProvider } from "./components/TaskActionProvider";
import { TaskDetailsDialog } from "./components/TaskDetailsDialog";
import { CategoriesView } from "./views/CategoriesView";
import { DashboardView } from "./views/DashboardView";
import { DiagnosticsView } from "./views/DiagnosticsView";
import { HistoryView } from "./views/HistoryView";
import { QueueView } from "./views/QueueView";
import { SchedulerView } from "./views/SchedulerView";
import { SettingsView } from "./views/SettingsView";
import "./App.css";
import "./views/Views.css";

const navigation: { id: ViewId; titleKey: Parameters<typeof translate>[1]; descKey: Parameters<typeof translate>[1]; icon: IconName }[] = [
  { id: "dashboard", titleKey: "nav.dashboard", descKey: "nav.dashboard.desc", icon: "dashboard" },
  { id: "queue", titleKey: "nav.queue", descKey: "nav.queue.desc", icon: "queue" },
  { id: "history", titleKey: "nav.history", descKey: "nav.history.desc", icon: "history" },
  { id: "scheduler", titleKey: "nav.scheduler", descKey: "nav.scheduler.desc", icon: "clock" },
  { id: "categories", titleKey: "nav.categories", descKey: "nav.categories.desc", icon: "folder" },
  { id: "settings", titleKey: "nav.settings", descKey: "nav.settings.desc", icon: "settings" },
  { id: "diagnostics", titleKey: "nav.diagnostics", descKey: "nav.diagnostics.desc", icon: "activity" },
];

function Shell({ locale, onLocale }: { locale: Locale; onLocale: (locale: Locale) => void }) {
  const state = useDownloads();
  const [view, setView] = useState<ViewId>("dashboard");
  const [adding, setAdding] = useState(false);
  const [taskId, setTaskId] = useState<string | null>(null);
  const [online, setOnline] = useState(() => navigator.onLine);
  const [theme, setTheme] = useState("dark");
  const [initialUrl, setInitialUrl] = useState("");
  const [power, setPower] = useState<PowerRequest | null>(null);
  const [cancelBusy, setCancelBusy] = useState(false);
  const [cancelError, setCancelError] = useState<IpcError | null>(null);
  const addingRef = useRef(false);
  const [connectionAttempt, setConnectionAttempt] = useState(0);
  const [eventError, setEventError] = useState<IpcError | null>(null);
  const [recovery, setRecovery] = useState<RecoveryEvent | null>(null);
  const [pendingConfirmations, setPendingConfirmations] = useState<PendingConfirmation[]>([]);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const pendingRef = useRef<string | null>(null);
  const categories = useResource(api.listCategories);
  const queues = useResource(api.listQueues);
  const [announcement, setAnnouncement] = useState("");
  const [progressAnnouncement, setProgressAnnouncement] = useState("");
  const mainRef = useRef<HTMLElement>(null);
  const progressBucket = useRef(new Map<string, number>());
  const current = navigation.find((entry) => entry.id === view) ?? navigation[0];
  const openAdd = (url = "") => {
    if (addingRef.current || document.querySelector("dialog[open]")) return;
    addingRef.current = true; setInitialUrl(url); setAdding(true);
  };
  const closeAdd = () => { addingRef.current = false; setAdding(false); };
  const cancelShutdown = async () => {
    if (!power || cancelBusy) return;
    setCancelBusy(true); setCancelError(null);
    try { await api.cancelShutdown(power.request_id); setPower(null); }
    catch (cause) { setCancelError(asIpcError(cause)); }
    finally { setCancelBusy(false); }
  };
  const activeCount = state.tasks.filter((task) => isActive(task.status)).length;
  const queueCount = state.tasks.filter((task) => !isTerminal(task.status)).length;
  const historyCount = state.tasks.filter((task) => isTerminal(task.status)).length;

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let refreshTimer: ReturnType<typeof setTimeout> | undefined;
    let announceTimer: ReturnType<typeof setTimeout> | undefined;
    let progressAnnounceTimer: ReturnType<typeof setTimeout> | undefined;
    let updateCount = 0;
    let latestStatus = "";
    setEventError(null);
    function refreshSoon() {
      if (disposed || refreshTimer !== undefined) return;
      refreshTimer = setTimeout(() => { refreshTimer = undefined; if (!disposed) void refreshTasks().catch(() => undefined); }, 400);
    }
    async function connect() {
      try {
        const cleanup = await subscribeCore({
          download_progress: (event) => {
            if (disposed) return;
            if (!downloadStore.progress(event)) refreshSoon();
            // Announce coarse milestone changes only: progress events arrive up
            // to every 250 ms and a live region must not chatter on each one.
            // 100% is not completion — only the core's status event confirms it.
            const total = event.total_bytes;
            if (!total || total <= 0 || event.status !== "downloading") return;
            const percent = Math.min(100, Math.floor((event.received_bytes / total) * 100));
            const bucket = Math.floor(percent / 25);
            if (progressBucket.current.get(event.task_id) === bucket) return;
            progressBucket.current.set(event.task_id, bucket);
            if (bucket >= 4) return;
            if (progressAnnounceTimer === undefined) progressAnnounceTimer = setTimeout(() => {
              progressAnnounceTimer = undefined;
              if (!disposed) setProgressAnnouncement(`Transfer progress: ${percent}% for a task.`);
            }, 900);
          },
          download_status: (event) => {
            if (disposed) return;
            downloadStore.status(event); refreshSoon();
            if (isTerminal(event.status)) progressBucket.current.delete(event.task_id);
            updateCount += 1; latestStatus = statusLabels[event.status];
            if (announceTimer === undefined) announceTimer = setTimeout(() => {
              announceTimer = undefined;
              if (!disposed) setAnnouncement(`${formatNumber(updateCount)} task state ${updateCount === 1 ? "update" : "updates"}. Latest status: ${latestStatus}.`);
              updateCount = 0;
            }, 900);
          },
          queue_changed: () => { if (!disposed) { downloadStore.bumpCatalog(); refreshSoon(); } },
          recovery: (event) => { if (!disposed) { setRecovery(event); refreshSoon(); } },
        });
        if (disposed) cleanup(); else unlisten = cleanup;
      } catch (error) { if (!disposed) setEventError(asIpcError(error)); }
      if (!disposed) {
        void bootDesktop(refreshTasks, api.getSettings, settings => {
          setTheme(settings.theme); onLocale(normalizeLocale(settings.locale));
        }, api.reportReady, () => !disposed).catch(cause => { if (!disposed) setEventError(asIpcError(cause)); });
      }
    }
    void connect();
    const poll = setInterval(() => { if (!disposed && hasDesktopCore() && document.visibilityState === "visible") void refreshTasks().catch(() => undefined); }, 10000);
    const focus = () => { if (hasDesktopCore()) refreshSoon(); };
    window.addEventListener("focus", focus);
    return () => {
      disposed = true; unlisten?.();
      clearInterval(poll); clearTimeout(refreshTimer); clearTimeout(announceTimer); clearTimeout(progressAnnounceTimer);
      window.removeEventListener("focus", focus);
      downloadStore.stopProgress();
    };
  }, [connectionAttempt]);

  useEffect(() => {
    function update() { setOnline(navigator.onLine); }
    window.addEventListener("online", update); window.addEventListener("offline", update);
    return () => { window.removeEventListener("online", update); window.removeEventListener("offline", update); };
  }, []);

  useEffect(() => {
    const preference = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => { document.documentElement.dataset.theme = theme === "system" ? (preference.matches ? "dark" : "light") : theme === "light" ? "light" : "dark"; };
    apply(); preference.addEventListener("change", apply);
    return () => preference.removeEventListener("change", apply);
  }, [theme]);

  useEffect(() => { applyDocumentLocale(locale); }, [locale]);

  // Optional public events: subscriptions are independent of the four durable
  // download events so an unavailable event cannot disable progress updates.
  useEffect(() => {
    let disposed = false;
    const cleanups: Array<() => void> = [];
    void Promise.allSettled([
      listenCore("open_add_download", () => { if (!disposed) openAdd(); }),
      listenCore("clipboard_url", (event) => {
        if (disposed || validateDownloadUrl(event.url)) return;
        openAdd(event.url); // Always user-confirmed in the form; never auto-download.
      }),
      listenCore("shutdown_pending", (event) => { if (!disposed) setPower(event); }),
      listenCore("shutdown_cancelled", () => { if (!disposed) setPower(null); }),
    ]).then((results) => {
      for (const result of results) if (result.status === "fulfilled") cleanups.push(result.value);
      if (disposed) cleanups.forEach((stop) => stop());
    });
    void api.getPowerRequest().then((request) => { if (!disposed && request) setPower(request); }).catch(() => undefined);
    return () => { disposed = true; cleanups.forEach((stop) => stop()); };
  }, []);

  useEffect(() => { pendingRef.current = pendingId; }, [pendingId]);

  // Browser handoffs can arrive while the app is closed. Pending confirmations
  // are persisted by the native host and picked up on the next launch.
  useEffect(() => {
    if (!hasDesktopCore()) return;
    let disposed = false;
    async function poll() {
      try {
        const rows = await api.listPendingConfirmations();
        if (disposed) return;
        setPendingConfirmations(rows);
        if (rows.length > 0 && !pendingRef.current && !document.querySelector("dialog[open]")) {
          setPendingId(rows[0].id);
        }
      } catch {
        // The core may still be starting; the next poll retries.
      }
    }
    void poll();
    const timer = window.setInterval(() => { void poll(); }, 2500);
    return () => { disposed = true; window.clearInterval(timer); };
  }, []);

  useEffect(() => {
    function over(event: DragEvent) { if (event.dataTransfer?.types.includes("text/uri-list") || event.dataTransfer?.types.includes("text/plain")) event.preventDefault(); }
    function drop(event: DragEvent) {
      const text = event.dataTransfer?.getData("text/uri-list") || event.dataTransfer?.getData("text/plain") || "";
      const url = text.split(/\r?\n/).map((line) => line.trim()).find((line) => line && !line.startsWith("#") && validateDownloadUrl(line) === null);
      if (!url) return;
      event.preventDefault();
      openAdd(url);
    }
    window.addEventListener("dragover", over);
    window.addEventListener("drop", drop);
    return () => { window.removeEventListener("dragover", over); window.removeEventListener("drop", drop); };
  }, []);

  useEffect(() => {
    document.title = "SwiftFetch — Download manager";
    const favicon = document.querySelector<HTMLLinkElement>('link[rel="icon"]');
    if (favicon) favicon.href = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="8" fill="#163d3c"/><path d="m18 4-12 14h9l-1 10 12-16h-9z" fill="#78ebd5"/></svg>')}`;
    function shortcut(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "n" && !document.querySelector("dialog[open]")) { event.preventDefault(); openAdd(); }
    }
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  }, []);

  useEffect(() => { mainRef.current?.focus(); }, [view]);

  function settingsSaved(settings: Settings) { setTheme(settings.theme); onLocale(normalizeLocale(settings.locale)); downloadStore.bumpCatalog(); }

  return <div className="app-shell">
    <a href="#main-content" className="skip-link">{translate(locale, "app.skip")}</a>
    <aside className="sidebar"><div className="brand"><span className="brand-mark"><Icon name="bolt" size={25} /></span><div><strong>SwiftFetch</strong><span>DOWNLOAD MANAGER</span></div></div><div className="sidebar-label">WORKSPACE</div>
      <nav className="sidebar-nav" aria-label="Main navigation">{navigation.map((entry, index) => <button key={entry.id} className={`nav-item ${view === entry.id ? "nav-active" : ""} ${index === 5 ? "nav-separated" : ""}`} aria-current={view === entry.id ? "page" : undefined} onClick={() => setView(entry.id)}><Icon name={entry.icon} size={19} /><span>{translate(locale, entry.titleKey)}</span>{entry.id === "queue" && queueCount > 0 && <span className="nav-count">{formatNumber(queueCount)}</span>}{entry.id === "history" && historyCount > 0 && <span className="nav-count">{formatNumber(historyCount)}</span>}</button>)}</nav>
      <div className="sidebar-bottom"><div className="local-badge"><Icon name="shield" size={17} /><div><strong>{translate(locale, "app.localTitle")}</strong><span>{translate(locale, "app.localBody")}</span></div></div><p><span className={`connection-dot ${state.last_synced_at && !state.error ? "connected" : ""}`} aria-hidden="true" />{state.last_synced_at && !state.error ? translate(locale, "app.coreConnected") : hasDesktopCore() ? translate(locale, "app.connecting") : translate(locale, "app.preview")}</p></div>
    </aside>
    <div className="workspace"><header className="topbar"><div className="breadcrumb"><Icon name="folder" size={16} /><span>{translate(locale, "nav.workspace")}</span><span aria-hidden="true">/</span><strong>{translate(locale, current.titleKey)}</strong></div><div className="topbar-actions"><span className="session-status">{online ? `${activeCount} ${translate(locale, "app.active")}` : translate(locale, "app.offline")}</span><button className="icon-button" disabled={state.loading} aria-label={translate(locale, "app.refresh")} title={translate(locale, "app.refresh")} onClick={() => { void refreshTasks().catch(() => undefined); }}><Icon name="retry" size={19} className={state.loading ? "spinning" : undefined} /></button><button className="button button-primary" onClick={() => openAdd()} title={translate(locale, "app.add.shortcut")}><Icon name="plus" size={18} />{translate(locale, "app.add")}</button></div></header>
      <main id="main-content" ref={mainRef} tabIndex={-1} className="main-content"><div className="page-heading"><div><h1>{translate(locale, current.titleKey)}</h1><p>{translate(locale, current.descKey)}</p></div><span className="workspace-label">PERSONAL WORKSPACE</span></div>
        <div className="global-notices">{power && <div className="notice notice-info" role="status"><Icon name="warning" /><div><strong>{translate(locale, "power.pending")}</strong><p>Requested window ends {formatDate(power.deadline)}. Active transfers are allowed to finish before shutdown.</p><ErrorBanner error={cancelError} /></div><button className="button button-small" disabled={cancelBusy} onClick={() => { void cancelShutdown(); }}>{translate(locale, "power.cancel")}</button></div>}{!online && <InfoBanner warning><strong>{translate(locale, "banner.offline.title")}</strong><p>{translate(locale, "banner.offline.body")}</p></InfoBanner>}<ErrorBanner error={state.error} onRetry={() => setConnectionAttempt((attempt) => attempt + 1)} />{eventError && hasDesktopCore() && <ErrorBanner error={eventError} onRetry={() => setConnectionAttempt((attempt) => attempt + 1)} />}{eventError && state.last_synced_at && <InfoBanner>{translate(locale, "banner.events")}</InfoBanner>}{recovery && <div className="notice notice-info" role="status"><Icon name="retry" /><div><strong>{translate(locale, "banner.recovery")}</strong><p>{formatNumber(recovery.paused)} paused · {formatNumber(recovery.restarted)} restarted after the previous session.</p></div><button className="icon-button" aria-label="Dismiss recovery notice" onClick={() => setRecovery(null)}><Icon name="close" size={17} /></button></div>}</div>
        {view === "dashboard" && <DashboardView onDetails={setTaskId} onAdd={() => openAdd()} />}
        {view === "queue" && <QueueView onDetails={setTaskId} />}
        {view === "history" && <HistoryView onDetails={setTaskId} />}
        {view === "scheduler" && <SchedulerView />}
        {view === "categories" && <CategoriesView />}
        {view === "settings" && <SettingsView onSaved={settingsSaved} />}
        {view === "diagnostics" && <DiagnosticsView />}
      </main><footer className="workspace-footer"><span>SwiftFetch · {translate(locale, "app.footer")}</span><span>{state.last_synced_at ? `Snapshot: ${formatDate(state.last_synced_at)}` : "Waiting for a core snapshot"}</span></footer>
    </div>
    <div className="sr-only" role="status" aria-live="polite" aria-atomic="true">{announcement}</div>
    <div className="sr-only" role="status" aria-live="polite" aria-atomic="true">{progressAnnouncement}</div>
    {taskId && <TaskDetailsDialog key={taskId} taskId={taskId} onClose={() => setTaskId(null)} />}
    {pendingId && pendingConfirmations.some((row) => row.id === pendingId) && (
      <BrowserDownloadConfirmation
        key={pendingId}
        pending={pendingConfirmations.find((row) => row.id === pendingId)!}
        categories={categories.data || []}
        queues={queues.data || []}
        onClose={() => setPendingId(null)}
        onResolved={() => {
          const id = pendingId;
          setPendingConfirmations((rows) => rows.filter((row) => row.id !== id));
          setPendingId(null);
          void refreshTasks().catch(() => undefined);
        }}
      />
    )}
    {adding && <AddDownloadDialog initialUrl={initialUrl} onClose={closeAdd} />}
  </div>;
}

export default function App() {
  const [locale, setLocale] = useState<Locale>("en");
  return <LocaleProvider locale={locale}><TaskActionProvider><Shell locale={locale} onLocale={setLocale} /></TaskActionProvider></LocaleProvider>;
}

