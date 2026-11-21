import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from "react";
import { actionLabels, availableActions, buildControlRequest, isOverwriteConflict, needsRestartConfirmation } from "../actions";
import { formatNumber } from "../format";
import { api, asIpcError, IpcError } from "../ipc";
import { downloadStore } from "../store";
import type { ControlAction, TaskSummary } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";
import { ErrorBanner } from "./Feedback";
import { Icon } from "./Icon";
import { PriorityDialog } from "./PriorityDialog";

interface Job { task: TaskSummary; action: ControlAction; priority?: number }
interface Confirmation { title: string; description: string; jobs: Job[]; danger: boolean }
interface Toast { id: number; message: string; error?: IpcError }
interface TaskFeedback { task_id: string; message: string; error?: IpcError }
interface TaskActionsContext {
  busyIds: ReadonlySet<string>;
  feedback: TaskFeedback | null;
  request: (task: TaskSummary, action: ControlAction) => void;
  requestBulk: (tasks: TaskSummary[], action: "pause" | "resume" | "cancel") => void;
  notify: (message: string, error?: IpcError) => void;
}

const Context = createContext<TaskActionsContext | null>(null);

export function useTaskActions() {
  const value = useContext(Context);
  if (!value) throw new Error("Task actions require TaskActionProvider.");
  return value;
}

