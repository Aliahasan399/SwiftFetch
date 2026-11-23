//! Transfer engine: probing, range segmentation, the disk writer, the
//! scheduler, retry/backoff, checksum verification and atomic finalisation.
//!
//! Implements specification sections 7.4–7.7, 12 and 19.

use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use chrono::{Datelike, Local};
use futures_util::StreamExt;
use tauri::Emitter;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};

use crate::domain::{
    self, codes, CommandResult, ControlResult, DownloadStatus, HeaderPair,
    OverwritePolicy, SegmentPlan, StartPolicy,
};
use crate::ratelimit::SpeedLimiter;
use crate::security;
use crate::settings::Settings;
use crate::store::{SegmentRow, Store, TaskRow};

/// Bytes between fsyncs of the partial file.
const SYNC_EVERY: u64 = 4 * 1024 * 1024;
/// Maximum connection attempts for a single segment before giving up.
const MAX_SEGMENT_ATTEMPTS: u32 = 5;
/// Bounded simultaneous connections to one host across all tasks.
const MAX_CONNECTIONS_PER_HOST: usize = 16;
/// Ceiling for exponential backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// How often the scheduler re-evaluates the queue.
const SCHEDULER_TICK: Duration = Duration::from_millis(500);
/// How often progress events are emitted.
const PROGRESS_TICK: Duration = Duration::from_millis(250);
/// Sentinel end offset meaning "to the end of the resource".
const END_TO_EOF: u64 = u64::MAX;
/// Checkpoint segment progress at least every 1 MiB written (spec §12.2).
const CHECKPOINT_EVERY_BYTES: u64 = 1 * 1024 * 1024;
/// Default antivirus scanner timeout in seconds.
const DEFAULT_SCANNER_TIMEOUT: u64 = 120;
/// Hard cap on antivirus scanner timeout in seconds.
const MAX_SCANNER_TIMEOUT: u64 = 600;
/// How often maintenance (retention + bounded event log) runs.
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(3600);
/// Upper bound on locally retained diagnostic events.
const MAX_EVENT_ROWS: i64 = 2000;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EngineError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl EngineError {
    fn new(code: &'static str, message: impl Into<String>, retryable: bool) -> Self {
        EngineError {
            code,
            message: message.into(),
            retryable,
        }
    }

    fn network(e: impl std::fmt::Display) -> Self {
        // reqwest's Display includes the full request URL, which may contain
        // userinfo (user:pass@host) or a signed-token query string. Never let
        // that reach the error envelope, the UI or the event log (spec §18.4).
        EngineError::new(
            codes::NETWORK_ERROR,
            format!("Network error: {}", redact_network_error(&e.to_string())),
            true,
        )
    }

    /// A cancellation is reported by the worker through this error. It is not a
    /// failure: `run_task` must settle the task as `cancelled`, never `failed`.
    fn is_cancellation(&self) -> bool {
        self.code == codes::INVALID_ACTION && self.message == "Download was cancelled."
    }

    fn timeout() -> Self {
        EngineError::new(codes::NETWORK_ERROR, "Request timed out", true)
    }

    fn http(status: u16) -> Self {
        let retryable = matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504);
        EngineError::new(
            codes::HTTP_ERROR,
            format!("Server returned HTTP {status}"),
            retryable,
        )
    }

    fn ranges_unsupported() -> Self {
        EngineError::new(
            codes::SERVER_DOES_NOT_SUPPORT_RANGES,
            "The server does not support segmented downloading.",
            false,
        )
    }

    fn checksum(expected: &str, actual: &str) -> Self {
        EngineError::new(
            codes::CHECKSUM_MISMATCH,
            format!("Checksum mismatch: expected {expected}, got {actual}"),
            false,
        )
    }

    fn cancelled() -> Self {
        EngineError::new(
            codes::INVALID_ACTION,
            "Download was cancelled.",
            false,
        )
    }

    fn internal(msg: impl Into<String>) -> Self {
        EngineError::new(codes::INTERNAL, msg, false)
    }

    fn store(e: rusqlite::Error) -> Self {
        EngineError::new(codes::STORE_ERROR, format!("Database error: {e}"), true)
    }

    pub fn to_envelope(&self) -> crate::domain::ErrorEnvelope {
        crate::domain::ErrorEnvelope {
            code: self.code.to_string(),
            message: self.message.clone(),
            retryable: self.retryable,
            user_action: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Event payloads
// ---------------------------------------------------------------------------

// Events are emitted as JSON through the `EventSink` trait, so the payload
// shapes live at the emit sites rather than in dedicated structs.

// ---------------------------------------------------------------------------
impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for EngineError {}

// ---------------------------------------------------------------------------
// Runtime state for one active download
// ---------------------------------------------------------------------------

pub struct Runtime {
    pub cancel: AtomicBool,
    pub paused: AtomicBool,
    pub speed_limiter: Arc<SpeedLimiter>,
    pub received: AtomicU64,
    pub total: AtomicU64,
    pub active_connections: AtomicU64,
    pub segment_done: AtomicU64,
    pub segment_total: AtomicU64,
    pub started_at: Instant,
    pub last_received: AtomicU64,
    pub last_emit: Mutex<Instant>,
}

impl Runtime {
    fn new(speed_limit_bps: u64) -> Self {
        Runtime {
            cancel: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            speed_limiter: Arc::new(SpeedLimiter::new(speed_limit_bps)),
            received: AtomicU64::new(0),
            total: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            segment_done: AtomicU64::new(0),
            segment_total: AtomicU64::new(0),
            started_at: Instant::now(),
            last_received: AtomicU64::new(0),
            last_emit: Mutex::new(Instant::now()),
        }
    }

}

struct Chunk {
    offset: u64,
    data: Vec<u8>,
    barrier: Option<tokio::sync::oneshot::Sender<()>>,
}

async fn until_control<F: std::future::Future>(rt: &Runtime, operation: F) -> Result<F::Output, EngineError> {
    tokio::pin!(operation);
    loop {
        if rt.cancel.load(Ordering::SeqCst) { return Err(EngineError::cancelled()); }
        if rt.paused.load(Ordering::SeqCst) {
            return Err(EngineError::new(codes::INVALID_ACTION, "Download was paused.", false));
        }
        tokio::select! {
            result = &mut operation => return Ok(result),
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Where engine events go.
///
/// The application installs a sink that forwards to the Tauri event bus. Tests
/// install a recording sink, which keeps the engine free of any runtime
/// dependency and makes event emission assertable.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

/// Drops events. The default sink before one is attached.
pub struct NoopSink;

impl EventSink for NoopSink {
    fn emit(&self, _event: &str, _payload: serde_json::Value) {}
}

/// Forwards events to the Tauri event bus.
pub struct TauriSink<R: tauri::Runtime> {
    app: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> TauriSink<R> {
    pub fn new(app: tauri::AppHandle<R>) -> Self {
        TauriSink { app }
    }
}

impl<R: tauri::Runtime> EventSink for TauriSink<R> {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        let _ = self.app.emit(event, payload);
    }
}

/// Records events in memory. Used by tests and by any non-Tauri host.
#[derive(Default)]
pub struct RecordingSink {
    pub events: Mutex<Vec<(String, serde_json::Value)>>,
}

impl EventSink for RecordingSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Ok(mut guard) = self.events.lock() {
            guard.push((event.to_string(), payload));
        }
    }
}

impl RecordingSink {
    pub fn count(&self, event: &str) -> usize {
        self.events
            .lock()
            .map(|g| g.iter().filter(|(e, _)| e == event).count())
            .unwrap_or(0)
    }

    pub fn statuses(&self, task_id: &str) -> Vec<String> {
        self.events
            .lock()
            .map(|g| {
                g.iter()
                    .filter(|(e, _)| e == "download_status")
                    .filter_map(|(_, p)| {
                        let id = p.get("task_id")?.as_str()?;
                        if id == task_id {
                            p.get("status").and_then(|s| s.as_str()).map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

pub struct Engine {
    client: std::sync::RwLock<reqwest::Client>,
    store: Arc<Store>,
    sink: Arc<dyn EventSink>,
    settings: Arc<Mutex<Settings>>,
    global_limiter: Arc<SpeedLimiter>,
    tasks: Mutex<HashMap<String, Arc<Runtime>>>,
    /// Exposed to the main-owned power service; the engine never powers off.
    shutdown_requested: AtomicBool,
    shutdown_announced: AtomicBool,
    last_cleanup: Mutex<Instant>,
    /// Last observed in-window state per queue, used to detect a genuine
    /// window ending rather than firing on every tick or at startup.
    window_state: Mutex<HashMap<String, bool>>,
    host_limits: Mutex<HashMap<String, Arc<Semaphore>>>,
}

impl Engine {
    pub fn new(
        client: reqwest::Client,
        store: Arc<Store>,
        settings: Arc<Mutex<Settings>>,
        global_limiter: Arc<SpeedLimiter>,
    ) -> Self {
        Engine {
            client: std::sync::RwLock::new(client),
            store,
            sink: Arc::new(NoopSink),
            settings,
            global_limiter,
            tasks: Mutex::new(HashMap::new()),
            shutdown_requested: AtomicBool::new(false),
            shutdown_announced: AtomicBool::new(false),
            last_cleanup: Mutex::new(Instant::now()),
            window_state: Mutex::new(HashMap::new()),
            host_limits: Mutex::new(HashMap::new()),
        }
    }

    /// Attach the event sink. The application passes a Tauri-backed sink; tests
    /// pass a recording one.
    pub fn with_sink<S: EventSink>(mut self, sink: S) -> Self {
        self.sink = Arc::new(sink);
        self
    }

    /// Attach an already-shared sink. Use this when the caller needs to keep a
    /// handle on it (tests do, to assert on emitted events).
    pub fn with_shared_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.sink = sink;
        self
    }

    /// Attach a Tauri-backed event sink without making the engine generic over
    /// the runtime. The sink is type-erased, so the engine stays testable.
    pub fn with_tauri_sink<R: tauri::Runtime>(self, app: tauri::AppHandle<R>) -> Self {
        self.with_sink(TauriSink::new(app))
    }

    pub fn sink(&self) -> &Arc<dyn EventSink> {
        &self.sink
    }

    fn client(&self) -> reqwest::Client {
        self.client.read().expect("HTTP client poisoned").clone()
    }

    pub(crate) fn set_http_client(&self, client: reqwest::Client) {
        *self.client.write().expect("HTTP client poisoned") = client;
    }

    /// Stable budget key for a URL. Redirects are already scheme-validated;
    /// using scheme+host (not path) bounds pressure on one origin.
    pub fn host_key(url: &str) -> String {
        match url::Url::parse(url) {
            Ok(parsed) => format!(
                "{}://{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or("unknown")
            ),
            Err(_) => "invalid://unknown".to_string(),
        }
    }

    async fn host_permit(&self, url: &str) -> Result<OwnedSemaphorePermit, EngineError> {
        let key = Self::host_key(url);
        let semaphore = {
            let mut limits = self.host_limits.lock().expect("host limits poisoned");
            Arc::clone(
                limits
                    .entry(key)
                    .or_insert_with(|| Arc::new(Semaphore::new(MAX_CONNECTIONS_PER_HOST))),
            )
        };
        semaphore
            .acquire_owned()
            .await
            .map_err(|_| EngineError::internal("host connection budget closed"))
    }

    // -------------------------------------------------------------- events

    fn emit_status(&self, task_id: &str, status: DownloadStatus) {
        self.sink.emit(
            "download_status",
            serde_json::json!({
                "task_id": task_id,
                "status": status.as_str(),
            }),
        );
    }

    fn emit_queue_changed(&self) {
        self.sink.emit("queue_changed", serde_json::json!({}));
    }

    fn emit_progress(&self, task_id: &str, rt: &Runtime, status: DownloadStatus) {
        let received = rt.received.load(Ordering::Relaxed);
        let total = rt.total.load(Ordering::Relaxed);
        let last = rt.last_received.swap(received, Ordering::Relaxed);
        let now = Instant::now();
        let elapsed = {
            let mut guard = rt.last_emit.lock().expect("emit mutex poisoned");
            let d = now.duration_since(*guard);
            *guard = now;
            d
        }
        .as_secs_f64()
        .max(0.001);

        let delta = received.saturating_sub(last);
        let bps = (delta as f64 / elapsed) as u64;
        let eta = if bps == 0 || total == 0 {
            None
        } else {
            Some(total.saturating_sub(received) / bps)
        };

        let _ = self.sink.emit(
            "download_progress",
            serde_json::json!({
                "task_id": task_id,
                "status": status.as_str(),
                "received_bytes": received,
                "total_bytes": if total == 0 { serde_json::Value::Null } else { serde_json::json!(total) },
                "bytes_per_second": bps,
                "eta_seconds": eta,
                "active_connections": rt.active_connections.load(Ordering::Relaxed),
                "segment_done": rt.segment_done.load(Ordering::Relaxed),
                "segment_total": rt.segment_total.load(Ordering::Relaxed),
            }),
        );
    }

    fn log(&self, task_id: Option<&str>, event: &str, payload: serde_json::Value) {
        let _ = self.store.log_event(
            task_id,
            event,
            &payload.to_string(),
        );
    }

    // ----------------------------------------------------------- lifecycle

    /// Recover tasks that were mid-flight when the process died, then start the
    /// scheduler and the progress emitter.
    pub fn start(self: &Arc<Self>) {
        let paused = match self.store.reconcile_after_restart() {
            Ok(n) => n,
            Err(e) => {
                eprintln!("swiftfetch: restart reconciliation failed: {e}");
                0
            }
        };
        let _ = self.sink.emit(
            "recovery",
            serde_json::json!({ "paused": paused, "restarted": 0 }),
        );
        self.log(None, "app_started", serde_json::json!({ "recovered_tasks": paused }));

        let engine = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(SCHEDULER_TICK).await;
                if let Err(e) = engine.tick().await {
                    eprintln!("swiftfetch: scheduler tick error: {e}");
                }
            }
        });

        let engine = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(PROGRESS_TICK).await;
                let snapshot: Vec<(String, Arc<Runtime>)> = engine
                    .tasks
                    .lock()
                    .expect("task map poisoned")
                    .iter()
                    .map(|(k, v)| (k.clone(), Arc::clone(v)))
                    .collect();
                for (task_id, rt) in snapshot {
                    if rt.cancel.load(Ordering::SeqCst) {
                        continue;
                    }
                    let status = engine
                        .store
                        .get_task(&task_id)
                        .ok()
                        .flatten()
                        .map(|r| r.status_enum())
                        .unwrap_or(DownloadStatus::Queued);
                    engine.emit_progress(&task_id, &rt, status);
                }
            }
        });
    }

    /// Register and spawn a task.
    pub fn spawn_task(self: &Arc<Self>, task_id: &str) {
        {
            let mut tasks = self.tasks.lock().expect("task map poisoned");
            if let Some(existing) = tasks.get(task_id) {
                // Already running; just clear a stale pause flag.
                existing.paused.store(false, Ordering::SeqCst);
                return;
            }
            let speed_limit = self
                .store
                .get_task(task_id)
                .ok()
                .flatten()
                .map(|r| r.speed_limit.max(0) as u64)
                .or_else(|| {
                    let s = self.settings.lock().expect("settings poisoned");
                    Some(s.default_task_speed_limit)
                })
                .unwrap_or(0);
            let rt = Arc::new(Runtime::new(speed_limit));
            tasks.insert(task_id.to_string(), Arc::clone(&rt));
        };

        let engine = Arc::clone(self);
        let id = task_id.to_string();
        tauri::async_runtime::spawn(async move {
            Arc::clone(&engine).run_task(&id).await;
            engine.tasks.lock().expect("task map poisoned").remove(&id);
            engine.emit_queue_changed();
        });
    }

    /// Stop tracking a task without changing its persisted state.
    pub fn detach(&self, task_id: &str) {
        self.tasks.lock().expect("task map poisoned").remove(task_id);
    }

    pub fn is_running(&self, task_id: &str) -> bool {
        self.tasks
            .lock()
            .expect("task map poisoned")
            .contains_key(task_id)
    }

    pub fn set_speed_limit(&self, task_id: &str, limit_bps: u64) {
        if let Some(rt) = self.tasks.lock().expect("task map poisoned").get(task_id) {
            rt.speed_limiter.set_limit(limit_bps);
        }
    }

    // --------------------------------------------------------- power/quiesce

    /// True once the engine has asked the host to arrange a shutdown. The host
    /// owns the actual power action; this is only a request/state flag.
    pub fn power_ready(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    /// Cancel a pending shutdown request. Idempotent.
    pub fn cancel_shutdown(&self) {
        self.shutdown_requested.store(false, Ordering::SeqCst);
        self.shutdown_announced.store(false, Ordering::SeqCst);
        self.sink.emit(
            "shutdown_cancelled",
            serde_json::json!({}),
        );
    }

    /// Update the per-queue window state. Returns true only for a real
    /// in-window → out-of-window transition (never on startup or repeated ticks).
    fn track_window(&self, queue_id: &str, in_window: bool) -> bool {
        let mut state = self.window_state.lock().expect("window state poisoned");
        let previous = state.insert(queue_id.to_string(), in_window).unwrap_or(false);
        previous && !in_window
    }

    fn queue_shutdown_enabled(&self, queue_id: &str) -> bool {
        self.store
            .list_schedules()
            .map(|schedules| {
                schedules.iter().any(|s| {
                    s.queue_id == queue_id && s.enabled && s.action_after == "shutdown"
                })
            })
            .unwrap_or(false)
    }

    fn request_shutdown_once(&self, reason: &str) {
        if self.shutdown_requested.swap(true, Ordering::SeqCst) {
            return;
        }
        self.shutdown_announced.store(true, Ordering::SeqCst);
        // Payload is deliberately empty: the host owns the countdown UI and
        // the public `shutdown_pending` projection.
        self.sink.emit("shutdown_requested", serde_json::json!({}));
        self.log(None, "shutdown_requested", serde_json::json!({ "reason": reason }));
    }

    /// Quiesce all transfers for the updater or an orderly exit.
    ///
    /// Pauses every worker, persists `paused` for every runtime-owned task, then
    /// waits for the runtime map to drain. Returns `Err` on timeout so the
    /// caller can decide whether to proceed. No task is cancelled or deleted.
    pub async fn quiesce(&self) -> Result<(), EngineError> {
        // Freeze scheduler candidates as well as active runtimes; otherwise it
        // can admit the next queued task while this method drains the first.
        for row in self.store.list_tasks(100_000).map_err(EngineError::store)? {
            if row.status_enum() == DownloadStatus::Queued {
                self.store.set_task_status(&row.id, DownloadStatus::Paused).map_err(EngineError::store)?;
                self.emit_status(&row.id, DownloadStatus::Paused);
            }
        }
        let ids: Vec<String> = {
            let tasks = self.tasks.lock().expect("task map poisoned");
            for rt in tasks.values() {
                rt.paused.store(true, Ordering::SeqCst);
            }
            tasks.keys().cloned().collect()
        };
        for id in &ids {
            let _ = self.store.set_task_status(id, DownloadStatus::Paused);
            self.emit_status(id, DownloadStatus::Paused);
        }
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if self.tasks.lock().expect("task map poisoned").is_empty() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(EngineError::new(
                    codes::INTERNAL,
                    "Timed out waiting for active downloads to pause",
                    false,
                ));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Backwards-compatible alias for callers that do not need the error.
    pub async fn pause_all_and_wait(&self) {
        let _ = self.quiesce().await;
    }

    // ------------------------------------------------------------- control

    pub fn control(&self, task_id: &str, action: &str, req_priority: Option<i64>) -> CommandResult<ControlResult> {
        let row = match self.store.get_task(task_id) {
            Ok(Some(r)) => r,
            Ok(None) => {
                return CommandResult::failure(codes::TASK_NOT_FOUND, "No such download task", false)
            }
            Err(e) => return CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
        };

        let status = row.status_enum();
        let running = self.is_running(task_id);

        match action {
            "start" | "resume" => {
                if status.is_terminal() {
                    return CommandResult::failure(
                        codes::INVALID_ACTION,
                        "A finished task cannot be restarted; add it again",
                        false,
                    );
                }
                let _ = self.store.set_task_status(task_id, DownloadStatus::Queued);
                self.emit_status(task_id, DownloadStatus::Queued);
                // The scheduler starts the queued task under its budgets.
                CommandResult::success(ControlResult {
                    task_id: task_id.to_string(),
                    status: DownloadStatus::Queued,
                })
            }
            "pause" => {
                if !running {
                    let _ = self.store.set_task_status(task_id, DownloadStatus::Paused);
                    self.emit_status(task_id, DownloadStatus::Paused);
                } else if let Some(rt) = self.tasks.lock().expect("task map poisoned").get(task_id) {
                    rt.paused.store(true, Ordering::SeqCst);
                    let _ = self.store.set_task_status(task_id, DownloadStatus::Paused);
                    self.emit_status(task_id, DownloadStatus::Paused);
                }
                CommandResult::success(ControlResult {
                    task_id: task_id.to_string(),
                    status: DownloadStatus::Paused,
                })
            }
            "cancel" => {
                if let Some(rt) = self.tasks.lock().expect("task map poisoned").get(task_id) {
                    rt.cancel.store(true, Ordering::SeqCst);
                    rt.paused.store(false, Ordering::SeqCst);
                }
                let _ = self.store.set_task_status(task_id, DownloadStatus::Cancelled);
                self.emit_status(task_id, DownloadStatus::Cancelled);
                self.log(Some(task_id), "download_cancelled", serde_json::json!({}));
                CommandResult::success(ControlResult {
                    task_id: task_id.to_string(),
                    status: DownloadStatus::Cancelled,
                })
            }
            "retry" => {
                if let Some(rt) = self.tasks.lock().expect("task map poisoned").get(task_id) {
                    rt.cancel.store(true, Ordering::SeqCst);
                }
                let _ = self.store.set_retry_count(task_id, 0);
                let _ = self.store.set_task_status(task_id, DownloadStatus::Queued);
                self.emit_status(task_id, DownloadStatus::Queued);
                CommandResult::success(ControlResult {
                    task_id: task_id.to_string(),
                    status: DownloadStatus::Queued,
                })
            }
            "verify" => match self.reverify(task_id, &row) {
                Ok(actual) => {
                    self.log(
                        Some(task_id),
                        "verify_completed",
                        serde_json::json!({ "checksum_actual": actual }),
                    );
                    CommandResult::success(ControlResult {
                        task_id: task_id.to_string(),
                        status,
                    })
                }
                Err(e) => CommandResult {
                    ok: false,
                    data: None,
                    error: Some(e.to_envelope()),
                },
            },
            "open_file" | "open_folder" => {
                let path = if action == "open_file" {
                    row.destination_path.clone()
                } else {
                    Path::new(&row.destination_path)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| row.destination_path.clone())
                };
                match reveal_path(&path) {
                    Ok(()) => CommandResult::success(ControlResult {
                        task_id: task_id.to_string(),
                        status,
                    }),
                    Err(e) => CommandResult::failure(codes::INTERNAL, e, false),
                }
            }
            "remove_history" => {
                let _ = self.store.delete_task(task_id);
                self.emit_queue_changed();
                CommandResult::success(ControlResult {
                    task_id: task_id.to_string(),
                    status: DownloadStatus::Cancelled,
                })
            }
            "set_priority" => {
                let new_priority = match req_priority {
                    Some(p) => p,
                    None => {
                        return CommandResult::failure(
                            codes::INVALID_ACTION,
                            "set_priority requires a priority value",
                            false,
                        )
                    }
                };
                // Match the UI's 0–100 priority scale. The store accepts any
                // integer, so without this check a direct IPC caller could set
                // a value the UI cannot represent or order meaningfully.
                if !(0..=100).contains(&new_priority) {
                    return CommandResult::failure(
                        codes::INVALID_ACTION,
                        "Priority must be between 0 and 100",
                        false,
                    );
                }
                if let Err(e) = self.store.set_priority(task_id, new_priority) {
                    return CommandResult::failure(codes::STORE_ERROR, e.to_string(), true);
                }
                self.emit_queue_changed();
                CommandResult::success(ControlResult {
                    task_id: task_id.to_string(),
                    status,
                })
            }
            "overwrite" | "rename" | "skip" => {
                self.resolve_collision(task_id, action)
            }
            other => CommandResult::failure(
                codes::INVALID_ACTION,
                format!("Unknown action '{other}'"),
                false,
            ),
        }
    }

    /// Resolve an Ask-policy collision from an explicit UI action.
    ///
    /// The working file was already downloaded, size-checked, hashed and
    /// scanned before the collision was detected, so this path never
    /// redownloads. The task stays `failed` until the action arrives.
    fn resolve_collision(&self, task_id: &str, action: &str) -> CommandResult<ControlResult> {
        let row = match self.store.get_task(task_id) {
            Ok(Some(r)) => r,
            Ok(None) => {
                return CommandResult::failure(codes::TASK_NOT_FOUND, "No such download task", false)
            }
            Err(e) => return CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
        };
        if row.status_enum() != DownloadStatus::Failed
            || row.last_error_code.as_deref() != Some(codes::OVERWRITE_CONFIRMATION_REQUIRED)
        {
            return CommandResult::failure(
                codes::INVALID_ACTION,
                "This task is not awaiting an overwrite decision",
                false,
            );
        }
        let policy = match action {
            "overwrite" => OverwritePolicy::Replace,
            "rename" => OverwritePolicy::Rename,
            "skip" => OverwritePolicy::Skip,
            _ => {
                return CommandResult::failure(
                    codes::INVALID_ACTION,
                    "Unknown collision action",
                    false,
                )
            }
        };

        let part = task_part_path(&row.destination_path, task_id);
        if !part.is_file() {
            return CommandResult::failure(
                codes::INTERNAL,
                "The verified working file is no longer available",
                false,
            );
        }
        // Resolve the collision against the same published name `finalize`
        // would use, so an applied server filename is honoured here too.
        let server_name = server_filename_for(self.store(), task_id);
        let desired = server_named_destination(&row.destination_path, server_name.as_deref())
            .unwrap_or_else(|| PathBuf::from(&row.destination_path));
        let final_path = match security::choose_final_path(&desired, policy) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return CommandResult::failure(
                    codes::OVERWRITE_CONFIRMATION_REQUIRED,
                    "Destination already exists; confirm overwrite, rename or skip",
                    false,
                )
            }
            Err(e) => {
                return CommandResult::failure(
                    codes::PERMISSION_DENIED,
                    format!("Could not choose a destination path: {e}"),
                    false,
                )
            }
        };

        if policy == OverwritePolicy::Skip && final_path == desired && desired.exists() {
            let _ = std::fs::remove_file(&part);
            let _ = self.store.set_task_status(task_id, DownloadStatus::Cancelled);
            self.emit_status(task_id, DownloadStatus::Cancelled);
            return CommandResult::success(ControlResult { task_id: task_id.to_string(), status: DownloadStatus::Cancelled });
        } else if policy == OverwritePolicy::Replace {
            if let Err(e) = std::fs::rename(&part, &final_path) {
                return CommandResult::failure(
                    codes::PERMISSION_DENIED,
                    format!("Could not finalise the download: {e}"),
                    false,
                );
            }
        } else if let Err(e) = publish_no_clobber(&part, &final_path) {
            let code = if e.kind() == std::io::ErrorKind::AlreadyExists {
                codes::OVERWRITE_CONFIRMATION_REQUIRED
            } else {
                codes::PERMISSION_DENIED
            };
            return CommandResult::failure(code, "Could not publish the download", false);
        }

        let _ = self
            .store
            .set_setting(&format!("pending_overwrite:{task_id}"), "");
        if let Err(error) = self.store.complete_task(task_id, row.received_bytes.max(0) as u64, &final_path.to_string_lossy()) {
            return CommandResult::failure(codes::STORE_ERROR, error.to_string(), true);
        }
        // The final path (now including any applied server name) is persisted
        // in the downloads row by complete_task; the transient marker is done.
        set_server_filename(self.store(), task_id, "");
        self.emit_status(task_id, DownloadStatus::Completed);
        self.log(
            Some(task_id),
            "download_completed",
            serde_json::json!({ "resolution": action }),
        );
        CommandResult::success(ControlResult {
            task_id: task_id.to_string(),
            status: DownloadStatus::Completed,
        })
    }

    // -------------------------------------------------------------- worker

    async fn run_task(self: Arc<Self>, task_id: &str) {
        let rt = match self.tasks.lock().expect("task map poisoned").get(task_id).cloned() {
            Some(rt) => rt,
            None => return,
        };

        // Sleep prevention is owned by the desktop layer on a dedicated,
        // stable OS thread. `SetThreadExecutionState` is per-thread, so calling
        // it from an async worker (which may migrate threads) cannot guarantee
        // that the matching release reaches the same thread.

        loop {
            if rt.cancel.load(Ordering::SeqCst) {
                let _ = self.store.set_task_status(task_id, DownloadStatus::Cancelled);
                self.emit_status(task_id, DownloadStatus::Cancelled);
                return;
            }
            if rt.paused.load(Ordering::SeqCst) { return; }

            let row = match self.store.get_task(task_id) {
                Ok(Some(r)) => r,
                _ => return,
            };
            if row.status_enum().is_terminal() {
                return;
            }

            let _ = self
                .store
                .set_task_status(task_id, DownloadStatus::Starting);
            self.emit_status(task_id, DownloadStatus::Starting);

            match self.execute(&rt, &row).await {
                Ok(()) => {
                    if rt.paused.load(Ordering::SeqCst) { return; }
                    let _ = self.store.set_task_status(task_id, DownloadStatus::Verifying);
                    self.emit_status(task_id, DownloadStatus::Verifying);
                    self.emit_progress(task_id, &rt, DownloadStatus::Verifying);
                    let latest = match self.store.get_task(task_id) {
                        Ok(Some(latest)) => latest,
                        _ => return,
                    };
                    match self.finalize(&rt, &latest).await {
                        Ok(final_path) => {
                            if let Err(error) = self.store.complete_task(task_id, rt.received.load(Ordering::Relaxed), &final_path) {
                                self.fail(task_id, &EngineError::store(error));
                                return;
                            }
                            // complete_task persisted the final (server-named)
                            // path; the transient override is no longer needed.
                            set_server_filename(self.store(), task_id, "");
                            self.emit_status(task_id, DownloadStatus::Completed);
                            self.log(
                                Some(task_id),
                                "download_completed",
                                serde_json::json!({}),
                            );
                            // Only an explicitly enabled completed batch, with
                            // nothing left active, asks the host for shutdown.
                            let wants_shutdown = self
                                .settings
                                .lock()
                                .map(|s| s.shutdown_after_complete)
                                .unwrap_or(false);
                            if wants_shutdown && self.store.count_active().unwrap_or(1) == 0 {
                                self.request_shutdown_once("completed_batch");
                            }
                            return;
                        }
                        Err(e) => {
                            if e.is_cancellation() {
                                let _ = self.store.set_task_status(task_id, DownloadStatus::Cancelled);
                                self.emit_status(task_id, DownloadStatus::Cancelled);
                                return;
                            }
                            self.fail(task_id, &e);
                            return;
                        }
                    }
                }
                Err(e) => {
                    if rt.paused.load(Ordering::SeqCst) { return; }
                    // A user cancel is a terminal control action, not a failed
                    // transfer. Without this branch the worker's cancellation
                    // error fell through to `fail()` and overwrote the
                    // `cancelled` status set by `control()` with `failed`.
                    if e.is_cancellation() {
                        let _ = self.store.set_task_status(task_id, DownloadStatus::Cancelled);
                        self.emit_status(task_id, DownloadStatus::Cancelled);
                        self.log(Some(task_id), "download_cancelled", serde_json::json!({}));
                        return;
                    }
                    self.log(
                        Some(task_id),
                        "download_error",
                        serde_json::json!({
                            "code": e.code,
                            "message": e.message,
                            "retryable": e.retryable,
                        }),
                    );
                    let attempts = row.retry_count;
                    if e.retryable && attempts < row.max_retries {
                        let _ = self.store.set_task_status(task_id, DownloadStatus::RetryWait);
                        self.emit_status(task_id, DownloadStatus::RetryWait);
                        self.store
                            .set_retry_count(task_id, attempts + 1)
                            .ok();
                        let _ = until_control(&rt, tokio::time::sleep(backoff(attempts.max(0).min(10) as u32))).await;
                        continue;
                    }
                    self.fail(task_id, &e);
                    return;
                }
            }
        }
    }

    fn fail(&self, task_id: &str, e: &EngineError) {
        let _ = self.store.set_task_status(task_id, DownloadStatus::Failed);
        let _ = self.store.set_error(task_id, e.code, &e.message);
        self.emit_status(task_id, DownloadStatus::Failed);
        self.log(
            Some(task_id),
            "download_failed",
            serde_json::json!({ "code": e.code, "message": e.message }),
        );

        self.cleanup_failed_working_file(task_id, e.code);
    }

    /// Discard a failed task's task-owned working file when the user opted out
    /// of keeping partials. Legacy `<destination>.part` files and any other file
    /// in the destination directory are never touched. A recoverable Ask
    /// collision always keeps its verified working file.
    pub fn cleanup_failed_working_file(&self, task_id: &str, error_code: &str) {
        let keep = error_code == codes::OVERWRITE_CONFIRMATION_REQUIRED
            || error_code == codes::DISK_FULL
            || self
                .settings
                .lock()
                .map(|s| s.keep_partial_on_failure)
                .unwrap_or(true);
        if keep {
            return;
        }
        if let Ok(Some(row)) = self.store.get_task(task_id) {
            let part = task_part_path(&row.destination_path, task_id);
            if part.is_file() {
                let _ = std::fs::remove_file(&part);
                let _ = self.store.clear_segments(task_id);
                self.log(Some(task_id), "partial_discarded", serde_json::json!({}));
            }
        }
    }

    // ----------------------------------------------------------- transfer

    fn static_error_code(code: &str) -> &'static str {
        match code {
            codes::INVALID_URL => codes::INVALID_URL,
            codes::INVALID_ACTION => codes::INVALID_ACTION,
            codes::UNSUPPORTED_SCHEME => codes::UNSUPPORTED_SCHEME,
            codes::HTTP_ERROR => codes::HTTP_ERROR,
            codes::PROTOCOL_ERROR => codes::PROTOCOL_ERROR,
            codes::NETWORK_ERROR => codes::NETWORK_ERROR,
            codes::INVALID_DESTINATION => codes::INVALID_DESTINATION,
            _ => codes::INTERNAL,
        }
    }

    /// Recognize server-side protocol/application errors that arrive with an
    /// HTTP 200 status. This is deliberately narrow: only known error schemas
    /// are rejected so legitimate binary downloads are unaffected.
    fn protocol_error_payload(content_type: Option<&str>, prefix: &[u8]) -> bool {
        let media_type = content_type.unwrap_or("").to_ascii_lowercase();
        if !media_type.contains("vnd.youtube-ump") {
            return false;
        }
        let text = String::from_utf8_lossy(prefix);
        text.contains("malformed_config") || text.contains("sabr.")
    }

    /// Download a selected HLS/DASH representation as one file by assembling
    /// its segments through this engine's existing working-file path.
    async fn execute_manifest(
        self: &Arc<Self>,
        rt: &Arc<Runtime>,
        row: &TaskRow,
    ) -> Result<(), EngineError> {
        use crate::media_manifest as media;

        let validated = security::validate_url(&row.original_url);
        if !validated.ok {
            let error = validated.error.unwrap();
            return Err(EngineError::new(Self::static_error_code(&error.code), error.message, false));
        }
        let url = validated.data.unwrap_or_default();
        let client = self.client();

        let response = match until_control(rt, client.get(&url).send()).await {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                return Err(EngineError::new(codes::NETWORK_ERROR, redact_network_error(&error.to_string()), true));
            }
            Err(error) => return Err(error),
        };
        if !response.status().is_success() {
            return Err(EngineError::new(
                codes::HTTP_ERROR,
                format!("Manifest request returned HTTP {}", response.status().as_u16()),
                false,
            ));
        }
        if response.content_length().is_some_and(|len| len as usize > media::MAX_MANIFEST_BYTES) {
            return Err(EngineError::new(codes::INVALID_ACTION, "The media manifest is too large.", false));
        }
        let manifest_bytes = response.bytes().await.map_err(|error| {
            EngineError::new(codes::NETWORK_ERROR, redact_network_error(&error.to_string()), true)
        })?;
        if manifest_bytes.len() > media::MAX_MANIFEST_BYTES {
            return Err(EngineError::new(codes::INVALID_ACTION, "The media manifest is too large.", false));
        }
        let text = String::from_utf8_lossy(&manifest_bytes);
        let parsed = media::expand(&url, &text);
        if !parsed.ok {
            let error = parsed.error.unwrap();
            return Err(EngineError::new(Self::static_error_code(&error.code), error.message, false));
        }
        let plan = parsed.data.unwrap_or_default();

        let part_path = task_part_path(&row.destination_path, &row.id);
        if let Some(parent) = Path::new(&row.destination_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if part_path.exists() {
            self.log(
                Some(&row.id),
                "manifest_restarted",
                serde_json::json!({ "reason": "segment_assembly_has_no_checkpoints" }),
            );
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&part_path)
            .await
            .map_err(|error| EngineError::new(codes::PERMISSION_DENIED, format!("Could not open the working file: {error}"), false))?;

        let mut total = 0u64;
        for segment in plan.segments {
            if rt.cancel.load(Ordering::SeqCst) {
                return Err(EngineError::cancelled());
            }
            if rt.paused.load(Ordering::SeqCst) {
                return Ok(());
            }
            let response = client.get(&segment).send().await.map_err(|error| {
                EngineError::new(codes::NETWORK_ERROR, redact_network_error(&error.to_string()), true)
            })?;
            if !response.status().is_success() {
                return Err(EngineError::new(
                    codes::HTTP_ERROR,
                    format!("Segment request returned HTTP {}", response.status().as_u16()),
                    false,
                ));
            }
            if response.content_length().is_some_and(|len| len > media::MAX_SEGMENT_BYTES) {
                return Err(EngineError::new(codes::INVALID_ACTION, "A media segment is too large.", false));
            }
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                if rt.cancel.load(Ordering::SeqCst) {
                    return Err(EngineError::cancelled());
                }
                let chunk = chunk.map_err(|error| {
                    EngineError::new(codes::NETWORK_ERROR, redact_network_error(&error.to_string()), true)
                })?;
                let length = chunk.len() as u64;
                if total.saturating_add(length) > media::MAX_TOTAL_BYTES {
                    return Err(EngineError::new(codes::INVALID_ACTION, "The assembled media exceeds the size limit.", false));
                }
                file.write_all(&chunk).await.map_err(|error| {
                    EngineError::new(codes::DISK_FULL, format!("Could not write a media segment: {error}"), false)
                })?;
                total = total.saturating_add(length);
                rt.received.fetch_add(length, Ordering::Relaxed);
            }
            self.emit_progress(row.id.as_str(), rt, DownloadStatus::Downloading);
        }
        file.flush().await.map_err(|error| {
            EngineError::new(codes::DISK_FULL, format!("Could not finalize the media file: {error}"), false)
        })?;
        drop(file);
        self.log(
            Some(&row.id),
            "manifest_assembled",
            serde_json::json!({ "kind": plan.kind, "bytes": total }),
        );
        Ok(())
    }

    async fn execute(
        self: &Arc<Self>,
        rt: &Arc<Runtime>,
        row: &TaskRow,
    ) -> Result<(), EngineError> {
        // Media manifest tasks reuse this engine's task lifecycle, working file,
        // cancellation, verification and finalisation. They only replace the
        // byte-acquisition step with bounded segment assembly.
        let lower = row.original_url.to_ascii_lowercase();
        if lower.contains(".m3u8") || lower.contains(".mpd") {
            return self.execute_manifest(rt, row).await;
        }
        let headers = parse_headers(&row.headers_json)?;
        let meta = until_control(rt, self.probe(&row.original_url, &headers)).await??;
        let mut headers = headers;
        if let Some(validator) = meta.etag.as_deref().filter(|etag| !etag.starts_with("W/"))
            .or(meta.last_modified.as_deref())
        {
            headers.push(HeaderPair { name: "If-Range".to_string(), value: validator.to_string() });
        }

        let part_path = task_part_path(&row.destination_path, &row.id);
        if let Some(parent) = Path::new(&row.destination_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // A pre-upgrade `<destination>.part` might belong to another task. We
        // never write to it and never delete it.
        if part_path_for(&row.destination_path).exists() {
            self.log(
                Some(&row.id),
                "legacy_partial_preserved",
                serde_json::json!({ "preserved": true }),
            );
        }

        // Resume compatibility: the remote representation must match what the
        // stored segments were downloaded from. Metadata whose working file is
        // missing (e.g. upgraded from a build that used the shared .part) is not
        // usable — the legacy file is left alone, but its checkpoints are not
        // applied to an empty file.
        let stored_segments = self.store.list_segments(&row.id).map_err(EngineError::store)?;
        let existing = if part_path.exists() {
            stored_segments
        } else {
            if !stored_segments.is_empty() {
                self.store.clear_segments(&row.id).map_err(EngineError::store)?;
                self.log(
                    Some(&row.id),
                    "resume_restarted",
                    serde_json::json!({ "reason": "working_file_missing" }),
                );
            }
            Vec::new()
        };
        // Checkpoints are written before the writer's final fsync, so after a
        // crash the metadata can be ahead of the bytes actually on disk. Never
        // trust a checkpoint whose end offset is beyond the working file: that
        // would resume at a hole and silently write zero-filled bytes.
        let part_len = if part_path.exists() {
            tokio::fs::metadata(&part_path)
                .await
                .map(|m| m.len())
                .ok()
        } else {
            None
        };

        let mut resume_offset: u64 = 0;
        let mut reuse_plan = false;
        if !existing.is_empty() {
            let same_size = meta.total_bytes == row.total_bytes.map(|v| v as u64);
            let same_validator = validators_match(&meta, row);
            let checkpoints_on_disk = match part_len {
                Some(len) => {
                    existing.iter().all(|s| {
                        let start = s.start_offset.max(0) as u64;
                        let got = s.received_bytes.max(0) as u64;
                        start.saturating_add(got) <= len
                    }) && meta.total_bytes.map(|t| len <= t).unwrap_or(true)
                }
                None => false,
            };
            if same_size && same_validator && checkpoints_on_disk {
                reuse_plan = true;
                resume_offset = existing
                    .iter()
                    .map(|s| s.received_bytes.max(0) as u64)
                    .sum();
                self.log(
                    Some(&row.id),
                    "resume_checked",
                    serde_json::json!({ "segments": existing.len(), "resume_offset": resume_offset }),
                );
            } else {
                self.store.clear_segments(&row.id).map_err(EngineError::store)?;
                let reason = if !same_size {
                    "size_changed"
                } else if !same_validator {
                    "validator_changed"
                } else {
                    "working_file_truncated"
                };
                self.log(
                    Some(&row.id),
                    "resume_restarted",
                    serde_json::json!({ "reason": reason }),
                );
            }
        } else if row.received_bytes > 0 {
            // Single-stream resume has no segment rows to validate. Its recorded
            // offset must still be covered by the working file, otherwise the
            // writer would seek past EOF and leave a zero-filled gap.
            let covered = part_len
                .map(|len| (row.received_bytes as u64) <= len)
                .unwrap_or(false);
            if !covered {
                let _ = self.store.set_received(&row.id, 0);
                self.log(
                    Some(&row.id),
                    "resume_restarted",
                    serde_json::json!({ "reason": "working_file_truncated" }),
                );
            }
        }

        let total = meta.total_bytes.unwrap_or(0);
        rt.total.store(total, Ordering::Relaxed);
        rt.received.store(resume_offset, Ordering::Relaxed);
        rt.last_received.store(resume_offset, Ordering::Relaxed);
        *rt.last_emit.lock().expect("emit mutex poisoned") = Instant::now();
        rt.segment_done.store(0, Ordering::Relaxed);

        // Apply a server-provided filename only for tasks that opted in at
        // creation AND still carry the automatic-name sentinel. Requiring the
        // durable `auto_name` marker (not just the name) means legacy/external
        // rows are never renamed by inference. We persist the chosen name under
        // a task-specific settings key and use it ONLY when the final file is
        // published — the task-owned `.part` path is left alone, so resuming
        // never renames or orphans partial data. Writing the name once also
        // makes the choice stable across retries and restarts.
        let suggested = choose_download_filename(
            &row.display_name,
            meta.content_disposition.as_deref(),
        );
        let opted_in = is_automatic_download_name(&row.display_name)
            && is_auto_name_opted_in(self.store(), &row.id);
        if suggested != row.display_name {
            self.log(
                Some(&row.id),
                "server_filename_suggested",
                serde_json::json!({ "name": suggested }),
            );
            if opted_in && server_filename_for(self.store(), &row.id).is_none() {
                if server_named_destination(&row.destination_path, Some(&suggested)).is_some() {
                    set_server_filename(self.store(), &row.id, &suggested);
                    self.log(
                        Some(&row.id),
                        "server_filename_applied",
                        serde_json::json!({ "name": suggested }),
                    );
                } else {
                    // Unsafe/over-long suggestion: keep the original name.
                    self.log(
                        Some(&row.id),
                        "server_filename_rejected",
                        serde_json::json!({ "name": suggested }),
                    );
                }
            } else {
                // Explicitly named task (or legacy row without the opt-in):
                // the suggestion was recorded but never applied.
                self.log(
                    Some(&row.id),
                    "server_filename_ignored",
                    serde_json::json!({ "name": suggested, "opted_in": opted_in }),
                );
            }
        }

        let _ = self.store.set_transfer_metadata(
            &row.id,
            meta.total_bytes,
            meta.etag.as_deref(),
            meta.last_modified.as_deref(),
            meta.accept_ranges,
            meta.content_type.as_deref(),
            Some(&meta.final_url),
            Some(&part_path.to_string_lossy()),
        );

        // The probe is complete and the transfer is about to start: publish the
        // `downloading` state now. Previously the engine jumped from `starting`
        // straight to `verifying`, so a crash mid-transfer left the persisted
        // status at `starting` even though megabytes were already on disk.
        if rt.cancel.load(Ordering::SeqCst) {
            return Err(EngineError::cancelled());
        }
        let _ = self.store.set_task_status(&row.id, DownloadStatus::Downloading);
        self.emit_status(&row.id, DownloadStatus::Downloading);
        self.emit_progress(&row.id, rt, DownloadStatus::Downloading);

        let connections = (row.connections.max(1) as u32).min(domain::MAX_CONNECTIONS_PER_TASK);
        let use_segments = meta.accept_ranges
            && total >= 2 * domain::MIN_SEGMENT_SIZE
            && connections > 1;

        let result = if use_segments {
            match self
                .transfer_segmented(rt, row, &meta, &part_path, &headers, connections, reuse_plan)
                .await
            {
                Ok(()) => Ok(()),
                Err(e) if e.code == codes::SERVER_DOES_NOT_SUPPORT_RANGES => {
                    // The server changed its mind mid-transfer: fall back.
                    self.log(
                        Some(&row.id),
                        "segmentation_fallback",
                        serde_json::json!({ "reason": e.message }),
                    );
                    self.transfer_single(rt, row, &meta, &part_path, &headers, false).await
                }
                Err(e) => Err(e),
            }
        } else {
            self.transfer_single(rt, row, &meta, &part_path, &headers, reuse_plan).await
        };

        // Persist the final received count.
        let received = rt.received.load(Ordering::Relaxed);
        self.store.set_received(&row.id, received).ok();
        result
    }

    async fn transfer_segmented(
        self: &Arc<Self>,
        rt: &Arc<Runtime>,
        row: &TaskRow,
        meta: &ResourceMetadata,
        part_path: &Path,
        headers: &[HeaderPair],
        connections: u32,
        resume: bool,
    ) -> Result<(), EngineError> {
        let total = meta.total_bytes.unwrap_or(0);
        let plan = domain::plan_segments(total, connections);
        if plan.is_empty() {
            return self
                .transfer_single(rt, row, meta, part_path, headers, resume)
                .await;
        }

        // Reuse the stored plan when resuming so segment indexes stay stable.
        let stored: Vec<SegmentRow> = if resume {
            self.store.list_segments(&row.id).map_err(EngineError::store)?
        } else {
            Vec::new()
        };

        let mut effective: Vec<SegmentPlan> = Vec::new();
        let mut restart = false;
        if resume && stored.len() == plan.len() {
            for (seg, old) in plan.iter().zip(stored.iter()) {
                let already = old.received_bytes.max(0) as u64;
                if already >= seg.len() {
                    rt.segment_done.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                effective.push(SegmentPlan {
                    index: seg.index,
                    start: seg.start + already,
                    end: seg.end,
                });
            }
        } else {
            // Either a fresh download, or a resume whose plan no longer matches
            // the stored one (the user changed the connection count). In the
            // latter case the stored offsets cannot be trusted, so discard them
            // and start from byte zero instead of leaving orphaned bytes in the
            // .part file.
            restart = resume;
            self.store.clear_segments(&row.id).map_err(EngineError::store)?;
            effective = plan.clone();
        }

        if restart {
            rt.received.store(0, Ordering::Relaxed);
            rt.last_received.store(0, Ordering::Relaxed);
            *rt.last_emit.lock().expect("emit mutex poisoned") = Instant::now();
        }

        rt.segment_total.store(plan.len() as u64, Ordering::Relaxed);

        // Persist the plan. `insert_segment` preserves existing progress on
        // conflict, so re-persisting a matching plan on resume is safe.
        for seg in &plan {
            let _ = self.store.insert_segment(
                &format!("{}-{}", row.id, seg.index),
                &row.id,
                seg.index as i64,
                seg.start as i64,
                seg.end as i64,
            );
        }

        let (tx, rx) = mpsc::channel::<Chunk>(128);
        let writer = tauri::async_runtime::spawn(run_writer(
            part_path.to_path_buf(),
            rx,
            Arc::clone(rt),
            Arc::clone(&self.global_limiter),
            !resume || restart,
        ));

        let mut joins = Vec::new();
        let expected_total = meta.total_bytes;
        let expected_etag = meta.etag.clone();
        let expected_last_modified = meta.last_modified.clone();
        for seg in effective {
            let engine = Arc::clone(self);
            let rt = Arc::clone(rt);
            let url = meta.final_url.clone();
            let hdrs = headers.to_vec();
            let task_id = row.id.clone();
            let tx = tx.clone();
            let etag = expected_etag.clone();
            let last_modified = expected_last_modified.clone();
            joins.push(tauri::async_runtime::spawn(async move {
                engine
                    .run_segment(
                        &rt,
                        &task_id,
                        &url,
                        &hdrs,
                        seg.index,
                        seg.start,
                        seg.end,
                        expected_total,
                        etag,
                        last_modified,
                        tx,
                    )
                    .await
            }));
        }
        drop(tx);

        let mut first_error: Option<EngineError> = None;
        for join in joins {
            match join.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    rt.segment_done.fetch_add(1, Ordering::Relaxed);
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(EngineError::internal(format!("worker panicked: {e}")));
                    }
                }
            }
        }

        // Wait for the writer to drain and fsync.
        match writer.await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                let remaining = rt
                    .total
                    .load(Ordering::Relaxed)
                    .saturating_sub(rt.received.load(Ordering::Relaxed));
                return Err(writer_error(e, remaining));
            }
            Err(e) => return Err(EngineError::internal(format!("writer task failed: {e}"))),
        }

        if let Some(e) = first_error {
            return Err(e);
        }

        let received = rt.received.load(Ordering::Relaxed);
        self.store.set_received(&row.id, received).ok();
        Ok(())
    }

    async fn transfer_single(
        self: &Arc<Self>,
        rt: &Arc<Runtime>,
        row: &TaskRow,
        meta: &ResourceMetadata,
        part_path: &Path,
        headers: &[HeaderPair],
        resume: bool,
    ) -> Result<(), EngineError> {
        let start_offset = if resume {
            rt.received.load(Ordering::Relaxed)
        } else {
            0
        };

        // This function also runs as the fallback after a segmented attempt has
        // already written (and counted) some chunks. `rt.received` still holds
        // that count, so without resetting it the final `received_bytes` would
        // be the byte count of the segmented attempt plus the whole file.
        // `start_offset` is exactly the number of bytes that remain valid in
        // the .part file for this attempt.
        rt.received.store(start_offset, Ordering::Relaxed);
        rt.last_received.store(start_offset, Ordering::Relaxed);
        *rt.last_emit.lock().expect("emit mutex poisoned") = Instant::now();
        rt.segment_total.store(1, Ordering::Relaxed);
        rt.segment_done.store(0, Ordering::Relaxed);

        if !resume { self.store.clear_segments(&row.id).map_err(EngineError::store)?; }
        self.store.insert_segment(
            &format!("{}-0", row.id), &row.id, 0, 0,
            meta.total_bytes.and_then(|total| total.checked_sub(1)).unwrap_or(i64::MAX as u64).min(i64::MAX as u64) as i64,
        ).map_err(EngineError::store)?;

        let (tx, rx) = mpsc::channel::<Chunk>(128);
        let writer = tauri::async_runtime::spawn(run_writer(
            part_path.to_path_buf(),
            rx,
            Arc::clone(rt),
            Arc::clone(&self.global_limiter),
            !resume,
        ));

        let engine = Arc::clone(self);
        let rt2 = Arc::clone(rt);
        let url = meta.final_url.clone();
        let hdrs = headers.to_vec();
        let task_id = row.id.clone();
        let expected_total = meta.total_bytes;
        let expected_etag = meta.etag.clone();
        let expected_last_modified = meta.last_modified.clone();
        let join = tauri::async_runtime::spawn(async move {
            engine
                .run_segment(
                    &rt2,
                    &task_id,
                    &url,
                    &hdrs,
                    0,
                    start_offset,
                    u64::MAX,
                    expected_total,
                    expected_etag,
                    expected_last_modified,
                    tx,
                )
                .await
        });

        let result = match join.await {
            Ok(r) => r,
            Err(e) => Err(EngineError::internal(format!("worker task failed: {e}"))),
        };

        match writer.await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                let remaining = rt
                    .total
                    .load(Ordering::Relaxed)
                    .saturating_sub(rt.received.load(Ordering::Relaxed));
                return Err(writer_error(e, remaining));
            }
            Err(e) => return Err(EngineError::internal(format!("writer task failed: {e}"))),
        }

        rt.segment_done.store(1, Ordering::Relaxed);
        let received = rt.received.load(Ordering::Relaxed);
        self.store.set_received(&row.id, received).ok();
        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_segment(
        self: &Arc<Self>,
        rt: &Arc<Runtime>,
        task_id: &str,
        url: &str,
        headers: &[HeaderPair],
        index: u32,
        mut start: u64,
        end: u64,
        expected_total: Option<u64>,
        expected_etag: Option<String>,
        expected_last_modified: Option<String>,
        tx: mpsc::Sender<Chunk>,
    ) -> Result<(), EngineError> {
        // Hold one host permit for the whole segment so retries cannot inflate
        // the per-host connection count beyond the global ceiling.
        let _host_permit = self.host_permit(url).await?;
        let mut attempts: u32 = 0;
        let mut last_checkpoint = Instant::now();

        loop {
            if rt.cancel.load(Ordering::SeqCst) {
                return Err(EngineError::cancelled());
            }
            if rt.paused.load(Ordering::SeqCst) {
                return Err(EngineError::new(codes::INVALID_ACTION, "Download was paused.", false));
            }
            if start > end {
                return Ok(());
            }

            let range_header = if end == END_TO_EOF {
                format!("bytes={start}-")
            } else {
                format!("bytes={start}-{end}")
            };

            rt.active_connections.fetch_add(1, Ordering::Relaxed);
            let req = apply_headers(self.client().get(url), headers)
                .header(reqwest::header::RANGE, range_header);
            let sent = until_control(rt, req.send()).await?;
            rt.active_connections.fetch_sub(1, Ordering::Relaxed);

            let resp = match sent {
                Ok(r) => r,
                Err(e) => {
                    attempts += 1;
                    if attempts > MAX_SEGMENT_ATTEMPTS {
                        return Err(EngineError::network(e));
                    }
                    if e.is_timeout() {
                        return Err(EngineError::timeout());
                    }
                    tokio::time::sleep(backoff(attempts)).await;
                    continue;
                }
            };

            check_redirect_target(url, &resp.url().to_string())?;
            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS
                || status.is_server_error()
                || status == reqwest::StatusCode::REQUEST_TIMEOUT
            {
                attempts += 1;
                if attempts > MAX_SEGMENT_ATTEMPTS {
                    return Err(EngineError::http(status.as_u16()));
                }
                // Honour a bounded Retry-After header when present (spec §7.6).
                let wait = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(parse_retry_after)
                    .unwrap_or_else(|| backoff(attempts));
                tokio::time::sleep(wait).await;
                continue;
            }
            if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                return Err(EngineError::ranges_unsupported());
            }
            // A 200 to a *fixed* range is never a valid range response, even
            // for the first segment: the body is the whole representation, so
            // writing it at a segment offset would corrupt the file.
            if !range_response_is_valid(status, start, end) {
                return Err(EngineError::ranges_unsupported());
            }
            if status != reqwest::StatusCode::OK && status != reqwest::StatusCode::PARTIAL_CONTENT {
                return Err(EngineError::http(status.as_u16()));
            }

            // Validate every part of Content-Range against the request. A
            // mismatch means the bytes cannot be trusted to belong at `offset`:
            // retry a bounded number of times, then fall back to a fresh plan.
            if status == reqwest::StatusCode::PARTIAL_CONTENT {
                let declared = resp
                    .headers()
                    .get(reqwest::header::CONTENT_RANGE)
                    .and_then(|v| v.to_str().ok())
                    .and_then(parse_content_range);
                let acceptable = match declared {
                    Some(cr) => {
                        cr.start == start
                            && (end == END_TO_EOF || cr.end == end)
                            && match (expected_total, cr.total) {
                                (Some(a), Some(b)) => a == b,
                                _ => true,
                            }
                    }
                    None => false,
                };
                if !acceptable {
                    attempts += 1;
                    if attempts > MAX_SEGMENT_ATTEMPTS {
                        return Err(EngineError::ranges_unsupported());
                    }
                    tokio::time::sleep(backoff(attempts)).await;
                    continue;
                }
            }

            // All segments must describe the same representation as the probe.
            // A changed ETag/Last-Modified means we would be stitching bytes
            // from different versions of the resource.
            let etag = header_string(resp.headers(), reqwest::header::ETAG);
            let last_modified = header_string(resp.headers(), reqwest::header::LAST_MODIFIED);
            if !representation_matches(&expected_etag, etag)
                || !representation_matches(&expected_last_modified, last_modified)
            {
                attempts += 1;
                if attempts > MAX_SEGMENT_ATTEMPTS {
                    return Err(EngineError::ranges_unsupported());
                }
                tokio::time::sleep(backoff(attempts)).await;
                continue;
            }

            let mut stream = resp.bytes_stream();
            let mut progressed = false;
            // True when the response body ran out early. reqwest surfaces a
            // truncated body as an error item, so this is the only signal that
            // distinguishes "server sent everything" from "connection died".
            let mut truncated = false;
            let mut bytes_since_checkpoint: u64 = 0;
            loop {
                let item = match until_control(rt, stream.next()).await {
                    Ok(Some(item)) => item,
                    Ok(None) => break,
                    Err(error) => {
                        self.checkpoint_written(&tx, task_id, index, start, end).await?;
                        return Err(error);
                    }
                };
                if rt.cancel.load(Ordering::SeqCst) {
                    return Err(EngineError::cancelled());
                }
                if rt.paused.load(Ordering::SeqCst) {
                    break;
                }
                match item {
                    Ok(bytes) => {
                        if bytes.is_empty() {
                            continue;
                        }
                        let offset = start;
                        // Never write past the assigned range, even if a
                        // malformed response sends a longer body. The writer
                        // seeks to `offset`, so an oversized chunk would
                        // otherwise spill into the next segment's bytes.
                        let allowed = remaining_in_segment(offset, end);
                        if allowed == 0 {
                            break;
                        }
                        let take = (bytes.len() as u64).min(allowed) as usize;
                        let payload = bytes[..take].to_vec();
                        start += take as u64;
                        progressed = true;
                        rt.received.fetch_add(take as u64, Ordering::Relaxed);
                        if tx
                            .send(Chunk {
                                offset,
                                data: payload,
                                barrier: None,
                            })
                            .await
                            .is_err()
                        {
                            return Err(EngineError::internal("disk writer stopped"));
                        }
                        bytes_since_checkpoint += take as u64;
                        if last_checkpoint.elapsed() >= Duration::from_secs(2)
                            || bytes_since_checkpoint >= CHECKPOINT_EVERY_BYTES
                        {
                            self.checkpoint_written(&tx, task_id, index, start, end).await?;
                            last_checkpoint = Instant::now();
                            bytes_since_checkpoint = 0;
                        }
                        if start > end {
                            break;
                        }
                    }
                    Err(_) => {
                        truncated = true;
                        break;
                    }
                }
            }

            if start > end {
                self.checkpoint_written(&tx, task_id, index, end + 1, end).await?;
                rt.segment_done.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }

            // Single-stream to EOF: `end` is the END_TO_EOF sentinel, so
            // `start > end` can never fire. A body that ran to its natural
            // close is complete. Without this the loop re-issues
            // `Range: bytes=<full-length>-`, the server answers 200 (ignoring
            // the range), and `start != 0` then fails the whole task with
            // SERVER_DOES_NOT_SUPPORT_RANGES — after every byte was already
            // written. Pause/cancel must not look like completion.
            if end == END_TO_EOF
                && !truncated
                && !rt.paused.load(Ordering::SeqCst)
                && !rt.cancel.load(Ordering::SeqCst)
            {
                self.checkpoint_written(&tx, task_id, index, start, end).await?;
                rt.segment_done.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }

            if !progressed {
                attempts += 1;
                if attempts > MAX_SEGMENT_ATTEMPTS {
                    return Err(EngineError::network("connection closed before any data"));
                }
            }
            tokio::time::sleep(backoff(attempts)).await;
        }
    }

    async fn checkpoint_written(
        &self,
        writer: &mpsc::Sender<Chunk>,
        task_id: &str,
        index: u32,
        start: u64,
        end: u64,
    ) -> Result<(), EngineError> {
        let (sent, committed) = tokio::sync::oneshot::channel();
        writer.send(Chunk { offset: 0, data: Vec::new(), barrier: Some(sent) }).await
            .map_err(|_| EngineError::internal("The disk writer stopped before checkpointing"))?;
        committed.await.map_err(|_| EngineError::internal("Could not sync download bytes before checkpointing"))?;
        self.checkpoint(task_id, index, start, end).map_err(EngineError::store)
    }

    fn checkpoint(
        &self,
        task_id: &str,
        index: u32,
        start: u64,
        end: u64,
    ) -> rusqlite::Result<()> {
        // Received bytes for this segment = start - original_start.
        let original_start = self
            .store
            .list_segments(task_id)?
            .into_iter()
            .find(|s| s.segment_index as u32 == index)
            .map(|s| s.start_offset.max(0) as u64)
            .unwrap_or(0);
        let received = start.saturating_sub(original_start);
        let status = if end != u64::MAX && start > end {
            "completed"
        } else {
            "active"
        };
        self.store
            .update_segment_progress(task_id, index as i64, received as i64, status)
    }

    // -------------------------------------------------------------- probe

    /// Bounded metadata probe used by the browser confirmation UI. It never
    /// starts a transfer and uses the same URL policy as the download engine.
    pub async fn probe_metadata(&self, url: &str) -> Result<ResourceMetadata, EngineError> {
        self.probe(url, &[]).await
    }

    async fn probe(
        &self,
        url: &str,
        headers: &[HeaderPair],
    ) -> Result<ResourceMetadata, EngineError> {
        let _host_permit = self.host_permit(url).await?;
        let mut resp = apply_headers(
            self.client()
                .get(url)
                .header(reqwest::header::RANGE, "bytes=0-0"),
            headers,
        )
        .send()
        .await
        .map_err(EngineError::network)?;

        let final_url = resp.url().to_string();
        check_redirect_target(url, &final_url)?;
        let status = resp.status();
        let h = resp.headers().clone();
        if status == reqwest::StatusCode::OK {
            let prefix = resp.chunk().await.ok().flatten().map(|bytes| bytes.to_vec()).unwrap_or_default();
            let content_type = header_string(&h, reqwest::header::CONTENT_TYPE);
            if Self::protocol_error_payload(content_type.as_deref(), &prefix) {
                return Err(EngineError::new(
                    codes::PROTOCOL_ERROR,
                    "The server returned a protocol error response instead of a downloadable file.",
                    false,
                ));
            }
        }

        if status == reqwest::StatusCode::PARTIAL_CONTENT {
            // We asked for exactly byte 0. Anything else is a malformed or
            // hostile response; do not use its length for a segmented plan.
            let total = match h
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_content_range)
            {
                Some(cr) if cr.start == 0 && cr.end == 0 => cr.total,
                _ => return Err(EngineError::ranges_unsupported()),
            };
            Ok(ResourceMetadata {
                final_url,
                total_bytes: total,
                accept_ranges: true,
                etag: header_string(&h, reqwest::header::ETAG),
                last_modified: header_string(&h, reqwest::header::LAST_MODIFIED),
                content_type: header_string(&h, reqwest::header::CONTENT_TYPE),
                content_disposition: header_string(&h, reqwest::header::CONTENT_DISPOSITION),
            })
        } else if status == reqwest::StatusCode::OK {
            let total = h
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            Ok(ResourceMetadata {
                final_url,
                total_bytes: total,
                accept_ranges: false,
                etag: header_string(&h, reqwest::header::ETAG),
                last_modified: header_string(&h, reqwest::header::LAST_MODIFIED),
                content_type: header_string(&h, reqwest::header::CONTENT_TYPE),
                content_disposition: header_string(&h, reqwest::header::CONTENT_DISPOSITION),
            })
        } else if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            Err(EngineError::ranges_unsupported())
        } else {
            Err(EngineError::http(status.as_u16()))
        }
    }

    // ---------------------------------------------------------- finalise

    async fn finalize(
        &self,
        rt: &Arc<Runtime>,
        row: &TaskRow,
    ) -> Result<String, EngineError> {
        // Hashing and antivirus scanning can take seconds. A cancel issued
        // during that window must win over the completion path.
        if rt.cancel.load(Ordering::SeqCst) {
            return Err(EngineError::cancelled());
        }
        let part_path = task_part_path(&row.destination_path, &row.id);
        let meta = tokio::fs::metadata(&part_path)
            .await
            .map_err(|e| EngineError::internal(format!("partial file missing: {e}")))?;

        // Size reconciliation.
        if let Some(total) = row.total_bytes {
            if total > 0 && meta.len() != total as u64 {
                return Err(EngineError::new(
                    codes::CHECKSUM_MISMATCH,
                    format!(
                        "Size mismatch: expected {total} bytes, got {}",
                        meta.len()
                    ),
                    false,
                ));
            }
        }

        // Checksum verification.
        if let Some(algo) = row.checksum_algorithm.as_deref() {
            if let Some(expected) = row.checksum_expected.as_deref() {
                if !expected.trim().is_empty() {
                    let actual = hash_file(&part_path, algo, Some(&rt.cancel)).await?;
                    let verified = actual.eq_ignore_ascii_case(expected.trim());
                    self.store
                        .set_checksum_result(&row.id, Some(actual.clone()), verified)
                        .ok();
                    if !verified {
                        return Err(EngineError::checksum(expected, &actual));
                    }
                }
            }
        }

        if rt.cancel.load(Ordering::SeqCst) {
            return Err(EngineError::cancelled());
        }

        // Optional antivirus scan. Runs the user-configured scanner command
        // against the completed file before finalising. The command is split
        // on whitespace into an executable and arguments; the file path is
        // appended as a separate argument — never interpolated into a shell
        // string — so a malicious command cannot inject extra arguments.
        let settings = self.settings.lock().expect("settings poisoned").clone();
        if settings.scan_with_antivirus {
            let _ = self.store.set_task_status(&row.id, DownloadStatus::Scanning);
            self.emit_status(&row.id, DownloadStatus::Scanning);
            match self.run_antivirus_scan(&settings, &part_path).await {
                Ok(()) => {} // clean — continue to finalise
                Err(e) => {
                    let _ = self.store.set_task_status(&row.id, DownloadStatus::Failed);
                    let _ = self.store.set_error(&row.id, codes::SCANNER_FAILED, &e.message);
                    self.emit_status(&row.id, DownloadStatus::Failed);
                    return Err(e);
                }
            }
        }

        if rt.cancel.load(Ordering::SeqCst) {
            return Err(EngineError::cancelled());
        }

        // Choose the destination according to the overwrite policy. A pending
        // explicit confirmation (overwrite|rename|skip) overrides the stored
        // Ask policy for this attempt only; it is cleared once resolved.
        // For automatically named tasks the persisted server filename replaces
        // only the *published* name; the working file keeps its original path.
        let server_name = server_filename_for(self.store(), &row.id);
        let desired = server_named_destination(&row.destination_path, server_name.as_deref())
            .unwrap_or_else(|| PathBuf::from(&row.destination_path));
        let policy = self
            .store
            .get_setting(&format!("pending_overwrite:{}", row.id))
            .and_then(|s| OverwritePolicy::parse(&s))
            .unwrap_or_else(|| row.overwrite_enum());
        let final_path = match security::choose_final_path(&desired, policy) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Recoverable: the working file is intact and no redownload is
                // needed. The UI resolves this with a control action.
                return Err(EngineError::new(
                    codes::OVERWRITE_CONFIRMATION_REQUIRED,
                    "Destination already exists; confirm overwrite, rename or skip",
                    false,
                ));
            }
            Err(e) => {
                return Err(EngineError::new(
                    codes::PERMISSION_DENIED,
                    format!("Could not choose a destination path: {e}"),
                    false,
                ))
            }
        };
        if let Some(parent) = final_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Skip leaves an existing file untouched: discard only our own working
        // file and report the already-present destination.
        if policy == OverwritePolicy::Skip && final_path == desired && desired.exists() {
            return Err(EngineError::new(codes::PERMISSION_DENIED,
                "Destination already exists. The existing file is unchanged and the new partial is retained.", false));
        }

        if rt.cancel.load(Ordering::SeqCst) {
            return Err(EngineError::cancelled());
        }

        if policy == OverwritePolicy::Replace {
            // The only policy allowed to replace a destination. Windows rename
            // already has replace semantics for an existing, unlocked file.
            tokio::fs::rename(&part_path, &final_path)
                .await
                .map_err(|e| {
                    EngineError::new(
                        codes::PERMISSION_DENIED,
                        format!("Could not finalise the download: {e}"),
                        false,
                    )
                })?;
        } else if let Err(e) = publish_no_clobber(&part_path, &final_path) {
            // MoveFileExW(flags=0) is an atomic no-clobber publish on all
            // Windows filesystems, including FAT/exFAT where hard links are
            // unavailable. A collision still fails safely; nothing is copied
            // over an existing file.
            return Err(EngineError::new(
                codes::PERMISSION_DENIED,
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    "Destination already exists".to_string()
                } else {
                    format!("Could not finalise the download: {e}")
                },
                false,
            ));
        }

        let _ = self.store.set_transfer_metadata(
            &row.id,
            Some(meta.len()),
            None,
            None,
            false,
            None,
            None,
            None,
        );

        Ok(final_path.to_string_lossy().to_string())
    }

    /// Run the configured antivirus scanner against a file.
    ///
    /// The command supports quoted paths and arguments. The download path is
    /// passed as one final argument. No shell is involved.
    ///
    /// A scan timeout of `DEFAULT_SCANNER_TIMEOUT` seconds (hard cap
    /// `MAX_SCANNER_TIMEOUT`) prevents a hung scanner from stalling the
    /// engine forever.
    pub async fn run_antivirus_scan(
        &self,
        settings: &Settings,
        file_path: &Path,
    ) -> Result<(), EngineError> {
        let timeout_secs = DEFAULT_SCANNER_TIMEOUT.clamp(1, MAX_SCANNER_TIMEOUT);
        crate::scanner::run(&settings.scanner_command, file_path, Duration::from_secs(timeout_secs)).await
    }

    fn reverify(&self, task_id: &str, row: &TaskRow) -> Result<Option<String>, EngineError> {
        let path = PathBuf::from(&row.destination_path);
        if !path.exists() {
            return Err(EngineError::new(
                codes::INVALID_ACTION,
                "The file is no longer present",
                false,
            ));
        }
        let algo = row
            .checksum_algorithm
            .clone()
            .unwrap_or_else(|| "sha256".to_string());
        let actual = hash_file_blocking(&path, &algo)?;
        let verified = row
            .checksum_expected
            .as_deref()
            .map(|e| actual.eq_ignore_ascii_case(e.trim()))
            .unwrap_or(false);
        self.store
            .set_checksum_result(task_id, Some(actual.clone()), verified)
            .ok();
        if let Some(expected) = row.checksum_expected.as_deref() {
            if !verified {
                return Err(EngineError::checksum(expected, &actual));
            }
        }
        Ok(Some(actual))
    }

    // --------------------------------------------------------- scheduler

    async fn tick(self: &Arc<Self>) -> Result<(), EngineError> {
        let settings = self.settings.lock().expect("settings poisoned").clone();
        self.global_limiter.set_limit(settings.global_speed_limit);

        let maintenance_due = {
            let mut last = self.last_cleanup.lock().expect("cleanup lock poisoned");
            if last.elapsed() >= MAINTENANCE_INTERVAL {
                *last = Instant::now();
                true
            } else {
                false
            }
        };
        if maintenance_due {
            let _ = self.store.cleanup_retention(settings.retention_days);
            let _ = self.store.trim_event_log(MAX_EVENT_ROWS);
        }

        let queues = self.store.list_queues().map_err(EngineError::store)?;
        let schedules = self.store.list_schedules().map_err(EngineError::store)?;
        let global_active = self.store.count_active().map_err(EngineError::store)?;
        let mut budget = (settings.max_active_tasks as i64) - global_active;
        for queue in queues {
            if !queue.enabled {
                continue;
            }
            let in_window = self.queue_in_window(&queue.id);
            let window_ended = self.track_window(&queue.id, in_window);
            if settings.scheduler_enabled && !in_window {
                self.apply_action_after(&queue.id);
                if window_ended && self.queue_shutdown_enabled(&queue.id) {
                    self.request_shutdown_once("schedule_window_ended");
                }
                continue;
            }
            if budget <= 0 { continue; }
            let allow_scheduled = settings.scheduler_enabled && schedules.iter().any(|schedule| schedule.enabled && schedule.queue_id == queue.id);
            let active_in_queue = self
                .store
                .count_active_in_queue(&queue.id)
                .map_err(EngineError::store)?;
            let room = (queue.max_active_tasks.max(0) - active_in_queue).min(budget);
            if room <= 0 {
                continue;
            }
            let candidates = self
                .store
                .next_queued_in_queue(&queue.id, room, allow_scheduled)
                .map_err(EngineError::store)?;
            for row in candidates {
                if budget <= 0 {
                    break;
                }
                if self.is_running(&row.id) {
                    continue;
                }
                if row.start_policy_enum() == StartPolicy::Manual {
                    continue;
                }
                self.store.set_task_status(&row.id, DownloadStatus::Starting).map_err(EngineError::store)?;
                self.spawn_task(&row.id);
                budget -= 1;
            }
        }
        Ok(())
    }

    pub fn queue_in_window(&self, queue_id: &str) -> bool {
        let schedules = match self.store.list_schedules() {
            Ok(schedules) => schedules,
            Err(_) => return false,
        };

        let queue_schedules: Vec<&crate::store::Schedule> = schedules
            .iter()
            .filter(|s| s.queue_id == queue_id && s.enabled)
            .collect();

        // No enabled schedules for this queue → always in window.
        if queue_schedules.is_empty() {
            return true;
        }

        let now = Local::now();
        queue_schedules.iter().any(|schedule| schedule_allows(schedule, now.weekday(), now.time()))
    }

    /// Apply the schedule's `action_after` to running tasks in `queue_id` when
    /// the queue is outside its active window.
    pub fn apply_action_after(&self, queue_id: &str) {
        let schedules = match self.store.list_schedules() {
            Ok(schedules) => schedules,
            Err(_) => return,
        };
        let Some(sched) = schedules
            .iter()
            .find(|s| s.queue_id == queue_id && s.enabled)
        else {
            return;
        };

        match sched.action_after.as_str() {
            "pause" => {
                let tasks = self.tasks.lock().expect("tasks poisoned");
                for (task_id, rt) in tasks.iter() {
                    if let Ok(Some(row)) = self.store.get_task(task_id) {
                        if row.queue_id == queue_id {
                            rt.paused.store(true, Ordering::SeqCst);
                            let _ = self.store.set_task_status(task_id, DownloadStatus::Paused);
                            self.emit_status(task_id, DownloadStatus::Paused);
                        }
                    }
                }
                drop(tasks);
                self.emit_queue_changed();
            }
            "shutdown" => {
                // This candidate cannot request a shutdown safely: on startup
                // there are no active tasks yet, and on every later tick an
                // idle queue would re-issue the request. Rather than power off
                // a machine at startup or repeatedly, the option is honestly
                // unavailable here and is recorded as such. No shutdown is
                // ever requested from the scheduler in this build.
                // Preserve old settings without repeating an event every tick.
            }
            "nothing" | _ => {}
        }
    }

    // ------------------------------------------------------------ helpers

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn settings(&self) -> MutexGuard<'_, Settings> {
        self.settings.lock().expect("settings mutex poisoned")
    }

    /// Replace the in-memory settings and keep the global limiter in sync.
    pub fn set_settings(&self, settings: Settings) {
        let normalized = settings.normalized();
        match self.settings.lock() {
            Ok(mut guard) => *guard = normalized,
            Err(_) => {}
        }
        let limit = self
            .settings
            .lock()
            .map(|s| s.global_speed_limit)
            .unwrap_or(0);
        self.global_limiter.set_limit(limit);
    }

    pub fn global_limiter(&self) -> &SpeedLimiter {
        &self.global_limiter
    }
}


// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

fn apply_headers(
    mut req: reqwest::RequestBuilder,
    headers: &[HeaderPair],
) -> reqwest::RequestBuilder {
    for h in headers {
        if is_valid_header_name(&h.name) {
            req = req.header(h.name.as_str(), h.value.as_str());
        }
    }
    req
}

fn is_valid_header_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 {
        return false;
    }
    name.bytes().all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(b, b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.'
                | b'^' | b'_' | b'`' | b'|' | b'~')
    })
}

fn parse_headers(raw: &str) -> Result<Vec<HeaderPair>, EngineError> {
    if raw.trim().is_empty() || raw == "[]" {
        return Ok(Vec::new());
    }
    serde_json::from_str::<Vec<HeaderPair>>(raw)
        .map_err(|e| EngineError::new(codes::INTERNAL, format!("Invalid stored headers: {e}"), false))
}

fn header_string(h: &reqwest::header::HeaderMap, name: reqwest::header::HeaderName) -> Option<String> {
    h.get(name).and_then(|v| v.to_str().ok()).map(|s| s.to_string())
}

/// A parsed, syntactically valid `Content-Range` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentRange {
    start: u64,
    end: u64,
    total: Option<u64>,
}

/// Strictly parse `bytes <start>-<end>/<total>` (or `/ *`).
///
/// The previous parsers looked only at the start and the total, so a server
/// could send `bytes 0-0/999999999` for a 1024-byte resource and the engine
/// would happily accept it. Every field is now checked for syntax and internal
/// consistency before any byte is trusted.
fn parse_content_range(value: &str) -> Option<ContentRange> {
    let value = value.trim();
    let (unit, rest) = value.split_once(' ')?;
    if !unit.eq_ignore_ascii_case("bytes") {
        return None;
    }
    let (range, total_text) = rest.rsplit_once('/')?;
    let (start_text, end_text) = range.split_once('-')?;
    let start: u64 = start_text.parse().ok()?;
    let end: u64 = end_text.parse().ok()?;
    if start > end {
        return None;
    }
    let total = if total_text.trim() == "*" {
        None
    } else {
        Some(total_text.parse().ok()?)
    };
    if let Some(total) = total {
        if end >= total {
            return None;
        }
    }
    Some(ContentRange {
        start,
        end,
        total,
    })
}

