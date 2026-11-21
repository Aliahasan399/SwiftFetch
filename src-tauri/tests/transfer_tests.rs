//! End-to-end transfer tests.
//!
//! These start the real SwiftFetch fixture server (tools/fixtures/server.py) on
//! a free loopback port, then drive the actual transfer engine and assert on
//! the bytes that land on disk.
//!
//! The engine is exercised through its public surface (`spawn_task`, `control`,
//! the store) rather than through Tauri's IPC layer. Tauri's mock runtime is
//! not usable here: enabling its `test` feature in a binary crate that also
//! links `tauri-runtime-wry` produces a test executable that Windows refuses to
//! load (`STATUS_ENTRYPOINT_NOT_FOUND`). The command layer is covered by the
//! frontend regression suite and by `cargo check`; this file covers the part
//! that actually moves bytes over HTTP.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use swiftfetch_lib::domain::{codes, to_rfc3339, DownloadStatus, StartPolicy};
use swiftfetch_lib::engine::{task_part_path, Engine, RecordingSink};
use swiftfetch_lib::ratelimit::SpeedLimiter;
use swiftfetch_lib::settings::Settings;
use swiftfetch_lib::store::{Store, TaskRow};

// Cargo runs test binaries with the package directory as cwd; the fixture
// server lives one level up under tools/.
const FIXTURE_SERVER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tools/fixtures/server.py"
);
const CHUNK: &[u8] = b"SWIFTFETCH-FIXTURE-DATA-CHUNK-0123456789\n";

// ---------------------------------------------------------------------------
// Fixture server harness
// ---------------------------------------------------------------------------

struct Fixture {
    child: Child,
    port: u16,
}

