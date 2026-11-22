//! Domain types for SwiftFetch.
//!
//! These types are the shared contract between the Rust core, the SQLite store
//! and the React frontend. Field names and variants are part of the wire
//! protocol — rename them only with a protocol version bump.

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

/// Wire protocol version. Bump when a contract field changes incompatibly.
pub const PROTOCOL: u32 = 1;

/// Default connection budget per eligible task (spec: 4–8, configurable).
pub const DEFAULT_CONNECTIONS_PER_TASK: u32 = 4;
/// Hard ceiling for connections per task regardless of user settings.
pub const MAX_CONNECTIONS_PER_TASK: u32 = 16;
/// Default number of simultaneously active tasks.
pub const DEFAULT_MAX_ACTIVE_TASKS: u32 = 3;
/// Hard ceiling for simultaneously active tasks.
pub const MAX_ACTIVE_TASKS: u32 = 16;
/// Default global speed limit in bytes/second (0 = unlimited).
pub const DEFAULT_GLOBAL_SPEED_LIMIT: u64 = 0;
/// Default per-task speed limit in bytes/second (0 = unlimited).
pub const DEFAULT_TASK_SPEED_LIMIT: u64 = 0;
/// Segment size floor — ranges smaller than this are merged.
pub const MIN_SEGMENT_SIZE: u64 = 64 * 1024;
/// Default destination directory name.
pub const DEFAULT_CATEGORY: &str = "Downloads";

/// DPAPI-backed protection for saved request headers at rest. The
/// implementation lives in `src-tauri/src/secrets.rs`; exposing it here keeps
/// the helper self-contained without requiring a change to the crate root.
/// Public API: `swiftfetch_lib::domain::secrets::{encrypt, decrypt,
/// is_envelope, is_available}`.
#[path = "secrets.rs"]
pub mod secrets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Queued,
    Starting,
    Downloading,
    Paused,
    RetryWait,
    Verifying,
    Scanning,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            DownloadStatus::Queued => "queued",
            DownloadStatus::Starting => "starting",
            DownloadStatus::Downloading => "downloading",
            DownloadStatus::Paused => "paused",
            DownloadStatus::RetryWait => "retry_wait",
            DownloadStatus::Verifying => "verifying",
            DownloadStatus::Scanning => "scanning",
            DownloadStatus::Completed => "completed",
            DownloadStatus::Failed => "failed",
            DownloadStatus::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "queued" => DownloadStatus::Queued,
            "starting" => DownloadStatus::Starting,
            "downloading" => DownloadStatus::Downloading,
            "paused" => DownloadStatus::Paused,
            "retry_wait" => DownloadStatus::RetryWait,
            "verifying" => DownloadStatus::Verifying,
            "scanning" => DownloadStatus::Scanning,
            "completed" => DownloadStatus::Completed,
            "failed" => DownloadStatus::Failed,
            "cancelled" => DownloadStatus::Cancelled,
            _ => return None,
        })
    }

    /// Terminal states never transition again without an explicit user action.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
        )
    }

    /// States that occupy a scheduler slot.
    pub fn is_active(self) -> bool {
        matches!(
            self,
            DownloadStatus::Starting
                | DownloadStatus::Downloading
                | DownloadStatus::Verifying
                | DownloadStatus::Scanning
        )
    }
}

impl std::fmt::Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Start policy for a newly added task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartPolicy {
    Immediate,
    Manual,
    Scheduled,
}

impl StartPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            StartPolicy::Immediate => "immediate",
            StartPolicy::Manual => "manual",
            StartPolicy::Scheduled => "scheduled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "immediate" => StartPolicy::Immediate,
            "manual" => StartPolicy::Manual,
            "scheduled" => StartPolicy::Scheduled,
            _ => return None,
        })
    }
}

/// What the scheduler should do when a task exhausts its retry budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryPolicy {
    Never,
    Always,
    OnlyNetworkErrors,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy::OnlyNetworkErrors
    }
}

/// Overwrite behaviour when the final path already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverwritePolicy {
    /// Rename the new file to a unique sibling name.
    #[default]
    Rename,
    /// Replace the existing file (requires explicit confirmation in the UI).
    Replace,
    /// Leave the existing file and mark the task completed without writing.
    Skip,
    /// Ask the user before finalising.
    Ask,
}