/// Maximum number of body bytes that may still be written for this segment.
/// A validator that is present in the probe but absent on a later response is
/// inconclusive (not proof of a new representation). A present, different
/// validator is a mismatch and must restart the affected bytes.
/// Classify a disk-writer failure without exposing a private path/URL.
pub fn classify_writer_error(error: &std::io::Error) -> &'static str {
    match error.raw_os_error() {
        // Windows ERROR_DISK_FULL / ERROR_HANDLE_EOF on exhausted volumes.
        Some(112) | Some(39) => codes::DISK_FULL,
        Some(5) => codes::PERMISSION_DENIED,
        _ if error.kind() == std::io::ErrorKind::StorageFull => codes::DISK_FULL,
        _ if error.kind() == std::io::ErrorKind::PermissionDenied => codes::PERMISSION_DENIED,
        _ => codes::INTERNAL,
    }
}

pub fn disk_full_error(remaining: u64) -> EngineError {
    EngineError::new(
        codes::DISK_FULL,
        format!("Disk full while writing the partial file; about {remaining} bytes remain to be written"),
        false,
    )
}

fn writer_error(error: std::io::Error, remaining: u64) -> EngineError {
    let code = classify_writer_error(&error);
    let message = match code {
        codes::DISK_FULL => disk_full_error(remaining).message,
        codes::PERMISSION_DENIED => "Cannot write the partial file: permission denied".to_string(),
        _ => format!("Cannot write the partial file: {error}"),
    };
    EngineError::new(code, message, false)
}