impl Fixture {
    fn start() -> Option<Fixture> {
        let port = free_port()?;
        let python = python_bin()?;
        let mut child = Command::new(&python)
            .arg(FIXTURE_SERVER)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        // Wait for the health endpoint.
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if Instant::now() > deadline {
                let _ = child.kill();
                return None;
            }
            if probe(port) {
                return Some(Fixture { child, port });
            }
            std::thread::sleep(Duration::from_millis(150));
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> Option<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    listener.local_addr().ok().map(|a| a.port())
}

fn python_bin() -> Option<PathBuf> {
    for candidate in ["python", "python3", "py"] {
        if Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

fn probe(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/__health");
    ureq::get(&url)
        .timeout(Duration::from_secs(2))
        .call()
        .map(|r| r.status() == 200)
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Engine harness
// ---------------------------------------------------------------------------

struct Harness {
    store: Arc<Store>,
    engine: Arc<Engine>,
    sink: Arc<RecordingSink>,
}

/// Builds the engine with a recording event sink.
///
/// The engine is runtime-agnostic: events go through a type-erased `EventSink`,
/// so no Tauri app handle is needed here. The application attaches a Tauri
/// sink; this test attaches one that records.
fn harness() -> Option<Harness> {
    let mut db_path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    db_path.push(format!("swiftfetch-e2e-{}-{nanos}.sqlite", std::process::id()));
    let store = Arc::new(Store::open(&db_path).expect("open store"));
    store.ensure_default_queue().expect("default queue");

    let settings = Arc::new(Mutex::new(Settings::default()));
    let limiter = Arc::new(SpeedLimiter::new(0));

    let async_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .expect("http client");

    let sink = Arc::new(RecordingSink::default());
    let shared_sink: Arc<dyn swiftfetch_lib::engine::EventSink> = sink.clone();
    let engine = Arc::new(
        Engine::new(async_client, Arc::clone(&store), settings, limiter)
            .with_shared_sink(shared_sink),
    );

    Some(Harness {
        store,
        engine,
        sink,
    })
}

fn add_task(
    store: &Store,
    url: &str,
    destination: &str,
    connections: i64,
) -> TaskRow {
    let now = to_rfc3339(chrono::Utc::now());
    let task = TaskRow {
        id: format!("e2e-{}", uuid::Uuid::new_v4()),
        queue_id: "default".to_string(),
        category_id: None,
        original_url: url.to_string(),
        final_url: None,
        headers_json: "[]".to_string(),
        display_name: "file.bin".to_string(),
        destination_path: destination.to_string(),
        temporary_path: None,
        status: DownloadStatus::Queued.as_str().to_string(),
        total_bytes: None,
        received_bytes: 0,
        etag: None,
        last_modified: None,
        accept_ranges: false,
        content_type: None,
        checksum_algorithm: None,
        checksum_expected: None,
        checksum_actual: None,
        checksum_verified: false,
        retry_count: 0,
        max_retries: 0,
        last_error_code: None,
        last_error_message: None,
        priority: 50,
        start_policy: StartPolicy::Immediate.as_str().to_string(),
        connections,
        speed_limit: 0,
        overwrite_policy: "rename".to_string(),
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    };
    store.insert_task(&task).expect("insert task");
    task
}

fn wait_for_status(store: &Store, id: &str, want: DownloadStatus, timeout: Duration) -> TaskRow {
    let deadline = Instant::now() + timeout;
    loop {
        let row = store.get_task(id).expect("get task").expect("task");
        if row.status_enum() == want {
            return row;
        }
        if Instant::now() > deadline {
            panic!(
                "timed out waiting for {want}; last status was {} ({:?})",
                row.status, row.last_error_message
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("swiftfetch-e2e-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&p).expect("mkdir");
    p
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn downloads_a_single_stream_file_end_to_end() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("single");
    let dest = dir.join("single.bin");
    let url = fixture.url("/bytes/65536");
    let task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 1);

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(h.store.as_ref(), &task.id, DownloadStatus::Completed, Duration::from_secs(30));

    let bytes = std::fs::read(&dest).expect("read downloaded file");
    assert_eq!(bytes.len(), 65536, "byte count");
    assert_eq!(row.received_bytes, 65536, "recorded received bytes");
    assert!(row.completed_at.is_some(), "completed_at recorded");
    // The partial file must be gone after finalisation.
    assert!(!dest.with_extension("bin.part").exists(), "partial file removed");
}

/// Regression: a server that answers a ranged request with 200 (ignoring
/// `Range`) must not fail a single-stream task. The engine once re-issued
/// `bytes=<length>-` after the body closed, got another 200, and then treated
/// `start != 0` as fatal — failing the task *after* every byte had been
/// written. 4 connections also exercises the path the UI produces.
#[test]
fn completes_a_single_stream_when_the_server_ignores_ranges() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("ignore-range");
    let dest = dir.join("ignore-range.bin");
    let url = fixture.url("/bytes/131072");
    // >1 connection is what the UI sends by default; the probe still reports
    // no range support, so this must stay single-stream and still finish.
    let task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 4);

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Completed,
        Duration::from_secs(30),
    );

    let bytes = std::fs::read(&dest).expect("read downloaded file");
    assert_eq!(bytes.len(), 131072, "byte count");
    assert_eq!(row.received_bytes, 131072, "recorded received bytes");
    assert_eq!(row.last_error_code, None, "no error recorded");
    assert!(row.completed_at.is_some(), "completed_at recorded");
    assert!(
        !dest.with_extension("bin.part").exists(),
        "partial file removed"
    );
}

#[test]
fn downloads_a_segmented_file_end_to_end() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("segmented");
    let dest = dir.join("segmented.bin");
    // Large enough to exceed 2 * MIN_SEGMENT_SIZE so segmentation kicks in.
    let url = fixture.url("/range/1048576");
    let task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 4);

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(h.store.as_ref(), &task.id, DownloadStatus::Completed, Duration::from_secs(45));

    let bytes = std::fs::read(&dest).expect("read downloaded file");
    assert_eq!(bytes.len(), 1048576, "byte count");

    // The content must match the fixture's deterministic body exactly.
    let mut expected = Vec::with_capacity(1048576);
    while expected.len() < 1048576 {
        expected.extend_from_slice(CHUNK);
    }
    expected.truncate(1048576);
    assert_eq!(bytes, expected, "content matches the fixture byte-for-byte");

    assert_eq!(row.received_bytes, 1048576);
    let (total, done) = h.store.segment_counts(&task.id);
    assert_eq!(total, 4, "four segments planned");
    assert_eq!(done, 4, "all segments completed");
}

#[test]
fn falls_back_to_single_stream_when_ranges_are_ignored() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("fallback");
    let dest = dir.join("fallback.bin");
    let url = fixture.url("/ignore-range/262144");
    let task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 4);

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(h.store.as_ref(), &task.id, DownloadStatus::Completed, Duration::from_secs(30));

    let bytes = std::fs::read(&dest).expect("read downloaded file");
    assert_eq!(bytes.len(), 262144);
    assert_eq!(row.received_bytes, 262144);
}

