# SwiftFetch — Implementation Contract

This file is the single source of truth for the split of work between the main
agent and the two subagents. **Do not deviate from the command names, event
names, or JSON field names.** They are a wire protocol.

- Product: SwiftFetch, an IDM-style Windows download manager.
- Stack: Tauri v2 + React 19 + TypeScript (Vite) frontend, Rust core.
- Spec: `E:\CloneIdm\IDM-DOWNLOAD-MANAGER-ARCHITECTURE.md` (read it before coding).

## 0. Absolute rules (apply to every agent)

1. **Never delete an existing file or directory.** No `rm`, `del`, `Remove-Item`,
   `git clean`, `git reset --hard`, or equivalent.
2. **Never rename or move an existing file.**
3. **Never overwrite an existing file without a timestamped backup** next to it
   (e.g. `foo.rs.bak-20261110-101500`). Exception: files you created yourself in
   this session.
4. **Stay inside `E:\CloneIdm\swiftfetch`.** Do not touch `E:\CloneIdm`'s docs
   or any other directory.
5. Do not print API keys, tokens, or credentials.
6. Do not commit, push, or publish anything.
7. Match the existing code style: 4-space indent Rust, serde snake_case JSON.

## 1. File ownership

| Agent | Owns | Must not edit |
| --- | --- | --- |
| Main | `tools/**`, `tests/**`, `CONTRACT.md`, integration | `src/**`, `src-tauri/src/**` |
| Subagent 1 (backend) | `src-tauri/src/engine.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`, `src-tauri/tests/**` | `src/**`, `tools/**`, `domain.rs`, `security.rs`, `store.rs`, `ratelimit.rs`, `settings.rs` |
| Subagent 2 (frontend) | `src/**` (all of it), `index.html`, `vite.config.ts`, `package.json` (only to add scripts/deps the UI needs) | `src-tauri/**`, `tools/**` |

Already written by the main agent — **read these first, do not rewrite them**:
- `src-tauri/src/domain.rs` — all shared types, status enum, error codes, segment planner
- `src-tauri/src/security.rs` — URL / redirect / path safety policy
- `src-tauri/src/store.rs` — SQLite schema + repository (`Store`)
- `src-tauri/src/ratelimit.rs` — `SpeedLimiter`
- `src-tauri/src/settings.rs` — `Settings` struct + defaults

If one of those files genuinely blocks you, make the smallest possible addition
and report it in your final message. Do not restructure them.

## 2. IPC contract

### Commands (Rust `#[tauri::command]` → TS `invoke`)

All commands return `CommandResult<T>`:

```ts
interface CommandResult<T> {
  ok: boolean;
  data?: T;
  error?: { code: string; message: string; retryable: boolean; user_action?: string };
}
```

| Command | Request | Response `data` |
| --- | --- | --- |
| `add_download` | `{ req: AddDownloadRequest }` | `DownloadProjection` |
| `control_download` | `{ req: ControlRequest }` | `ControlResult` |
| `list_tasks` | `{ limit: number; cursor?: string; status?: DownloadStatus }` | `TaskListResponse` |
| `get_task` | `{ taskId: string }` | `DownloadProjection` |
| `get_settings` | `{}` | `Settings` |
| `save_settings` | `{ settings: Settings }` | `null` |
| `list_categories` | `{}` | `CategoryDto[]` |
| `save_category` | `{ cat: CategoryDto }` | `null` |
| `list_queues` | `{}` | `QueueDto[]` |
| `list_schedules` | `{}` | `ScheduleDto[]` |
| `save_schedule` | `{ schedule: ScheduleDto }` | `null` |
| `delete_schedule` | `{ id: string }` | `null` |
| `get_diagnostics` | `{}` | `Diagnostics` |
| `clear_history` | `{}` | `null` |
| `get_task_segments` | `{ taskId: string }` | Segment rows: IDs, index, offsets, bytes, status |
| `read_clipboard_url` | `{}` | `string \| null` |
| `report_ready` | `{}` | `null` |
| `get_power_request` | `{}` | `{ request_id: string; deadline: string } \| null` |
| `cancel_shutdown` | `{ requestId: string }` | `null` |
| `get_update_state` | `{}` | `UpdateState` |
| `check_for_updates` | `{}` | `UpdateState` |
| `install_update` | `{}` | `UpdateState` |
| `rollback_update` | `{}` | `UpdateState` |
| `export_queue` | `{}` | Redacted metadata inventory JSON string |
| `backup_metadata` | `{}` | Private consistent SQLite backup path |
| `import_queue` | `{ metadata: string }` | Imported row count; unfinished tasks remain paused/manual |
| `get_recovery_state` | `{}` | `{ message: string \| null, data_directory: string, backups: { name: string, size_bytes: number }[] }` |
| `restore_metadata` | `{ backupName: string, confirm: boolean }` | `{ restored_tasks: number, safety_backup: string, restart_required: boolean }` |
| `restart_application` | `{}` | Pauses work and restarts the application |
| `reset_startup_network` | `{ confirm: boolean }` | Clears only the saved proxy setting |
| `get_disk_space` | `{ path?: string }` | `{ available_bytes: number, total_bytes: number } \| null` |
| `encrypt_saved_secrets` | `{ confirm: boolean }` | `{ enabled: boolean; rows_rewritten: number }` |

