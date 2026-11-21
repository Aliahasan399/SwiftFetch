import { useCallback, useState } from "react";
import { formatBytes, formatDate, formatNumber, redactSensitiveText } from "../format";
import { useResource } from "../hooks";
import { api } from "../ipc";
import { EmptyState } from "../components/EmptyState";
import { ErrorBanner, InfoBanner, LoadingState } from "../components/Feedback";
import { Icon } from "../components/Icon";
import { PAGE_SIZE, Pagination } from "../components/Pagination";
import { RecoveryPanel } from "../components/RecoveryPanel";
import { useTaskActions } from "../components/TaskActionProvider";

export function DiagnosticsView() {
  const resource = useResource(api.getDiagnostics);
  const loadDisk = useCallback(() => api.getDiskSpace(), []);
  const disk = useResource(loadDisk);
  const actions = useTaskActions();
  const [page, setPage] = useState(1);
  const [showBundle, setShowBundle] = useState(false);
  const data = resource.data;
  const events = data?.events ?? [];
  const safePage = Math.min(page, Math.max(1, Math.ceil(events.length / PAGE_SIZE)));
  const safeBundle = data ? JSON.stringify({ ...data, database_path: "[local database path omitted from copied bundle]", events: data.events.map((event) => ({ ...event, event_type: redactSensitiveText(event.event_type) })) }, null, 2) : "";
  return <div className="view-stack"><InfoBanner>Local diagnostics only. This view shows the core's redacted bundle; nothing is uploaded. Copying additionally omits the local database path. Review the preview before sharing.</InfoBanner><ErrorBanner error={resource.error} onRetry={() => { void resource.reload(); }} />
    <section className="panel"><header className="panel-header"><div><h2>Core diagnostics</h2><p>Recorded facts, not inferred health checks.</p></div><button className="button" disabled={resource.loading} onClick={() => { void resource.reload(); }}><Icon name="retry" size={17} />Refresh</button></header>
      {resource.loading && !data ? <LoadingState /> : data && <div className="diagnostics-summary"><dl className="detail-grid"><div><dt>Core version</dt><dd>{data.version}</dd></div><div><dt>Database size</dt><dd>{formatBytes(data.database_size_bytes)}</dd></div><div><dt>Tasks in database</dt><dd>{formatNumber(data.task_count)}</dd></div><div><dt>Active tasks</dt><dd>{formatNumber(data.active_count)}</dd></div></dl><dl className="detail-list"><div><dt>Database location (local view)</dt><dd className="mono break-anywhere">{data.database_path}</dd></div></dl><div className="detail-actions"><button className="button button-small" aria-expanded={showBundle} onClick={() => setShowBundle(!showBundle)}>{showBundle ? "Hide" : "Preview"} shareable bundle</button>{showBundle && <button className="button button-small" onClick={() => {
        if (!navigator.clipboard) { actions.notify("Clipboard unavailable. Select and copy the preview manually."); return; }
        void navigator.clipboard.writeText(safeBundle).then(() => actions.notify("Redacted diagnostic bundle copied. Nothing was uploaded.")).catch(() => actions.notify("Could not copy diagnostics. Select and copy the preview manually."));
      }}><Icon name="copy" size={16} />Copy preview</button>}</div>{showBundle && <pre className="diagnostic-json" tabIndex={0} aria-label="Shareable diagnostic JSON">{safeBundle}</pre>}</div>}
    </section>
    <section className="panel"><header className="panel-header"><div><h2>Destination disk space</h2><p>Reported by the core for the configured download folder.</p></div><button className="button" disabled={disk.loading} onClick={() => { void disk.reload(); }}><Icon name="retry" size={17} />Refresh</button></header>
      {disk.loading && !disk.data && <LoadingState />}
      <ErrorBanner error={disk.error} onRetry={() => { void disk.reload(); }} />
      {disk.data === null && !disk.loading && !disk.error && <InfoBanner>Disk space could not be read for the current destination. Check that the download folder points to an existing local drive; this is not a successful free-space check.</InfoBanner>}
      {disk.data && <div className="diagnostics-summary"><dl className="detail-grid"><div><dt>Available</dt><dd>{formatBytes(disk.data.available_bytes)}</dd></div><div><dt>Total capacity</dt><dd>{formatBytes(disk.data.total_bytes)}</dd></div></dl></div>}
    </section>
    <RecoveryPanel />
    <section className="panel"><header className="panel-header"><div><h2>Recent core events</h2><p>Only event type and timestamp are exposed. No task-level retry timeline is available.</p></div><span className="count-badge">{events.length}</span></header>{events.length ? <><div className="table-scroll" tabIndex={0} role="region" aria-label="Diagnostic events"><table className="simple-table"><thead><tr><th scope="col">Event type</th><th scope="col">Recorded at</th></tr></thead><tbody>{events.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE).map((event, index) => <tr key={`${safePage}-${index}`}><td className="mono">{redactSensitiveText(event.event_type)}</td><td>{formatDate(event.created_at)}</td></tr>)}</tbody></table></div><Pagination page={safePage} total={events.length} onChange={setPage} /></> : <EmptyState icon="activity" title={resource.error ? "Events unavailable" : "No diagnostic events"} description="Events will appear when they are returned by the core. Absence of events is not proof of system health." />}</section>
    <div className="panel support-notes"><h3>What this contract cannot diagnose</h3><p>Disk free space is reported separately by get_disk_space and may be unavailable for an invalid path. Antivirus readiness, browser-bridge connectivity and native power/startup integration are not returned by get_diagnostics. Their absence is not a successful check. For network or disk errors, inspect the affected task; for a missing bridge, use manual Add download.</p></div>
  </div>;
}
