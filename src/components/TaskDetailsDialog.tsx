import { useCallback, useEffect, useState } from "react";
import { actionLabels, availableActions, isOverwriteConflict } from "../actions";
import { errorGuidance, formatBytes, formatDate, formatEta, formatNumber, formatSpeed, safeUrl } from "../format";
import { useResource } from "../hooks";
import { api, IpcError } from "../ipc";
import type { SegmentRow } from "../types";
import { downloadStore, useDownloads } from "../store";
import { Dialog } from "./Dialog";
import { ErrorBanner, InfoBanner, LoadingState } from "./Feedback";
import { Icon } from "./Icon";
import { ProgressBar } from "./ProgressBar";
import { StatusPill } from "./StatusPill";
import { useTaskActions } from "./TaskActionProvider";

export function TaskDetailsDialog({ taskId, onClose }: { taskId: string; onClose: () => void }) {
  const store = useDownloads();
  const summary = store.tasks.find((task) => task.id === taskId);
  const status = summary?.status;
  const load = useCallback(async () => {
    void status;
    const revision = downloadStore.getRevision();
    const task = await api.getTask(taskId);
    downloadStore.upsert(task, revision);
    return task;
  }, [taskId, status]);
  const resource = useResource(load);
  const actions = useTaskActions();
  const [segments, setSegments] = useState<SegmentRow[] | null>(null);
  useEffect(() => {
    let live = true;
    setSegments(null);
    api.getTaskSegments(taskId).then((rows) => { if (live) setSegments(rows); }).catch(() => { if (live) setSegments(null); });
    return () => { live = false; };
  }, [taskId]);
  const task = resource.data ? { ...resource.data, ...summary } : null;
  const guidance = errorGuidance(task?.error_code);
  const busy = actions.busyIds.has(taskId);

  return <Dialog title="Task details" description="Live state and metadata reported by the download core." onClose={onClose} wide>
    <div className="dialog-body form-stack">
      {resource.loading && !task && <LoadingState />}
      <ErrorBanner error={resource.error} onRetry={() => { void resource.reload(); }} />
      {task && resource.error?.code !== "TASK_NOT_FOUND" && <>
        <div className="detail-title"><div><h3 className="break-anywhere">{task.display_name}</h3><StatusPill status={task.status} /></div><button className="button button-small" title="Copy full filename" onClick={() => {
          void navigator.clipboard?.writeText(task.display_name).then(() => actions.notify("Filename copied.")).catch(() => actions.notify("Could not copy the filename.", new IpcError({ code: "CLIPBOARD_UNAVAILABLE", message: "Clipboard access is unavailable. Select and copy the filename manually.", retryable: false })));
          if (!navigator.clipboard) actions.notify("Clipboard access is unavailable. Select and copy the filename manually.");
        }}><Icon name="copy" size={16} />Copy filename</button></div>
        <ProgressBar received={task.bytes_received} total={task.total_bytes} status={task.status} label="Download progress" />
        {task.error_code && <InfoBanner warning><strong>{task.error_code}</strong><p>{guidance ?? task.error_message ?? "The core reported a task error."}</p>{task.error_code === "DISK_FULL" && <p>Estimated data still to write: {task.total_bytes === null ? "unknown" : formatBytes(Math.max(0, task.total_bytes - task.bytes_received))}. Actual free space is not reported.</p>}</InfoBanner>}
        <section className="detail-section"><h3>Transfer</h3><dl className="detail-grid">
          <div><dt>Speed</dt><dd>{formatSpeed(task.bytes_per_second)}</dd></div><div><dt>Estimated time remaining</dt><dd>{formatEta(task.eta_seconds)}</dd></div>
          <div><dt>Configured connections</dt><dd>{formatNumber(task.connections)}</dd></div><div><dt>Active connections</dt><dd>{store.active_connections[taskId] === undefined ? "Not reported yet" : formatNumber(store.active_connections[taskId])}</dd></div>
          <div><dt>Completed segments</dt><dd>{formatNumber(task.completed_segments)} / {formatNumber(task.segment_count)}</dd></div><div><dt>Resume support</dt><dd>{task.can_resume ? "Core reports resumable" : "Not currently resumable"}</dd></div>
          <div><dt>Queue / priority</dt><dd>{task.queue_id} / {formatNumber(task.priority)}</dd></div><div><dt>Category</dt><dd>{task.category_id ?? "Default / automatic"}</dd></div>
        </dl>{segments && segments.length > 0 && <div className="table-scroll" tabIndex={0} role="region" aria-label="Segment ranges"><table className="simple-table"><caption className="sr-only">Segment ranges</caption><thead><tr><th scope="col">#</th><th scope="col">Range</th><th scope="col">Received</th><th scope="col">Status</th></tr></thead><tbody>{segments.map((segment) => <tr key={segment.id ?? segment.segment_index}><td>{formatNumber(segment.segment_index)}</td><td className="mono">{formatBytes(segment.start_offset)}–{formatBytes(segment.end_offset)}</td><td>{formatBytes(segment.received_bytes)}</td><td>{segment.status}</td></tr>)}</tbody></table></div>}{segments === null && <p className="field-hint">Segment detail is not available from this core build.</p>}<p className="field-hint">A single segment does not by itself prove that ranges are unsupported.</p></section>
        <section className="detail-section"><h3>Location & response summary</h3><dl className="detail-list">
          <div><dt>Destination</dt><dd className="break-anywhere mono">{task.destination_path}</dd></div>
          <div><dt>Temporary file</dt><dd className="break-anywhere mono">{task.temporary_path ?? "None reported"}</dd></div>
          <div><dt>Original URL</dt><dd className="break-anywhere">{safeUrl(task.original_url)}</dd></div>
          <div><dt>Final URL</dt><dd className="break-anywhere">{safeUrl(task.final_url)}</dd></div>
          <div><dt>Content-Type</dt><dd>{task.content_type ?? "Not reported"}</dd></div><div><dt>Resource length</dt><dd>{formatBytes(task.total_bytes)}</dd></div>
          <div><dt>ETag</dt><dd className="mono break-anywhere">Not exposed by core contract</dd></div>
          <div><dt>Last-Modified</dt><dd className="mono break-anywhere">Not exposed by core contract</dd></div>
        </dl><p className="field-hint">URL credentials, queries and fragments are hidden. Raw response/request headers are not exposed; no header values are reconstructed.</p></section>
        <section className="detail-section"><h3>Integrity & scanning</h3><dl className="detail-list">
          <div><dt>Checksum status</dt><dd>{task.checksum_verified ? "Verified by core" : task.checksum_expected ? "Not verified" : "No expected checksum supplied"}</dd></div>
          <div><dt>Algorithm</dt><dd>{task.checksum_algorithm?.toUpperCase() ?? "None"}</dd></div>
          <div><dt>Expected checksum</dt><dd className="mono break-anywhere">{task.checksum_expected ?? "Not supplied"}</dd></div>
          <div><dt>Actual checksum</dt><dd className="mono break-anywhere">{task.checksum_actual ?? "Not computed"}</dd></div>
          <div><dt>Antivirus</dt><dd>{task.status === "scanning" ? "Scan in progress" : "Scan result is not exposed by the core contract"}</dd></div>
        </dl><InfoBanner>Completion is not a safety certificate. If antivirus is unavailable, check the scanner settings before opening a file. Files never open automatically.</InfoBanner></section>
        <section className="detail-section"><h3>Retry & lifecycle summary</h3><dl className="detail-grid">
          <div><dt>Retry count / maximum</dt><dd>{formatNumber(task.retry_count)} / {formatNumber(task.max_retries)}</dd></div><div><dt>Created</dt><dd>{formatDate(task.created_at)}</dd></div>
          <div><dt>Last updated</dt><dd>{formatDate(task.updated_at)}</dd></div><div><dt>Completed</dt><dd>{formatDate(task.completed_at)}</dd></div>
        </dl><p className="field-hint">Per-attempt timestamps and reasons are not in the task projection. Diagnostics contains only the core's redacted, uncorrelated event list.</p></section>
        {actions.feedback?.task_id === taskId && <div><p role="status" className="field-hint">{actions.feedback.message}</p><ErrorBanner error={actions.feedback.error ?? null} /></div>}
        {task.status === "failed" && isOverwriteConflict(task.error_code) && <InfoBanner warning><strong>Destination collision</strong><p>The new download is retained because the destination exists. Rename keeps both files; Replace needs confirmation. Skip cancels this task and removes only its new working file.</p><div className="topbar-actions">{(["rename", "overwrite", "skip"] as const).map((action) => <button className="button button-small" key={action} disabled={busy} onClick={() => actions.request(task, action)}>{actionLabels[action]}</button>)}</div></InfoBanner>}
        <div className="detail-actions">{availableActions(task.status).map((action) => <button className={`button button-small ${action === "remove_history" || action === "cancel" ? "danger-text" : ""}`} key={action} disabled={busy || (action === "verify" && !task.checksum_expected)} onClick={() => actions.request(task, action)}>{actionLabels[action]}</button>)}</div>
      </>}
    </div>
    <footer className="dialog-footer"><button className="button" disabled={resource.loading} onClick={() => { void resource.reload(); }}><Icon name="retry" size={16} />Refresh details</button><button className="button button-primary" onClick={onClose}>Done</button></footer>
  </Dialog>;
}