impl OverwritePolicy {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "rename" => OverwritePolicy::Rename,
            "replace" => OverwritePolicy::Replace,
            "skip" => OverwritePolicy::Skip,
            "ask" => OverwritePolicy::Ask,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            OverwritePolicy::Rename => "rename",
            OverwritePolicy::Replace => "replace",
            OverwritePolicy::Skip => "skip",
            OverwritePolicy::Ask => "ask",
        }
    }
}

/// Expected checksum for a download. Only verified when a value is supplied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Checksum {
    pub algorithm: String,
    pub value: Option<String>,
}

impl Checksum {
    pub fn is_configured(&self) -> bool {
        matches!(self.algorithm.as_str(), "sha256" | "sha1" | "md5" | "sha512")
            && self
                .value
                .as_deref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
    }
}

/// A single byte range assigned to one segment worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentPlan {
    pub index: u32,
    pub start: u64,
    pub end: u64,
}

impl SegmentPlan {
    pub fn len(&self) -> u64 {
        self.end.saturating_sub(self.start) + 1
    }
}

/// Split `total` bytes into at most `max_segments` contiguous ranges.
///
/// Ranges smaller than [`MIN_SEGMENT_SIZE`] are merged into their neighbour so
/// we never open a connection for a handful of kilobytes.
pub fn plan_segments(total: u64, max_segments: u32) -> Vec<SegmentPlan> {
    if total == 0 {
        return Vec::new();
    }
    let max_segments = max_segments.clamp(1, MAX_CONNECTIONS_PER_TASK);
    let mut count = max_segments as u64;
    // Do not create more segments than MIN_SEGMENT_SIZE allows.
    let max_by_size = (total / MIN_SEGMENT_SIZE).max(1);
    if count > max_by_size {
        count = max_by_size;
    }
    let base = total / count;
    let rem = total % count;
    let mut out = Vec::with_capacity(count as usize);
    let mut cursor = 0u64;
    for i in 0..count {
        let len = base + if i < rem { 1 } else { 0 };
        let start = cursor;
        let end = start + len - 1;
        out.push(SegmentPlan {
            index: i as u32,
            start,
            end,
        });
        cursor = end + 1;
    }
    out
}

/// Metadata captured from the probe request.
#[derive(Debug, Clone, Default)]
pub struct ResourceMetadata {
    pub final_url: String,
    pub total_bytes: Option<u64>,
    pub accept_ranges: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_type: Option<String>,
    pub content_disposition: Option<String>,
}

/// A persisted download task as exposed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProjection {
    pub id: String,
    pub display_name: String,
    pub status: DownloadStatus,
    pub bytes_received: u64,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: u64,
    pub eta_seconds: Option<u64>,
    pub queue_id: String,
    pub category_id: Option<String>,
    pub priority: i64,
    pub can_resume: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub original_url: String,
    pub final_url: Option<String>,
    pub destination_path: String,
    pub temporary_path: Option<String>,
    pub content_type: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub checksum_expected: Option<String>,
    pub checksum_actual: Option<String>,
    pub checksum_verified: bool,
    pub retry_count: i64,
    pub max_retries: i64,
    pub connections: u32,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub segment_count: u32,
    pub completed_segments: u32,
}

/// A browser-originated download waiting for explicit user confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingConfirmation {
    pub id: String,
    pub url: String,
    pub page_origin: String,
    pub suggested_name: Option<String>,
    pub content_type: String,
    pub size_hint: Option<u64>,
    pub mime: Option<String>,
    pub kind: String,
    pub site_rule: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Resolution of a pending confirmation from the desktop UI.
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveConfirmationRequest {
    pub id: String,
    /// `start`, `queue` or `cancel`.
    pub action: String,
    /// Optional validated variant URL selected from a media manifest.
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub suggested_name: Option<String>,
    #[serde(default)]
    pub destination_directory: Option<String>,
    #[serde(default)]
    pub category_id: Option<String>,
    #[serde(default)]
    pub queue_id: Option<String>,
}

