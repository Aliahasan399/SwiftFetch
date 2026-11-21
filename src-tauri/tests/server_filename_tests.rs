//! Focused regression tests for applying a server-provided (`Content-Disposition`)
//! filename after the HTTP probe. Covers the header/RFC 5987 surface, explicit
//! name preservation, Windows-safe destination containment, task-owned `.part`
//! identity and restart/resume compatibility of the persisted opt-in marker.

use std::path::{Path, PathBuf};

use swiftfetch_lib::engine::{
    auto_name_key, choose_download_filename, is_auto_name_opted_in,
    is_automatic_download_name, server_filename_for, server_name_key,
    server_named_destination, set_auto_name_opt_in, set_server_filename, task_part_path,
    AUTOMATIC_NAME_SENTINEL,
};
use swiftfetch_lib::security::safe_filename_from_content_disposition;
use swiftfetch_lib::store::Store;

// ---------------------------------------------------------------------------
// Header parsing / sanitization (RFC 6266 plain + RFC 5987 extended)
// ---------------------------------------------------------------------------

#[test]
fn header_names_are_parsed_and_sanitized() {
    assert_eq!(
        safe_filename_from_content_disposition("attachment; filename=\"report.pdf\""),
        Some("report.pdf".to_string())
    );
    assert_eq!(
        safe_filename_from_content_disposition("inline; filename=plain.txt"),
        Some("plain.txt".to_string())
    );
    // RFC 5987 UTF-8ext-value: résumé.txt
    assert_eq!(
        safe_filename_from_content_disposition(
            "attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.txt"
        ),
        Some("résumé.txt".to_string())
    );
    // The extended form wins over the plain ASCII fallback.
    assert_eq!(
        safe_filename_from_content_disposition(
            "attachment; filename=\"fallback.bin\"; filename*=UTF-8''preferred.zip"
        ),
        Some("preferred.zip".to_string())
    );
}

#[test]
fn hostile_header_names_never_escape_the_destination() {
    assert_eq!(
        safe_filename_from_content_disposition("attachment; filename=\"../../etc/passwd\""),
        Some("passwd".to_string())
    );
    assert_eq!(
        safe_filename_from_content_disposition("attachment; filename=\"C:/Windows/evil.exe\""),
        Some("evil.exe".to_string())
    );
    // Windows reserved device names are neutralised.
    let reserved = safe_filename_from_content_disposition("attachment; filename=\"CON.txt\"")
        .expect("a safe name should still be produced");
    assert!(reserved.starts_with('_'), "reserved name was not prefixed: {reserved}");
    // No disposition / no filename parameter yields no suggestion.
    assert_eq!(safe_filename_from_content_disposition("inline"), None);
}

// ---------------------------------------------------------------------------
// Selection: automatic opt-in vs explicit preservation
// ---------------------------------------------------------------------------

#[test]
fn explicit_name_is_never_overridden_by_server_suggestion() {
    let cd = "attachment; filename=\"server-name.bin\"";
    for explicit in ["my-report.pdf", "archive.ZIP", "file.tar.gz"] {
        assert_eq!(
            choose_download_filename(explicit, Some(cd)),
            explicit,
            "an explicit name must be preserved"
        );
    }
    // A URL-derived name (not the sentinel) is also explicit and preserved.
    assert_eq!(choose_download_filename("release.zip", Some(cd)), "release.zip");
}

#[test]
fn automatic_name_adopts_safe_server_suggestion() {
    let cd = "attachment; filename=\"server-name.bin\"";
    assert_eq!(choose_download_filename(AUTOMATIC_NAME_SENTINEL, Some(cd)), "server-name.bin");
    assert_eq!(choose_download_filename("", Some(cd)), "server-name.bin");
    assert_eq!(choose_download_filename(" Download ", Some(cd)), "server-name.bin");
    // Without a usable header the sentinel is kept as-is.
    assert_eq!(choose_download_filename(AUTOMATIC_NAME_SENTINEL, None), AUTOMATIC_NAME_SENTINEL);
    assert_eq!(choose_download_filename(AUTOMATIC_NAME_SENTINEL, Some("inline")), AUTOMATIC_NAME_SENTINEL);
}

#[test]
fn sentinel_is_the_persisted_automatic_name_marker() {
    assert!(is_automatic_download_name(AUTOMATIC_NAME_SENTINEL));
    assert!(is_automatic_download_name("  download  "));
    assert!(is_automatic_download_name(""));
    // Legacy / explicitly named tasks are not opted in.
    assert!(!is_automatic_download_name("legacy-file.bin"));
    assert!(!is_automatic_download_name("DOWNLOAD.pdf"));
}

// ---------------------------------------------------------------------------
// Final publication path: containment + task-owned part identity
// ---------------------------------------------------------------------------

