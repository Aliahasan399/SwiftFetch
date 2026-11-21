import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { redactSensitiveText } from "./format";
import type {
  AddDownloadRequest, CategoryDto, CommandResult, ControlRequest, ControlResult,
  CoreEvents, Diagnostics, DiskSpace, DownloadProjection, ErrorEnvelope, PowerRequest, QueueDto,
  RecoveryState, RestoreResult, ScheduleDto, SegmentRow, Settings, TaskListResponse, UpdateState,
} from "./types";

export { isTauri as hasDesktopCore };

export class IpcError extends Error {
  readonly code: string;
  readonly retryable: boolean;
  readonly user_action?: string;

  constructor(error: ErrorEnvelope) {
    super(redactSensitiveText(error.message));
    this.name = "IpcError";
    this.code = error.code;
    this.retryable = error.retryable;
    this.user_action = error.user_action;
  }
}

export function asIpcError(error: unknown): IpcError {
  if (error instanceof IpcError) return error;
  // Inspect transport failures only for classification, never display their raw contents.
  const message = typeof error === "string" ? error : error instanceof Error ? error.message : "";
  if (/command.*(?:not found|not registered|unknown)|unknown command/i.test(message)) {
    return new IpcError({ code: "COMMAND_UNAVAILABLE", message: "The desktop core does not implement this command yet.", retryable: false });
  }
  return new IpcError({ code: "IPC_ERROR", message: "The desktop core could not complete the request. Reconnect and try again.", retryable: true });
}

interface Commands {
  add_download: { request: AddDownloadRequest; response: DownloadProjection };
  control_download: { request: ControlRequest; response: ControlResult };
  list_tasks: { request: { limit: number; cursor?: string; status?: string }; response: TaskListResponse };
  get_task: { request: { taskId: string }; response: DownloadProjection };
  get_settings: { request: Record<string, never>; response: Settings };
  save_settings: { request: Settings; response: null };
  list_categories: { request: Record<string, never>; response: CategoryDto[] };
  save_category: { request: CategoryDto; response: null };
  list_queues: { request: Record<string, never>; response: QueueDto[] };
  list_schedules: { request: Record<string, never>; response: ScheduleDto[] };
  save_schedule: { request: ScheduleDto; response: null };
  delete_schedule: { request: { id: string }; response: null };
  get_diagnostics: { request: Record<string, never>; response: Diagnostics };
  clear_history: { request: Record<string, never>; response: null };
  // Optional lanes: a missing command is reported as COMMAND_UNAVAILABLE, which
  // each caller handles without blocking the core download workflow.
  read_clipboard_url: { request: Record<string, never>; response: string | null };
  report_ready: { request: Record<string, never>; response: null };
  get_power_request: { request: Record<string, never>; response: PowerRequest | null };
  cancel_shutdown: { request: { requestId: string }; response: null };
  get_task_segments: { request: { taskId: string }; response: SegmentRow[] };
  get_update_state: { request: Record<string, never>; response: UpdateState };
  check_for_updates: { request: Record<string, never>; response: UpdateState };
  install_update: { request: Record<string, never>; response: UpdateState };
  rollback_update: { request: Record<string, never>; response: UpdateState };
  export_queue: { request: Record<string, never>; response: string };
  backup_metadata: { request: Record<string, never>; response: string };
  import_queue: { request: { metadata: string }; response: number };
  get_recovery_state: { request: Record<string, never>; response: RecoveryState };
  // Tauri binds outer arguments in camelCase; `backup_name` becomes backupName.
  restore_metadata: { request: { backupName: string; confirm: boolean }; response: RestoreResult };
  restart_application: { request: Record<string, never>; response: null };
  get_disk_space: { request: { path?: string }; response: DiskSpace | null };
  encrypt_saved_secrets: { request: { confirm: boolean }; response: { enabled: boolean; rows_rewritten: number } };
}

/**
 * Tauri binds invoke arguments to Rust command parameters by name. Commands
 * that take one request struct therefore need that struct wrapped in an object
 * keyed by the Rust parameter name (`req`, `settings`, `cat`, `schedule`).
 */
