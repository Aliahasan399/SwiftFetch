//! Publisher-configured, signed updates. A verified copy of the current release
//! is retained before installation so recovery never depends on unsigned bytes.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::domain::CommandResult;

const MAX_PACKAGE: usize = 128 * 1024 * 1024;

#[derive(Clone, Serialize)]
pub struct UpdateState {
    configured: bool,
    current_version: String,
    available_version: Option<String>,
    notes: Option<String>,
    phase: String,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    rollback_available: bool,
    error: Option<String>,
}

pub struct UpdateManager {
    state: Mutex<UpdateState>,
    pending: Mutex<Option<Update>>,
    busy: AtomicBool,
    root: PathBuf,
}

#[derive(Clone)]
struct Publisher {
    endpoint: url::Url,
    archive: String,
    pubkey: String,
}

fn safe_endpoint(value: &str) -> Result<url::Url, &'static str> {
    let url = url::Url::parse(value).map_err(|_| "The release endpoint is invalid.")?;
    if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty()
        || url.password().is_some() || url.fragment().is_some()
    {
        return Err("Release endpoints must use HTTPS without embedded credentials.");
    }
    Ok(url)
}

fn publisher() -> Option<Publisher> {
    let endpoint = option_env!("SWIFTFETCH_UPDATE_ENDPOINT")?;
    let archive = option_env!("SWIFTFETCH_UPDATE_ARCHIVE")?;
    let pubkey = option_env!("SWIFTFETCH_UPDATE_PUBLIC_KEY")?;
    if pubkey.trim().is_empty() || !archive.contains("{{version}}") { return None; }
    safe_endpoint(&archive.replace("{{version}}", env!("CARGO_PKG_VERSION"))).ok()?;
    Some(Publisher { endpoint: safe_endpoint(endpoint).ok()?, archive: archive.to_string(), pubkey: pubkey.to_string() })
}

struct Busy<'a>(&'a AtomicBool);
impl Drop for Busy<'_> { fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); } }

impl UpdateManager {
    pub fn new(data: &Path) -> Arc<Self> {
        let configured = publisher().is_some();
        Arc::new(Self {
            state: Mutex::new(UpdateState {
                configured, current_version: env!("CARGO_PKG_VERSION").to_string(),
                available_version: None, notes: None,
                phase: if configured { "idle" } else { "not_configured" }.to_string(),
                downloaded_bytes: 0, total_bytes: None,
                rollback_available: data.join("updates/rollback.json").is_file(), error: None,
            }),
            pending: Mutex::new(None), busy: AtomicBool::new(false), root: data.join("updates"),
        })
    }

    fn snapshot(&self) -> UpdateState { self.state.lock().expect("update state poisoned").clone() }
    fn phase(&self, phase: &str, error: Option<&str>) {
        let mut state = self.state.lock().expect("update state poisoned");
        state.phase = phase.to_string(); state.error = error.map(str::to_string);
    }
    fn acquire(&self) -> Result<Busy<'_>, &'static str> {
        self.busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| Busy(&self.busy)).map_err(|_| "An update operation is already running.")
    }
}

fn updater(app: &tauri::AppHandle, publisher: &Publisher, endpoint: url::Url, rollback: bool)
    -> Result<tauri_plugin_updater::Updater, &'static str>
{
    let mut builder = app.updater_builder().pubkey(&publisher.pubkey)
        .endpoints(vec![endpoint]).map_err(|_| "Invalid release endpoint.")?
        .timeout(Duration::from_secs(60));
    if rollback { builder = builder.version_comparator(|_, _| true); }
    builder.build().map_err(|_| "The release signing configuration is invalid.")
}

