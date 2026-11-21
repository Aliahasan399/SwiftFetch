use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde::Serialize;
use tauri::{Manager, State};
use crate::domain::CommandResult;
use crate::engine::AppEngine;

pub struct RecoveryLocation { pub path: PathBuf, pub message: Mutex<Option<String>> }

#[derive(Serialize)]
pub struct BackupInfo { name: String, size_bytes: u64 }
#[derive(Serialize)]
pub struct RecoveryState { message: Option<String>, data_directory: String, backups: Vec<BackupInfo> }
#[derive(Serialize)]
pub struct RestoreResult { restored_tasks: usize, safety_backup: String, restart_required: bool }

fn selected_backup(location: &RecoveryLocation, name: &str) -> Result<PathBuf, &'static str> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || !name.ends_with(".sqlite") { return Err("Choose a local queue backup."); }
    let folder = location.path.parent().unwrap_or_else(|| Path::new(".")).join("backups");
    let folder = folder.canonicalize().map_err(|_| "The backup folder is unavailable.")?;
    let file = folder.join(name).canonicalize().map_err(|_| "The selected backup is unavailable.")?;
    if !file.starts_with(folder) { return Err("The backup must remain in the local backup folder."); }
    Ok(file)
}

#[tauri::command]
pub fn get_recovery_state(location: State<'_, RecoveryLocation>) -> CommandResult<RecoveryState> {
    let folder = location.path.parent().unwrap_or_else(|| Path::new("."));
    let mut backups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(folder.join("backups")) {
        for entry in entries.flatten().take(1_000) {
            if entry.path().extension().is_some_and(|extension| extension == "sqlite") {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() { backups.push(BackupInfo { name: entry.file_name().to_string_lossy().to_string(), size_bytes: metadata.len() }); }
                }
            }
        }
    }
    backups.sort_by(|left, right| right.name.cmp(&left.name));
    CommandResult::success(RecoveryState {
        message: location.message.lock().ok().and_then(|message| message.clone()),
        data_directory: folder.to_string_lossy().to_string(), backups,
    })
}

