//! Core lifecycle tests: secret-free metadata export, retention, recoverable
//! collisions, partial-file policy and quiesce. No network or OS power action.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use swiftfetch_lib::domain::{codes, DownloadStatus, StartPolicy, to_rfc3339};
use swiftfetch_lib::engine::{task_part_path, Engine, RecordingSink};
use swiftfetch_lib::ratelimit::SpeedLimiter;
use swiftfetch_lib::settings::Settings;
use swiftfetch_lib::store::{Store, TaskRow};

fn temp_db(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "swiftfetch-lifecycle-{tag}-{}-{nanos}.sqlite",
        std::process::id()
    ));
    p
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "swiftfetch-lifecycle-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn task(id: &str, dest: &str, status: DownloadStatus) -> TaskRow {
    let now = to_rfc3339(chrono::Utc::now());
    TaskRow {
        id: id.to_string(),
        queue_id: "default".to_string(),
        category_id: None,
        original_url: "https://example.com/file.zip?token=TOPSECRET".to_string(),
        final_url: None,
        headers_json: r#"[{"name":"Authorization","value":"Bearer TOPSECRET"}]"#.to_string(),
        display_name: "file.zip".to_string(),
        destination_path: dest.to_string(),
        temporary_path: None,
        status: status.as_str().to_string(),
        total_bytes: Some(16),
        received_bytes: 0,
        etag: None,
        last_modified: None,
        accept_ranges: false,
        content_type: None,
        checksum_algorithm: Some("sha256".to_string()),
        checksum_expected: Some("abc".to_string()),
        checksum_actual: None,
        checksum_verified: false,
        retry_count: 0,
        max_retries: 3,
        last_error_code: None,
        last_error_message: Some("cookie=TOPSECRET".to_string()),
        priority: 50,
        start_policy: StartPolicy::Immediate.as_str().to_string(),
        connections: 1,
        speed_limit: 0,
        overwrite_policy: "ask".to_string(),
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    }
}

fn make_engine(
    store: Arc<Store>,
    settings: Settings,
) -> (Arc<Engine>, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let shared: Arc<dyn swiftfetch_lib::engine::EventSink> = sink.clone();
    let engine = Arc::new(
        Engine::new(
            reqwest::Client::new(),
            store,
            Arc::new(Mutex::new(settings)),
            Arc::new(SpeedLimiter::new(0)),
        )
        .with_shared_sink(shared),
    );
    (engine, sink)
}

#[test]
fn metadata_export_excludes_request_secrets() {
    let store = Store::open(&temp_db("export")).unwrap();
    store.ensure_default_queue().unwrap();
    let dir = temp_dir("export");
    let dest = dir.join("file.zip");
    store.insert_task(&task("t1", &dest.to_string_lossy(), DownloadStatus::Queued)).unwrap();

    let json = store.export_metadata_json().unwrap();
    assert!(!json.contains("TOPSECRET"), "no URL/header/cookie secrets");
    assert!(!json.contains("Authorization"));
    assert!(!json.contains("cookie="));
    assert!(json.contains("https://example.com"), "host is preserved");
    assert!(json.contains("\"protocol\""));

    let restored = Store::open(&temp_db("import")).unwrap();
    restored.ensure_default_queue().unwrap();
    let n = restored.import_metadata_json(&json).unwrap();
    assert!(n > 0);
    let row = restored.get_task("t1").unwrap().unwrap();
    assert_eq!(row.headers_json, "[]");
    assert_eq!(row.original_url, "https://import.invalid/metadata");
    assert_eq!(row.display_name, "file.zip");
}

#[test]
fn retention_and_event_log_are_bounded() {
    let store = Store::open(&temp_db("retention")).unwrap();
    store.ensure_default_queue().unwrap();
    let dir = temp_dir("retention");

    let mut old = task("done", &dir.join("done.zip").to_string_lossy(), DownloadStatus::Completed);
    old.completed_at = Some("2020-01-01T00:00:00.000Z".to_string());
    store.insert_task(&old).unwrap();
    store.insert_task(&task(
        "active",
        &dir.join("active.zip").to_string_lossy(),
        DownloadStatus::Downloading,
    ))
    .unwrap();

    for _ in 0..50 {
        store.log_event(Some("active"), "checkpoint", "{}").unwrap();
    }

    assert_eq!(store.cleanup_retention(30).unwrap(), 1);
    assert!(store.get_task("done").unwrap().is_none());
    assert!(store.get_task("active").unwrap().is_some());
    store.trim_event_log(10).unwrap();
    assert!(store.list_recent_events(100).unwrap().len() <= 10);
}