async fn check_release(app: &tauri::AppHandle, config: &Publisher, endpoint: url::Url, rollback: bool) -> Result<Option<Update>, &'static str> {
    let endpoint = endpoint.as_str().replace("{{current_version}}", env!("CARGO_PKG_VERSION"))
        .replace("{{target}}", "windows").replace("{{arch}}", "x86_64")
        .replace("%7B%7Bcurrent_version%7D%7D", env!("CARGO_PKG_VERSION"))
        .replace("%7B%7Btarget%7D%7D", "windows").replace("%7B%7Barch%7D%7D", "x86_64");
    let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 5 || attempt.url().scheme() != "https" {
                attempt.error("Release redirect rejected")
            } else { attempt.follow() }
        })).build().map_err(|_| "Could not initialize the release connection.")?;
    let response = client.get(safe_endpoint(&endpoint)?).send().await.map_err(|_| "Could not reach the release endpoint.")?;
    if response.status() == reqwest::StatusCode::NO_CONTENT { return Ok(None); }
    if !response.status().is_success() { return Err("The release server rejected the manifest request."); }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "Could not read the release manifest.")?;
        if bytes.len() + chunk.len() > 128 * 1024 { return Err("The release manifest exceeds its size limit."); }
        bytes.extend_from_slice(&chunk);
    }
    let raw = serde_json::from_slice(&bytes).map_err(|_| "Invalid release manifest.")?;
    updater(app, config, config.endpoint.clone(), rollback)?.restore_update(raw)
        .map_err(|_| "Invalid release version, platform, or signature metadata.")
}

async fn package(update: &Update, manager: &UpdateManager) -> Result<Vec<u8>, &'static str> {
    safe_endpoint(update.download_url.as_str())?;
    let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 5 || attempt.url().scheme() != "https"
                || !attempt.url().username().is_empty() || attempt.url().password().is_some()
            { attempt.error("Release redirect rejected") } else { attempt.follow() }
        })).build().map_err(|_| "Could not create the update connection.")?;
    let response = client.get(update.download_url.clone()).send().await
        .map_err(|_| "Could not download the signed release.")?;
    if !response.status().is_success() { return Err("The release server rejected the package request."); }
    if response.content_length().is_some_and(|size| size > MAX_PACKAGE as u64) { return Err("The update package exceeds the 128 MiB limit."); }
    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "The update download was interrupted.")?;
        if chunk.len() > MAX_PACKAGE.saturating_sub(bytes.len()) { return Err("The update package exceeds the 128 MiB limit."); }
        bytes.extend_from_slice(&chunk);
        let mut state = manager.state.lock().expect("update state poisoned");
        state.downloaded_bytes = bytes.len() as u64; state.total_bytes = total;
    }
    update.verify(&bytes).map_err(|_| "The update signature or signed version is invalid. Nothing was installed.")?;
    Ok(bytes)
}

#[derive(Clone, Serialize, Deserialize)]
struct CachedRelease { manifest: serde_json::Value, package: String }

const STAGING_BUDGET: u64 = 1024 * 1024 * 1024;

/// One recursive budget applies to every file the updater stages: signed
/// packages, DB snapshots, helper/installer copies and journal backups.
fn staging_used(root: &Path) -> Result<u64, &'static str> {
    fn walk(path: &Path, total: &mut u64) -> std::io::Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() { walk(&entry.path(), total)?; }
            else if metadata.is_file() { *total = total.saturating_add(metadata.len()); }
        }
        Ok(())
    }
    if !root.exists() { return Ok(0); }
    let mut total = 0;
    walk(root, &mut total).map_err(|_| "Could not inspect update staging")?;
    Ok(total)
}

fn reserve_staging(root: &Path, bytes: usize) -> Result<(), &'static str> {
    if staging_used(root)?.saturating_add(bytes as u64) > STAGING_BUDGET {
        return Err("Update staging reached its 1 GiB limit. Review old update backups before retrying.");
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct InstallAttempt {
    id: String, parent_pid: u32, install_path: PathBuf, target_version: String,
    update: CachedRelease, rollback: CachedRelease, explicit_rollback: bool,
    #[serde(default)] phase: String,
    #[serde(default)] metadata_snapshot: Option<String>,
    #[serde(default)] database_path: Option<PathBuf>,
}

pub fn reconcile_on_startup(root: &Path) -> Option<&'static str> {
    let Ok(entries) = std::fs::read_dir(root) else { return None };
    let mut interrupted = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("attempt-") || !name.ends_with(".json") { continue; }
        let Ok(bytes) = std::fs::read(entry.path()) else { continue };
        let Ok(mut attempt) = serde_json::from_slice::<InstallAttempt>(&bytes) else { continue };
        if matches!(attempt.phase.as_str(), "installing" | "health") || attempt.phase.is_empty() {
            attempt.phase = "interrupted".to_string();
            let _ = journal(root, &name, &attempt);
            interrupted = Some("A previous update was interrupted. The signed rollback remains available.");
        }
    }
    interrupted
}