async function command<K extends keyof Commands>(
  name: K,
  args: Record<string, unknown>,
): Promise<Commands[K]["response"]> {
  if (!isTauri()) {
    throw new IpcError({ code: "CORE_UNAVAILABLE", message: "Desktop core unavailable — no download has been started.", retryable: true });
  }
  try {
    const result = await invoke<CommandResult<Commands[K]["response"]>>(name, args);
    if (!result || typeof result.ok !== "boolean") {
      throw new IpcError({ code: "INVALID_RESPONSE", message: "The core returned an invalid command envelope.", retryable: false });
    }
    if (!result.ok) {
      throw new IpcError(result.error ?? { code: "INTERNAL", message: "The core rejected the request without an error description.", retryable: false });
    }
    if (!("data" in result)) {
      throw new IpcError({ code: "INVALID_RESPONSE", message: "The core response is missing its data field.", retryable: false });
    }
    return result.data as Commands[K]["response"];
  } catch (error) {
    throw asIpcError(error);
  }
}

export const api = {
  addDownload: (request: AddDownloadRequest) => command("add_download", { req: request }),
  controlDownload: (request: ControlRequest) =>
    command("control_download", { req: { ...request, delete_file: false } }),
  listTasks: (limit = 1000, cursor?: string, status?: string) => {
    const args: Record<string, unknown> = { limit };
    if (cursor !== undefined) args.cursor = cursor;
    if (status !== undefined) args.status = status;
    return command("list_tasks", args);
  },
  getTask: (taskId: string) => command("get_task", { taskId }),
  getSettings: () => command("get_settings", {}),
  saveSettings: (settings: Settings) => command("save_settings", { settings }),
  listCategories: () => command("list_categories", {}),
  saveCategory: (category: CategoryDto) => command("save_category", { cat: category }),
  listQueues: () => command("list_queues", {}),
  listSchedules: () => command("list_schedules", {}),
  saveSchedule: (schedule: ScheduleDto) => command("save_schedule", { schedule }),
  deleteSchedule: (id: string) => command("delete_schedule", { id }),
  getDiagnostics: () => command("get_diagnostics", {}),
  clearHistory: () => command("clear_history", {}),
  readClipboardUrl: () => command("read_clipboard_url", {}),
  reportReady: () => command("report_ready", {}),
  getPowerRequest: () => command("get_power_request", {}),
  cancelShutdown: (requestId: string) => command("cancel_shutdown", { requestId }),
  getTaskSegments: (taskId: string) => command("get_task_segments", { taskId }),
  getUpdateState: () => command("get_update_state", {}),
  checkForUpdates: () => command("check_for_updates", {}),
  installUpdate: () => command("install_update", {}),
  rollbackUpdate: () => command("rollback_update", {}),
  exportQueue: () => command("export_queue", {}),
  backupMetadata: () => command("backup_metadata", {}),
  importQueue: (metadata: string) => command("import_queue", { metadata }),
  getRecoveryState: () => command("get_recovery_state", {}),
  restoreMetadata: (backupName: string) => command("restore_metadata", { backupName, confirm: true }),
  restartApplication: () => command("restart_application", {}),
  getDiskSpace: (path?: string) => command("get_disk_space", path === undefined ? {} : { path }),
  encryptSavedSecrets: () => command("encrypt_saved_secrets", { confirm: true }),
};

export function listenCore<K extends keyof CoreEvents>(name: K, handler: (payload: CoreEvents[K]) => void): Promise<UnlistenFn> {
  if (!isTauri()) {
    return Promise.reject(new IpcError({ code: "CORE_UNAVAILABLE", message: "Live events require the desktop core.", retryable: true }));
  }
  return listen<CoreEvents[K]>(name, (event) => handler(event.payload));
}

// The 4 durable core events are subscribed together; tray/clipboard/power
// public events are subscribed individually by the shell so an unavailable
// event cannot take down the download event channel.
export type CoreHandlers = {
  download_progress: (payload: CoreEvents["download_progress"]) => void;
  download_status: (payload: CoreEvents["download_status"]) => void;
  queue_changed: (payload: CoreEvents["queue_changed"]) => void;
  recovery: (payload: CoreEvents["recovery"]) => void;
};

// Clean up successful listeners even if another registration fails part way through.
export async function subscribeCore(handlers: CoreHandlers): Promise<UnlistenFn> {
  const registrations = await Promise.allSettled([
    listenCore("download_progress", handlers.download_progress),
    listenCore("download_status", handlers.download_status),
    listenCore("queue_changed", handlers.queue_changed),
    listenCore("recovery", handlers.recovery),
  ]);
  const unlisteners = registrations.flatMap((entry) => entry.status === "fulfilled" ? [entry.value] : []);
  const failed = registrations.find((entry) => entry.status === "rejected");
  if (failed?.status === "rejected") {
    unlisteners.forEach((unlisten) => unlisten());
    throw asIpcError(failed.reason);
  }
  return () => unlisteners.forEach((unlisten) => unlisten());
}