export function TaskActionProvider({ children }: { children: ReactNode }) {
  const locked = useRef(new Set<string>());
  const [busyIds, setBusyIds] = useState<ReadonlySet<string>>(new Set());
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [confirmBusy, setConfirmBusy] = useState(false);
  const [priorityJob, setPriorityJob] = useState<Job | null>(null);
  const [priorityBusy, setPriorityBusy] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [feedback, setFeedback] = useState<TaskFeedback | null>(null);
  const nextToast = useRef(0);
  const notify = useCallback((message: string, error?: IpcError) => {
    setToasts((current) => [...current.slice(-3), { id: ++nextToast.current, message, error }]);
  }, []);

  async function run(jobs: Job[]) {
    const eligible = jobs.filter(({ task, action }) => {
      if (locked.current.has(task.id)) return false;
      if (["overwrite", "rename", "skip"].includes(action)) return task.status === "failed" && isOverwriteConflict(task.error_code);
      return availableActions(downloadStore.getSnapshot().tasks.find((entry) => entry.id === task.id)?.status ?? task.status).includes(action);
    });
    if (eligible.length === 0) { notify("No eligible tasks. Their state may have changed; refresh the list."); return; }
    eligible.forEach(({ task }) => locked.current.add(task.id));
    setBusyIds(new Set(locked.current));
    const failures: IpcError[] = [];
    let succeeded = 0;
    let cursor = 0;
    const worker = async () => {
      while (cursor < eligible.length) {
        const { task, action, priority } = eligible[cursor++];
        try {
          await api.controlDownload(buildControlRequest(task, action, priority));
          if (action === "remove_history") {
            try {
              await api.getTask(task.id);
              throw new IpcError({ code: "READBACK_FAILED", message: "The core acknowledged removal, but the task is still present. Refresh before trying again.", retryable: true });
            } catch (error) {
              if (asIpcError(error).code !== "TASK_NOT_FOUND") throw error;
              downloadStore.remove(task.id);
            }
          } else {
            const revision = downloadStore.getRevision();
            const current = await api.getTask(task.id);
            downloadStore.upsert(current, revision);
          }
          succeeded += 1;
          setFeedback({ task_id: task.id, message: `${actionLabels[action]} acknowledged; task state read back from the core.` });
        } catch (error) {
          const failure = asIpcError(error);
          failures.push(failure);
          setFeedback({ task_id: task.id, message: `${actionLabels[action]} was not confirmed. Refresh before repeating the action.`, error: failure });
        } finally {
          locked.current.delete(task.id);
          setBusyIds(new Set(locked.current));
        }
      }
    };
    await Promise.all(Array.from({ length: Math.min(3, eligible.length) }, () => worker()));
    const label = eligible.length === 1 ? actionLabels[eligible[0].action] : "Bulk action";
    if (failures.length) notify(`${label}: ${formatNumber(succeeded)} confirmed, ${formatNumber(failures.length)} not confirmed. Refresh before repeating an unconfirmed action.`, failures[0]);
    else notify(`${label} request acknowledged for ${formatNumber(succeeded)} ${succeeded === 1 ? "task" : "tasks"}. State refreshed from the core.`);
  }

  function request(task: TaskSummary, action: ControlAction) {
    if (locked.current.has(task.id)) return;
    // set_priority needs a value; the queue's inline editor calls the command
    // directly, while every other surface uses this dialog.
    if (action === "set_priority") { setPriorityJob({ task, action }); return; }
    let description: string | null = null;
    if (action === "cancel") description = "Stop this transfer? This request does not delete the completed file. Retention of partial data follows your core settings.";
    if (action === "remove_history") description = "Remove this task's history record? The downloaded file will stay on disk. This cannot restore removed task metadata.";
    if (action === "open_file") description = "Open this downloaded file with its default Windows application? Only open files from sources you trust. A checksum is not a guarantee of safety, and scan results are not exposed by this core contract.";
    if (needsRestartConfirmation(task, action)) description = "Saved bytes may not be resumable or the server's copy has changed. The core may restart this download from zero. Continue only if you accept losing partial progress.";
    if (action === "overwrite") description = "Replace the existing file with this download. The previous file cannot be recovered from SwiftFetch. Continue only if you meant to overwrite it.";
    if (action === "rename") description = "Finalize this download under a new, non-colliding name. The existing file is left untouched.";
    if (action === "skip") description = "Skip this file and cancel the task. The verified working file is retained for recovery; no file is deleted.";
    if (description) {
      setConfirmation({ title: `${actionLabels[action]}?`, description, jobs: [{ task, action }], danger: ["cancel", "remove_history", "open_file"].includes(action) || needsRestartConfirmation(task, action) });
    } else {
      void run([{ task, action }]);
    }
  }

  function requestBulk(tasks: TaskSummary[], action: "pause" | "resume" | "cancel") {
    const jobs = tasks.filter((task) => !locked.current.has(task.id)).flatMap((task): Job[] => {
      const effective = action === "resume" && task.status === "queued" ? "start" : action;
      return availableActions(task.status).includes(effective) ? [{ task, action: effective }] : [];
    });
    if (!jobs.length) { notify("None of the selected tasks support this action in their current state."); return; }
    const restarts = jobs.some(({ task, action: effective }) => needsRestartConfirmation(task, effective));
    setConfirmation({
      title: `${actionLabels[action]} ${formatNumber(jobs.length)} selected tasks?`, jobs,
      danger: action === "cancel" || restarts,
      description: `${formatNumber(jobs.length)} eligible tasks will be updated. Other selected tasks are unchanged. ${action === "cancel" ? "No completed-file deletion is requested. Partial retention follows core settings." : "Task state changes only after the core responds."}${restarts ? " Some transfers cannot safely resume and may restart from zero, losing partial progress." : ""}`,
    });
  }

  return <Context.Provider value={{ busyIds, feedback, request, requestBulk, notify }}>
    {children}
    {priorityJob && <PriorityDialog displayName={priorityJob.task.display_name} current={priorityJob.task.priority} busy={priorityBusy}
      onClose={() => { if (!priorityBusy) setPriorityJob(null); }}
      onSubmit={(priority) => {
        if (priorityBusy) return;
        setPriorityBusy(true);
        void run([{ ...priorityJob, priority }]).finally(() => { setPriorityBusy(false); setPriorityJob(null); });
      }} />}
    <section className="toast-stack" aria-label="Action notifications">
      {toasts.map((toast) => <div className="toast" key={toast.id}><div><p role="status">{toast.message}</p>{toast.error && <ErrorBanner error={toast.error} />}</div><button className="icon-button" aria-label="Dismiss notification" onClick={() => setToasts((current) => current.filter((entry) => entry.id !== toast.id))}><Icon name="close" size={16} /></button></div>)}
    </section>
    {confirmation && <ConfirmDialog title={confirmation.title} confirmLabel={confirmation.jobs.length === 1 ? actionLabels[confirmation.jobs[0].action] : "Apply to selected tasks"} danger={confirmation.danger} busy={confirmBusy} onClose={() => { if (!confirmBusy) setConfirmation(null); }} onConfirm={() => {
      if (confirmBusy) return;
      setConfirmBusy(true);
      void run(confirmation.jobs).finally(() => { setConfirmBusy(false); setConfirmation(null); });
    }}><p>{confirmation.description}</p>{confirmation.jobs.length === 1 && <p className="break-anywhere"><strong>{confirmation.jobs[0].task.display_name}</strong></p>}</ConfirmDialog>}
  </Context.Provider>;
}