fn write_cache(root: &Path, update: &Update, bytes: &[u8]) -> Result<CachedRelease, &'static str> {
    std::fs::create_dir_all(root).map_err(|_| "Could not create update staging.")?;
    use sha2::{Digest, Sha256};
    let name = format!("{:x}.package", Sha256::digest(bytes));
    let path = root.join(&name);
    if path.exists() {
        let stored = std::fs::read(&path).map_err(|_| "Could not validate the cached package")?;
        update.verify(&stored).map_err(|_| "The existing update cache is damaged")?;
        return Ok(CachedRelease { manifest: update.raw_json.clone(), package: name });
    }
    reserve_staging(root, bytes.len())?;
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(root.join(&name))
        .map_err(|_| "Could not stage the verified update.")?;
    use std::io::Write;
    file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|_| "Could not sync the verified update.")?;
    Ok(CachedRelease { manifest: update.raw_json.clone(), package: name })
}

fn journal(root: &Path, name: &str, value: &impl Serialize) -> Result<(), &'static str> {
    std::fs::create_dir_all(root).map_err(|_| "Could not create update state.")?;
    let path = root.join(name);
    if path.exists() {
        std::fs::copy(&path, root.join(format!("{name}.bak-{}", uuid::Uuid::new_v4())))
            .map_err(|_| "Could not back up the update journal.")?;
    }
    let data = serde_json::to_vec(value).map_err(|_| "Could not serialize the update state.")?;
    use std::io::Write;
    let temporary = root.join(format!("journal-{}.tmp", uuid::Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|_| "Could not save the update journal.")?;
    file.write_all(&data).and_then(|_| file.sync_all()).map_err(|_| "Could not sync the update journal.")?;
    drop(file);
    std::fs::rename(temporary, path).map_err(|_| "Could not activate the update journal.")
}

fn cached(root: &Path) -> Result<CachedRelease, &'static str> {
    let data = std::fs::read(root.join("rollback.json")).map_err(|_| "No verified rollback release is cached.")?;
    if data.len() > 128 * 1024 { return Err("Invalid rollback journal."); }
    let cache: CachedRelease = serde_json::from_slice(&data).map_err(|_| "Invalid rollback journal.")?;
    if cache.package.contains(['/', '\\', ':']) || !cache.package.ends_with(".package") { return Err("Invalid rollback package path."); }
    Ok(cache)
}

#[tauri::command]
pub fn get_update_state(state: State<'_, Arc<UpdateManager>>) -> CommandResult<UpdateState> {
    CommandResult::success(state.snapshot())
}