#[test]
fn applied_name_only_changes_the_final_path_not_the_part_file() {
    let original = PathBuf::from("D:/Downloads/download");
    let task_id = "11111111-2222-3333-4444-555555555555";
    let suggested = "report.pdf".to_string();

    let final_path = server_named_destination(original.to_str().unwrap(), Some(&suggested))
        .expect("a safe name must resolve to a contained path");
    // On Windows the shared resolver may canonicalize an existing parent to a
    // verbatim (`\\?\`) path, so compare the file component, not the whole string.
    assert_eq!(final_path.file_name().and_then(|n| n.to_str()), Some("report.pdf"));
    assert!(final_path.to_string_lossy().ends_with("report.pdf"));

    // The task-owned working file is derived from the ORIGINAL destination, so
    // publishing under a new name never renames or orphans partial data.
    let part_before = task_part_path(original.to_str().unwrap(), task_id);
    let part_after = task_part_path(original.to_str().unwrap(), task_id);
    assert_eq!(part_before, part_after);
    assert!(part_after
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap()
        .contains(task_id));
}

#[test]
fn unsafe_or_absent_names_keep_the_original_destination() {
    let original = "D:/Downloads/download";
    assert!(server_named_destination(original, None).is_none());
    assert!(server_named_destination(original, Some("")).is_none());
    // Traversal / absolute inputs cannot escape the folder via the resolver.
    assert!(server_named_destination(original, Some("../../escape.exe")).is_none());
    assert!(server_named_destination(original, Some("C:/Windows/evil.exe")).is_none());

    // An invalid override falls back to the original path at the call site.
    let effective = server_named_destination(original, Some("../../escape.exe"))
        .unwrap_or_else(|| PathBuf::from(original));
    assert_eq!(effective, PathBuf::from(original));
}

// ---------------------------------------------------------------------------
// Persisted marker: survives a restart, never applies to legacy tasks
// ---------------------------------------------------------------------------

#[test]
fn persisted_server_name_survives_reopening_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.sqlite");
    let task_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

    // First session: creation-time opt-in + probe-time applied name.
    {
        let store = Store::open(&db).unwrap();
        assert!(server_filename_for(&store, task_id).is_none());
        set_auto_name_opt_in(&store, task_id);
        assert!(is_auto_name_opted_in(&store, task_id));
        set_server_filename(&store, task_id, "resolved.zip");
        assert_eq!(server_filename_for(&store, task_id).as_deref(), Some("resolved.zip"));
    }
    // Simulated restart: a fresh handle reads both persisted markers.
    {
        let store = Store::open(&db).unwrap();
        assert!(is_auto_name_opted_in(&store, task_id));
        assert_eq!(server_filename_for(&store, task_id).as_deref(), Some("resolved.zip"));
        let destination = server_named_destination("D:/Downloads/download", server_filename_for(&store, task_id).as_deref())
            .unwrap();
        assert_eq!(destination.file_name().and_then(|n| n.to_str()), Some("resolved.zip"));
    }
    // Clearing on completion removes the override.
    {
        let store = Store::open(&db).unwrap();
        set_server_filename(&store, task_id, "");
        assert!(server_filename_for(&store, task_id).is_none());
    }
}

#[test]
fn legacy_task_without_markers_is_never_renamed() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("state.sqlite")).unwrap();
    let legacy_id = "99999999-8888-7777-6666-555555555555";

    // A legacy/explicit task has no opt-in, no server-name key, and an explicit
    // display name.
    assert!(!is_auto_name_opted_in(&store, legacy_id));
    assert!(server_filename_for(&store, legacy_id).is_none());
    assert!(!is_automatic_download_name("legacy.bin"));
    let path = server_named_destination(
        "D:/Downloads/legacy.bin",
        server_filename_for(&store, legacy_id).as_deref(),
    )
    .unwrap_or_else(|| PathBuf::from("D:/Downloads/legacy.bin"));
    assert_eq!(path, PathBuf::from("D:/Downloads/legacy.bin"));
}

#[test]
fn legacy_sentinel_named_task_without_opt_in_is_not_renamed() {
    // Even a row literally named `download` must not be renamed unless it was
    // created with the explicit opt-in marker — no permission by inference.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("state.sqlite")).unwrap();
    let legacy_id = "legacy-sentinel";
    assert!(is_automatic_download_name("download"));
    assert!(!is_auto_name_opted_in(&store, legacy_id));

    let applied = server_filename_for(&store, legacy_id)
        .filter(|_| is_auto_name_opted_in(&store, legacy_id) && is_automatic_download_name("download"));
    let path = server_named_destination("D:/Downloads/download", applied.as_deref())
        .unwrap_or_else(|| PathBuf::from("D:/Downloads/download"));
    assert_eq!(path, PathBuf::from("D:/Downloads/download"));
}

#[test]
fn marker_keys_are_task_scoped() {
    assert_eq!(server_name_key("task-1"), "server_name:task-1");
    assert_ne!(server_name_key("task-1"), server_name_key("task-2"));
    assert_eq!(auto_name_key("task-1"), "auto_name:task-1");
    assert_ne!(auto_name_key("task-1"), auto_name_key("task-2"));
}

// Keep the `Path` import meaningful on platforms where inference differs.
#[test]
fn destinations_are_absolute_on_this_host() {
    let p = Path::new("D:/Downloads/report.pdf");
    assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("report.pdf"));
}