#[test]
fn marks_the_task_failed_on_a_404() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("missing");
    let dest = dir.join("missing.bin");
    let url = fixture.url("/status/404");
    let task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 1);

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(h.store.as_ref(), &task.id, DownloadStatus::Failed, Duration::from_secs(30));

    assert_eq!(row.last_error_code.as_deref(), Some("HTTP_ERROR"));
    assert!(!dest.exists(), "no file written for a 404");
}

#[test]
fn verifies_a_checksum_and_fails_on_mismatch() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("checksum");
    let dest = dir.join("checksum.bin");

    // Ask for a wrong expected hash: the fixture corrupts the body, so the
    // engine must detect the mismatch and fail rather than complete.
    let url = fixture.url("/checksum/0000000000000000000000000000000000000000000000000000000000000000");
    let mut task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 1);
    task.checksum_algorithm = Some("sha256".to_string());
    task.checksum_expected = Some("1111111111111111111111111111111111111111111111111111111111111111".to_string());
    h.store.delete_task(&task.id).expect("delete seeded task");
    h.store.insert_task(&task).expect("insert");

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(h.store.as_ref(), &task.id, DownloadStatus::Failed, Duration::from_secs(30));

    assert_eq!(row.last_error_code.as_deref(), Some("CHECKSUM_MISMATCH"));
    assert!(!dest.exists(), "a failed checksum must not leave the final file");
}

// ---------------------------------------------------------------------------
// Antivirus scanner tests
// ---------------------------------------------------------------------------

/// Write `content` to a temp file and return its path.
fn scan_temp_file(content: &[u8]) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!(
        "swiftfetch-scan-test-{}-{nanos}.bin",
        std::process::id()
    ));
    std::fs::write(&p, content).expect("write test file");
    p
}

/// Build settings with the antivirus scanner enabled and the given command.
fn scanner_settings(command: &str) -> Settings {
    let mut s = Settings::default();
    s.scan_with_antivirus = true;
    s.scanner_command = command.to_string();
    s
}

/// Block on an async future using a throwaway tokio runtime.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
        .block_on(fut)
}

#[test]
fn antivirus_clean_scanner_succeeds() {
    let file = scan_temp_file(b"clean file content");
    let settings = scanner_settings("cmd /C exit 0");
    let harness = harness().expect("engine harness");
    let result = block_on(harness.engine.run_antivirus_scan(&settings, &file));
    assert!(result.is_ok(), "clean scanner should succeed: {result:?}");
}

#[test]
fn antivirus_infected_scanner_fails_with_scanner_failed() {
    let file = scan_temp_file(b"infected file content");
    let settings = scanner_settings("cmd /C exit 1");
    let harness = harness().expect("engine harness");
    let result = block_on(harness.engine.run_antivirus_scan(&settings, &file));
    assert!(result.is_err(), "infected scanner should fail");
    let err = result.unwrap_err();
    assert_eq!(err.code, codes::SCANNER_FAILED);
}

#[test]
fn antivirus_empty_command_fails() {
    let file = scan_temp_file(b"file");
    let settings = scanner_settings("");
    let harness = harness().expect("engine harness");
    let result = block_on(harness.engine.run_antivirus_scan(&settings, &file));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, codes::INTERNAL);
}

#[test]
fn antivirus_scanner_command_is_not_shell_interpolated() {
    // A command with shell metacharacters should be passed literally as the
    // executable name, not interpolated through a shell. If a shell were used,
    // the semicolon would split into two commands; instead the whole first
    // token (including the semicolon) is the executable name, which does not
    // exist, so Command::new fails to launch it.
    let file = scan_temp_file(b"file");
    let settings = scanner_settings("nonexistent-scanner-xyz; del /q C:\\windows\\system32");
    let harness = harness().expect("engine harness");
    let result = block_on(harness.engine.run_antivirus_scan(&settings, &file));
    assert!(result.is_err(), "shell metacharacters must not be interpolated");
    let err = result.unwrap_err();
    assert_eq!(err.code, codes::INTERNAL, "launch failure, not scanner exit code");
}