/// Bounded metadata probe for the confirmation UI.
#[derive(Debug, Clone, Deserialize)]
pub struct ProbeRequest {
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetadataProbe {
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub suggested_name: Option<String>,
}

/// Payload for the `add_download` command.
#[derive(Debug, Clone, Deserialize)]
pub struct AddDownloadRequest {
    pub url: String,
    #[serde(default)]
    pub suggested_name: Option<String>,
    #[serde(default)]
    pub destination_directory: Option<String>,
    #[serde(default)]
    pub queue_id: Option<String>,
    #[serde(default)]
    pub category_id: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub start_policy: Option<String>,
    #[serde(default)]
    pub headers: Vec<HeaderPair>,
    #[serde(default)]
    pub checksum: Option<Checksum>,
    #[serde(default)]
    pub connections: Option<u32>,
    #[serde(default)]
    pub speed_limit: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeaderPair {
    pub name: String,
    pub value: String,
}

/// Actions accepted by the `control_download` command.
#[derive(Debug, Clone, Deserialize)]
pub struct ControlRequest {
    pub task_id: String,
    /// start | pause | resume | cancel | retry | verify | open_file | open_folder | remove_history
    pub action: String,
    #[serde(default)]
    pub delete_file: bool,
    /// New priority, only used by the `set_priority` action.
    #[serde(default)]
    pub priority: Option<i64>,
}

/// Result of a control command.
#[derive(Debug, Clone, Serialize)]
pub struct ControlResult {
    pub task_id: String,
    pub status: DownloadStatus,
}

/// Result envelope shared by every command.
#[derive(Debug, Clone, Serialize)]
pub struct CommandResult<T> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorEnvelope>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_action: Option<String>,
}

impl<T> CommandResult<T> {
    pub fn success(data: T) -> Self {
        CommandResult {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(code: &str, message: impl Into<String>, retryable: bool) -> Self {
        CommandResult {
            ok: false,
            data: None,
            error: Some(ErrorEnvelope {
                code: code.to_string(),
                message: message.into(),
                retryable,
                user_action: None,
            }),
        }
    }

    pub fn with_action(mut self, action: &str) -> Self {
        if let Some(e) = self.error.as_mut() {
            e.user_action = Some(action.to_string());
        }
        self
    }

    pub fn is_ok(&self) -> bool {
        self.ok
    }

    pub fn is_err(&self) -> bool {
        !self.ok
    }

    /// Returns the payload, panicking when the command failed. Callers that
    /// have already branched on `ok` can use this safely.
    pub fn unwrap(self) -> T {
        assert!(self.ok, "command failed: {:?}", self.error);
        self.data.expect("successful command returned no data")
    }
}

/// Error codes. Stable strings — the UI branches on them.
pub mod codes {
    pub const INVALID_URL: &str = "INVALID_URL";
    pub const UNSUPPORTED_SCHEME: &str = "UNSUPPORTED_SCHEME";
    pub const URL_TOO_LONG: &str = "URL_TOO_LONG";
    pub const TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";
    pub const INVALID_ACTION: &str = "INVALID_ACTION";
    pub const INVALID_DESTINATION: &str = "INVALID_DESTINATION";
    pub const PATH_TRAVERSAL: &str = "PATH_TRAVERSAL";
    pub const RESERVED_NAME: &str = "RESERVED_NAME";
    pub const HTTP_ERROR: &str = "HTTP_ERROR";
    pub const NETWORK_ERROR: &str = "NETWORK_ERROR";
    pub const SERVER_DOES_NOT_SUPPORT_RANGES: &str = "SERVER_DOES_NOT_SUPPORT_RANGES";
    pub const CHECKSUM_MISMATCH: &str = "CHECKSUM_MISMATCH";
    pub const DISK_FULL: &str = "DISK_FULL";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const RESUME_INCOMPATIBLE: &str = "RESUME_INCOMPATIBLE";
    pub const RATE_LIMITED: &str = "RATE_LIMITED";
    pub const STORE_ERROR: &str = "STORE_ERROR";
    pub const SCANNER_FAILED: &str = "SCANNER_FAILED";
    /// A collision that needs an explicit user decision (Ask policy). The
    /// working file is retained; no bytes were lost.
    pub const DESTINATION_EXISTS: &str = "DESTINATION_EXISTS";
    /// Ask-policy collision awaiting an explicit `overwrite|rename|skip` action.
    pub const OVERWRITE_CONFIRMATION_REQUIRED: &str = "OVERWRITE_CONFIRMATION_REQUIRED";
    pub const INTERNAL: &str = "INTERNAL";
}

/// Format a byte count for display.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let exp = (bytes as f64).log(1024.0).floor() as usize;
    let exp = exp.min(UNITS.len() - 1);
    let value = bytes as f64 / 1024f64.powi(exp as i32);
    if exp == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", value, UNITS[exp])
    }
}

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

pub fn now_local() -> DateTime<Local> {
    Local::now()
}

pub fn to_rfc3339(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
