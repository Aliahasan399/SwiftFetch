import { memo, useCallback, useState } from "react";
import { actionLabels, availableActions, normalizePriority, PRIORITY_MAX, PRIORITY_MIN } from "../actions";
import { errorGuidance, formatBytes, formatEta, formatNumber, formatSpeed, isActive } from "../format";
import type { ControlAction, TaskSummary } from "../types";
import { Icon, type IconName } from "./Icon";
import { ProgressBar } from "./ProgressBar";
import { StatusPill } from "./StatusPill";
import "./Task.css";

const actionIcons: Record<ControlAction, IconName> = {
  start: "play", pause: "pause", resume: "play", cancel: "cancel", retry: "retry",
  verify: "shield", open_file: "file", open_folder: "folder", remove_history: "trash",
  set_priority: "edit",
  overwrite: "file", rename: "edit", skip: "cancel",
};

interface Props {
  task: TaskSummary;
  onDetails: (id: string) => void;
  onAction: (task: TaskSummary, action: ControlAction) => void;
  onPriorityChange?: (id: string, priority: number) => void;
  busy: boolean;
  showPriority?: boolean;
  selected?: boolean;
  onSelect?: (id: string, selected: boolean) => void;
}

export const TaskRow = memo(function TaskRow({ task, onDetails, onAction, onPriorityChange, busy, showPriority = false, selected = false, onSelect }: Props) {
  const [pendingPriority, setPendingPriority] = useState<string>("");
  const [editingPriority, setEditingPriority] = useState(false);

  const commitPriority = useCallback(() => {
    if (!onPriorityChange || !editingPriority) return;
    const parsed = parseInt(pendingPriority, 10);
    if (Number.isNaN(parsed)) { setPendingPriority(String(task.priority)); setEditingPriority(false); return; }
    const clamped = normalizePriority(parsed, task.priority);
    setPendingPriority(String(clamped));
    onPriorityChange(task.id, clamped);
    setEditingPriority(false);
  }, [onPriorityChange, task.id, task.priority, pendingPriority, editingPriority]);

  const actions = availableActions(task.status).filter((action) => action !== "verify" && action !== "remove_history" && action !== "set_priority");
  return <tr className={selected ? "task-selected" : undefined}>
    {onSelect && <td className="selection-cell"><input type="checkbox" aria-label={`Select ${task.display_name}`} checked={selected} onChange={(event) => onSelect(task.id, event.target.checked)} /></td>}
    <td className="task-name-cell"><div className="task-name"><span className="file-icon"><Icon name="file" /></span><div className="task-name-text"><button className="text-button filename" title={task.display_name} onClick={() => onDetails(task.id)}>{task.display_name}</button><span className="muted">{formatBytes(task.total_bytes)} · {formatNumber(task.connections)} configured {task.connections === 1 ? "connection" : "connections"}</span></div></div>
      {task.error_code && <button className="task-error text-button" title={errorGuidance(task.error_code) ?? task.error_message ?? task.error_code} onClick={() => onDetails(task.id)}><Icon name="warning" size={14} />{task.error_code.replace(/_/g, " ").toLowerCase()}</button>}
    </td>
    <td className="task-progress-cell"><StatusPill status={task.status} /><ProgressBar received={task.bytes_received} total={task.total_bytes} status={task.status} label={`${task.display_name} progress`} showText={false} /><span className="muted">{formatBytes(task.bytes_received)} transferred</span></td>
    <td className="numeric"><strong>{isActive(task.status) ? formatSpeed(task.bytes_per_second) : "—"}</strong><span className="muted">{isActive(task.status) ? formatEta(task.eta_seconds) : "Not transferring"}</span></td>
    {showPriority && <td className="priority-cell">
      {editingPriority && onPriorityChange ? (
        <input
          className="priority-input priority-editing"
          autoFocus
          value={pendingPriority}
          onChange={(event) => setPendingPriority(event.target.value.replace(/[^0-9-]/g, ""))}
          onBlur={commitPriority}
          onKeyDown={(event) => { if (event.key === "Enter") commitPriority(); if (event.key === "Escape") { setPendingPriority(String(task.priority)); setEditingPriority(false); } }}
          aria-label={`Priority for ${task.display_name}; current ${task.priority}`}
          title={`Set priority (${PRIORITY_MIN}-${PRIORITY_MAX}). Current: ${task.priority}`}
        />
      ) : (
        <button
          className="text-button priority-button"
          onClick={() => { setPendingPriority(String(task.priority)); setEditingPriority(true); }}
          aria-label={`Edit priority for ${task.display_name}; current ${task.priority}`}
          title={`Set priority (${PRIORITY_MIN}-${PRIORITY_MAX}). Current: ${task.priority}`}
        >
          {task.priority}
        </button>
      )}
    </td>}
    <td><div className="task-actions">{actions.map((action) => <button key={action} className={`icon-button ${action === "cancel" ? "danger-text" : ""}`} title={actionLabels[action]} aria-label={`${actionLabels[action]} ${task.display_name}`} disabled={busy} onClick={() => onAction(task, action)}><Icon name={actionIcons[action]} size={17} /></button>)}<button className="icon-button" title="Task details" aria-label={`Details for ${task.display_name}`} onClick={() => onDetails(task.id)}><Icon name="chevron" size={17} /></button></div>{busy && <span className="muted" role="status">Waiting for core…</span>}</td>
  </tr>;
});
