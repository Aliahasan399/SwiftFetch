use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::Serialize;
use tauri::{Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::domain::CommandResult;
use crate::engine::{AppEngine, EventSink};

#[derive(Clone, Serialize)]
pub struct PowerRequest { request_id: String, deadline: String }
#[derive(Default)]
pub struct PowerManager(Mutex<Option<PowerRequest>>);

#[derive(Clone, Serialize)]
pub struct DiskSpace { pub available_bytes: u64, pub total_bytes: u64 }

pub(crate) fn disk_space_for(path: &std::path::Path) -> Option<DiskSpace> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetDiskFreeSpaceExW(path: *const u16, available: *mut u64, total: *mut u64, free: *mut u64) -> i32;
        }
        let mut existing = path;
        while !existing.is_dir() { existing = existing.parent()?; }
        let wide: Vec<u16> = existing.as_os_str().encode_wide().chain(Some(0)).collect();
        let (mut available, mut total, mut free) = (0, 0, 0);
        if unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) } != 0 {
            return Some(DiskSpace { available_bytes: available, total_bytes: total });
        }
    }
    let _ = path;
    None
}

#[tauri::command]
pub fn get_disk_space(app: tauri::AppHandle, path: Option<String>) -> CommandResult<Option<DiskSpace>> {
    let selected = path.or_else(|| app.try_state::<Arc<AppEngine>>().map(|engine| engine.settings().destination_root.clone()));
    CommandResult::success(selected.as_deref().and_then(|path| disk_space_for(std::path::Path::new(path))))
}

pub(crate) fn startup_error(message: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "user32")]
        unsafe extern "system" { fn MessageBoxW(window: *mut std::ffi::c_void, text: *const u16, title: *const u16, flags: u32) -> i32; }
        let wide = |text: &str| std::ffi::OsStr::new(text).encode_wide().chain(Some(0)).collect::<Vec<_>>();
        unsafe { MessageBoxW(std::ptr::null_mut(), wide(message).as_ptr(), wide("SwiftFetch startup recovery").as_ptr(), 0x10); }
    }
    eprintln!("swiftfetch: {message}");
}

pub(crate) fn focus_existing() -> bool {
    #[cfg(windows)]
    unsafe {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn FindWindowW(class: *const u16, title: *const u16) -> *mut std::ffi::c_void;
            fn ShowWindow(window: *mut std::ffi::c_void, command: i32) -> i32;
            fn SetForegroundWindow(window: *mut std::ffi::c_void) -> i32;
        }
        let title: Vec<u16> = "SwiftFetch".encode_utf16().chain(Some(0)).collect();
        let window = FindWindowW(std::ptr::null(), title.as_ptr());
        if !window.is_null() { ShowWindow(window, 9); SetForegroundWindow(window); return true; }
    }
    false
}

impl PowerManager {
    fn cancel(&self) { *self.0.lock().expect("power state poisoned") = None; }
    fn request(self: &Arc<Self>, app: tauri::AppHandle) {
        let request = {
            let mut pending = self.0.lock().expect("power state poisoned");
            if pending.is_some() { return; }
            let request = PowerRequest {
                request_id: uuid::Uuid::new_v4().to_string(),
                deadline: (chrono::Utc::now() + chrono::TimeDelta::seconds(60)).to_rfc3339(),
            };
            *pending = Some(request.clone()); request
        };
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show(); let _ = window.unminimize(); let _ = window.set_focus();
            let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
        }
        let _ = app.emit("shutdown_pending", &request);
        let manager = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            for _ in 0..60 {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if manager.0.lock().expect("power state poisoned").as_ref().is_none_or(|pending| pending.request_id != request.request_id) { return; }
                let engine = app.state::<Arc<AppEngine>>();
                if engine.store().count_active().unwrap_or(1) > 0 || engine.store().next_queued_tasks(1).is_ok_and(|tasks| !tasks.is_empty()) {
                    manager.cancel();
                    engine.cancel_shutdown(); return;
                }
            }
            let current = manager.0.lock().expect("power state poisoned").take();
            if current.is_some_and(|current| current.request_id == request.request_id) {
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    if let Some(system) = std::env::var_os("SystemRoot") {
                        let _ = std::process::Command::new(std::path::PathBuf::from(system).join("System32/shutdown.exe"))
                            .args(["/s", "/t", "0"]).creation_flags(0x0800_0000).spawn();
                    }
                }
            }
        });
    }
}

pub struct DesktopSink { pub app: tauri::AppHandle, pub power: Arc<PowerManager> }
impl EventSink for DesktopSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        if event == "shutdown_requested" { self.power.request(self.app.clone()); }
        else if event == "shutdown_cancelled" { self.power.cancel(); let _ = self.app.emit(event, payload); }
        else { let _ = self.app.emit(event, payload); }
    }
}