/// A 200 response is only valid for an open-ended single-stream request that
/// starts at byte zero. Every fixed-range request requires 206.
pub fn range_response_is_valid(status: reqwest::StatusCode, start: u64, end: u64) -> bool {
    if status == reqwest::StatusCode::PARTIAL_CONTENT {
        return true;
    }
    status == reqwest::StatusCode::OK && end == END_TO_EOF && start == 0
}

pub fn representation_matches(expected: &Option<String>, actual: Option<String>) -> bool {
    match expected {
        Some(expected) if !expected.is_empty() => match actual {
            Some(actual) => actual == *expected,
            None => true,
        },
        _ => true,
    }
}

fn remaining_in_segment(start: u64, end: u64) -> u64 {
    if end == END_TO_EOF {
        u64::MAX
    } else {
        end.checked_sub(start).map(|d| d + 1).unwrap_or(0)
    }
}

/// Reject a response that was redirected somewhere the URL policy forbids.
///
/// reqwest follows redirects with a bounded hop count, but without this check
/// an `https -> http` downgrade (or any future non-HTTP scheme) would be
/// accepted silently. The error is deliberately generic so a hostile redirect
/// target cannot inject itself into the error envelope or the event log.
fn check_redirect_target(requested: &str, final_url: &str) -> Result<(), EngineError> {
    if requested == final_url {
        return Ok(());
    }
    let from = url::Url::parse(requested)
        .map_err(|_| EngineError::internal("Invalid request URL after redirect"))?;
    let to = url::Url::parse(final_url)
        .map_err(|_| EngineError::internal("Invalid redirect target"))?;
    if security::validate_redirect(&from, &to).is_err() {
        return Err(EngineError::new(
            codes::UNSUPPORTED_SCHEME,
            "Redirect target was rejected by the URL policy",
            false,
        ));
    }
    Ok(())
}

