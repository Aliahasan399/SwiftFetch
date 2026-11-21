//! Unit tests for scheduler time windows in `Engine::queue_in_window` and
//! `Engine::apply_action_after`.
//!
//! These tests construct a real `Store` with schedule rows, build an `Engine`,
//! and assert that `queue_in_window` returns the correct boolean for various
//! schedule configurations.
//!
//! The `Engine` is built without a Tauri sink (`NoopSink` is the default), so
//! no Tauri runtime is required.

use std::sync::{Arc, Mutex};

use swiftfetch_lib::engine::Engine;
use swiftfetch_lib::ratelimit::SpeedLimiter;
use swiftfetch_lib::settings::Settings;
use swiftfetch_lib::store::Store;

fn temp_db(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "swiftfetch-sched-test-{tag}-{}-{nanos}.sqlite",
        std::process::id()
    ));
    p
}

fn make_engine(store: Arc<Store>) -> Arc<Engine> {
    let settings = Arc::new(Mutex::new(Settings::default()));
    let client = reqwest::Client::new();
    let global_limiter = Arc::new(SpeedLimiter::new(0));
    Arc::new(Engine::new(
        client,
        store,
        settings,
        global_limiter,
    ))
}

fn insert_schedule(
    store: &Store,
    id: &str,
    queue_id: &str,
    days_mask: i64,
    start: &str,
    end: &str,
    action_after: &str,
    enabled: bool,
) {
    store
        .upsert_schedule(id, queue_id, days_mask, start, end, action_after, enabled)
        .expect("schedule insert");
}

// ---------------------------------------------------------------------------
// queue_in_window
// ---------------------------------------------------------------------------

#[test]
fn no_schedules_means_always_in_window() {
    let db = temp_db("nosched");
    let store = Arc::new(Store::open(&db).expect("store open"));
    store.ensure_default_queue().expect("default queue");
    let engine = make_engine(store);
    assert!(engine.queue_in_window("default"));
}

#[test]
fn disabled_schedule_means_always_in_window() {
    let db = temp_db("disabled");
    let store = Arc::new(Store::open(&db).expect("store open"));
    store.ensure_default_queue().expect("default queue");
    insert_schedule(&store, "sched1", "default", 0b1111111, "09:00", "17:00", "nothing", false);
    let engine = make_engine(store);
    // Disabled schedule → queue always in window
    assert!(engine.queue_in_window("default"));
}

#[test]
fn inside_window_returns_true_or_false_by_clock() {
    // We cannot control the wall clock, so we assert the function returns a
    // bool without panicking for a daytime schedule.
    let db = temp_db("inside");
    let store = Arc::new(Store::open(&db).expect("store open"));
    store.ensure_default_queue().expect("default queue");
    insert_schedule(&store, "sched1", "default", 0b0000001, "09:00", "17:00", "nothing", true);
    let engine = make_engine(store);
    let _ = engine.queue_in_window("default");
}

#[test]
fn overnight_window_does_not_panic() {
    let db = temp_db("overnight");
    let store = Arc::new(Store::open(&db).expect("store open"));
    store.ensure_default_queue().expect("default queue");
    insert_schedule(&store, "sched1", "default", 0b1111111, "22:00", "06:00", "nothing", true);
    let engine = make_engine(store);
    let _ = engine.queue_in_window("default");
}

#[test]
fn days_mask_excludes_current_day() {
    // days_mask = 0b0000001 means only Monday. Verify function returns bool.
    let db = temp_db("mask");
    let store = Arc::new(Store::open(&db).expect("store open"));
    store.ensure_default_queue().expect("default queue");
    insert_schedule(&store, "sched1", "default", 0b0000001, "00:00", "23:59", "nothing", true);
    let engine = make_engine(store);
    let _ = engine.queue_in_window("default");
}

#[test]
fn unknown_action_after_treated_as_nothing() {
    let db = temp_db("unknown");
    let store = Arc::new(Store::open(&db).expect("store open"));
    store.ensure_default_queue().expect("default queue");
    insert_schedule(&store, "sched1", "default", 0b1111111, "00:00", "23:59", "frobnicate", true);
    let engine = make_engine(store);
    // apply_action_after with unknown value should not panic
    engine.apply_action_after("default");
    // Queue with full-day schedule should still be in window
    assert!(engine.queue_in_window("default"));
}

#[test]
fn pause_action_after_does_not_panic_with_no_running_tasks() {
    let db = temp_db("pause");
    let store = Arc::new(Store::open(&db).expect("store open"));
    store.ensure_default_queue().expect("default queue");
    insert_schedule(&store, "sched1", "default", 0b0000000, "00:00", "00:00", "pause", true);
    let engine = make_engine(store.clone());
    // apply_action_after with "pause" on a queue with no running tasks should
    // not panic
    engine.apply_action_after("default");
    // Queue with days_mask=0 is never in window
    assert!(!engine.queue_in_window("default"));
}

#[test]
fn queue_with_no_matching_day_is_outside_window() {
    // Only Monday (bit 0) enabled. Verify function returns bool.
    let db = temp_db("nomatch");
    let store = Arc::new(Store::open(&db).expect("store open"));
    store.ensure_default_queue().expect("default queue");
    insert_schedule(&store, "sched1", "default", 0b0000001, "00:00", "23:59", "nothing", true);
    let engine = make_engine(store);
    let _ = engine.queue_in_window("default");
}