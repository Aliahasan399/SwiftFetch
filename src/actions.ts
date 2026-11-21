import type { ControlAction, ControlRequest, DownloadStatus, TaskSummary } from "./types";

// The add dialog, the queue's inline editor and set_priority all use the same
// range as the core, with a default priority of 50.
export const PRIORITY_MIN = 0;
export const PRIORITY_MAX = 100;

export function normalizePriority(value: number, fallback = PRIORITY_MIN): number {
    if (!Number.isFinite(value)) return fallback;
    return Math.min(PRIORITY_MAX, Math.max(PRIORITY_MIN, Math.trunc(value)));
}

/** Build the exact control_download payload for a UI action. */
export function buildControlRequest(task: Pick<TaskSummary, "id" | "priority">, action: ControlAction, priority?: number): ControlRequest {
    const request: ControlRequest = { task_id: task.id, action, delete_file: false };
    if (action === "set_priority") {
        request.priority = normalizePriority(priority ?? task.priority, task.priority);
    }
    return request;
}

export function availableActions(status: DownloadStatus): ControlAction[] {
  switch (status) {
    case "queued": return ["start", "pause", "cancel", "set_priority"];
    case "starting": case "downloading": return ["pause", "cancel", "set_priority"];
    case "retry_wait": return ["pause", "retry", "cancel", "set_priority"];
    case "paused": return ["resume", "cancel", "set_priority"];
    case "verifying": case "scanning": return ["cancel", "set_priority"];
    case "completed": return ["open_file", "open_folder", "verify", "remove_history", "set_priority"];
    case "failed": case "cancelled": return ["retry", "open_folder", "remove_history", "set_priority"];
    // Overwrite-conflict actions are gated by error_code in TaskActionProvider.
  }
}

export const actionLabels: Record<ControlAction, string> = {
  start: "Start", pause: "Pause", resume: "Resume", cancel: "Cancel", retry: "Retry",
  verify: "Verify checksum", open_file: "Open file", open_folder: "Show in folder", remove_history: "Remove from history",
  set_priority: "Set priority",
  overwrite: "Replace existing file",
  rename: "Use a new name",
  skip: "Skip this file",
};

export function isOverwriteConflict(code: string | null | undefined): boolean {
    return code === "DESTINATION_EXISTS" || code === "OVERWRITE_CONFIRMATION_REQUIRED";
}

export function needsRestartConfirmation(task: TaskSummary, action: ControlAction): boolean {
  return ["start", "resume", "retry"].includes(action) && (
    task.error_code === "RESUME_INCOMPATIBLE" || task.error_code === "RESUME_METADATA_INVALID" ||
    (!task.can_resume && task.bytes_received > 0)
  );
}
