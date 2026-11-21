//! Encrypted local state tests: headers_json at rest via OS DPAPI.
//!
//! Covers the encryption round trip, legacy plaintext compatibility, fail-closed
//! behaviour on tampered envelopes, export redaction, and honest degradation
//! on platforms without DPAPI. No network or UI involvement.

use std::path::PathBuf;

use swiftfetch_lib::domain::{secrets, to_rfc3339, DownloadStatus, StartPolicy};
use swiftfetch_lib::settings::Settings;
use swiftfetch_lib::store::{Store, TaskRow};

const SECRET_JSON: &str = r#"[{"name":"Cookie","value":"SESSION=TOPSECRET"},{"name":"Authorization","value":"Bearer TOPSECRET"}]"#;

fn temp_db(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "swiftfetch-secrets-{tag}-{}-{nanos}.sqlite",
        std::process::id()
    ));
    p
}

fn new_store(tag: &str) -> Store {
    let store = Store::open(&temp_db(tag)).unwrap();
    store.ensure_default_queue().unwrap();
    store
}

fn task(id: &str, headers_json: &str) -> TaskRow {
    let now = to_rfc3339(chrono::Utc::now());
    TaskRow {
        id: id.to_string(),
        queue_id: "default".to_string(),
        category_id: None,
        original_url: "https://example.com/file.zip?token=TOPSECRET".to_string(),
        final_url: None,
        headers_json: headers_json.to_string(),
        display_name: "file.zip".to_string(),
        destination_path: "D:/Downloads/file.zip".to_string(),
        temporary_path: None,
        status: DownloadStatus::Queued.as_str().to_string(),
        total_bytes: Some(16),
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
        max_retries: 3,
        last_error_code: None,
        last_error_message: None,
        priority: 50,
        start_policy: StartPolicy::Immediate.as_str().to_string(),
        connections: 1,
        speed_limit: 0,
        overwrite_policy: "rename".to_string(),
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    }
}

