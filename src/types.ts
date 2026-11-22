// Wire types from CONTRACT.md §2, domain.rs and settings.rs. Keep snake_case.
export type DownloadStatus =
  | "queued"
  | "starting"
  | "downloading"
  | "paused"
  | "retry_wait"
  | "verifying"
  | "scanning"
  | "completed"
  | "failed"
  | "cancelled";

export interface ErrorEnvelope {
  code: string;
  message: string;
  retryable: boolean;
  user_action?: string;
}

export interface CommandResult<T> {
  ok: boolean;
  data?: T;
  error?: ErrorEnvelope;
}

export interface HeaderPair {
  name: string;
  value: string;
}

export interface Checksum {
  algorithm: string;
  value: string | null;
}

export interface AddDownloadRequest {
  url: string;
  suggested_name?: string;
  destination_directory?: string;
  queue_id?: string;
  category_id?: string;
  priority?: number;
  start_policy?: string;
  headers: HeaderPair[];
  checksum?: Checksum;
  connections?: number;
  speed_limit?: number;
}

export interface ControlRequest {
  task_id: string;
  action: string;
  delete_file?: boolean;
  priority?: number;
}

export interface ControlResult {
  task_id: string;
  status: DownloadStatus;
}

export interface DownloadProjection {
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

export interface CategoryDto {
  id: string;
  name: string;
  destination_template: string;
  extension_rules_json: string;
}

export interface QueueDto {
  id: string;
  name: string;
  enabled: boolean;
  max_active_tasks: number;
}

export interface ScheduleDto {
  id: string;
  queue_id: string;
  days_mask: number;
  start_local_time: string;
  end_local_time: string;
  action_after: string;
  enabled: boolean;
}

export interface TaskListResponse {
  tasks: DownloadProjection[];
  has_more: boolean;
}

export interface Diagnostics {
  version: string;
  database_path: string;
  task_count: number;
  active_count: number;
  database_size_bytes: number;
  events: { event_type: string; created_at: string }[];
}

export interface Settings {
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
  theme: string;
  retention_days: number;
  browser_integration: boolean;
  proxy: string;
  scheduler_enabled: boolean;
  shutdown_after_complete: boolean;
  keep_partial_on_failure: boolean;
  locale: string;
  clipboard_capture: boolean;
  encrypt_saved_headers: boolean;
}

export interface DownloadStatusEvent {
  task_id: string;
  status: DownloadStatus;
}

export interface DownloadProgressEvent {
  task_id: string;
  status: DownloadStatus;
  received_bytes: number;
  total_bytes: number | null;
  bytes_per_second: number;
  eta_seconds: number | null;
  active_connections: number;
  segment_done: number;
  segment_total: number;
}

export interface RecoveryEvent {
  paused: number;
  restarted: number;
}

/** Keyset page of segment rows from get_task_segments (snake_case DTO). */
export interface SegmentRow {
  id?: string;
  download_id?: string;
  segment_index: number;
  start_offset: number;
  end_offset: number;
  received_bytes: number;
  status: string;
}

/** get_power_request / shutdown_pending payload. */
export interface PowerRequest {
  request_id: string;
  deadline: string;
}

/**
 * Publisher-supplied updater state. `not_configured` is the honest default
 * when this build has no signing key or HTTPS endpoint; the UI never fetches
 * a remote URL itself.
 */
export interface UpdateState {
  configured: boolean;
  current_version: string;
  available_version: string | null;
  notes: string | null;
  phase: "idle" | "checking" | "available" | "downloading" | "ready" | "installing" | "failed" | "not_configured";
  downloaded_bytes: number;
  total_bytes: number | null;
  rollback_available: boolean;
  error: string | null;
}

export interface ClipboardUrlEvent {
  url: string;
}

/** Local queue backup list from get_recovery_state (snake_case DTO). */
export interface BackupInfo {
  name: string;
  size_bytes: number;
}

export interface RecoveryState {
  message: string | null;
  data_directory: string;
  backups: BackupInfo[];
}

/** Result of restore_metadata; the safety path must be shown to the user. */
export interface RestoreResult {
  restored_tasks: number;
  safety_backup: string;
  restart_required: boolean;
}

/** get_disk_space reports null when no valid local destination can be resolved. */
export interface DiskSpace {
  available_bytes: number;
  total_bytes: number;
}

export interface PendingConfirmation {
  id: string;
  url: string;
  page_origin: string;
  suggested_name: string | null;
  content_type: string;
  size_hint: number | null;
  mime: string | null;
  kind: string;
  site_rule: string;
  created_at: string;
  expires_at: string;
}

export interface MediaVariantDto {
  url: string;
  label: string;
  mime: string | null;
}

export interface MetadataProbe {
  size: number | null;
  content_type: string | null;
  suggested_name: string | null;
}

export interface CoreEvents {
  download_status: DownloadStatusEvent;
  download_progress: DownloadProgressEvent;
  queue_changed: Record<string, never>;
  recovery: RecoveryEvent;
  clipboard_url: ClipboardUrlEvent;
  shutdown_pending: PowerRequest;
  shutdown_cancelled: Record<string, never>;
  open_add_download: Record<string, never>;
}

// UI-only types do not change the wire protocol.
export type ControlAction =
  | "start" | "pause" | "resume" | "cancel" | "retry"
  | "verify" | "open_file" | "open_folder" | "remove_history"
  | "set_priority"
  // Overwrite-conflict resolution for failed tasks whose error_code is
  // OVERWRITE_CONFIRMATION_REQUIRED. The core accepts these action strings.
  | "overwrite" | "rename" | "skip";

export type ViewId =
  | "dashboard" | "queue" | "history" | "scheduler"
  | "categories" | "settings" | "diagnostics";

// URLs, full paths and checksum values live only in the open details dialog.
export type TaskSummary = Omit<DownloadProjection,
  | "original_url" | "final_url" | "destination_path" | "temporary_path"
  | "checksum_expected" | "checksum_actual"
>;
