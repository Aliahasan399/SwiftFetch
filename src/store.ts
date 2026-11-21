import { useSyncExternalStore } from "react";
import { api, asIpcError, type IpcError } from "./ipc";
import { isActive, isTerminal, redactSensitiveText } from "./format";
import type { DownloadProgressEvent, DownloadProjection, DownloadStatusEvent, TaskSummary, TaskListResponse } from "./types";

// The core clamps list_tasks.limit to 1..500. Keep the page size inside that
// bound; MAX_TASK_LIMIT is the cap on how many pages we retain in the UI.
export const INITIAL_TASK_LIMIT = 200;
export const MAX_PAGE_LIMIT = 500;
export const MAX_TASK_LIMIT = 10000;

export interface DownloadState {
  tasks: readonly TaskSummary[];
  active_connections: Readonly<Record<string, number>>;
  loading: boolean;
  error: IpcError | null;
  last_synced_at: number | null;
  limit: number;
  limit_reached: boolean;
  catalog_revision: number;
  cursor: string | null;
  has_more: boolean;
  status_filter: string | null;
}

export function toTaskSummary(task: DownloadProjection): TaskSummary {
  const {
    original_url: _original, final_url: _final, destination_path: _destination,
    temporary_path: _temporary, checksum_expected: _expected, checksum_actual: _actual,
    ...summary
  } = task;
  return { ...summary, error_message: summary.error_message ? redactSensitiveText(summary.error_message) : null };
}