Tauri names top-level arguments in camelCase. Serde uses snake_case inside the
request structs and response bodies. Build the next `list_tasks` cursor from
the last row's `${updated_at}|${id}` when `has_more` is `true`.

```ts
interface ControlResult { task_id: string; status: DownloadStatus; }
interface TaskListResponse { tasks: DownloadProjection[]; has_more: boolean; }

interface AddDownloadRequest {
  url: string;
  suggested_name?: string;
  destination_directory?: string;
  queue_id?: string;
  category_id?: string;
  priority?: number;
  start_policy?: string;       // "immediate" | "manual" | "scheduled"
  headers: { name: string; value: string }[];
  checksum?: { algorithm: string; value: string | null };
  connections?: number;
  speed_limit?: number;        // bytes/sec, 0 = unlimited
}

interface ControlRequest {
  task_id: string;
  action: string;              // start|pause|resume|cancel|retry|verify|open_file|open_folder|remove_history|set_priority|overwrite|rename|skip
  delete_file?: boolean;
  priority?: number;
}

interface CategoryDto {
  id: string; name: string; destination_template: string; extension_rules_json: string;
}
interface QueueDto { id: string; name: string; enabled: boolean; max_active_tasks: number; }
interface ScheduleDto {
  id: string; queue_id: string; days_mask: number;
  start_local_time: string; end_local_time: string;
  action_after: string; enabled: boolean;
}
interface Diagnostics {
  version: string;
  database_path: string;
  task_count: number;
  active_count: number;
  database_size_bytes: number;
  events: { event_type: string; created_at: string }[];
}
```

### Types mirrored from `domain.rs` (snake_case — serde does not rename)

```ts
type DownloadStatus =
  | "queued" | "starting" | "downloading" | "paused" | "retry_wait"
  | "verifying" | "scanning" | "completed" | "failed" | "cancelled";

interface DownloadProjection {
  id: string;
  display_name: string;
  status: DownloadStatus;
  bytes_received: number;
  total_bytes: number | null;
  bytes_per_second: number;
  eta_seconds: number | null;
  queue_id: string;
  category_id: string | null;
  priority: number;
  can_resume: boolean;
  error_code: string | null;
  error_message: string | null;
  original_url: string;
  final_url: string | null;
  destination_path: string;
  temporary_path: string | null;
  content_type: string | null;
  checksum_algorithm: string | null;
  checksum_expected: string | null;
  checksum_actual: string | null;
  checksum_verified: boolean;
  retry_count: number;
  max_retries: number;
  connections: number;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  segment_count: number;
  completed_segments: number;
}

interface Settings {
  destination_root: string;
  max_active_tasks: number;
  connections_per_task: number;
  global_speed_limit: number;
  default_task_speed_limit: number;
  max_retries: number;
  overwrite_policy: "rename" | "replace" | "skip" | "ask";
  verify_checksums: boolean;
  scan_with_antivirus: boolean;
  scanner_command: string;
  prevent_sleep: boolean;
  start_on_login: boolean;
  encrypt_saved_headers: boolean;
  theme: string;
  retention_days: number;
  browser_integration: boolean;
  proxy: string;
  scheduler_enabled: boolean;
  shutdown_after_complete: boolean;
  keep_partial_on_failure: boolean;
  locale: string;
  clipboard_capture: boolean;
}
```