/// Read the raw stored cell, bypassing transparent decryption. This is what a
/// person with a SQLite viewer (or malware) would actually see on disk.
fn raw_headers(store: &Store, id: &str) -> String {
    let conn = rusqlite::Connection::open_with_flags(
        store.path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    conn.query_row(
        "SELECT headers_json FROM downloads WHERE id = ?1",
        rusqlite::params![id],
        |row| row.get::<_, String>(0),
    )
    .unwrap()
}

/// Write a raw cell directly, to simulate a legacy or tampered on-disk value.
fn write_raw_headers(store: &Store, id: &str, value: &str) {
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
    conn.execute(
        "UPDATE downloads SET headers_json = ?2 WHERE id = ?1",
        rusqlite::params![id, value],
    )
    .unwrap();
}

fn enable_flag(store: &Store) {
    let mut settings = Settings::default();
    settings.encrypt_saved_headers = true;
    store.set_setting("app_settings", &settings.to_json()).unwrap();
}

#[test]
fn flag_defaults_off_and_plaintext_rows_are_untouched() {
    let store = new_store("default-off");
    assert!(
        !Settings::default().encrypt_saved_headers,
        "at-rest encryption must be opt-in"
    );
    store.insert_task(&task("legacy", SECRET_JSON)).unwrap();

    let raw = raw_headers(&store, "legacy");
    assert_eq!(raw, SECRET_JSON, "no setting means plaintext is preserved");
    assert!(!secrets::is_envelope(&raw));
    assert_eq!(store.get_task("legacy").unwrap().unwrap().headers_json, SECRET_JSON);
}

#[cfg(windows)]
#[test]
fn encrypted_headers_round_trip_without_readable_secrets() {
    assert!(secrets::is_available());
    let store = new_store("round-trip");
    enable_flag(&store);

    store.insert_task(&task("enc", SECRET_JSON)).unwrap();

    let raw = raw_headers(&store, "enc");
    assert!(secrets::is_envelope(&raw), "cell must use the recognizable envelope");
    for needle in ["Cookie", "Authorization", "Bearer", "TOPSECRET", "SESSION="] {
        assert!(!raw.contains(needle), "raw cell leaked {needle:?}: {raw}");
    }

    // Readers see the original plaintext transparently.
    assert_eq!(store.get_task("enc").unwrap().unwrap().headers_json, SECRET_JSON);

    // A row inserted before opting in is still readable plaintext (no data loss).
    let store2 = new_store("legacy-mixed");
    store2.insert_task(&task("old", SECRET_JSON)).unwrap();
    enable_flag(&store2);
    assert_eq!(raw_headers(&store2, "old"), SECRET_JSON);
    assert_eq!(store2.get_task("old").unwrap().unwrap().headers_json, SECRET_JSON);
}

#[cfg(windows)]
#[test]
fn migration_is_atomic_idempotent_and_reversible() {
    let store = new_store("migration");
    store.insert_task(&task("m1", SECRET_JSON)).unwrap();
    store.insert_task(&task("m2", r#"[{"name":"X","value":"y"}]"#)).unwrap();

    let first = store.encrypt_saved_secrets(true).unwrap();
    assert!(first.enabled);
    assert_eq!(first.rows_rewritten, 2);
    assert!(secrets::is_envelope(&raw_headers(&store, "m1")));
    assert!(secrets::is_envelope(&raw_headers(&store, "m2")));
    assert_eq!(store.get_task("m1").unwrap().unwrap().headers_json, SECRET_JSON);
    let persisted: swiftfetch_lib::settings::Settings =
        serde_json::from_str(&store.get_setting("app_settings").unwrap()).unwrap();
    assert!(persisted.encrypt_saved_headers);

    // Re-running does not double-encrypt.
    assert_eq!(store.encrypt_saved_secrets(true).unwrap().rows_rewritten, 0);
    assert_eq!(store.get_task("m1").unwrap().unwrap().headers_json, SECRET_JSON);

    // Disabling restores plaintext without losing data.
    let off = store.encrypt_saved_secrets(false).unwrap();
    assert!(!off.enabled);
    assert_eq!(off.rows_rewritten, 2);
    assert_eq!(raw_headers(&store, "m1"), SECRET_JSON);
    assert_eq!(store.get_task("m1").unwrap().unwrap().headers_json, SECRET_JSON);
}

#[cfg(windows)]
#[test]
fn tampered_envelope_fails_closed_without_panicking() {
    let good = secrets::encrypt(SECRET_JSON.as_bytes()).unwrap();
    assert!(secrets::is_envelope(&good));
    assert_eq!(secrets::decrypt(&good).unwrap(), SECRET_JSON);

    // Flip a base64 character inside the payload: DPAPI MAC must reject it.
    let mut tampered = good.clone();
    let last = tampered.len() - 1;
    let replacement = if tampered.as_bytes()[last] == b'A' { 'B' } else { 'A' };
    tampered.replace_range(last..last + 1, &replacement.to_string());
    assert!(secrets::decrypt(&tampered).is_err(), "tampered blob must fail closed");

    // Unknown version byte is malformed, not a panic. ("fwECAw==" = 7F 01 02 03)
    let unknown_version = "sfenc1:fwECAw==";
    assert_eq!(secrets::decrypt(unknown_version), Err(secrets::SecretError::Malformed));

    // Bad base64 is malformed too.
    assert_eq!(
        secrets::decrypt("sfenc1:!!!!"),
        Err(secrets::SecretError::Malformed)
    );

    // At the store boundary an undecryptable cell becomes empty headers, never
    // ciphertext, plaintext secret material, or a panic.
    let store = new_store("tamper-store");
    store.insert_task(&task("tamper", "[]")).unwrap();
    write_raw_headers(&store, "tamper", &tampered);
    let row = store.get_task("tamper").unwrap().unwrap();
    assert_eq!(row.headers_json, "[]", "fail closed to no headers");
}

#[cfg(windows)]
#[test]
fn export_remains_redacted_after_encryption() {
    let store = new_store("export-redaction");
    enable_flag(&store);
    store.insert_task(&task("exp", SECRET_JSON)).unwrap();
    assert!(secrets::is_envelope(&raw_headers(&store, "exp")));

    let exported = store.export_metadata_json().unwrap();
    for needle in ["Cookie", "Authorization", "Bearer", "TOPSECRET", "SESSION=", "sfenc1:", "token="] {
        assert!(!exported.contains(needle), "export leaked {needle:?}: {exported}");
    }
    assert!(exported.contains("file.zip"), "safe display name is retained");
    assert!(exported.contains("https://example.com"), "scheme+host is retained");
}

#[cfg(not(windows))]
#[test]
fn unavailable_platform_degrades_honestly() {
    assert!(!secrets::is_available());
    assert_eq!(secrets::encrypt(SECRET_JSON.as_bytes()), Err(secrets::SecretError::Unavailable));

    let store = new_store("no-dpapi");
    enable_flag(&store);
    // Fail closed: refuse to store readable secrets when encryption was asked
    // for but is impossible, instead of silently keeping plaintext.
    assert!(store.insert_task(&task("blocked", SECRET_JSON)).is_err());
    assert_eq!(store.count_tasks().unwrap(), 0);
    assert!(store.encrypt_saved_secrets(true).is_err());
}

#[cfg(not(windows))]
#[test]
fn legacy_plaintext_still_readable_without_dpapi() {
    let store = new_store("no-dpapi-legacy");
    store.insert_task(&task("plain", SECRET_JSON)).unwrap();
    assert_eq!(
        store.get_task("plain").unwrap().unwrap().headers_json,
        SECRET_JSON
    );
}