export function createDownloadStore() {
  let state: DownloadState = {
    tasks: [], active_connections: {}, loading: true, error: null,
    last_synced_at: null, limit: INITIAL_TASK_LIMIT, limit_reached: false, catalog_revision: 0,
    cursor: null, has_more: false, status_filter: null,
  };
  let revision = 0;
  const taskRevisions = new Map<string, number>();
  const projectionRevisions = new Map<string, number>();
  const statusRevisions = new Map<string, number>();
  const progressRevisions = new Map<string, number>();
  const listeners = new Set<() => void>();
  const buffered = new Map<string, DownloadProgressEvent>();
  let timer: ReturnType<typeof setTimeout> | null = null;

  function publish(patch: Partial<DownloadState>) {
    state = { ...state, ...patch };
    listeners.forEach((listener) => listener());
  }

  function touch(id: string) {
    const at = ++revision;
    taskRevisions.set(id, at);
    return at;
  }

  function acceptRead(task: DownloadProjection, startedAt: number, current?: TaskSummary): TaskSummary | undefined {
    // A full projection/removal accepted after this read began wins over its stale response.
    if ((projectionRevisions.get(task.id) ?? 0) > startedAt) return current;
    const next = toTaskSummary(task);
    if (current) {
      const newerProgress = (progressRevisions.get(task.id) ?? 0) > startedAt;
      const newerStatus = (statusRevisions.get(task.id) ?? 0) > startedAt;
      if (newerProgress) {
        next.bytes_received = current.bytes_received;
        next.total_bytes = current.total_bytes;
        next.bytes_per_second = current.bytes_per_second;
        next.eta_seconds = current.eta_seconds;
        next.segment_count = current.segment_count;
        next.completed_segments = current.completed_segments;
      }
      if (newerProgress || newerStatus) next.status = current.status;
    }
    if (!isActive(next.status)) { next.bytes_per_second = 0; next.eta_seconds = null; }
    projectionRevisions.set(task.id, touch(task.id));
    return next;
  }

  function flushProgress() {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    if (buffered.size === 0) return;
    const connections = { ...state.active_connections };
    const tasks = state.tasks.map((task) => {
      const event = buffered.get(task.id);
      if (!event || (isTerminal(task.status) && event.status !== task.status)) return task;
      connections[task.id] = isActive(event.status) ? event.active_connections : 0;
      return {
        ...task, status: event.status, bytes_received: event.received_bytes,
        total_bytes: event.total_bytes, bytes_per_second: isActive(event.status) ? event.bytes_per_second : 0,
        eta_seconds: isActive(event.status) ? event.eta_seconds : null,
        completed_segments: event.segment_done, segment_count: event.segment_total,
      };
    });
    buffered.clear();
    publish({ tasks, active_connections: connections });
  }

  return {
    getSnapshot: () => state,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },
    getRevision: () => { flushProgress(); return revision; },
    setLoading: () => publish({ loading: true }),
    fail: (error: unknown) => publish({ loading: false, error: asIpcError(error) }),
    setLimit: (limit: number) => publish({ limit: Math.min(MAX_PAGE_LIMIT, Math.max(INITIAL_TASK_LIMIT, limit)) }),
    bumpCatalog: () => publish({ catalog_revision: state.catalog_revision + 1 }),
    replace(tasks: DownloadProjection[], startedAt: number) {
      flushProgress();
      const current = new Map(state.tasks.map((task) => [task.id, task]));
      const merged = new Map<string, TaskSummary>();
      for (const task of tasks) {
        const accepted = acceptRead(task, startedAt, current.get(task.id));
        if (accepted) merged.set(task.id, accepted);
      }
      for (const task of state.tasks) {
        if (merged.has(task.id)) continue;
        if ((taskRevisions.get(task.id) ?? 0) > startedAt) merged.set(task.id, task);
        else projectionRevisions.set(task.id, touch(task.id));
      }
      const connections: Record<string, number> = {};
      for (const [id, count] of Object.entries(state.active_connections)) {
        const task = merged.get(id);
        if (task) connections[id] = isActive(task.status) ? count : 0;
      }
      // Keep removal revisions so older in-flight get requests cannot resurrect a row.
      publish({
        tasks: [...merged.values()], active_connections: connections, loading: false,
        error: null, last_synced_at: Date.now(), limit_reached: false,
      });
    },
    // Merge a freshly read page without discarding rows loaded from earlier
    // pages. Used for routine refreshes and cursor appends; only an explicit
    // filter change uses replace().
    merge(tasks: DownloadProjection[], startedAt: number) {
      flushProgress();
      const current = new Map(state.tasks.map((task) => [task.id, task]));
      for (const task of tasks) {
        const accepted = acceptRead(task, startedAt, current.get(task.id));
        if (accepted) current.set(task.id, accepted);
      }
      publish({ tasks: [...current.values()], loading: false, error: null, last_synced_at: Date.now() });
    },
    upsert(task: DownloadProjection, startedAt = revision) {
      flushProgress();
      const current = state.tasks.find((entry) => entry.id === task.id);
      const summary = acceptRead(task, startedAt, current);
      if (!summary || summary === current) return;
      publish({
        tasks: current ? state.tasks.map((entry) => entry.id === task.id ? summary : entry) : [summary, ...state.tasks],
        active_connections: isActive(summary.status) ? state.active_connections : { ...state.active_connections, [task.id]: 0 },
      });
    },
    remove(id: string) {
      projectionRevisions.set(id, touch(id));
      buffered.delete(id);
      const connections = { ...state.active_connections };
      delete connections[id];
      publish({ tasks: state.tasks.filter((task) => task.id !== id), active_connections: connections });
    },
    status(event: DownloadStatusEvent) {
      buffered.delete(event.task_id);
      const known = state.tasks.some((task) => task.id === event.task_id);
      if (known) {
        statusRevisions.set(event.task_id, touch(event.task_id));
        publish({
          tasks: state.tasks.map((task) => task.id === event.task_id ? {
            ...task, status: event.status,
            bytes_per_second: isActive(event.status) ? task.bytes_per_second : 0,
            eta_seconds: isActive(event.status) ? task.eta_seconds : null,
          } : task),
          active_connections: { ...state.active_connections, [event.task_id]: 0 },
        });
      }
      return known;
    },
    progress(event: DownloadProgressEvent) {
      const task = state.tasks.find((entry) => entry.id === event.task_id);
      if (!task) return false;
      if (isTerminal(task.status) && event.status !== task.status) return true;
      progressRevisions.set(event.task_id, touch(event.task_id));
      buffered.set(event.task_id, event);
      if (timer === null) timer = setTimeout(flushProgress, 250);
      return true;
    },
    flushProgress,
    publish,
    stopProgress() {
      if (timer !== null) clearTimeout(timer);
      timer = null;
      buffered.clear();
    },
  };
}