#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> CommandResult<UpdateState> {
    let state = app.state::<Arc<UpdateManager>>().inner().clone();
    let Some(config) = publisher() else { return CommandResult::success(state.snapshot()) };
    let Ok(_busy) = state.acquire() else { return CommandResult::failure("UPDATE_BUSY", "An update operation is already running.", true) };
    state.phase("checking", None);
    let result = check_release(&app, &config, config.endpoint.clone(), false).await;
    match result {
        Ok(update) => {
            let mut projection = state.state.lock().expect("update state poisoned");
            projection.available_version = update.as_ref().map(|update| update.version.clone());
            projection.notes = update.as_ref().and_then(|update| update.body.as_ref().map(|body| body.chars().take(8_000).collect()));
            projection.phase = if update.is_some() { "available" } else { "idle" }.to_string();
            *state.pending.lock().expect("pending update poisoned") = update;
        }
        Err(error) => state.phase("failed", Some(error)),
    }
    CommandResult::success(state.snapshot())
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> CommandResult<UpdateState> {
    let state = app.state::<Arc<UpdateManager>>().inner().clone();
    let Some(config) = publisher() else { return CommandResult::failure("UPDATE_NOT_CONFIGURED", "This build has no publisher-configured update channel.", false) };
    let Ok(_busy) = state.acquire() else { return CommandResult::failure("UPDATE_BUSY", "An update operation is already running.", true) };
    let pending = state.pending.lock().expect("pending update poisoned").clone();
    let Some(update) = pending else { return CommandResult::failure("UPDATE_UNAVAILABLE", "Check for updates first.", false) };
    state.phase("downloading", None);
    let result = async {
        // An immutable version endpoint supplies the current signed installer.
        // Refuse the upgrade if its rollback package cannot be verified first.
        let archive = safe_endpoint(&config.archive.replace("{{version}}", env!("CARGO_PKG_VERSION")))?;
        let previous = check_release(&app, &config, archive, true).await?
            .filter(|release| release.version == env!("CARGO_PKG_VERSION"))
            .ok_or("The rollback endpoint must supply this installed version.")?;
        let previous_bytes = package(&previous, &state).await?;
        let rollback = write_cache(&state.root, &previous, &previous_bytes)?;
        // Restore enforces a version bound into the Minisign signature.
        updater(&app, &config, config.endpoint.clone(), true)?.restore_update(rollback.manifest.clone())
            .map_err(|_| "Could not validate rollback metadata.")?.ok_or("Invalid rollback release.")?
            .verify(&previous_bytes).map_err(|_| "Rollback package needs a signature bound to its version.")?;
        journal(&state.root, "rollback.json", &rollback)?;
        let bytes = package(&update, &state).await?;
        let next = write_cache(&state.root, &update, &bytes)?;
        let engine = app.state::<Arc<crate::engine::AppEngine>>().inner().clone();
        // Bind a consistent snapshot of the current metadata to this attempt so
        // a signed rollback can also restore the queue it started from.
        let database = engine.store().path().to_path_buf();
        let snapshot_name = format!("before-{}-{}.sqlite", env!("CARGO_PKG_VERSION"), uuid::Uuid::new_v4());
        reserve_staging(&state.root, engine.store().path().metadata().map(|m| m.len() as usize).unwrap_or(0))?;
        engine.store().backup_database(&state.root.join(&snapshot_name))
            .map_err(|_| "Could not snapshot the queue before updating.")?;
        engine.quiesce().await.map_err(|_| "Downloads could not be paused safely. Update cancelled.")?;
        state.phase("installing", None);
        start_installer(&app, &state.root, next, rollback, false, Some(snapshot_name), Some(database)).await?;
        Ok::<(), &'static str>(())
    }.await;
    if let Err(error) = result {
        // A failed launch leaves the current app in charge. Cancel a watchdog
        // that may have started before the installer returned its error.
        if state.root.join("health.json").exists() {
            let _ = journal(&state.root, "health.json", &serde_json::json!({"healthy":true,"installation_aborted":true}));
        }
        state.phase("failed", Some(error));
    }
    CommandResult::success(state.snapshot())
}

#[tauri::command]
pub async fn rollback_update(app: tauri::AppHandle) -> CommandResult<UpdateState> {
    let state = app.state::<Arc<UpdateManager>>().inner().clone();
    let Some(config) = publisher() else { return CommandResult::failure("UPDATE_NOT_CONFIGURED", "This build has no publisher-configured update channel.", false) };
    let Ok(_busy) = state.acquire() else { return CommandResult::failure("UPDATE_BUSY", "An update operation is already running.", true) };
    let result = async {
        let cache = cached(&state.root)?;
        let update = updater(&app, &config, config.endpoint.clone(), true)?.restore_update(cache.manifest.clone())
            .map_err(|_| "Cached rollback metadata was rejected.")?.ok_or("No cached rollback release.")?;
        let file = state.root.join(&cache.package);
        if std::fs::metadata(&file).map_err(|_| "Rollback package is unavailable.")?.len() > MAX_PACKAGE as u64 { return Err("Invalid rollback package size."); }
        let bytes = std::fs::read(file).map_err(|_| "Could not read the rollback package.")?;
        update.verify(&bytes).map_err(|_| "Rollback signature is invalid. Nothing was installed.")?;
        let engine = app.state::<Arc<crate::engine::AppEngine>>().inner().clone();
        engine.quiesce().await.map_err(|_| "Downloads could not be paused safely.")?;
        start_installer(&app, &state.root, cache.clone(), cache, true, None, None).await?;
        Ok::<(), &'static str>(())
    }.await;
    if let Err(error) = result { state.phase("failed", Some(error)); }
    CommandResult::success(state.snapshot())
}