fn validators_match(meta: &ResourceMetadata, row: &TaskRow) -> bool {
    // ETag is the strong validator. If both sides have it and it differs,
    // the resource changed — resume is incompatible.
    match (meta.etag.as_deref(), row.etag.as_deref()) {
        (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() && !a.starts_with("W/") && !b.starts_with("W/") => {
            if a != b {
                return false;
            }
            return true; // ETag match is sufficient
        }
        _ => {}
    }
    // Fall back to Last-Modified when ETag is unavailable on either side.
    match (meta.last_modified.as_deref(), row.last_modified.as_deref()) {
        (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => a == b,
        _ => false,
    }
}

/// The engine type used by the application.
pub type AppEngine = Engine;

/// `display_name` value that marks a task as *automatically named* — i.e. the
/// caller supplied no explicit file name and the URL had no usable last path
/// segment. It is persisted in the `downloads.display_name` column, so it acts
/// as the durable opt-in marker for server-provided names: an explicitly named
/// task (user- or URL-derived) never carries this sentinel and is never renamed.
pub const AUTOMATIC_NAME_SENTINEL: &str = "download";

/// Settings key prefix for the per-task persisted server-name override.
///
/// Stored through the existing key/value `settings` table (the same mechanism
/// used for `pending_overwrite:<id>`), so no schema migration is required. The
/// value is the sanitized filename chosen from `Content-Disposition` at probe
/// time. It is written once, survives restarts, and is cleared on completion.
pub const SERVER_NAME_KEY_PREFIX: &str = "server_name:";

/// Settings key prefix for the durable "opted in to automatic naming" marker.
///
/// Written at task-creation time only for tasks whose name is the automatic
/// sentinel. Its presence — not merely the name — grants permission to adopt a
/// server-provided filename, so legacy/external tasks are never renamed by
/// inference (e.g. a legacy row that happens to be named `download`).
pub const AUTO_NAME_KEY_PREFIX: &str = "auto_name:";

/// True when the task's name is the automatic-name sentinel. This is the
/// persisted "opt in to server naming" marker: only these tasks may adopt a
/// filename suggested by the server.
pub fn is_automatic_download_name(display_name: &str) -> bool {
    let trimmed = display_name.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case(AUTOMATIC_NAME_SENTINEL)
}

/// Per-task key under which the applied server filename is persisted.
pub fn server_name_key(task_id: &str) -> String {
    format!("{SERVER_NAME_KEY_PREFIX}{task_id}")
}

/// Per-task key for the durable automatic-naming opt-in marker.
pub fn auto_name_key(task_id: &str) -> String {
    format!("{AUTO_NAME_KEY_PREFIX}{task_id}")
}

/// Whether a task explicitly opted in to server-provided naming at creation.
pub fn is_auto_name_opted_in(store: &Store, task_id: &str) -> bool {
    store
        .get_setting(&auto_name_key(task_id))
        .is_some_and(|value| value.trim() == "1")
}

/// Persist the automatic-naming opt-in for a newly created task.
pub fn set_auto_name_opt_in(store: &Store, task_id: &str) {
    let _ = store.set_setting(&auto_name_key(task_id), "1");
}

/// Read the persisted, applied server filename for a task (if any).
pub fn server_filename_for(store: &Store, task_id: &str) -> Option<String> {
    store
        .get_setting(&server_name_key(task_id))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Persist the applied server filename for a task. Empty names clear the marker
/// and are treated as "no override" by [`server_filename_for`].
pub fn set_server_filename(store: &Store, task_id: &str, name: &str) {
    let _ = store.set_setting(&server_name_key(task_id), name);
}

/// Compute the final publish destination for a task that carries an applied
/// server filename. The name is re-validated through the shared destination
/// resolver, so a hostile or over-long name can never escape the original
/// download folder. Returns `None` when no safe override is available, in
/// which case callers must keep the task's original destination.
///
/// Only the *final* path is affected. The task-owned `.part` working file is
/// always derived from the original destination (see [`task_part_path`]), so a
/// resumed download never has its partial data renamed or orphaned.
pub fn server_named_destination(destination: &str, server_name: Option<&str>) -> Option<PathBuf> {
    let name = server_name?;
    if name.trim().is_empty() {
        return None;
    }
    let parent = Path::new(destination).parent()?;
    let resolved = security::resolve_destination(parent, name);
    if resolved.ok { resolved.data } else { None }
}

/// Pick the filename for a download. The server's `Content-Disposition` is
/// only a sanitized *suggestion* and never overrides an explicit user/URL name;
/// the browser/command layer opts into it by passing an empty/`download`
/// display name (see [`is_automatic_download_name`]). This function performs
/// the selection once, centrally.
pub fn choose_download_filename(display_name: &str, content_disposition: Option<&str>) -> String {
    if is_automatic_download_name(display_name) {
        if let Some(header) = content_disposition {
            if let Some(safe) = security::safe_filename_from_content_disposition(header) {
                return safe;
            }
        }
    }
    display_name.to_string()
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(existing: *const u16, new_name: *const u16, flags: u32) -> i32;
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

/// Atomically publish a verified working file without clobbering an existing
/// destination. Windows `MoveFileExW` with flags=0 fails when the target exists
/// and works on FAT/exFAT as well as NTFS (hard links do not). The working file
/// always sits next to the destination, so no cross-volume copy is needed.
pub fn publish_no_clobber(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let source = wide_path(from);
        let target = wide_path(to);
        // SAFETY: both buffers are owned, nul-terminated UTF-16 and live for
        // the duration of the call; no flags are passed, so no replacement.
        if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 0) } == 0 {
            let error = std::io::Error::last_os_error();
            return Err(if error.raw_os_error() == Some(183) {
                std::io::Error::new(std::io::ErrorKind::AlreadyExists, error)
            } else {
                error
            });
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::hard_link(from, to)?;
        std::fs::remove_file(from)
    }
}

/// Legacy `<destination>.part` name. Kept only so tests and diagnostics can
/// recognise it; the engine never writes to it. An old shared partial may
/// belong to a different task, so it is preserved untouched.
pub fn part_path_for(destination: &str) -> PathBuf {
    let mut s = String::with_capacity(destination.len() + 5);
    s.push_str(destination);
    s.push_str(".part");
    PathBuf::from(s)
}

/// Token derived from a task id that is safe inside a Windows filename.
fn task_file_token(task_id: &str) -> String {
    let token: String = task_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    if token.is_empty() {
        "task".to_string()
    } else {
        token
    }
}

/// Private working file for one task, adjacent to its destination so the final
/// publish stays on the same volume.
///
/// Two downloads aimed at the same destination used to share
/// `<destination>.part` and corrupt each other. Including the task id makes the
/// working file task-owned. The name is length-bounded so adding the suffix
/// cannot push a path over the Windows component limit.
pub fn task_part_path(destination: &str, task_id: &str) -> PathBuf {
    let suffix = format!(".{}.part", task_file_token(task_id));
    let path = PathBuf::from(destination);
    let parent_len = path
        .parent()
        .map(|p| p.as_os_str().len())
        .unwrap_or(0);
    // Leave headroom for a separator and a trailing NUL on Windows.
    let budget = 250usize.saturating_sub(parent_len + 1 + suffix.len());
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("download");
    let stem: String = if file_name.len() > budget {
        let cut = file_name
            .char_indices()
            .nth(budget)
            .map(|(i, _)| i)
            .unwrap_or(file_name.len());
        file_name[..cut].to_string()
    } else {
        file_name.to_string()
    };
    let name = format!("{stem}{suffix}");
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(name),
        _ => PathBuf::from(name),
    }
}

/// Strip request URLs out of a transport error before it is surfaced.
///
/// reqwest renders failures as `error sending request for url (<URL>): ...`.
/// The URL can carry embedded credentials or signed query parameters, so only
/// a generic marker is kept. The error *code* and the remaining description
/// (timeout, connection reset, ...) are preserved for diagnostics.
fn redact_network_error(msg: &str) -> String {
    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;
    while let Some(open) = rest.find('(') {
        let (before, after) = rest.split_at(open);
        out.push_str(before);
        if let Some(close) = after.find(')') {
            let inner = &after[1..close];
            if inner.contains("://") {
                out.push_str("[url redacted]");
                rest = &after[close + 1..];
                continue;
            }
        }
        out.push('(');
        rest = &after[1..];
    }
    out.push_str(rest);
    out
}

fn schedule_allows(schedule: &crate::store::Schedule, day: chrono::Weekday, time: chrono::NaiveTime) -> bool {
    let Ok(start) = chrono::NaiveTime::parse_from_str(&schedule.start_local_time, "%H:%M") else { return false };
    let Ok(end) = chrono::NaiveTime::parse_from_str(&schedule.end_local_time, "%H:%M") else { return false };
    let bit = day.num_days_from_monday();
    let today = schedule.days_mask & (1i64 << bit) != 0;
    if start < end { today && time >= start && time < end }
    else if start > end {
        let yesterday = schedule.days_mask & (1i64 << ((bit + 6) % 7)) != 0;
        (today && time >= start) || (yesterday && time < end)
    } else { false }
}

/// Exponential backoff with bounded, varying jitter. Uses a mix of the process
/// id, attempt number and a monotonic timestamp so concurrent retries do not
/// synchronise (spec §7.6). No extra RNG dependency is needed — the jitter only
/// needs to be unpredictable enough to break lockstep, not cryptographically
/// secure.
fn backoff(attempt: u32) -> Duration {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let exp = 2u32.saturating_pow(attempt.min(6));
    let base_ms = 500u64.saturating_mul(exp as u64);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let entropy = (std::process::id() as u64)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(seq)
        .wrapping_add(nanos);
    let jitter_ms = entropy % 250;
    Duration::from_millis(base_ms.saturating_add(jitter_ms)).min(MAX_BACKOFF)
}

/// Parse an HTTP `Retry-After` header (delta-seconds or HTTP-date). Returns a
/// bounded delay; values above `MAX_BACKOFF` are capped so a hostile server
/// cannot stall a worker indefinitely (spec §7.6).
fn parse_retry_after(value: &str) -> Option<Duration> {
    let value = value.trim();
    if let Ok(secs) = value.parse::<u64>() {
        return Some(Duration::from_secs(secs).min(MAX_BACKOFF));
    }
    // HTTP-date, e.g. "Wed, 21 Oct 2015 07:28:00 GMT".
    use chrono::DateTime;
    if let Ok(date) = DateTime::parse_from_rfc2822(value) {
        let now = chrono::Utc::now();
        let delta = date.with_timezone(&chrono::Utc).signed_duration_since(now);
        if delta > chrono::Duration::zero() {
            return Some(Duration::from_secs(delta.num_seconds().max(0) as u64).min(MAX_BACKOFF));
        }
        return Some(Duration::from_secs(0));
    }
    None
}

async fn run_writer(
    part_path: PathBuf,
    mut rx: mpsc::Receiver<Chunk>,
    rt: Arc<Runtime>,
    global: Arc<SpeedLimiter>,
    truncate: bool,
) -> std::io::Result<u64> {
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(truncate)
        .open(&part_path)
        .await?;
    let mut since_sync = 0u64;
    let mut total = 0u64;
    while let Some(chunk) = rx.recv().await {
        // A cancel stops further writes immediately; already written bytes
        // remain in the private working file for a safe resume.
        if rt.cancel.load(Ordering::SeqCst) {
            return Ok(total);
        }
        if let Some(committed) = chunk.barrier {
            file.flush().await?;
            file.sync_all().await?;
            let _ = committed.send(());
            continue;
        }
        if chunk.data.is_empty() {
            continue;
        }
        file.seek(SeekFrom::Start(chunk.offset)).await?;
        file.write_all(&chunk.data).await?;
        let len = chunk.data.len() as u64;
        total += len;
        since_sync += len;
        // Drain bytes already queued before acknowledging a pause, without
        // waiting on a bandwidth delay after the network workers have stopped.
        let _ = until_control(&rt, rt.speed_limiter.consume(len)).await;
        let _ = until_control(&rt, global.consume(len)).await;
        if since_sync >= SYNC_EVERY {
            file.flush().await?;
            since_sync = 0;
        }
    }
    file.flush().await?;
    file.sync_all().await?;
    Ok(total)
}

pub async fn hash_file(
    path: &Path,
    algorithm: &str,
    cancel: Option<&AtomicBool>,
) -> Result<String, EngineError> {
    // All four hashers share digest 0.10, so one read pass feeds every algorithm.
    use sha2::Digest;
    if cancel.map(|c| c.load(Ordering::SeqCst)).unwrap_or(false) {
        return Err(EngineError::cancelled());
    }
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| EngineError::internal(format!("Could not open file for hashing: {e}")))?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut sha256 = sha2::Sha256::new();
    let mut sha512 = sha2::Sha512::new();
    let mut sha1 = sha1::Sha1::new();
    let mut md5 = md5::Md5::new();
    loop {
        if cancel.map(|c| c.load(Ordering::SeqCst)).unwrap_or(false) {
            return Err(EngineError::cancelled());
        }
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut buf)
            .await
            .map_err(|e| EngineError::internal(format!("Read error while hashing: {e}")))?;
        if n == 0 {
            break;
        }
        sha256.update(&buf[..n]);
        sha512.update(&buf[..n]);
        sha1.update(&buf[..n]);
        md5.update(&buf[..n]);
    }
    Ok(match algorithm.to_ascii_lowercase().as_str() {
        "sha1" => hex(&sha1.finalize()),
        "md5" => hex(&md5.finalize()),
        "sha512" => hex(&sha512.finalize()),
        "sha256" => hex(&sha256.finalize()),
        other => {
            return Err(EngineError::internal(format!(
                "Unsupported checksum algorithm: {other}"
            )))
        }
    })
}