/// Regression: `/bytes/` advertises `Accept-Ranges: bytes` but always answers
/// 200 with the full body, ignoring Range. With connections > 1 the engine must
/// fall back to a single stream and still complete — not fail with
/// SERVER_DOES_NOT_SUPPORT_RANGES after receiving every byte.
#[test]
fn completes_when_server_advertises_ranges_but_ignores_them() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("lying-ranges");
    let dest = dir.join("lying-ranges.bin");
    let url = fixture.url("/bytes/5242880");
    let task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 4);

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Completed,
        Duration::from_secs(60),
    );

    let bytes = std::fs::read(&dest).expect("read downloaded file");
    assert_eq!(bytes.len(), 5242880, "file must be complete");
    assert_eq!(row.received_bytes, 5242880, "received count must not double-count");
    assert_eq!(row.last_error_code, None, "no error on a successful fallback");
}

/// Build a task and override its overwrite policy before persisting it.
fn add_task_with_policy(
    store: &Store,
    url: &str,
    destination: &str,
    policy: &str,
) -> TaskRow {
    let mut task = add_task(store, url, destination, 1);
    store.delete_task(&task.id).expect("delete seeded task");
    task.overwrite_policy = policy.to_string();
    store.insert_task(&task).expect("insert task with policy");
    task
}

/// Regression: `OverwritePolicy::Skip` returned the existing path from
/// `choose_final_path`, and the final rename then replaced that file. Skip must
/// leave an existing user file byte-for-byte untouched.
#[test]
fn skip_overwrite_policy_leaves_existing_file_untouched() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("skip");
    let dest = dir.join("skip.bin");
    std::fs::write(&dest, b"ORIGINAL USER FILE").expect("seed existing file");
    let url = fixture.url("/bytes/1024");
    let task = add_task_with_policy(h.store.as_ref(), &url, &dest.to_string_lossy(), "skip");

    h.engine.spawn_task(&task.id);
    let _ = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Failed,
        Duration::from_secs(30),
    );

    assert_eq!(
        std::fs::read(&dest).unwrap(),
        b"ORIGINAL USER FILE",
        "an existing file must not be overwritten by the skip policy"
    );
    assert!(swiftfetch_lib::engine::task_part_path(&dest.to_string_lossy(), &task.id).exists(), "new partial retained for an explicit recovery decision");
}

/// The engine must publish `downloading` once bytes begin moving (it used to
/// stay at `starting` until `verifying`), and a cancel issued mid-transfer must
/// settle as `cancelled`, not `failed`.
#[test]
fn publishes_downloading_and_settles_cancel_as_cancelled() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("cancel");
    let dest = dir.join("cancel.bin");
    let url = fixture.url("/slow/1500");
    let task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 1);

    h.engine.spawn_task(&task.id);
    let downloading = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Downloading,
        Duration::from_secs(15),
    );
    assert!(downloading.total_bytes.is_some(), "probe metadata captured");

    let result = h.engine.control(&task.id, "cancel", None);
    assert!(result.ok, "cancel command must succeed: {result:?}");

    let settled = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Cancelled,
        Duration::from_secs(15),
    );
    assert_ne!(settled.status_enum(), DownloadStatus::Failed);
    assert_eq!(settled.last_error_code.as_deref(), None, "cancel is not an error");

    assert!(
        h.sink
            .statuses(&task.id)
            .iter()
            .any(|s| s == "downloading"),
        "a download_status event for `downloading` must be emitted"
    );
}

fn expected_body(size: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        out.extend_from_slice(CHUNK);
    }
    out.truncate(size);
    out
}

/// Two tasks aimed at the same destination must not share a working file or
/// clobber each other's final output.
#[test]
fn concurrent_tasks_use_task_owned_working_files() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("shared-dest");
    let dest = dir.join("shared.bin");
    let dest_s = dest.to_string_lossy().to_string();
    let url = fixture.url("/bytes/2048");
    let a = add_task(h.store.as_ref(), &url, &dest_s, 1);
    let b = add_task(h.store.as_ref(), &url, &dest_s, 1);

    // Finalize sequentially: a simultaneous no-clobber race is expected to
    // reject one publisher safely, which is a separate property. This test
    // verifies the working files stay independent across two tasks.
    h.engine.spawn_task(&a.id);
    let first = wait_for_status(
        h.store.as_ref(),
        &a.id,
        DownloadStatus::Completed,
        Duration::from_secs(30),
    );
    assert_eq!(first.received_bytes, 2048);
    h.engine.spawn_task(&b.id);
    let second = wait_for_status(
        h.store.as_ref(),
        &b.id,
        DownloadStatus::Completed,
        Duration::from_secs(30),
    );
    assert_eq!(second.received_bytes, 2048);

    assert!(!task_part_path(&dest_s, &a.id).exists());
    assert!(!task_part_path(&dest_s, &b.id).exists());
    assert_eq!(std::fs::read(&dest).unwrap(), expected_body(2048));
    // Default Rename policy: the loser publishes a unique sibling, never over
    // the winner's file.
    assert_eq!(
        std::fs::read(dir.join("shared (1).bin")).unwrap(),
        expected_body(2048)
    );
}