#[tauri::command]
pub fn get_power_request(state: State<'_, Arc<PowerManager>>) -> CommandResult<Option<PowerRequest>> {
    CommandResult::success(state.0.lock().expect("power state poisoned").clone())
}

#[tauri::command]
pub fn cancel_shutdown(app: tauri::AppHandle, state: State<'_, Arc<PowerManager>>, request_id: String) -> CommandResult<()> {
    let matches = state.0.lock().expect("power state poisoned").as_ref().is_some_and(|pending| pending.request_id == request_id);
    if matches {
        state.cancel();
        app.state::<Arc<AppEngine>>().cancel_shutdown();
    }
    CommandResult::success(())
}

#[tauri::command]
pub async fn read_clipboard_url(app: tauri::AppHandle) -> CommandResult<Option<String>> {
    match capture_clipboard(app).await {
        Ok(url) => CommandResult::success(url),
        Err(()) => CommandResult::failure("CLIPBOARD_UNAVAILABLE", "The clipboard is temporarily unavailable. Try again or paste the link manually.", true),
    }
}

async fn capture_clipboard(app: tauri::AppHandle) -> Result<Option<String>, ()> {
    tauri::async_runtime::spawn_blocking(move || {
        for attempt in 0..5 {
            if let Ok(text) = app.clipboard().read_text() { return Ok(clipboard_url(&text)); }
            if attempt < 4 { std::thread::sleep(Duration::from_millis(40)); }
        }
        Err(())
    }).await.unwrap_or(Err(()))
}

fn clipboard_url(text: &str) -> Option<String> {
    if text.len() > 2_048 { return None; }
    let validated = crate::security::validate_url(text);
    if validated.ok { validated.data } else { None }
}

#[tauri::command]
pub fn report_ready(engine: State<'_, Arc<AppEngine>>) -> CommandResult<()> {
    if engine.store().count_tasks().is_err() {
        return CommandResult::failure("STORE_ERROR", "The queue did not pass its desktop health check.", false);
    }
    if let Some(data) = engine.store().path().parent() { crate::updates::mark_healthy(data); }
    CommandResult::success(())
}

pub fn install(app: &tauri::AppHandle, engine: Arc<AppEngine>) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
    let open = MenuItem::with_id(app, "open", "Open SwiftFetch", true, None::<&str>)?;
    let add = MenuItem::with_id(app, "add", "Add download", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause all downloads", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Pause downloads and exit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &add, &pause, &quit])?;
    let mut tray = TrayIconBuilder::new().tooltip("SwiftFetch download manager").menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if matches!(event.id.as_ref(), "open" | "add") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show(); let _ = window.unminimize(); let _ = window.set_focus();
                }
                if event.id.as_ref() == "add" { let _ = app.emit("open_add_download", ()); }
            } else if matches!(event.id.as_ref(), "pause" | "quit") {
                let exit = event.id.as_ref() == "quit";
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let engine = app.state::<Arc<AppEngine>>().inner().clone();
                    if engine.quiesce().await.is_ok() && exit { app.exit(0); }
                });
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }) {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show(); let _ = window.unminimize(); let _ = window.set_focus();
                }
            }
        });
    if let Some(icon) = app.default_window_icon() { tray = tray.icon(icon.clone()); }
    tray.build(app)?;
    // Windows execution-state requests belong to a thread. Keep acquisition and
    // release on this dedicated thread rather than a migrating async worker.
    let weak = Arc::downgrade(&engine);
    std::thread::Builder::new().name("swiftfetch-power".to_string()).spawn(move || {
        let mut awake = false;
        loop {
            let Some(engine) = weak.upgrade() else { break };
            let keep_awake = engine.settings().prevent_sleep && engine.store().count_active().unwrap_or(0) > 0;
            if keep_awake != awake { crate::engine::set_prevent_sleep(keep_awake); awake = keep_awake; }
            drop(engine);
            std::thread::sleep(Duration::from_secs(1));
        }
        if awake { crate::engine::set_prevent_sleep(false); }
    }).ok();
    let application = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut last = None;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let enabled = engine.settings().clipboard_capture;
            if !enabled { last = None; continue; }
            let candidate = capture_clipboard(application.clone()).await.ok().flatten();
            if candidate != last {
                if let Some(url) = candidate.as_ref() { let _ = application.emit("clipboard_url", serde_json::json!({"url":url})); }
                last = candidate;
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipboard_capture_accepts_only_a_single_permitted_url() {
        assert_eq!(clipboard_url("https://example.com/file.bin").as_deref(), Some("https://example.com/file.bin"));
        for value in ["unrelated personal text", "https://user:secret@example.com/file", "file:///C:/private", "https://example.com/a\nhttps://example.com/b"] {
            assert!(clipboard_url(value).is_none());
        }
    }
}