fn hash_file_blocking(path: &Path, algorithm: &str) -> Result<String, EngineError> {
    use sha2::Digest;
    let mut file = std::fs::File::open(path)
        .map_err(|e| EngineError::internal(format!("Could not open file for hashing: {e}")))?;
    use std::io::Read;
    let mut buf = vec![0u8; 64 * 1024];
    let mut sha256 = sha2::Sha256::new();
    let mut sha512 = sha2::Sha512::new();
    let mut sha1 = sha1::Sha1::new();
    let mut md5 = md5::Md5::new();
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| EngineError::internal(format!("Read error while hashing: {e}")))?;
        if n == 0 {
            break;
        }
        sha256.update(&buf[..n]);
        sha512.update(&buf[..n]);
        sha1.update(&buf[..n]);
        md5.update(&buf[..n]);
    }
    Ok(match algorithm.to_ascii_lowercase().as_str() {
        "sha1" => hex(&sha1.finalize()),
        "md5" => hex(&md5.finalize()),
        "sha512" => hex(&sha512.finalize()),
        "sha256" => hex(&sha256.finalize()),
        other => {
            return Err(EngineError::internal(format!(
                "Unsupported checksum algorithm: {other}"
            )))
        }
    })
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn reveal_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Nothing to open".to_string());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open '{path}': {e}"))
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open '{path}': {e}"))
    }
}