#[test]
fn ask_collision_rejects_without_replacing() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("ask");
    let dest = dir.join("ask.bin");
    std::fs::write(&dest, b"ORIGINAL USER FILE").unwrap();
    let url = fixture.url("/bytes/2048");
    let task = add_task_with_policy(h.store.as_ref(), &url, &dest.to_string_lossy(), "ask");

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Failed,
        Duration::from_secs(30),
    );
    assert_eq!(
        row.last_error_code.as_deref(),
        Some(codes::OVERWRITE_CONFIRMATION_REQUIRED)
    );
    assert_eq!(std::fs::read(&dest).unwrap(), b"ORIGINAL USER FILE");
}

#[test]
fn only_explicit_replace_overwrites_the_destination() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("replace");
    let dest = dir.join("replace.bin");
    std::fs::write(&dest, b"ORIGINAL USER FILE").unwrap();
    let url = fixture.url("/bytes/2048");
    let task = add_task_with_policy(h.store.as_ref(), &url, &dest.to_string_lossy(), "replace");

    h.engine.spawn_task(&task.id);
    let _ = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Completed,
        Duration::from_secs(30),
    );
    assert_eq!(std::fs::read(&dest).unwrap(), expected_body(2048));
}

/// A recorded offset with no working file must not be treated as progress.
#[test]
fn stale_offset_without_working_file_restarts_from_zero() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("stale-offset");
    let dest = dir.join("stale.bin");
    let url = fixture.url("/bytes/65536");
    let task = add_task(h.store.as_ref(), &url, &dest.to_string_lossy(), 1);
    h.store.set_received(&task.id, 65536).unwrap();
    assert!(!task_part_path(&dest.to_string_lossy(), &task.id).exists());

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Completed,
        Duration::from_secs(30),
    );
    assert_eq!(row.received_bytes, 65536);
    assert_eq!(std::fs::read(&dest).unwrap(), expected_body(65536));
}

/// Pre-fsync checkpoints can be ahead of disk. A short working file with stale
/// segment progress must restart and produce byte-exact content, never a file
/// with zero-filled holes marked complete.
#[test]
fn truncated_working_file_does_not_get_marked_complete() {
    let fixture = match Fixture::start() {
        Some(f) => f,
        None => {
            eprintln!("SKIP: fixture server or python unavailable");
            return;
        }
    };
    let h = match harness() {
        Some(h) => h,
        None => {
            eprintln!("SKIP: engine harness unavailable");
            return;
        }
    };

    let dir = temp_dir("truncated");
    let dest = dir.join("truncated.bin");
    let dest_s = dest.to_string_lossy().to_string();
    let url = fixture.url("/range/1048576");
    let task = add_task(h.store.as_ref(), &url, &dest_s, 4);
    h.store
        .set_transfer_metadata(
            &task.id,
            Some(1048576),
            Some("\"range-1048576\""),
            None,
            true,
            None,
            None,
            None,
        )
        .unwrap();

    let part = task_part_path(&dest_s, &task.id);
    std::fs::write(&part, vec![0u8; 100]).unwrap();
    for i in 0..4u32 {
        let start = (i as u64) * 262144;
        h.store
            .insert_segment(
                &format!("{}-{}", task.id, i),
                &task.id,
                i as i64,
                start as i64,
                (start + 262143) as i64,
            )
            .unwrap();
        h.store
            .update_segment_progress(&task.id, i as i64, 200000, "active")
            .unwrap();
    }

    h.engine.spawn_task(&task.id);
    let row = wait_for_status(
        h.store.as_ref(),
        &task.id,
        DownloadStatus::Completed,
        Duration::from_secs(45),
    );
    let bytes = std::fs::read(&dest).unwrap();
    assert_eq!(bytes.len(), 1048576);
    assert_eq!(bytes, expected_body(1048576), "no zero-filled holes");
    assert_eq!(row.received_bytes, 1048576);
}