### Events (Rust `app.emit(...)` → TS `listen`)

| Event | Payload |
| --- | --- |
| `download_status` | `{ task_id: string; status: DownloadStatus }` |
| `download_progress` | `{ task_id: string; status: DownloadStatus; received_bytes: number; total_bytes: number \| null; bytes_per_second: number; eta_seconds: number \| null; active_connections: number; segment_done: number; segment_total: number }` |
| `queue_changed` | `{}` |
| `recovery` | `{ paused: number; restarted: number }` |
| `clipboard_url` | `{ url: string }`; only after clipboard-capture opt-in |
| `open_add_download` | `{}`; tray user action |
| `shutdown_pending` | `{ request_id: string; deadline: string }` |
| `shutdown_cancelled` | payload ignored; dismiss the countdown |

`UpdateState` contains `configured`, `current_version`, nullable
`available_version` and `notes`, `phase`, `downloaded_bytes`, nullable
`total_bytes`, `rollback_available` and nullable `error`. Update checks report
`available` when installation can be confirmed. Unconfigured builds return
`not_configured` without network access. The client cannot supply a release key
or endpoint through IPC.

`restore_metadata` accepts only a selected SQLite filename within the local backup
folder and requires `confirm: true`. It validates a separate staged database and
preserves a safety copy before replacing metadata. Restored unfinished tasks stay
paused/manual; clipboard, browser, scanner, shutdown and schedule execution require
fresh opt-in. The startup recovery page exposes these recovery commands even when
the engine cannot initialize. See [docs/RECOVERY.md](docs/RECOVERY.md).

An Ask collision parks the task as `failed` with
`OVERWRITE_CONFIRMATION_REQUIRED`. Only that state accepts `overwrite`, `rename`
or `skip`. Overwrite and Rename publish the retained file without redownloading;
Skip cancels and removes only its task-owned working file.

Emit `download_progress` at most every 250 ms per task. Emit `download_status`
on every transition. Emit `queue_changed` after any queue mutation.

## 3. Subagent 1 — backend requirements

Implement `src-tauri/src/engine.rs` and `src-tauri/src/commands.rs`, and wire
them in `lib.rs` / `main.rs`.

### Engine behaviour (spec §7.4, §7.5, §7.6, §12)

- `Engine` holds: `reqwest::Client`, `Arc<Store>`, `tauri::AppHandle`,
  `Arc<Mutex<Settings>>`, `Arc<SpeedLimiter>` (global), and a map of
  `task_id -> TaskRuntime`.
- Probe with `GET` + `Range: bytes=0-0`. `206` ⇒ ranges supported, parse
  `Content-Range: bytes 0-0/TOTAL` for the size. `200` ⇒ single-stream, size from
  `Content-Length`. Anything else ⇒ error.
- Segment only when ranges are supported, the size is known and
  `>= 2 * MIN_SEGMENT_SIZE`, and `connections > 1`. Otherwise single stream.
  Use `domain::plan_segments`.
- **Never** treat a `200 OK` to a ranged request as a valid range response.
  Validate `Content-Range` against the requested offset.
- One writer task per download owns the `.part` file (`<destination>.part`).
  Segment workers send `{offset, bytes}` chunks over an mpsc channel; the writer
  seeks and writes. This keeps concurrent writes safe and gives one place to
  apply the speed limit.
- Persist segment checkpoints at most every 1 MiB or 2 s — not per packet.
- Pause = stop reading, keep the file, persist checkpoints. Resume = re-probe,
  compare `ETag`/`Last-Modified`/size; if they changed, clear segments and
  restart from zero (log a `resume_restarted` event).