pub(crate) fn restore_offline(database: &Path, backup: &Path) -> Result<(usize, PathBuf), &'static str> {
    let folder = database.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(folder.join("backups")).map_err(|_| "Could not create the safety backup folder.")?;
    let stage = folder.join(format!("restore-{}.sqlite", uuid::Uuid::new_v4()));
    let count = crate::store::Store::stage_recovery_backup(backup, &stage, false)
        .map_err(|_| "Backup integrity or schema validation failed. The current queue was not replaced.")?;
    let safety = folder.join("backups").join(format!("before-recovery-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&safety).map_err(|_| "Could not preserve the current queue files.")?;
    let mut moved = Vec::new();
    let result = (|| -> std::io::Result<()> {
        // Keep the main database in place until the validated stage can replace
        // it atomically. Only WAL/SHM must be moved aside first (a stale sidecar
        // would otherwise be replayed onto the restored database). The main db
        // is copied to the safety folder, so a crash before activation still
        // leaves the original at its configured path.
        if database.exists() {
            let saved = safety.join(database.file_name().ok_or_else(|| std::io::Error::other("Invalid queue path"))?);
            std::fs::copy(database, &saved)?;
        }
        for suffix in ["-wal", "-shm"] {
            let old = PathBuf::from(format!("{}{suffix}", database.display()));
            if old.exists() {
                let saved = safety.join(old.file_name().ok_or_else(|| std::io::Error::other("Invalid queue path"))?);
                std::fs::rename(&old, &saved)?;
                moved.push((old, saved));
            }
        }
        std::fs::rename(&stage, database)
    })();
    if let Err(error) = result {
        let mut rollback = Vec::new();
        for (old, saved) in moved.into_iter().rev() {
            if let Err(failure) = std::fs::rename(&saved, &old) {
                rollback.push(format!("{}: {}", saved.display(), failure));
            }
        }
        let _ = std::fs::remove_file(&stage);
        let detail = if rollback.is_empty() {
            String::new()
        } else {
            format!(" Sidecar rollback needs attention: {}.", rollback.join("; "))
        };
        let message = format!("Could not activate the validated backup ({error}). The previous queue was preserved in {}.{}", safety.display(), detail);
        return Err(Box::leak(message.into_boxed_str()));
    }
    Ok((count, safety))
}

#[tauri::command]
pub async fn restore_metadata(app: tauri::AppHandle, backup_name: String, confirm: bool) -> CommandResult<RestoreResult> {
    if !confirm { return CommandResult::failure("CONFIRMATION_REQUIRED", "Confirm replacing the queue with this backup. A safety backup will be kept.", false); }
    let source = {
        let location = app.state::<RecoveryLocation>();
        match selected_backup(&location, &backup_name) { Ok(source) => source, Err(message) => return CommandResult::failure("INVALID_BACKUP", message, false) }
    };
    let database = app.state::<RecoveryLocation>().path.clone();
    let engine = app.try_state::<Arc<AppEngine>>().map(|engine| Arc::clone(engine.inner()));
    if let Some(engine) = engine {
        if engine.quiesce().await.is_err() { return CommandResult::failure("RECOVERY_BUSY", "Active transfers could not be paused. The queue was not replaced.", true); }
        engine.cancel_shutdown();
        let startup = engine.settings().start_on_login;
        let worker = Arc::clone(&engine);
        let restored = tauri::async_runtime::spawn_blocking(move || worker.store().restore_database(&source, startup)).await;
        match restored {
            Ok(Ok((count, safety))) => {
                let settings = engine.store().get_setting("app_settings").map(|raw| crate::settings::Settings::from_json(&raw)).unwrap_or_default().normalized();
                if let Ok(client) = crate::build_http_client(&settings.proxy) { engine.set_http_client(client); }
                engine.set_settings(settings);
                CommandResult::success(RestoreResult { restored_tasks: count, safety_backup: safety.to_string_lossy().to_string(), restart_required: false })
            }
            _ => CommandResult::failure("INVALID_BACKUP", "The backup could not be validated or restored. Existing data was preserved or safety-backed up.", false),
        }
    } else {
        match tauri::async_runtime::spawn_blocking(move || restore_offline(&database, &source)).await {
            Ok(Ok((count, safety))) => CommandResult::success(RestoreResult { restored_tasks: count, safety_backup: safety.to_string_lossy().to_string(), restart_required: true }),
            Ok(Err(message)) => CommandResult::failure("INVALID_BACKUP", message, false),
            Err(_) => CommandResult::failure("RECOVERY_FAILED", "The restore operation did not finish. Preserve the queue and safety files for recovery.", false),
        }
    }
}

#[tauri::command]
pub async fn restart_application(app: tauri::AppHandle) -> CommandResult<()> {
    if let Some(engine) = app.try_state::<Arc<AppEngine>>().map(|engine| Arc::clone(engine.inner())) {
        if engine.quiesce().await.is_err() { return CommandResult::failure("RECOVERY_BUSY", "Wait for transfers to pause before restarting.", true); }
    }
    app.restart()
}

#[tauri::command]
pub fn reset_startup_network(location: State<'_, RecoveryLocation>, confirm: bool) -> CommandResult<()> {
    if !confirm { return CommandResult::failure("CONFIRMATION_REQUIRED", "Confirm resetting the saved proxy setting.", false); }
    let store = match crate::store::Store::open_existing(&location.path) { Ok(store) => store, Err(_) => return CommandResult::failure("STORE_ERROR", "Restore a valid queue backup before resetting network settings.", false) };
    let mut settings = store.get_setting("app_settings").map(|raw| crate::settings::Settings::from_json(&raw)).unwrap_or_default();
    settings.proxy.clear();
    match store.set_setting("app_settings", &settings.to_json()) {
        Ok(()) => CommandResult::success(()), Err(_) => CommandResult::failure("STORE_ERROR", "Could not save the network reset.", false),
    }
}

pub(crate) fn show_startup(app: &tauri::AppHandle, message: &str) -> tauri::Result<()> {
    *app.state::<RecoveryLocation>().message.lock().expect("recovery state poisoned") = Some(message.to_string());
    let mut configuration = app.config().app.windows.iter().find(|window| window.label == "main").cloned().unwrap_or_default();
    configuration.label = "main".to_string();
    configuration.url = tauri::WebviewUrl::App("recovery.html".into());
    tauri::WebviewWindowBuilder::from_config(app, &configuration)?.build()?;
    Ok(())
}

/// A shareable inventory, deliberately not a resumable queue backup: signed
/// URLs, authorization, request headers and local partial identities are absent.
#[tauri::command]
pub fn export_queue(engine: State<'_, Arc<AppEngine>>) -> CommandResult<String> {
    match engine.store().export_metadata_json() {
        Ok(json) => CommandResult::success(json),
        Err(_) => CommandResult::failure("STORE_ERROR", "Could not export queue metadata.", true),
    }
}

#[tauri::command]
pub fn backup_metadata(engine: State<'_, Arc<AppEngine>>) -> CommandResult<String> {
    let Some(parent) = engine.store().path().parent() else { return CommandResult::failure("STORE_ERROR", "The queue has no backup directory.", false) };
    let folder = parent.join("backups");
    if std::fs::create_dir_all(&folder).is_err() { return CommandResult::failure("STORE_ERROR", "Could not create the local backup folder.", false) }
    let path = folder.join(format!("state-{}-{}.sqlite", chrono::Utc::now().format("%Y%m%d-%H%M%S"), uuid::Uuid::new_v4()));
    match engine.store().backup_database(&path) {
        Ok(()) => CommandResult::success(path.to_string_lossy().to_string()),
        Err(_) => CommandResult::failure("STORE_ERROR", "Could not create a consistent local queue backup.", true),
    }
}

#[tauri::command]
pub fn import_queue(engine: State<'_, Arc<AppEngine>>, metadata: String) -> CommandResult<usize> {
    if metadata.len() > 8 * 1024 * 1024 { return CommandResult::failure("INVALID_ACTION", "The inventory exceeds its size limit.", false); }
    match engine.store().import_metadata_json(&metadata) {
        Ok(count) => CommandResult::success(count),
        Err(_) => CommandResult::failure("INVALID_ACTION", "The inventory is invalid. Existing files and tasks were kept.", false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn queue(path: &Path, id: &str) -> Store {
        let store = Store::open(path).unwrap();
        store.ensure_default_queue().unwrap();
        store.ensure_default_category().unwrap();
        let connection = rusqlite::Connection::open(path).unwrap();
        connection.execute("INSERT INTO downloads(id,queue_id,original_url,display_name,destination_path,status,start_policy,overwrite_policy,created_at,updated_at) VALUES(?1,'default','https://example.invalid/a','a.zip','C:/Downloads/a.zip','downloading','immediate','replace','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')", [id]).unwrap();
        store
    }

    #[test]
    fn full_restore_preserves_current_metadata_and_resets_execution_consent() {
        let folder = tempfile::tempdir().unwrap();
        let source = queue(&folder.path().join("source.sqlite"), "restored");
        let mut settings = crate::settings::Settings::default();
        settings.clipboard_capture = true;
        settings.browser_integration = true;
        settings.shutdown_after_complete = true;
        settings.start_on_login = true;
        settings.scan_with_antivirus = true;
        settings.scanner_command = "unapproved-scanner.exe {file}".into();
        settings.overwrite_policy = crate::domain::OverwritePolicy::Replace;
        source.set_setting("app_settings", &settings.to_json()).unwrap();
        let snapshot = folder.path().join("snapshot.sqlite");
        source.backup_database(&snapshot).unwrap();
        let current = queue(&folder.path().join("current.sqlite"), "previous");
        let (count, safety) = current.restore_database(&snapshot, false).unwrap();
        assert_eq!(count, 1);
        let restored = current.get_task("restored").unwrap().unwrap();
        assert_eq!(restored.status, "paused");
        assert_eq!(restored.start_policy, "manual");
        assert_eq!(restored.overwrite_policy, "rename");
        assert!(current.get_task("previous").unwrap().is_none());
        let previous = Store::open_existing(&safety).unwrap();
        assert!(previous.get_task("previous").unwrap().is_some());
        let settings = crate::settings::Settings::from_json(&current.get_setting("app_settings").unwrap());
        assert!(!settings.clipboard_capture && !settings.browser_integration && !settings.shutdown_after_complete);
        assert!(!settings.start_on_login && !settings.scan_with_antivirus);
        assert!(settings.scanner_command.is_empty());
        assert_eq!(settings.overwrite_policy, crate::domain::OverwritePolicy::Rename);
    }

    #[test]
    fn incomplete_backup_schema_cannot_replace_a_usable_queue() {
        let folder = tempfile::tempdir().unwrap();
        let source = queue(&folder.path().join("source.sqlite"), "broken");
        let snapshot = folder.path().join("snapshot.sqlite");
        source.backup_database(&snapshot).unwrap();
        rusqlite::Connection::open(&snapshot).unwrap().execute("ALTER TABLE downloads DROP COLUMN headers_json", []).unwrap();
        let current = queue(&folder.path().join("current.sqlite"), "retained");
        assert!(current.restore_database(&snapshot, false).is_err(), "a backup missing runtime-required fields must be rejected before activation");
        assert!(current.get_task("retained").unwrap().is_some());
    }

    #[test]
    fn orphaned_backup_rows_cannot_replace_a_usable_queue() {
        let folder = tempfile::tempdir().unwrap();
        let source = queue(&folder.path().join("source.sqlite"), "orphan");
        let snapshot = folder.path().join("snapshot.sqlite");
        source.backup_database(&snapshot).unwrap();
        let corrupt = rusqlite::Connection::open(&snapshot).unwrap();
        corrupt.pragma_update(None, "foreign_keys", "OFF").unwrap();
        corrupt.execute("UPDATE downloads SET queue_id='missing'", []).unwrap();
        drop(corrupt);
        let current = queue(&folder.path().join("current.sqlite"), "retained");
        assert!(current.restore_database(&snapshot, false).is_err());
        assert!(current.get_task("retained").unwrap().is_some());
    }

    #[test]
    fn corrupt_backup_preserves_the_original_queue() {
        let folder = tempfile::tempdir().unwrap();
        let current = queue(&folder.path().join("current.sqlite"), "retained");
        let corrupt = folder.path().join("corrupt.sqlite");
        std::fs::write(&corrupt, b"not a sqlite backup").unwrap();
        assert!(current.restore_database(&corrupt, false).is_err());
        assert!(current.get_task("retained").unwrap().is_some());
    }

    #[test]
    fn offline_restore_preserves_corrupt_database_and_its_sidecars() {
        let folder = tempfile::tempdir().unwrap();
        let source = queue(&folder.path().join("source.sqlite"), "restored");
        let snapshot = folder.path().join("snapshot.sqlite");
        source.backup_database(&snapshot).unwrap();
        let database = folder.path().join("broken.sqlite");
        for (suffix, bytes) in [("", b"corrupt-db".as_slice()), ("-wal", b"original-wal".as_slice()), ("-shm", b"original-shm".as_slice())] {
            std::fs::write(format!("{}{suffix}", database.display()), bytes).unwrap();
        }
        let (count, safety) = restore_offline(&database, &snapshot).unwrap();
        assert_eq!(count, 1);
        assert_eq!(std::fs::read(safety.join("broken.sqlite")).unwrap(), b"corrupt-db");
        assert_eq!(std::fs::read(safety.join("broken.sqlite-wal")).unwrap(), b"original-wal");
        assert_eq!(std::fs::read(safety.join("broken.sqlite-shm")).unwrap(), b"original-shm");
        let restored = Store::open(&database).unwrap();
        assert_eq!(restored.get_task("restored").unwrap().unwrap().status, "paused");
    }
}
