import { useMemo } from "react";
import { formatBytes, formatEta, formatNumber, formatSpeed, isActive, isTerminal } from "../format";
import { useDownloads } from "../store";
import { Icon } from "../components/Icon";
import { LoadingState } from "../components/Feedback";
import { TaskLimitNotice } from "../components/TaskLimitNotice";
import { TaskTable } from "../components/TaskTable";

export function DashboardView({ onDetails, onAdd }: { onDetails: (id: string) => void; onAdd: () => void }) {
  const state = useDownloads();
  const stats = useMemo(() => {
    const active = state.tasks.filter((task) => isActive(task.status));
    const moving = state.tasks.filter((task) => isActive(task.status) || task.status === "retry_wait");
    const speed = active.reduce((sum, task) => sum + task.bytes_per_second, 0);
    const known = active.length > 0 && active.every((task) => task.eta_seconds !== null);
    return {
      active, moving, speed,
      eta: known ? Math.max(...active.map((task) => task.eta_seconds ?? 0)) : null,
      waiting: state.tasks.filter((task) => !isActive(task.status) && !isTerminal(task.status)).length,
      completed: state.tasks.filter((task) => task.status === "completed").length,
      transferred: state.tasks.reduce((sum, task) => sum + task.bytes_received, 0),
    };
  }, [state.tasks]);
  return <div className="view-stack">
    <section className="dashboard-hero" aria-label="Transfer overview"><div><span className="eyebrow">YOUR DOWNLOADS, UNDER CONTROL</span><h2>{stats.active.length ? "Making progress." : "Ready when you are."}</h2><p>Reliable transfers. Clear status. Everything stays on your device.</p><button className="button button-primary" onClick={onAdd}><Icon name="plus" size={18} />Add a download</button></div><div className="throughput"><span className="eyebrow"><Icon name="activity" size={16} />AGGREGATE THROUGHPUT</span><strong>{formatSpeed(stats.speed)}</strong><span className="muted">{state.last_synced_at ? "Reported by the core · active transfers only" : "Awaiting the desktop core"}</span></div></section>
    <div className="stat-grid"><section className="stat-card"><span><Icon name="download" />Active transfers</span><strong>{formatNumber(stats.active.length)}</strong><small>Currently occupying a queue slot</small></section><section className="stat-card"><span><Icon name="queue" />Waiting / paused</span><strong>{formatNumber(stats.waiting)}</strong><small>Ready to start, resume or retry</small></section><section className="stat-card"><span><Icon name="check" />Completed</span><strong>{formatNumber(stats.completed)}</strong><small>Confirmed by the core</small></section><section className="stat-card"><span><Icon name="clock" />Active transfer ETA</span><strong className="stat-text">{stats.active.length ? formatEta(stats.eta) : "—"}</strong><small>Longest estimate; excludes queued tasks</small></section></div>
    <TaskLimitNotice />
    <section className="panel"><header className="panel-header"><div><h2>In progress <span className="count-badge">{stats.moving.length}</span></h2><p>Live progress, including verification and retry waits.</p></div><span className="muted">{formatBytes(stats.transferred)} received across loaded tasks</span></header>
      {state.loading && !state.last_synced_at ? <LoadingState /> : <TaskTable tasks={stats.moving} onDetails={onDetails} emptyTitle={state.error ? "Core not connected" : "No active transfers"} emptyDescription={state.error ? "Reconnect the desktop core to load your existing downloads. This is not an empty-queue confirmation." : "Add a URL, or start a waiting task from the queue. Completed files appear in History."} caption="Active downloads" />}
    </section>
    <div className="quiet-note"><Icon name="shield" size={18} /><p>100% transferred is not the same as completed. SwiftFetch waits for the core to verify and finalize every file.</p></div>
  </div>;
}