async fn start_installer(app: &tauri::AppHandle, root: &Path, update: CachedRelease, rollback: CachedRelease, explicit_rollback: bool, metadata_snapshot: Option<String>, database_path: Option<PathBuf>) -> Result<(), &'static str> {
    let executable = std::env::current_exe().map_err(|_| "Could not locate the updater recovery helper.")?;
    let mut attempt = InstallAttempt {
        id: uuid::Uuid::new_v4().to_string(), parent_pid: std::process::id(), install_path: executable.clone(),
        target_version: update.manifest["version"].as_str().ok_or("Missing signed release version")?.to_string(),
        update, rollback, explicit_rollback, phase: "staged".to_string(),
        metadata_snapshot, database_path,
    };
    let journal_name = format!("attempt-{}.json", attempt.id);
    journal(root, &journal_name, &attempt)?;
    attempt.phase = "verified".to_string();
    journal(root, &journal_name, &attempt)?;
    journal(root, "health.json", &serde_json::json!({"attempt_id":attempt.id,"target_version":attempt.target_version,"healthy":false}))?;
    let helper = root.join(format!("recovery-{}.exe", uuid::Uuid::new_v4()));
    reserve_staging(root, std::fs::metadata(&executable).map_err(|_| "Could not size the recovery helper")?.len() as usize)?;
    std::fs::copy(executable, &helper).map_err(|_| "Could not prepare the recovery helper.")?;
    let mut process = std::process::Command::new(helper).arg("--apply-update").arg(root).arg(&journal_name)
        .spawn().map_err(|_| "Could not start the update recovery helper.")?;
    // The helper verifies both packages before the current window is closed.
    // A failed helper launch/configuration therefore leaves this app usable.
    let ready = root.join(format!("{}.ready", attempt.id));
    for _ in 0..150 {
        if ready.is_file() { app.exit(0); return Ok(()); }
        if process.try_wait().ok().flatten().is_some() { return Err("The recovery helper could not validate the staged releases."); }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _ = process.kill();
    Err("The recovery helper did not become ready. The update was cancelled.")
}

pub fn mark_healthy(data: &Path) {
    let root = data.join("updates");
    let Ok(bytes) = std::fs::read(root.join("health.json")) else { return };
    let Ok(mut state) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return };
    if state["target_version"].as_str() == Some(env!("CARGO_PKG_VERSION")) {
        state["healthy"] = true.into();
        let _ = journal(&root, "health.json", &state);
    }
}

/// A recovery helper starts only from an explicitly staged updater invocation.
pub fn watchdog_requested() -> bool { false }

fn load_verified(app: &tauri::AppHandle, config: &Publisher, root: &Path, cached: &CachedRelease) -> Result<(Update, Vec<u8>), &'static str> {
    if cached.package.contains(['/', '\\', ':']) || !cached.package.ends_with(".package") { return Err("Invalid staged release path"); }
    let update = updater(app, config, config.endpoint.clone(), true)?.restore_update(cached.manifest.clone())
        .map_err(|_| "Invalid signed recovery manifest")?.ok_or("Recovery unavailable")?;
    let file = root.join(&cached.package);
    if std::fs::metadata(&file).map_err(|_| "Missing recovery package")?.len() > MAX_PACKAGE as u64 { return Err("Recovery package exceeds its size limit"); }
    let bytes = std::fs::read(file).map_err(|_| "Missing recovery package")?;
    update.verify(&bytes).map_err(|_| "Recovery package signature is invalid")?;
    Ok((update, bytes))
}

/// Restore the metadata snapshot bound to this attempt. Only a snapshot that
/// exists in this staging directory and passes a full consistency check may be
/// applied, and restored tasks are always paused/manual with consent reset.
fn restore_bound_metadata(root: &Path, attempt: &InstallAttempt) -> Result<(), &'static str> {
    let (Some(name), Some(database)) = (&attempt.metadata_snapshot, &attempt.database_path) else { return Ok(()) };
    if name.contains(['/', '\\', ':']) || !name.ends_with(".sqlite") { return Err("Invalid metadata snapshot name"); }
    let snapshot = root.join(name);
    if !snapshot.is_file() { return Err("Bound metadata snapshot is missing"); }
    let store = crate::store::Store::open(database).map_err(|_| "Could not open the queue for metadata recovery")?;
    store.restore_database(&snapshot, false).map_err(|_| "Bound metadata snapshot failed validation")?;
    Ok(())
}