// ---------------------------------------------------------------------------
// OS integration (Windows-only; no-op on other platforms)
// ---------------------------------------------------------------------------

/// Windows execution-state flags for `SetThreadExecutionState`.
#[cfg(windows)]
const ES_CONTINUOUS: u32 = 0x80000000;
#[cfg(windows)]
const ES_SYSTEM_REQUIRED: u32 = 0x00000001;

// Tell the OS to keep the system awake while downloads are active.
// Called when a task starts transferring; released when it finishes.
#[cfg(windows)]
unsafe extern "system" {
    fn SetThreadExecutionState(flags: u32) -> u32;
}

pub fn set_prevent_sleep(enable: bool) {
    #[cfg(windows)]
    unsafe {
        if enable {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
        } else {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
    #[cfg(not(windows))]
    let _ = enable;
}

/// Add or remove SwiftFetch from the Windows user Run key so it starts on login.
pub fn set_start_on_login(enable: bool) -> Result<(), &'static str> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|_| "Could not locate the installed application")?;
        let key = windows_registry::CURRENT_USER.create(r"Software\Microsoft\Windows\CurrentVersion\Run")
            .map_err(|_| "Could not access the Windows startup setting")?;
        if enable {
            let command = format!("\"{}\"", exe.display());
            key.set_string("SwiftFetch", &command).map_err(|_| "Could not enable Windows startup")?;
            if key.get_string("SwiftFetch").ok().as_deref() != Some(command.as_str()) { return Err("Windows did not confirm the startup setting"); }
        } else {
            if key.get_string("SwiftFetch").is_ok() {
                key.remove_value("SwiftFetch").map_err(|_| "Could not disable Windows startup")?;
            }
        }
    }
    #[cfg(not(windows))]
    let _ = enable;
    Ok(())
}

