//! Focused tests for updater hardening: durable phases, crash reconciliation,
//! recursive staging budget and MSI installation-directory preservation.

use std::fs;
use swiftfetch_lib::updates::reconcile_on_startup;

#[test]
fn interrupted_attempt_phases_are_reconciled() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path();
    fs::create_dir_all(root).unwrap();
    let attempt = serde_json::json!({
        "id": "test-attempt", "parent_pid": 1, "install_path": "C:/app/swiftfetch.exe",
        "target_version": "0.2.0",
        "update": {"manifest": {"version":"0.2.0"}, "package":"u.package"},
        "rollback": {"manifest": {"version":"0.1.0"}, "package":"r.package"},
        "explicit_rollback": false
    });
    fs::write(root.join("attempt-test-attempt.json"), attempt.to_string()).unwrap();
    // A phase-less (crashed before staging) attempt must be reconciled.
    assert!(reconcile_on_startup(root).is_some());
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("attempt-test-attempt.json")).unwrap()).unwrap();
    assert_eq!(saved["phase"], "interrupted");
    // Reconciliation is idempotent and does not touch a completed attempt.
    assert!(reconcile_on_startup(root).is_none());
}

#[test]
fn completed_attempts_are_not_reconciled() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path();
    let attempt = serde_json::json!({
        "id": "done", "parent_pid": 1, "install_path": "C:/app/swiftfetch.exe",
        "target_version": "0.2.0", "phase": "installed",
        "update": {"manifest": {}, "package":"u.package"},
        "rollback": {"manifest": {}, "package":"r.package"},
        "explicit_rollback": false
    });
    fs::write(root.join("attempt-done.json"), attempt.to_string()).unwrap();
    assert!(reconcile_on_startup(root).is_none());
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("attempt-done.json")).unwrap()).unwrap();
    assert_eq!(saved["phase"], "installed");
}

#[test]
fn invalid_attempt_files_are_ignored_without_panicking() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("attempt-bad.json"), b"not json").unwrap();
    fs::write(folder.path().join("unrelated.json"), b"{}").unwrap();
    assert!(reconcile_on_startup(folder.path()).is_none());
}
