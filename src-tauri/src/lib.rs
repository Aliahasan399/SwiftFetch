//! SwiftFetch application shell.
//!
//! Builds the SQLite store, the HTTP client, the settings document and the
//! transfer engine, then registers every Tauri command.

pub mod commands;
pub mod domain;
pub mod engine;
pub mod media_manifest;
pub mod ratelimit;
pub mod security;
pub mod settings;
pub mod store;
mod native_host;
mod scanner;
pub mod updates;
mod desktop;
mod recovery;

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Phase 3: narrowly scoped, privacy-safe startup timing. Emits phase labels
/// with elapsed milliseconds only when SWIFTFETCH_STARTUP_TRACE=1 is set;
/// never records URLs, paths, settings, or other user data.
fn startup_trace(started: Instant, phase: &str) {
    if std::env::var_os("SWIFTFETCH_STARTUP_TRACE").is_some() {
        eprintln!("swiftfetch-startup: phase={phase} elapsed_ms={}", started.elapsed().as_millis());
    }
}

use engine::Engine;
use ratelimit::SpeedLimiter;
use settings::Settings;
use store::Store;
use tauri::Manager;

/// Resolve the SQLite database location.
///
/// Precedence — the first two let an operator park the database on a drive that
/// actually has room, which matters on hosts whose `APPDATA` drive is small:
///
/// 1. `SWIFTFETCH_DB_PATH` — explicit path to the database file.
/// 2. `SWIFTFETCH_DATA_DIR` — directory containing `state.sqlite`.
/// 3. `%APPDATA%/swiftfetch` — the standard per-user location.
/// 4. `%USERPROFILE%/.swiftfetch`
/// 5. `./state.sqlite` — last resort in constrained environments.
///
/// Split from the wrapper so the precedence can be unit-tested without
/// mutating the process environment (which would race under `cargo test`'s
/// parallel test threads).
fn resolve_db_path_with(get: impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(p) = get("SWIFTFETCH_DB_PATH").filter(|v| !v.trim().is_empty()) {
        return PathBuf::from(p);
    }
    if let Some(dir) = get("SWIFTFETCH_DATA_DIR").filter(|v| !v.trim().is_empty()) {
        return PathBuf::from(dir).join("state.sqlite");
    }
    if let Some(appdata) = get("APPDATA").filter(|v| !v.trim().is_empty()) {
        return PathBuf::from(appdata).join("swiftfetch").join("state.sqlite");
    }
    if let Some(home) = get("USERPROFILE").filter(|v| !v.trim().is_empty()) {
        return PathBuf::from(home).join(".swiftfetch").join("state.sqlite");
    }
    PathBuf::from("state.sqlite")
}

fn resolve_db_path() -> PathBuf {
    resolve_db_path_with(|key| std::env::var(key).ok())
}

/// Handle a one-shot browser invocation before starting the window or engine.
pub fn run_native_host_if_requested() -> bool {
    native_host::run_if_requested(resolve_db_path)
}

pub fn run_update_helper_if_requested() -> bool {
    updates::watchdog_requested() || updates::recovery_requested()
}

