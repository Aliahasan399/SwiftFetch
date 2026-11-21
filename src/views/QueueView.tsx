import { useCallback, useMemo, useState } from "react";
import { actionLabels, availableActions } from "../actions";
import { isTerminal, statusLabels } from "../format";
import { useResource } from "../hooks";
import { api, asIpcError } from "../ipc";
import { useDownloads } from "../store";
import type { DownloadStatus } from "../types";
import { ErrorBanner, InfoBanner, LoadingState } from "../components/Feedback";
import { Icon } from "../components/Icon";
import { TaskLimitNotice } from "../components/TaskLimitNotice";
import { TaskTable } from "../components/TaskTable";
import { useTaskActions } from "../components/TaskActionProvider";

export function QueueView({ onDetails }: { onDetails: (id: string) => void }) {
  const state = useDownloads();
  const revision = state.catalog_revision;
  const loadQueues = useCallback(() => { void revision; return api.listQueues(); }, [revision]);
  const queues = useResource(loadQueues);
  const ctx = useTaskActions();
  const [query, setQuery] = useState("");
  const [queue, setQueue] = useState("");
  const [status, setStatus] = useState("");
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set());
  const [priorityEditing, setPriorityEditing] = useState<Record<string, boolean>>({});
  const tasks = useMemo(() => state.tasks.filter((task) => task.status !== "completed" && (!queue || task.queue_id === queue) && (!status || task.status === status) && task.display_name.toLocaleLowerCase().includes(query.toLocaleLowerCase())).sort((a, b) => b.priority - a.priority || a.created_at.localeCompare(b.created_at)), [state.tasks, queue, query, status]);

  const handlePriorityChange = useCallback(async (id: string, priority: number) => {
    setPriorityEditing((current) => ({ ...current, [id]: true }));
    try {
      await api.controlDownload({ task_id: id, action: "set_priority", priority, delete_file: false });
      await api.getTask(id);
      void queues.reload();
    } catch (error) {
      ctx.notify(`${actionLabels.set_priority} failed for task ${id}: ${asIpcError(error).message}`);
    } finally {
      setPriorityEditing((current) => { const next = { ...current }; delete next[id]; return next; });
    }
  }, [queues, ctx]);

  const tasksWithPriority = useMemo(() => tasks.map((task) => ({
    ...task,
    priorityEditing: !!priorityEditing[task.id],
  })), [tasks, priorityEditing]);
  const selectedTasks = tasks.filter((task) => selected.has(task.id));
  const statuses: DownloadStatus[] = ["queued", "starting", "downloading", "paused", "retry_wait", "verifying", "scanning", "failed", "cancelled"];
  function select(id: string, checked: boolean) {
    setSelected((current) => { const next = new Set(current); if (checked) next.add(id); else next.delete(id); return next; });
  }
  function clearSelection() { setSelected(new Set()); }
  return <div className="view-stack">
    <div className="queue-cards">{queues.data?.map((entry) => <button key={entry.id} className={`queue-card ${queue === entry.id ? "is-selected" : ""}`} aria-pressed={queue === entry.id} onClick={() => { setQueue(queue === entry.id ? "" : entry.id); clearSelection(); }}><Icon name="queue" /><div><strong>{entry.name}</strong><span>{entry.enabled ? "Enabled" : "Disabled"} · {entry.max_active_tasks} concurrent</span></div><span className="count-badge">{state.tasks.filter((task) => task.queue_id === entry.id && !isTerminal(task.status)).length}</span></button>)}</div>
    <ErrorBanner error={queues.error} onRetry={() => { void queues.reload(); }} />
    <InfoBanner>Priority is set when adding a task; click a task's priority in the queue to change it. Higher numbers run first.</InfoBanner>
    <TaskLimitNotice />
    <section className="panel"><header className="panel-header"><div><h2>Download queue</h2><p>Sorted by priority, then oldest first. Failed and cancelled tasks can be retried here.</p></div></header>
      <div className="list-toolbar"><label className="search-field"><Icon name="search" size={18} /><span className="sr-only">Search queue filenames</span><input type="search" value={query} onChange={(event) => { setQuery(event.target.value); clearSelection(); }} placeholder="Search filenames…" /></label><label className="filter-field"><span className="sr-only">Filter queue</span><select value={queue} onChange={(event) => { setQueue(event.target.value); clearSelection(); }}><option value="">All queues</option>{queues.data?.map((entry) => <option value={entry.id} key={entry.id}>{entry.name}</option>)}</select></label><label className="filter-field"><span className="sr-only">Filter task status</span><select value={status} onChange={(event) => { setStatus(event.target.value); clearSelection(); }}><option value="">All statuses</option>{statuses.map((entry) => <option key={entry} value={entry}>{statusLabels[entry]}</option>)}</select></label></div>
      <div className="bulk-toolbar"><span>{selectedTasks.length} selected in this filter</span><button className="button button-small" disabled={!selectedTasks.some((task) => availableActions(task.status).includes("pause") && !ctx.busyIds.has(task.id))} onClick={() => ctx.requestBulk(selectedTasks, "pause")}><Icon name="pause" size={15} />Pause selected</button><button className="button button-small" disabled={!selectedTasks.some((task) => ["paused", "queued"].includes(task.status) && !ctx.busyIds.has(task.id))} onClick={() => ctx.requestBulk(selectedTasks, "resume")}><Icon name="play" size={15} />Start / resume</button><button className="button button-small danger-text" disabled={!selectedTasks.some((task) => availableActions(task.status).includes("cancel") && !ctx.busyIds.has(task.id))} onClick={() => ctx.requestBulk(selectedTasks, "cancel")}><Icon name="cancel" size={15} />Cancel selected</button>{selectedTasks.length > 0 && <button className="text-button" onClick={clearSelection}>Clear selection</button>}</div>
      {state.loading && !state.last_synced_at ? <LoadingState /> : <TaskTable key={`${queue}/${status}/${query}`} tasks={tasksWithPriority} onDetails={onDetails} selected={selected} onSelect={select} showPriority onPriorityChange={handlePriorityChange} caption="Download queue" emptyTitle={state.error ? "Queue could not be loaded" : "No matching downloads"} emptyDescription={state.error ? "Reconnect the core to retrieve the queue. Your existing data has not been removed." : "Try a different search or filter, or add a new download."} />}
    </section>
  </div>;
}