fn wait_parent(pid: u32) -> Result<(), &'static str> {
    #[cfg(windows)]
    unsafe {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
            fn WaitForSingleObject(handle: *mut std::ffi::c_void, millis: u32) -> u32;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }
        let process = OpenProcess(0x0010_0000, 0, pid);
        if process.is_null() { return Ok(()); }
        let result = WaitForSingleObject(process, 120_000);
        CloseHandle(process);
        if result != 0 { return Err("The previous application has not exited"); }
    }
    #[cfg(not(windows))]
    { let _ = pid; return Err("The installer helper is Windows-only"); }
    Ok(())
}

fn install_and_wait(root: &Path, update: &Update, bytes: &[u8], destination: &Path) -> Result<(), &'static str> {
    // These bytes were just verified above; no downloaded program is opened as
    // a download action. This is the publisher's explicit signed update path.
    update.verify(bytes).map_err(|_| "Installer signature was rejected")?;
    let is_msi = update.download_url.path().to_ascii_lowercase().ends_with(".msi");
    let installer = root.join(format!("installer-{}.{}", uuid::Uuid::new_v4(), if is_msi {"msi"} else {"exe"}));
    std::fs::write(&installer, bytes).map_err(|_| "Could not stage the signed installer")?;
    let mut command = if is_msi {
        let system = std::env::var_os("SystemRoot").ok_or("Windows directory unavailable")?;
        let mut command = std::process::Command::new(PathBuf::from(system).join("System32/msiexec.exe"));
        // Preserve the existing installation directory across a signed MSI
        // upgrade/rollback. INSTALLDIR is a documented MSI property.
        let folder = destination.parent().ok_or("Missing installation folder")?.to_string_lossy();
        if folder.contains(['"', '\r', '\n']) { return Err("Invalid installation folder"); }
        command.arg("/i").arg(&installer).args(["/passive", "/norestart"])
            .arg(format!("INSTALLDIR={folder}")); command
    } else {
        let mut command = std::process::Command::new(&installer);
        command.arg("/P");
        let folder = destination.parent().ok_or("Missing installation folder")?.to_string_lossy();
        if folder.contains(['"', '\r', '\n']) { return Err("Invalid installation folder"); }
        #[cfg(windows)]
        { use std::os::windows::process::CommandExt; command.raw_arg(format!("/D={folder}")); }
        #[cfg(not(windows))]
        command.arg(format!("/D={folder}"));
        command
    };
    let status = command.status().map_err(|_| "The signed installer could not be started")?;
    if status.success() || status.code() == Some(3010) { Ok(()) } else { Err("The signed installer did not complete successfully") }
}