fn acquire_desktop_lock(db_path: &Path) -> std::io::Result<Option<File>> {
    if let Some(parent) = db_path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut lock_path = db_path.as_os_str().to_owned();
    lock_path.push(".lock");
    let file = OpenOptions::new().write(true).create(true).truncate(false).open(lock_path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

#[cfg(test)]
mod db_path_tests {
    use super::*;

    /// A stub environment: only the named keys resolve, the rest report unset.
        /// Owns its data so the returned closure needs no lifetime.
        fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
            let owned: Vec<(String, String)> = pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            move |key: &str| owned.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
        }

    #[test]
    fn explicit_db_path_wins_over_everything() {
        let got = resolve_db_path_with(env(&[
            ("SWIFTFETCH_DB_PATH", r"E:\store\custom.sqlite"),
            ("SWIFTFETCH_DATA_DIR", r"E:\store\other"),
            ("APPDATA", r"C:\Users\x\AppData\Roaming"),
        ]));
        assert_eq!(got, PathBuf::from(r"E:\store\custom.sqlite"));
    }

    #[test]
    fn data_dir_is_second_and_gets_the_db_filename() {
        let got = resolve_db_path_with(env(&[
            ("SWIFTFETCH_DATA_DIR", r"E:\CloneIdm\swiftfetch\data"),
            ("APPDATA", r"C:\Users\x\AppData\Roaming"),
        ]));
        assert_eq!(
            got,
            PathBuf::from(r"E:\CloneIdm\swiftfetch\data").join("state.sqlite")
        );
    }

    #[test]
    fn falls_back_to_appdata_when_no_override_is_set() {
        let got = resolve_db_path_with(env(&[
            ("APPDATA", r"C:\Users\x\AppData\Roaming"),
            ("USERPROFILE", r"C:\Users\x"),
        ]));
        assert_eq!(
            got,
            PathBuf::from(r"C:\Users\x\AppData\Roaming")
                .join("swiftfetch")
                .join("state.sqlite")
        );
    }

    #[test]
    fn falls_back_to_userprofile_then_cwd() {
        let got = resolve_db_path_with(env(&[("USERPROFILE", r"C:\Users\x")]));
        assert_eq!(
            got,
            PathBuf::from(r"C:\Users\x").join(".swiftfetch").join("state.sqlite")
        );

        let got = resolve_db_path_with(env(&[]));
        assert_eq!(got, PathBuf::from("state.sqlite"));
    }

    #[test]
    fn blank_overrides_are_ignored() {
        // An empty value must not win: an unset-with-empty-string export is a
        // common accident and would otherwise point the store at a directory.
        let got = resolve_db_path_with(env(&[
            ("SWIFTFETCH_DB_PATH", ""),
            ("SWIFTFETCH_DATA_DIR", "   "),
            ("APPDATA", r"C:\Users\x\AppData\Roaming"),
        ]));
        assert_eq!(
            got,
            PathBuf::from(r"C:\Users\x\AppData\Roaming")
                .join("swiftfetch")
                .join("state.sqlite")
        );
    }
}

#[cfg(test)]
#[test]
fn two_desktop_engines_cannot_own_the_same_queue() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.sqlite");
    let lock = acquire_desktop_lock(&path).unwrap().unwrap();
    assert!(acquire_desktop_lock(&path).unwrap().is_none());
    assert!(acquire_desktop_lock(&directory.path().join("other.sqlite")).unwrap().is_some());
    drop(lock);
    assert!(acquire_desktop_lock(&path).unwrap().is_some());
}