#[test]
fn database_backup_is_a_valid_copy() {
    let path = temp_db("backup-src");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();
    store.insert_task(&task("t1", "C:/Downloads/file.zip", DownloadStatus::Queued)).unwrap();

    let backup = temp_db("backup-copy");
    store.backup_database(&backup).unwrap();
    let copy = Store::open(&backup).unwrap();
    assert_eq!(copy.count_tasks().unwrap(), 1);
    assert!(copy.get_task("t1").unwrap().is_some());
}

fn colliding_task(store: &Store, id: &str, dest: &str) {
    let mut row = task(id, dest, DownloadStatus::Failed);
    row.last_error_code = Some(codes::OVERWRITE_CONFIRMATION_REQUIRED.to_string());
    store.insert_task(&row).unwrap();
}

#[test]
fn ask_collision_actions_resolve_without_redownloading() {
    for (action, expect_renamed, expect_original) in
        [("overwrite", false, false), ("rename", true, false), ("skip", false, true)]
    {
        let store = Arc::new(Store::open(&temp_db(action)).unwrap());
        store.ensure_default_queue().unwrap();
        let dir = temp_dir(action);
        let dest = dir.join("file.zip");
        std::fs::write(&dest, b"ORIGINAL").unwrap();
        let id = format!("task-{action}");
        colliding_task(&store, &id, &dest.to_string_lossy());

        let part = task_part_path(&dest.to_string_lossy(), &id);
        std::fs::write(&part, b"VERIFIED-DOWNLOAD").unwrap();

        let (engine, _) = make_engine(store.clone(), Settings::default());
        let result = engine.control(&id, action, None);
        assert!(result.ok, "{action}: {result:?}");
        assert_eq!(
            result.unwrap().status,
            if action == "skip" { DownloadStatus::Cancelled } else { DownloadStatus::Completed },
            "{action}"
        );

        if expect_original {
            assert_eq!(std::fs::read(&dest).unwrap(), b"ORIGINAL");
        } else {
            let published = if expect_renamed {
                dir.join("file (1).zip")
            } else {
                dest.clone()
            };
            assert_eq!(std::fs::read(&published).unwrap(), b"VERIFIED-DOWNLOAD");
        }
        // No working file is left behind after any resolution.
        assert!(!part.exists(), "{action}");
    }
}

#[test]
fn failed_task_cleanup_only_removes_task_owned_files() {
    let store = Arc::new(Store::open(&temp_db("partials")).unwrap());
    store.ensure_default_queue().unwrap();
    let dir = temp_dir("partials");
    let dest = dir.join("file.zip");
    let id = "task-partials";
    store.insert_task(&task(id, &dest.to_string_lossy(), DownloadStatus::Failed)).unwrap();

    let part = task_part_path(&dest.to_string_lossy(), id);
    let legacy = dest.with_extension("zip.part");
    std::fs::write(&part, b"working").unwrap();
    std::fs::write(&legacy, b"legacy").unwrap();

    let settings = Settings {
        keep_partial_on_failure: false,
        ..Settings::default()
    };
    let (engine, _) = make_engine(store, settings);
    engine.cleanup_failed_working_file(id, codes::NETWORK_ERROR);
    assert!(!part.exists(), "opted-out task-owned partial is removed");
    assert!(legacy.exists(), "legacy partial must never be deleted");
}

#[test]
fn quiesce_succeeds_when_idle_and_power_state_starts_clear() {
    let store = Arc::new(Store::open(&temp_db("quiesce")).unwrap());
    store.ensure_default_queue().unwrap();
    let (engine, _) = make_engine(store, Settings::default());
    assert!(!engine.power_ready(), "no shutdown at startup");
    engine.cancel_shutdown();
    assert!(!engine.power_ready());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert!(runtime.block_on(engine.quiesce()).is_ok());
}