export const downloadStore = createDownloadStore();

export function useDownloads(): DownloadState {
  return useSyncExternalStore(downloadStore.subscribe, downloadStore.getSnapshot, downloadStore.getSnapshot);
}

let refreshInFlight: Promise<void> | null = null;

function requestTasks(status: string | null, target: number): Promise<void> {
  if (refreshInFlight) return refreshInFlight;
  const revision = downloadStore.getRevision();
  const snapshot = downloadStore.getSnapshot();
  downloadStore.setLoading();
  refreshInFlight = (async () => {
      const tasks: DownloadProjection[] = [];
      let cursor: string | undefined;
      let hasMore = false;
      while (tasks.length < target) {
        const result: TaskListResponse = await api.listTasks(Math.min(snapshot.limit, target - tasks.length), cursor, status ?? undefined);
        tasks.push(...result.tasks);
        const last = result.tasks[result.tasks.length - 1];
        const next = last ? `${last.updated_at}|${last.id}` : undefined;
        hasMore = result.has_more;
        if (!next || next === cursor) { hasMore = false; break; }
        cursor = next;
        if (!hasMore) break;
      }
      // Refresh the complete loaded prefix, including its older pages. This
      // removes deleted history while retaining newer events through revisions.
      downloadStore.replace(tasks, revision);
      downloadStore.publish({
        cursor: cursor ?? null,
        has_more: hasMore,
        status_filter: status,
        limit_reached: hasMore,
      });
    })()
    .catch((error: unknown) => { downloadStore.fail(error); throw error; })
    .finally(() => { refreshInFlight = null; });
  return refreshInFlight;
}

/**
 * Re-read the loaded prefix from its first page using bounded keyset queries.
 * Passing an explicit status starts a new catalog view.
 */
export async function refreshTasks(status?: string | null): Promise<void> {
  if (refreshInFlight) {
    if (status === undefined) return refreshInFlight;
    await refreshInFlight.catch(() => undefined);
  }
  const snapshot = downloadStore.getSnapshot();
  const effectiveStatus = status === undefined ? snapshot.status_filter : status;
  const target = status !== undefined && status !== snapshot.status_filter
    ? snapshot.limit
    : Math.max(snapshot.limit, snapshot.tasks.length);
  return requestTasks(effectiveStatus, Math.min(target, MAX_TASK_LIMIT));
}

export async function refreshTasksAfterMutation(): Promise<void> {
  // An in-flight poll may predate the write and cannot serve as its read-back.
  if (refreshInFlight) await refreshInFlight.catch(() => undefined);
  await refreshTasks();
}

/** Extend the prefix using keyset pages within the core's per-query bound. */
export async function loadMoreTasks(): Promise<void> {
  if (refreshInFlight) await refreshInFlight.catch(() => undefined);
  const snapshot = downloadStore.getSnapshot();
  if (!snapshot.has_more || !snapshot.cursor || snapshot.tasks.length >= MAX_TASK_LIMIT) return;
  await requestTasks(snapshot.status_filter, Math.min(MAX_TASK_LIMIT, snapshot.tasks.length + snapshot.limit));
}

/** Restore the unfiltered catalog when a filtered view such as History unmounts. */
export async function resetTaskFilter(): Promise<void> {
  if (refreshInFlight) await refreshInFlight.catch(() => undefined);
  await refreshTasks(null);
}