fn build_http_client(proxy: &str) -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let ua = format!("SwiftFetch/{} (Windows; x64)", env!("CARGO_PKG_VERSION"));
    let mut builder = reqwest::Client::builder()
        .user_agent(ua)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let target = attempt.url();
            if attempt.previous().len() > 10 {
                return attempt.error("Redirect limit exceeded");
            }
            if !target.username().is_empty() || target.password().is_some() {
                return attempt.error("Redirect target contains credentials");
            }
            if let Some(previous) = attempt.previous().last() {
                if security::validate_redirect(previous, target).is_err() {
                    return attempt.error("Redirect rejected by the URL policy");
                }
            }
            attempt.follow()
        }))
        .connect_timeout(std::time::Duration::from_secs(30))
        .read_timeout(std::time::Duration::from_secs(60));
    if !proxy.trim().is_empty() {
        let url = url::Url::parse(proxy.trim())?;
        if proxy.len() > 2_048 || !matches!(url.scheme(), "http" | "https") || !url.username().is_empty()
            || url.password().is_some() || url.host_str().is_none() || url.query().is_some() || url.fragment().is_some()
        {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Use an HTTP(S) proxy without credentials").into());
        }
        builder = builder.proxy(reqwest::Proxy::all(url.as_str())?);
    }
    Ok(builder.build()?)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let trace_start = Instant::now();
    startup_trace(trace_start, "process_start");
    let db_path = resolve_db_path();
    // Keep exactly one scheduler per queue database. Native Messaging can
    // enqueue a task while the scheduler owns the database lock.
    let _lock = match acquire_desktop_lock(&db_path) {
        Ok(Some(file)) => file,
        Ok(None) => {
            if !desktop::focus_existing() { desktop::startup_error("SwiftFetch is already using this queue. Open its window from the system tray, or close the existing instance before retrying."); }
            return;
        }
        Err(error) => {
            desktop::startup_error(&format!("SwiftFetch could not access its local queue ({:?}). Check that the configured data folder is writable. Existing files were not deleted.", error.kind()));
            return;
        }
    };
    // Reconcile any update attempt interrupted by a crash before the queue is
    // opened. This only rewrites updater journals; it cannot start downloads.
    if let Some(data) = db_path.parent() {
        if let Some(message) = updates::reconcile_on_startup(&data.join("updates")) {
            eprintln!("swiftfetch: {message}");
        }
    }
    startup_trace(trace_start, "before_builder");
    let outcome = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Keep active transfers visible through the tray. The tray's
                // explicit Exit action drains workers before terminating.
                if window.app_handle().try_state::<Arc<engine::AppEngine>>()
                    .is_some_and(|engine| engine.store().count_active().unwrap_or(0) > 0)
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            startup_trace(trace_start, "setup_enter");
            app.manage(recovery::RecoveryLocation { path: db_path.clone(), message: Mutex::new(None) });
            let store = match Store::open(&db_path) {
                Ok(s) => Arc::new(s),
                Err(_) => {
                    recovery::show_startup(app.handle(), "The saved queue could not be opened. Select a verified backup to restore, or preserve this data folder for recovery. No download file was removed.")?;
                    return Ok(());
                }
            };

            startup_trace(trace_start, "store_opened");
            // Seed the default queue and category exactly once.
            if store.ensure_default_queue().is_err() || store.ensure_default_category().is_err() {
                recovery::show_startup(app.handle(), "The queue schema or default catalogs could not be initialized. Restore a verified backup or check data-folder permissions.")?;
                return Ok(());
            }

            let settings_json = store
                .get_setting("app_settings")
                .unwrap_or_else(|| Settings::default().to_json());
            let settings = Arc::new(Mutex::new(Settings::from_json(&settings_json).normalized()));

            let global_limit = settings
                .lock()
                .map(|s| s.global_speed_limit)
                .unwrap_or(0);
            let global_limiter = Arc::new(SpeedLimiter::new(global_limit));

            let proxy = settings.lock().expect("settings poisoned").proxy.clone();
            let client = match build_http_client(&proxy) {
                Ok(client) => client,
                Err(_) => {
                    recovery::show_startup(app.handle(), "The saved network configuration could not be initialized. You can reset the proxy setting below and retry without deleting the queue.")?;
                    return Ok(());
                }
            };

            startup_trace(trace_start, "settings_and_http_ready");
            let power = Arc::new(desktop::PowerManager::default());
            let engine: Arc<engine::AppEngine> = Arc::new(
                Engine::new(
                    client,
                    Arc::clone(&store),
                    settings,
                    global_limiter,
                )
                .with_sink(desktop::DesktopSink { app: app.handle().clone(), power: Arc::clone(&power) }),
            );

            let data = db_path.parent().unwrap_or_else(|| Path::new("."));
            app.manage(updates::UpdateManager::new(data));
            app.manage(power);
            // Publish native state before creating a WebView or emitting events.
            // The configured main window is deferred until assets.setup() and
            // all application adapters are ready; an eager initial navigation
            // can otherwise remain on about:blank during cold startup.
            app.manage(Arc::clone(&engine));
            startup_trace(trace_start, "engine_built");
            let configuration = app.config().app.windows.iter().find(|window| window.label == "main")
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "The main window configuration is missing"))?
                .clone();
            // Create the WebView before optional shell adapters and workers.
            // Tray registration can be slow on Windows; doing it before the
            // first paint delays a usable window. A failed WebView build must
            // also leave the engine stopped.
            tauri::WebviewWindowBuilder::from_config(app.handle(), &configuration)?.build()?;
            startup_trace(trace_start, "webview_window_built");
            if desktop::install(app.handle(), Arc::clone(&engine)).is_err() {
                *app.state::<recovery::RecoveryLocation>().message.lock().expect("recovery state poisoned") = Some("The optional system tray could not initialize. The main download window remains available.".to_string());
            }
            startup_trace(trace_start, "tray_ready");
            engine.start();
            startup_trace(trace_start, "engine_started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_download,
            commands::control_download,
            commands::list_pending_confirmations,
            commands::resolve_browser_confirmation,
            commands::probe_download_metadata,
            commands::inspect_media_manifest,
            commands::list_tasks,
            commands::get_task,
            commands::get_task_segments,
            commands::get_settings,
            commands::save_settings,
            commands::list_categories,
            commands::save_category,
            commands::list_queues,
            commands::list_schedules,
            commands::save_schedule,
            commands::delete_schedule,
            commands::get_diagnostics,
            commands::clear_history,
            commands::encrypt_saved_secrets,
            desktop::read_clipboard_url,
            desktop::get_power_request,
            desktop::cancel_shutdown,
            desktop::report_ready,
            desktop::get_disk_space,
            updates::get_update_state,
            updates::check_for_updates,
            updates::install_update,
            updates::rollback_update,
            recovery::export_queue,
            recovery::backup_metadata,
            recovery::import_queue,
            recovery::get_recovery_state,
            recovery::restore_metadata,
            recovery::restart_application,
            recovery::reset_startup_network,
        ])
        .run(tauri::generate_context!());
    if outcome.is_err() {
        desktop::startup_error("The SwiftFetch desktop could not start. Check the WebView2 runtime and data-folder permissions, then retry. Your queue and downloaded files were not removed.");
    }
}