pub fn recovery_requested() -> bool {
    if std::env::args().nth(1).as_deref() != Some("--apply-update") { return false; }
    let Some(root) = std::env::args_os().nth(2).map(PathBuf::from) else { return true };
    let Some(name) = std::env::args().nth(3) else { return true };
    if name.contains(['/', '\\', ':']) || !name.starts_with("attempt-") || !name.ends_with(".json") { return true; }
    let Ok(executable) = std::env::current_exe() else { return true };
    if executable.parent().and_then(|path| path.canonicalize().ok()) != root.canonicalize().ok() { return true; }
    let Some(config) = publisher() else { return true };
    let mut context = tauri::generate_context!();
    for window in &mut context.config_mut().app.windows { window.visible = false; }
    tauri::Builder::default().plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let restore = || -> Result<(), &'static str> {
                    let attempt: InstallAttempt = serde_json::from_slice(&std::fs::read(root.join(&name)).map_err(|_| "Recovery state unavailable")?)
                        .map_err(|_| "Invalid recovery attempt")?;
                    let (update, bytes) = load_verified(&app, &config, &root, &attempt.update)?;
                    let (rollback, rollback_bytes) = load_verified(&app, &config, &root, &attempt.rollback)?;
                    journal(&root, &format!("{}.ready", attempt.id), &true)?;
                    wait_parent(attempt.parent_pid)?;
                    let installed = install_and_wait(&root, &update, &bytes, &attempt.install_path);
                    if installed.is_err() {
                        if !attempt.explicit_rollback {
                            let _ = install_and_wait(&root, &rollback, &rollback_bytes, &attempt.install_path);
                            let _ = restore_bound_metadata(&root, &attempt);
                        }
                        let _ = std::process::Command::new(&attempt.install_path).spawn();
                        return installed;
                    }
                    // Only now, after the installer has exited, measure health.
                    let launched = std::process::Command::new(&attempt.install_path).spawn();
                    let mut launched = match launched {
                        Ok(process) => process,
                        Err(_) => {
                            if !attempt.explicit_rollback {
                                install_and_wait(&root, &rollback, &rollback_bytes, &attempt.install_path)?;
                                let _ = restore_bound_metadata(&root, &attempt);
                            }
                            let _ = std::process::Command::new(&attempt.install_path).spawn();
                            return Err("The updated application could not start; signed rollback was attempted");
                        }
                    };
                    for _ in 0..180 {
                        let healthy = std::fs::read(root.join("health.json")).ok()
                            .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok())
                            .is_some_and(|health| health["attempt_id"] == attempt.id && health["healthy"] == true);
                        if healthy { return Ok(()); }
                        if launched.try_wait().ok().flatten().is_some() { break; }
                        std::thread::sleep(Duration::from_secs(1));
                    }
                    let _ = launched.kill(); let _ = launched.wait();
                    if !attempt.explicit_rollback {
                        install_and_wait(&root, &rollback, &rollback_bytes, &attempt.install_path)?;
                        let _ = restore_bound_metadata(&root, &attempt);
                    }
                    let _ = std::process::Command::new(&attempt.install_path).spawn();
                    journal(&root, "recovery-result.json", &serde_json::json!({"restored":!attempt.explicit_rollback,"reason":"health_check_failed"}))?;
                    Ok(())
                };
                if let Err(error) = restore() {
                    let _ = journal(&root, "recovery-result.json", &serde_json::json!({"restored":false,"error":error}));
                }
                app.exit(0);
            });
            Ok(())
        }).run(context).ok();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    #[test]
    fn update_endpoints_reject_credentials_and_insecure_transport() {
        assert!(safe_endpoint("https://example.com/releases/latest.json").is_ok());
        for url in ["http://example.com/u", "file:///C:/u", "https://name:secret@example.com/u", "https://example.com/u#fragment"] {
            assert!(safe_endpoint(url).is_err());
        }
    }
    #[test]
    fn corrupted_or_escaping_rollback_metadata_cannot_be_used() {
        let folder = tempfile::tempdir().unwrap();
        journal(folder.path(), "rollback.json", &serde_json::json!({"manifest":{},"package":"../elsewhere.exe"})).unwrap();
        assert!(cached(folder.path()).is_err());
    }

    #[test]
    fn signed_update_and_rollback_bytes_reject_tampering_and_version_replay() {
        // Public test vector from tauri-plugin-updater 2.14.1, MIT/Apache-2.0.
        // Only the public key/signature are stored; no signing key is shipped.
        let public = "untrusted comment: minisign public key: A49EF4746FA6E6D7\nRWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe";
        let signature = "untrusted comment: signature from tauri secret key\nRUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\ntrusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\nDDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA==";
        let encode = |value: &str| base64::engine::general_purpose::STANDARD.encode(value);
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().plugins.0.insert("updater".to_string(), serde_json::json!({"pubkey":"","requireSignedVersion":true}));
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().pubkey(encode(public)).build())
            .build(context).unwrap();
        let updater = app.updater_builder().target("windows-x86_64").executable_path(std::env::current_exe().unwrap())
            .endpoints(vec!["https://example.invalid/update".parse().unwrap()]).unwrap()
            .version_comparator(|_, _| true).build().unwrap();
        let manifest = |version: &str| serde_json::json!({
            "version":version,"url":"https://example.invalid/package","signature":encode(signature),
        });
        let valid = updater.restore_update(manifest("2.0.0")).unwrap().unwrap();
        valid.verify(b"test").unwrap();
        assert!(valid.verify(b"tampered bytes").is_err());
        let replay = updater.restore_update(manifest("3.0.0")).unwrap().unwrap();
        assert!(replay.verify(b"test").is_err());
        let folder = tempfile::tempdir().unwrap();
        let cache = write_cache(folder.path(), &valid, b"test").unwrap();
        journal(folder.path(), "rollback.json", &cache).unwrap();
        let saved = cached(folder.path()).unwrap();
        let rollback = updater.restore_update(saved.manifest).unwrap().unwrap();
        rollback.verify(&std::fs::read(folder.path().join(saved.package)).unwrap()).unwrap();
    }
}