- Retryable: timeout, connection reset, DNS transient, 408, 425, 429, 500, 502,
  503, 504. Not retryable: bad URL, bad scheme, 401/403, 404, 410, checksum
  mismatch, permission denied, disk full. Exponential backoff with jitter,
  honour `Retry-After` when valid and bounded.
- Finalise: verify size, verify checksum if `Checksum::is_configured()`, then
   task-owned working file → destination using the `OverwritePolicy`.
  `Rename` ⇒ publish a free sibling without replacing an existing file;
  `Replace` ⇒ explicit overwrite; `Ask`/`Skip` collisions fail safely and retain
  the existing file and new working file.
- Scheduler tick every 500 ms: start queued tasks by `priority DESC, created_at
  ASC` while `active < max_active_tasks` (global and per-queue). Honour
  `schedules` windows and `scheduler_enabled`.
- On startup call `store.reconcile_after_restart()` and emit `recovery`.
- Never auto-execute a downloaded file.

### Commands

Implement every command in the table above. `control_download` actions:
`start`, `pause`, `resume`, `cancel`, `retry`, `verify`, `open_file`,
`open_folder`, `remove_history`. `delete_file` must be an explicit, separate
command path with the target path returned in the result.

`lib.rs` must build the `Store` at `%APPDATA%/swiftfetch/state.sqlite` (fall back
to the executable directory if `APPDATA` is unset), open it once, wrap in
`Arc`, manage it as Tauri application state, register **all** commands with
`tauri::generate_handler!`, and install a tokio runtime for the engine.

### Definition of done for Subagent 1

- `cargo check` passes with no warnings that indicate real bugs.
- `cargo test --lib` passes.
- Report the exact commands you ran and their output.

## 4. Subagent 2 — frontend requirements

Build the whole UI in `src/`. Use only `react`, `react-dom`,
`@tauri-apps/api` (already installed). **Do not add UI frameworks** (no
Tailwind, no component library) — hand-written CSS in `src/App.css` plus
per-module CSS files. Dark theme by default.

### Modules

```
src/
  types.ts          exact mirrors of §2 types
  ipc.ts            invoke + listen wrappers, typed
  format.ts         bytes, dates, ETA, speeds
  store.ts          tiny external store (useSyncExternalStore) for projections
  App.tsx           shell: sidebar nav + view router + global event wiring
  components/       AddDownloadDialog, TaskRow, ProgressBar, StatusPill,
                    EmptyState, ConfirmDialog, SettingsForm, ...
  views/            DashboardView, QueueView, HistoryView, SchedulerView,
                    CategoriesView, SettingsView, DiagnosticsView
```

### Required behaviour

- Dashboard: active downloads with live progress, aggregate throughput, ETA.
- Add download dialog: URL, suggested name, destination, category, priority,
  connections, speed limit, optional checksum. Client-side URL validation.
- Queue: pause/resume/cancel/retry per task, bulk pause/resume/cancel,
  priority editing.
- Task details: segments, headers summary, retry history, checksum status.
- Scheduler: create/edit/delete schedules, enable/disable.
- Categories: name, destination template, extension rules.
- Settings: every field of `Settings`, persisted via `save_settings`.
- History: completed/failed/cancelled with search; clear history (never deletes
  files).
- Diagnostics: show the redacted bundle from `get_diagnostics`.
- Keyboard reachable, focus-trapped dialogs, accessible progress text, colour
  never the only state indicator, virtualized or paginated long lists.
- Handle these states explicitly: offline, disk full, ranges unsupported,
  resume metadata invalid, antivirus unavailable, browser bridge unavailable.
- Never show a task as completed before the core says so.

### Definition of done for Subagent 2

- `npm run build` (which runs `tsc && vite build`) passes.
- Report the exact command you ran and its output.

## 5. Verification (main agent)

- `cargo check`, `cargo test`
- `npm run build`
- `npm run tauri build` (installer)
- Fixture-server integration run
