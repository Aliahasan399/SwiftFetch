import { useEffect, useMemo, useRef, useState } from "react";
import { isTerminal } from "../format";
import { api, asIpcError, type IpcError } from "../ipc";
import { refreshTasks, refreshTasksAfterMutation, loadMoreTasks, resetTaskFilter, useDownloads } from "../store";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { LoadingState } from "../components/Feedback";
import { Icon } from "../components/Icon";
import { TaskLimitNotice } from "../components/TaskLimitNotice";
import { TaskTable } from "../components/TaskTable";
import { useTaskActions } from "../components/TaskActionProvider";

export function HistoryView({ onDetails }: { onDetails: (id: string) => void }) {
  const state = useDownloads();
  const actions = useTaskActions();
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("");
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const clearing = useRef(false);
  const history = useMemo(() => state.tasks.filter((task) => isTerminal(task.status)), [state.tasks]);
  const filtered = useMemo(() => history.filter((task) => (!status || task.status === status) && task.display_name.toLocaleLowerCase().includes(query.toLocaleLowerCase())).sort((a, b) => (b.completed_at ?? b.updated_at).localeCompare(a.completed_at ?? a.updated_at)), [history, query, status]);

  useEffect(() => {
    // An explicit status (null = unfiltered) replaces the catalog view.
    void refreshTasks(status || null).catch(() => undefined);
  }, [status]);
  useEffect(() => () => { void resetTaskFilter().catch(() => undefined); }, []);
  async function clear() {
    if (clearing.current) return;
    clearing.current = true; setBusy(true); setError(null);
    try {
      await api.clearHistory();
      await refreshTasksAfterMutation();
      setConfirm(false);
      actions.notify("History-clear request acknowledged and the task list reloaded. No downloaded-file deletion was requested; new completions may appear immediately.");
    } catch (cause) { setError(asIpcError(cause)); }
    finally { clearing.current = false; setBusy(false); }
  }
  return <div className="view-stack"><TaskLimitNotice /><section className="panel"><header className="panel-header"><div><h2>Download history <span className="count-badge">{history.length}</span></h2><p>Completed, failed and cancelled tasks. Your files remain yours.</p></div><button className="button danger-text" disabled={history.length === 0 || state.error !== null} onClick={() => { setError(null); setConfirm(true); }}><Icon name="trash" size={17} />Clear history</button></header>
    <div className="list-toolbar"><label className="search-field"><Icon name="search" size={18} /><span className="sr-only">Search history filenames</span><input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search filenames…" /></label><label className="filter-field"><span className="sr-only">Filter history status</span><select value={status} onChange={(event) => setStatus(event.target.value)}><option value="">All history</option><option value="completed">Completed</option><option value="failed">Failed</option><option value="cancelled">Cancelled</option></select></label></div>
    {state.loading && !state.last_synced_at ? <LoadingState /> : <TaskTable key={`${status}/${query}`} tasks={filtered} onDetails={onDetails} caption="Download history" emptyTitle={state.error ? "History is unavailable" : "No matching history"} emptyDescription={state.error ? "Connect the desktop core to retrieve stored history." : "Finished downloads will appear here. Try clearing the search or filter."} hasMore={state.has_more} onLoadMore={loadMoreTasks} />}
  </section><p className="quiet-note"><Icon name="shield" size={18} />Clearing history only removes metadata. It never sends a file-delete request.</p>
    {confirm && <ConfirmDialog title="Clear all download history?" confirmLabel="Clear history, keep files" danger busy={busy} error={error} onClose={() => { if (!busy) setConfirm(false); }} onConfirm={() => { void clear(); }}><p>Remove all completed, failed and cancelled history records, including records outside the current search or loaded page?</p><p>Active and paused tasks remain. Downloaded files are not deleted. Removed history cannot be restored from this screen.</p></ConfirmDialog>}
  </div>;
}
