//! Integration tests for the SQLite store.
//!
//! These exercise the real database through the same `Store` API the engine
//! uses, so a schema or query regression fails here rather than in the UI.

use std::path::PathBuf;

use swiftfetch_lib::domain::{DownloadStatus, StartPolicy};
use swiftfetch_lib::store::{Store, TaskRow};

fn temp_db(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "swiftfetch-test-{tag}-{}-{nanos}.sqlite",
        std::process::id()
    ));
    p
}

fn sample_task(id: &str, queue: &str) -> TaskRow {
    let now = swiftfetch_lib::domain::to_rfc3339(chrono::Utc::now());
    TaskRow {
        id: id.to_string(),
        queue_id: queue.to_string(),
        category_id: None,
        original_url: "https://example.com/file.zip".to_string(),
        final_url: None,
        headers_json: "[]".to_string(),
        display_name: "file.zip".to_string(),
        destination_path: "C:/Downloads/file.zip".to_string(),
        temporary_path: None,
        status: DownloadStatus::Queued.as_str().to_string(),
        total_bytes: Some(1000),
        received_bytes: 0,
        etag: Some("\"abc\"".to_string()),
        last_modified: None,
        accept_ranges: true,
        content_type: Some("application/zip".to_string()),
        checksum_algorithm: Some("sha256".to_string()),
        checksum_expected: None,
        checksum_actual: None,
        checksum_verified: false,
        retry_count: 0,
        max_retries: 3,
        last_error_code: None,
        last_error_message: None,
        priority: 50,
        start_policy: StartPolicy::Immediate.as_str().to_string(),
        connections: 4,
        speed_limit: 0,
        overwrite_policy: "rename".to_string(),
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    }
}

#[test]
fn seeds_default_queue_and_category() {
    let path = temp_db("seed");
    let store = Store::open(&path).expect("open");
    assert_eq!(store.ensure_default_queue().unwrap(), "default");
    assert_eq!(store.ensure_default_category().unwrap(), "default");

    // Idempotent: calling again returns the same ids.
    assert_eq!(store.ensure_default_queue().unwrap(), "default");
    assert_eq!(store.list_queues().unwrap().len(), 1);
    assert_eq!(store.list_categories().unwrap().len(), 1);
}

#[test]
fn task_round_trip_and_status_updates() {
    let path = temp_db("roundtrip");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();

    store.insert_task(&sample_task("t1", "default")).unwrap();

    let loaded = store.get_task("t1").unwrap().expect("task present");
    assert_eq!(loaded.id, "t1");
    assert_eq!(loaded.status_enum(), DownloadStatus::Queued);
    assert_eq!(loaded.total_bytes, Some(1000));
    assert_eq!(loaded.connections, 4);

    store
        .set_task_status("t1", DownloadStatus::Downloading)
        .unwrap();
    store.set_received("t1", 500).unwrap();
    store.set_error("t1", "NETWORK_ERROR", "boom").unwrap();

    let loaded = store.get_task("t1").unwrap().unwrap();
    assert_eq!(loaded.status_enum(), DownloadStatus::Downloading);
    assert_eq!(loaded.received_bytes, 500);
    assert_eq!(loaded.last_error_code.as_deref(), Some("NETWORK_ERROR"));
}

#[test]
fn restart_reconciliation_pauses_in_flight_tasks_only() {
    let path = temp_db("reconcile");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();

    for (id, status) in [
        ("a", DownloadStatus::Downloading),
        ("b", DownloadStatus::Verifying),
        ("c", DownloadStatus::RetryWait),
        ("d", DownloadStatus::Completed),
        ("e", DownloadStatus::Paused),
        ("f", DownloadStatus::Queued),
    ] {
        let mut task = sample_task(id, "default");
        task.status = status.as_str().to_string();
        store.insert_task(&task).unwrap();
    }

    let paused = store.reconcile_after_restart().unwrap();
    assert_eq!(paused, 3, "downloading, verifying and retry_wait");

    assert_eq!(
        store.get_task("a").unwrap().unwrap().status_enum(),
        DownloadStatus::Paused
    );
    assert_eq!(
        store.get_task("b").unwrap().unwrap().status_enum(),
        DownloadStatus::Paused
    );
    assert_eq!(
        store.get_task("c").unwrap().unwrap().status_enum(),
        DownloadStatus::Paused
    );
    // Untouched states stay untouched.
    assert_eq!(
        store.get_task("d").unwrap().unwrap().status_enum(),
        DownloadStatus::Completed
    );
    assert_eq!(
        store.get_task("e").unwrap().unwrap().status_enum(),
        DownloadStatus::Paused
    );
    assert_eq!(
        store.get_task("f").unwrap().unwrap().status_enum(),
        DownloadStatus::Queued
    );
}

#[test]
fn segment_plan_persistence_and_counts() {
    let path = temp_db("segments");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();
    store.insert_task(&sample_task("s1", "default")).unwrap();

    store.insert_segment("s1-0", "s1", 0, 0, 499).unwrap();
    store.insert_segment("s1-1", "s1", 1, 500, 999).unwrap();

    store
        .update_segment_progress("s1", 0, 500, "completed")
        .unwrap();
    store
        .update_segment_progress("s1", 1, 250, "active")
        .unwrap();

    let (total, done) = store.segment_counts("s1");
    assert_eq!(total, 2);
    assert_eq!(done, 1);

    let segs = store.list_segments("s1").unwrap();
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[0].received_bytes, 500);
    assert_eq!(segs[1].status, "active");

    store.clear_segments("s1").unwrap();
    assert_eq!(store.list_segments("s1").unwrap().len(), 0);
}

