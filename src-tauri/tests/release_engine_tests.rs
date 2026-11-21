//! Focused release-critical regression tests for the core transfer engine.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use swiftfetch_lib::domain::codes;
use swiftfetch_lib::engine::{
    choose_download_filename, classify_writer_error, disk_full_error, hash_file,
    publish_no_clobber, range_response_is_valid, representation_matches, Engine,
};

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut p = std::env::temp_dir();
    p.push(format!("swiftfetch-release-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn content_disposition_name_is_sanitized_and_never_overrides_user_choice() {
    let header = "attachment; filename=\"../../etc/passwd\"";
    assert_eq!(
        choose_download_filename("download", Some(header)),
        "passwd"
    );
    assert_eq!(
        choose_download_filename("my-file.zip", Some(header)),
        "my-file.zip"
    );
    assert_eq!(choose_download_filename("chosen.zip", None), "chosen.zip");
}

#[test]
fn http_200_is_rejected_for_fixed_ranges_only() {
    assert!(!range_response_is_valid(reqwest::StatusCode::OK, 0, 100));
    // Even the first fixed segment must not accept a full 200 response.
    assert!(!range_response_is_valid(reqwest::StatusCode::OK, 0, 99));
    assert!(range_response_is_valid(
        reqwest::StatusCode::OK,
        0,
        u64::MAX
    ));
    assert!(range_response_is_valid(
        reqwest::StatusCode::PARTIAL_CONTENT,
        100,
        199
    ));
}

#[test]
fn changed_segment_validators_are_rejected() {
    let expected = Some("\"v1\"".to_string());
    assert!(representation_matches(&expected, Some("\"v1\"".to_string())));
    assert!(representation_matches(&expected, None));
    assert!(!representation_matches(&expected, Some("\"v2\"".to_string())));
    assert!(representation_matches(&None, Some("\"v2\"".to_string())));
}

#[test]
fn host_budget_key_is_stable_per_origin() {
    assert_eq!(
        Engine::host_key("https://cdn.example.com/a/file.zip"),
        Engine::host_key("https://cdn.example.com/b/other.zip")
    );
    assert_ne!(
        Engine::host_key("https://cdn.example.com/a"),
        Engine::host_key("http://cdn.example.com/a")
    );
    assert_ne!(
        Engine::host_key("https://a.example.com/a"),
        Engine::host_key("https://b.example.com/a")
    );
}

#[test]
fn no_clobber_publish_never_replaces_an_existing_file() {
    let dir = temp_dir("publish");
    let source = dir.join("verified.part");
    let target = dir.join("final.bin");
    std::fs::write(&source, b"VERIFIED").unwrap();
    std::fs::write(&target, b"ORIGINAL").unwrap();

    let err = publish_no_clobber(&source, &target).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read(&target).unwrap(), b"ORIGINAL");
    assert!(source.exists(), "working file retained on collision");

    std::fs::remove_file(&target).unwrap();
    publish_no_clobber(&source, &target).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"VERIFIED");
    assert!(!source.exists());
}

#[test]
fn disk_writer_failures_are_classified_safely() {
    assert_eq!(
        classify_writer_error(&std::io::Error::from_raw_os_error(112)),
        codes::DISK_FULL
    );
    assert_eq!(
        classify_writer_error(&std::io::Error::from_raw_os_error(39)),
        codes::DISK_FULL
    );
    assert_eq!(
        classify_writer_error(&std::io::Error::from_raw_os_error(5)),
        codes::PERMISSION_DENIED
    );
    assert_eq!(
        classify_writer_error(&std::io::Error::new(
            std::io::ErrorKind::Other,
            "unexpected"
        )),
        codes::INTERNAL
    );

    // Disk full is terminal: retrying cannot create space and would risk
    // duplicating already-written bytes.
    let err = disk_full_error(4096);
    assert_eq!(err.code, codes::DISK_FULL);
    assert!(!err.retryable);
    assert!(err.message.contains("4096"));
    assert!(!err.message.contains("http"));
}

#[tokio::test]
async fn hashing_stops_when_cancelled() {
    let dir = temp_dir("hash");
    let file = dir.join("file.bin");
    std::fs::write(&file, b"abc").unwrap();
    let cancel = AtomicBool::new(true);
    let err = hash_file(&file, "sha256", Some(&cancel)).await.unwrap_err();
    assert_eq!(err.code, codes::INVALID_ACTION);
    assert_eq!(err.message, "Download was cancelled.");
}