/// Power off the machine once all active work has drained.
pub fn request_shutdown() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("shutdown")
            .args(["/s", "/t", "0"])
            .output();
    }
    #[cfg(not(windows))]
    {}
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Checksum algorithm coverage
// ---------------------------------------------------------------------------

#[cfg(test)]
mod hash_tests {
    use super::*;

    /// Hash `content` with `algorithm` via a temp file and return the hex digest.
    fn hash_of(content: &[u8], algorithm: &str) -> String {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        path.push(format!(
            "swiftfetch-hash-{}-{nanos}.bin",
            std::process::id()
        ));
        std::fs::write(&path, content).expect("write hash input");
        let result = hash_file_blocking(&path, algorithm).expect("hash");
        let _ = std::fs::remove_file(&path);
        result
    }

    /// Regression: the engine once hashed every file with SHA-256 no matter which
    /// algorithm was requested, so any user-supplied SHA-1/MD5/SHA-512 checksum
    /// always failed. These are the known digests of the ASCII string "abc".
    #[test]
    fn hashes_each_supported_algorithm_correctly() {
        assert_eq!(
            hash_of(b"abc", "sha256"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hash_of(b"abc", "sha1"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hash_of(b"abc", "md5"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hash_of(b"abc", "sha512"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn rejects_an_unsupported_algorithm() {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "swiftfetch-hash-bad-{}.bin",
            std::process::id()
        ));
        std::fs::write(&path, b"abc").expect("write input");
        let err = hash_file_blocking(&path, "crc32").unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(
            err.to_string().contains("Unsupported checksum algorithm"),
            "unexpected error: {err}"
        );
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn overnight_schedules_use_the_day_the_window_started() {
        let schedule = crate::store::Schedule {
            id: "overnight".to_string(), queue_id: "default".to_string(), days_mask: 1,
            start_local_time: "22:00".to_string(), end_local_time: "06:00".to_string(),
            action_after: "nothing".to_string(), enabled: true,
        };
        let at = |hour| chrono::NaiveTime::from_hms_opt(hour, 0, 0).unwrap();
        assert!(schedule_allows(&schedule, chrono::Weekday::Mon, at(23)));
        assert!(schedule_allows(&schedule, chrono::Weekday::Tue, at(1)));
        assert!(!schedule_allows(&schedule, chrono::Weekday::Tue, at(6)));
        assert!(!schedule_allows(&schedule, chrono::Weekday::Tue, at(23)));
        let sunday = crate::store::Schedule { days_mask: 64, ..schedule };
        assert!(schedule_allows(&sunday, chrono::Weekday::Mon, at(1)));
    }

    #[tokio::test]
    async fn pause_interrupts_a_pending_network_operation() {
        let runtime = Runtime::new(0);
        runtime.paused.store(true, Ordering::SeqCst);
        let result = until_control(&runtime, std::future::pending::<()>()).await;
        assert!(result.is_err());
    }

    #[test]
    fn network_error_messages_do_not_leak_urls_or_credentials() {
        let err = EngineError::network(
            "error sending request for url (https://user:topsecret@example.com/dl?token=abc): tcp error",
        );
        assert!(!err.message.contains("example.com"));
        assert!(!err.message.contains("topsecret"));
        assert!(!err.message.contains("token=abc"));
        assert!(err.message.contains("Network error"));
    }

    #[test]
    fn redirect_policy_is_enforced_on_the_final_url() {
        assert!(check_redirect_target(
            "https://example.com/a",
            "https://cdn.example.com/b"
        )
        .is_ok());
        assert!(check_redirect_target("https://example.com/a", "http://example.com/b").is_err());
        assert!(check_redirect_target("https://example.com/a", "file:///C:/Windows/x").is_err());
    }

    #[test]
    fn content_range_is_parsed_strictly() {
        let cr = parse_content_range("bytes 100-199/1000").unwrap();
        assert_eq!(cr.start, 100);
        assert_eq!(cr.end, 199);
        assert_eq!(cr.total, Some(1000));

        assert_eq!(parse_content_range("bytes 0-99/*").unwrap().total, None);
        // A truncated total, a reversed range, an end past the total and a
        // missing unit are all rejected.
        assert!(parse_content_range("bytes 0-99/").is_none());
        assert!(parse_content_range("bytes 99-0/1000").is_none());
        assert!(parse_content_range("bytes 0-1000/1000").is_none());
        assert!(parse_content_range("0-99/1000").is_none());
    }

    #[test]
    fn body_is_clamped_to_the_assigned_range() {
        assert_eq!(remaining_in_segment(0, 99), 100);
        assert_eq!(remaining_in_segment(100, 99), 0);
        assert_eq!(remaining_in_segment(7, END_TO_EOF), u64::MAX);
    }
}