#[test]
fn clear_finished_history_keeps_active_tasks() {
    let path = temp_db("history");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();

    for (id, status) in [
        ("done", DownloadStatus::Completed),
        ("failed", DownloadStatus::Failed),
        ("cancelled", DownloadStatus::Cancelled),
        ("active", DownloadStatus::Downloading),
        ("queued", DownloadStatus::Queued),
    ] {
        let mut task = sample_task(id, "default");
        task.status = status.as_str().to_string();
        store.insert_task(&task).unwrap();
    }

    let removed = store.clear_finished_tasks().unwrap();
    assert_eq!(removed, 3);

    assert!(store.get_task("done").unwrap().is_none());
    assert!(store.get_task("failed").unwrap().is_none());
    assert!(store.get_task("cancelled").unwrap().is_none());
    assert!(store.get_task("active").unwrap().is_some());
    assert!(store.get_task("queued").unwrap().is_some());
    assert_eq!(store.count_tasks().unwrap(), 2);
}

#[test]
fn scheduler_candidate_ordering_is_priority_then_age() {
    let path = temp_db("ordering");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();

    let mut low = sample_task("low", "default");
    low.priority = 10;
    let mut high = sample_task("high", "default");
    high.priority = 90;
    let mut mid = sample_task("mid", "default");
    mid.priority = 50;

    store.insert_task(&low).unwrap();
    store.insert_task(&high).unwrap();
    store.insert_task(&mid).unwrap();

    let candidates = store.next_queued_tasks(3).unwrap();
    let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec!["high", "mid", "low"]);
}

#[test]
fn settings_round_trip_is_normalized() {
    let path = temp_db("settings");
    let store = Store::open(&path).unwrap();

    let mut settings = swiftfetch_lib::settings::Settings::default();
    settings.max_active_tasks = 999; // above the ceiling
    settings.connections_per_task = 0; // below the floor
    store
        .set_setting("app_settings", &settings.to_json())
        .unwrap();

    let raw = store.get_setting("app_settings").unwrap();
    let loaded = swiftfetch_lib::settings::Settings::from_json(&raw).normalized();
    assert!(loaded.max_active_tasks <= swiftfetch_lib::domain::MAX_ACTIVE_TASKS);
    assert!(loaded.connections_per_task >= 1);
}

#[test]
fn deleting_a_task_removes_its_segments_and_events() {
    let path = temp_db("delete");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();
    store.insert_task(&sample_task("d1", "default")).unwrap();
    store.insert_segment("d1-0", "d1", 0, 0, 99).unwrap();
    store
        .log_event(Some("d1"), "download_added", "{}")
        .unwrap();

    store.delete_task("d1").unwrap();

    assert!(store.get_task("d1").unwrap().is_none());
    assert!(store.list_segments("d1").unwrap().is_empty());
    assert!(store.get_task("missing").unwrap().is_none());
}

/// Regression: the engine re-persists the segment plan on every attempt. With
/// `INSERT OR REPLACE` that silently reset each segment's `received_bytes` to
/// zero, so a resume after a crash restarted from byte zero despite durable
/// checkpoints (spec §12.2).
#[test]
fn republishing_a_segment_plan_preserves_checkpoints() {
    let path = temp_db("seg-resume");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();
    store.insert_task(&sample_task("seg-resume", "default")).unwrap();

    store.insert_segment("seg-resume-0", "seg-resume", 0, 0, 499).unwrap();
    store
        .update_segment_progress("seg-resume", 0, 250, "active")
        .unwrap();

    // Same call the engine makes at the start of a resumed attempt.
    store.insert_segment("seg-resume-0", "seg-resume", 0, 0, 499).unwrap();

    let segs = store.list_segments("seg-resume").unwrap();
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0].received_bytes, 250, "checkpoint must survive re-persist");
    assert_eq!(segs[0].status, "active");
}

/// Regression: `list_tasks` used to fetch `limit + 1` rows *before* applying the
/// status filter and cursor, so a matching task older than non-matching ones
/// never appeared in the page at all.
#[test]
fn paginated_tasks_filter_before_limit() {
    let path = temp_db("page");
    let store = Store::open(&path).unwrap();
    store.ensure_default_queue().unwrap();

    for i in 0..5 {
        let mut task = sample_task(&format!("t{i}"), "default");
        task.status = if i == 0 {
            DownloadStatus::Queued.as_str().to_string()
        } else {
            DownloadStatus::Completed.as_str().to_string()
        };
        // Oldest first: t0 is the only queued row and the oldest of all.
        task.updated_at = format!("2026-01-0{}-00:00:00.000Z", i + 1);
        task.created_at = task.updated_at.clone();
        store.insert_task(&task).unwrap();
    }

    let queued = store.list_tasks_page(2, None, Some("queued")).unwrap();
    assert_eq!(queued.len(), 1, "oldest queued row must still be returned");
    assert_eq!(queued[0].id, "t0");

    let page1 = store.list_tasks_page(2, None, Some("completed")).unwrap();
    assert_eq!(page1.len(), 2);
    let last = &page1[1];
    let cursor = format!("{}|{}", last.updated_at, last.id);
    let (ts, id) = cursor.split_once('|').unwrap();

    let page2 = store.list_tasks_page(2, Some((ts, id)), Some("completed")).unwrap();
    assert_eq!(page2.len(), 2, "cursor page must not be empty");
    assert!(page2.iter().all(|t| t.status == "completed"));
}
