//! Tauri command layer. Every command mirrors CONTRACT.md section 2 exactly —
//! names, argument shapes and the `CommandResult` envelope.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::domain::{
    codes, to_rfc3339, AddDownloadRequest, CommandResult, ControlRequest, ControlResult,
    DownloadProjection, DownloadStatus, MetadataProbe, PendingConfirmation, ProbeRequest,
    ResolveConfirmationRequest, StartPolicy,
};
use crate::engine::AppEngine;
use crate::security;
use crate::settings::Settings;
use crate::store::{
    BrowserIntake, Category, PendingConfirmationRow, Queue, Schedule, SecretMigrationResult, Store,
    TaskRow,
};

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub destination_template: String,
    pub extension_rules_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub max_active_tasks: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDto {
    pub id: String,
    pub queue_id: String,
    pub days_mask: i64,
    pub start_local_time: String,
    pub end_local_time: String,
    pub action_after: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    pub version: String,
    pub database_path: String,
    pub task_count: i64,
    pub active_count: i64,
    pub database_size_bytes: i64,
    pub events: Vec<EventRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventRow {
    pub event_type: String,
    pub created_at: String,
}

/// Paginated list of tasks. `has_more` is true when the server holds additional
/// rows beyond the requested limit.
#[derive(Debug, Clone, Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<DownloadProjection>,
    pub has_more: bool,
}

/// Safe per-segment diagnostic view. Contains offsets and progress only — no
/// headers, URLs, cookies or file contents.
#[derive(Debug, Clone, Serialize)]
pub struct SegmentDto {
    pub id: String,
    pub download_id: String,
    pub segment_index: i64,
    pub start_offset: i64,
    pub end_offset: i64,
    pub received_bytes: i64,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn data_of<T>(r: CommandResult<T>) -> Result<T, CommandResult<T>> {
    if r.ok && r.data.is_some() {
        Ok(r.data.expect("checked"))
    } else {
        Err(r)
    }
}

fn store_err(e: rusqlite::Error) -> CommandResult<()> {
    CommandResult::failure(codes::STORE_ERROR, e.to_string(), true)
}

/// Derive a filename from the URL's last path segment.
fn filename_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let last = parsed.path().rsplit('/').next()?;
    if last.is_empty() {
        return None;
    }
    Some(percent_decode(last))
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn add_download(
    state: State<'_, Arc<AppEngine>>,
    req: AddDownloadRequest,
) -> CommandResult<DownloadProjection> {
    let settings = state.settings().clone();
    let task = match prepare_download(state.store(), &settings, req) {
        Ok(task) => task,
        Err(error) => return error,
    };
    if let Some(parent) = std::path::Path::new(&task.destination_path).parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            return CommandResult::failure(
                codes::INVALID_DESTINATION,
                format!("Could not create the destination folder: {error}"),
                false,
            );
        }
    }
    if let Err(error) = state.store().insert_task(&task) {
        return CommandResult::failure(codes::STORE_ERROR, error.to_string(), true);
    }
    // Durable opt-in for server-provided naming. Only automatically named
    // tasks carry it; explicitly named and legacy tasks are never renamed.
    if crate::engine::is_automatic_download_name(&task.display_name) {
        crate::engine::set_auto_name_opt_in(state.store(), &task.id);
    }
    let _ = state.store().log_event(
        Some(&task.id),
        "download_added",
        &serde_json::json!({ "queue_id": task.queue_id }).to_string(),
    );
    // The scheduler admits queued tasks under the same limits whether the
    // request came from the UI or from a browser host.
    CommandResult::success(task.to_projection())
}

fn prepare_download(
    store: &Store,
    settings: &Settings,
    req: AddDownloadRequest,
) -> Result<TaskRow, CommandResult<DownloadProjection>> {
    let url = match data_of(security::validate_url(&req.url)) {
        Ok(u) => u,
        Err(r) => return Err(CommandResult { ok: false, data: None, error: r.error }),
    };
    if let Err(r) = data_of(security::validate_headers(&req.headers)) {
        return Err(CommandResult { ok: false, data: None, error: r.error });
    }

    let suggested = req
        .suggested_name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .or_else(|| filename_from_url(&url))
        .unwrap_or_else(|| "download".to_string());

    let categories = store.list_categories().map_err(|_| CommandResult::failure(
        codes::STORE_ERROR, "Could not read download categories", true,
    ))?;
    let category = if let Some(id) = req.category_id.as_deref() {
        Some(categories.iter().find(|category| category.id == id).ok_or_else(|| CommandResult::failure(
            codes::INVALID_ACTION, "The selected download category is unavailable", false,
        ))?)
    } else {
        let extension = std::path::Path::new(&suggested).extension().and_then(|value| value.to_str()).unwrap_or("");
        categories.iter().find(|category| category_matches(category, extension))
    };
    let root = match req.destination_directory.as_deref().filter(|value| !value.trim().is_empty()) {
        Some(explicit) => explicit.to_string(),
        None => match category {
            Some(category) => expand_category_template(
                &category.destination_template,
                std::env::var("USERPROFILE").ok().as_deref(),
                &settings.destination_root,
            ).ok_or_else(|| CommandResult::failure(codes::INVALID_DESTINATION, "The category destination template is invalid", false))?,
            None => settings.destination_root.clone(),
        },
    };
    if !std::path::Path::new(&root).is_absolute() {
        return Err(CommandResult::failure(codes::INVALID_DESTINATION, "Choose a full destination folder path", false));
    }

    let final_path = match data_of(security::resolve_destination(std::path::Path::new(&root), &suggested)) {
        Ok(p) => p,
        Err(r) => return Err(CommandResult { ok: false, data: None, error: r.error }),
    };

    let queue_id = req
        .queue_id
        .clone()
        .filter(|q| !q.trim().is_empty())
        .unwrap_or_else(|| "default".to_string());
    let queue_ok = store
        .list_queues()
        .map(|qs| qs.iter().any(|q| q.id == queue_id))
        .unwrap_or(false);
    if !queue_ok {
        return Err(CommandResult::failure(
            codes::INVALID_ACTION,
            format!("Unknown queue '{queue_id}'"),
            false,
        ));
    }

    let start_policy = req
        .start_policy
        .as_deref()
        .and_then(StartPolicy::parse)
        .unwrap_or(StartPolicy::Immediate);
    if req.start_policy.as_deref().is_some_and(|value| StartPolicy::parse(value).is_none()) {
        return Err(CommandResult::failure(codes::INVALID_ACTION, "Unknown download start policy", false));
    }
    if let Some(checksum) = &req.checksum {
        if let Some(value) = checksum.value.as_deref().filter(|value| !value.trim().is_empty()) {
            let length = match checksum.algorithm.to_ascii_lowercase().as_str() {
                "sha256" => 64, "sha512" => 128, "sha1" => 40, "md5" => 32,
                _ => 0,
            };
            if value.trim().len() != length || !value.trim().bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(CommandResult::failure(codes::INVALID_ACTION, "Provide a valid hexadecimal checksum for a supported algorithm", false));
            }
        }
    }
    let now = to_rfc3339(chrono::Utc::now());
    let headers_json = serde_json::to_string(&req.headers).unwrap_or_else(|_| "[]".to_string());
    let checksum = req.checksum.clone();

    Ok(TaskRow {
        id: new_uuid(),
        queue_id,
        category_id: category.map(|category| category.id.clone()),
        original_url: url,
        final_url: None,
        headers_json,
        display_name: suggested,
        destination_path: final_path.to_string_lossy().to_string(),
        temporary_path: None,
        status: DownloadStatus::Queued.as_str().to_string(),
        total_bytes: None,
        received_bytes: 0,
        etag: None,
        last_modified: None,
        accept_ranges: false,
        content_type: None,
        checksum_algorithm: checksum.as_ref().map(|c| c.algorithm.clone()),
        checksum_expected: checksum
            .as_ref()
            .and_then(|c| c.value.clone())
            .filter(|v| !v.trim().is_empty()),
        checksum_actual: None,
        checksum_verified: false,
        retry_count: 0,
        max_retries: settings.max_retries,
        last_error_code: None,
        last_error_message: None,
        priority: req.priority.unwrap_or(50).clamp(0, 100),
        start_policy: start_policy.as_str().to_string(),
        connections: req
            .connections
            .map(|c| c.clamp(1, crate::domain::MAX_CONNECTIONS_PER_TASK) as i64)
            .unwrap_or(settings.connections_per_task as i64),
        speed_limit: req
            .speed_limit
            .unwrap_or(settings.default_task_speed_limit)
            .min(i64::MAX as u64) as i64,
        overwrite_policy: settings.overwrite_policy.as_str().to_string(),
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    })
}

fn category_matches(category: &Category, extension: &str) -> bool {
    if extension.is_empty() { return false }
    serde_json::from_str::<Vec<String>>(&category.extension_rules_json).is_ok_and(|rules| {
        rules.iter().any(|rule| rule.trim_start_matches('.').eq_ignore_ascii_case(extension))
    })
}

fn expand_category_template(template: &str, user_profile: Option<&str>, downloads: &str) -> Option<String> {
    let mut path = template.trim().replace("%DOWNLOADS%", downloads);
    if path.contains("%USERPROFILE%") {
        path = path.replace("%USERPROFILE%", user_profile?);
    }
    (!path.is_empty() && !path.contains('%') && !path.chars().any(char::is_control)).then_some(path)
}

pub(crate) fn queue_browser_download(
    store: &Store,
    request: AddDownloadRequest,
    id: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), (&'static str, &'static str)> {
    const DISABLED: (&str, &str) = ("bridge_disabled", "Enable browser integration in SwiftFetch Settings before sending downloads.");
    let raw = store.get_setting("app_settings").ok_or(DISABLED)?;
    let mut settings = Settings::from_json(&raw).normalized();
    if !settings.browser_integration {
        return Err(DISABLED);
    }
    settings.overwrite_policy = crate::domain::OverwritePolicy::Rename;
    let mut task = prepare_download(store, &settings, request)
        .map_err(|_| ("download_rejected", "SwiftFetch could not prepare this download. Check the download folder and try again."))?;
    task.id = id.to_string();
    match store.insert_browser_task(&task, &raw, expires_at) {
        Ok(BrowserIntake::Accepted) => {
            if crate::engine::is_automatic_download_name(&task.display_name) {
                crate::engine::set_auto_name_opt_in(store, &task.id);
            }
            Ok(())
        },
        Ok(BrowserIntake::SettingsChanged) => Err(("settings_changed", "SwiftFetch settings changed. Send the download again.")),
        Ok(BrowserIntake::Replayed) => Err(("replayed_intent", "SwiftFetch already received this download request.")),
        Ok(BrowserIntake::Expired) => Err(("expired_intent", "The download request expired. Send it again.")),
        Err(_) => Err(("queue_unavailable", "SwiftFetch could not save the download to its local queue.")),
    }
}

/// Input recorded by the native host when a browser download needs confirmation.
pub(crate) struct BrowserConfirmationInput {
    pub id: String,
    pub url: String,
    pub page_origin: String,
    pub suggested_name: Option<String>,
    pub content_type: String,
    pub size_hint: Option<u64>,
    pub mime: Option<String>,
    pub kind: String,
    pub site_rule: String,
    pub media_json: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Persist a pending confirmation. The native host may call this before the
/// desktop UI is running; the UI picks the row up on launch.
pub(crate) fn prepare_browser_confirmation(
    store: &Store,
    input: BrowserConfirmationInput,
) -> Result<(), (&'static str, &'static str)> {
    let raw = store
        .get_setting("app_settings")
        .ok_or(("bridge_disabled", "SwiftFetch settings are unavailable."))?;
    let settings = Settings::from_json(&raw).normalized();
    if !settings.browser_integration {
        return Err(("bridge_disabled", "Enable browser integration in SwiftFetch Settings before sending downloads."));
    }
    let pending = PendingConfirmationRow {
        id: input.id,
        url: input.url,
        page_origin: input.page_origin,
        suggested_name: input.suggested_name,
        content_type: input.content_type,
        size_hint: input.size_hint.map(|value| value as i64),
        mime: input.mime,
        kind: input.kind,
        site_rule: input.site_rule,
        media_json: input.media_json,
        settings_json: raw,
        created_at: to_rfc3339(chrono::Utc::now()),
        expires_at: to_rfc3339(input.expires_at),
    };
    match store.insert_pending_confirmation(&pending) {
        Ok(BrowserIntake::Accepted) => Ok(()),
        Ok(BrowserIntake::Replayed) => Err(("replayed_intent", "This download request was already received.")),
        Ok(BrowserIntake::Expired) => Err(("expired_intent", "The download request expired before confirmation.")),
        Ok(BrowserIntake::SettingsChanged) => Err(("settings_changed", "SwiftFetch settings changed. Send the download again.")),
        Err(_) => Err(("queue_unavailable", "SwiftFetch could not save the confirmation locally.")),
    }
}

#[tauri::command]
pub fn list_pending_confirmations(state: State<'_, Arc<AppEngine>>) -> CommandResult<Vec<PendingConfirmation>> {
    match state.store().list_pending_confirmations() {
        Ok(rows) => CommandResult::success(rows),
        Err(error) => CommandResult::failure(codes::STORE_ERROR, format!("Could not load pending confirmations: {error}"), true),
    }
}

#[tauri::command]
pub fn resolve_browser_confirmation(
    state: State<'_, Arc<AppEngine>>,
    req: ResolveConfirmationRequest,
) -> CommandResult<()> {
    let store = state.store();
    let pending = match store.pending_confirmation(&req.id) {
        Ok(Some(row)) => row,
        Ok(None) => return CommandResult::failure(codes::TASK_NOT_FOUND, "This confirmation is no longer available.", false),
        Err(error) => return CommandResult::failure(codes::STORE_ERROR, format!("Could not load the confirmation: {error}"), true),
    };
    if req.action == "cancel" {
        let _ = store.resolve_pending_confirmation(&req.id, false);
        return CommandResult::success(());
    }
    if req.action != "start" && req.action != "queue" {
        return CommandResult::failure(codes::INVALID_ACTION, "Unknown confirmation action.", false);
    }
    let selected_url = match &req.url {
        Some(value) => {
            let checked = security::validate_url(value);
            if !checked.ok {
                let error = checked.error.unwrap();
                return CommandResult::failure(&error.code, error.message, error.retryable);
            }
            checked.data.unwrap_or_default()
        }
        None => pending.url.clone(),
    };
    let request = AddDownloadRequest {
        url: selected_url,
        suggested_name: req.suggested_name.or(pending.suggested_name),
        destination_directory: req.destination_directory,
        queue_id: req.queue_id,
        category_id: req.category_id,
        priority: None,
        start_policy: Some(if req.action == "start" { "immediate" } else { "manual" }.to_string()),
        headers: Vec::new(),
        checksum: None,
        connections: None,
        speed_limit: None,
    };
    match store.resolve_pending_confirmation(&req.id, true) {
        Ok(()) => {}
        Err(error) => return CommandResult::failure(codes::STORE_ERROR, format!("Could not update the confirmation: {error}"), true),
    }
    let expiry = chrono::DateTime::parse_from_rfc3339(&pending.expires_at)
        .map(|value| value.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now() + chrono::TimeDelta::minutes(5));
    match queue_browser_download(&store, request, &req.id, expiry) {
        Ok(()) => CommandResult::success(()),
        Err((code, message)) => CommandResult::failure(code, message, true),
    }
}

#[tauri::command]
pub fn probe_download_metadata(
    state: State<'_, Arc<AppEngine>>,
    req: ProbeRequest,
) -> CommandResult<MetadataProbe> {
    let validated = security::validate_url(&req.url);
    if !validated.ok {
        let error = validated.error.unwrap();
        return CommandResult::failure(&error.code, error.message, error.retryable);
    }
    let url = validated.data.unwrap_or_default();
    let engine = Arc::clone(&state);
    match tauri::async_runtime::block_on(engine.probe_metadata(&url)) {
        Ok(meta) => {
            let suggested = meta
                .content_disposition
                .as_deref()
                .and_then(security::safe_filename_from_content_disposition);
            CommandResult::success(MetadataProbe {
                size: meta.total_bytes,
                content_type: meta.content_type,
                suggested_name: suggested,
            })
        }
        Err(error) => CommandResult::failure(error.code, error.message, error.retryable),
    }
}

#[tauri::command]
pub fn inspect_media_manifest(
    req: ProbeRequest,
) -> CommandResult<Vec<crate::media_manifest::MediaVariant>> {
    let validated = security::validate_url(&req.url);
    if !validated.ok {
        let error = validated.error.unwrap();
        return CommandResult::failure(&error.code, error.message, error.retryable);
    }
    let url = validated.data.unwrap_or_default();
    let result = tauri::async_runtime::block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|_| codes::NETWORK_ERROR.to_string())?;
        let response = client.get(&url).send().await.map_err(|error| error.to_string())?;
        if response.status().is_success() == false {
            return Err(format!("manifest request returned HTTP {}", response.status().as_u16()));
        }
        if response.content_length().is_some_and(|len| len as usize > crate::media_manifest::MAX_MANIFEST_BYTES) {
            return Err("The media manifest is too large.".to_string());
        }
        let bytes = response.bytes().await.map_err(|error| error.to_string())?;
        if bytes.len() > crate::media_manifest::MAX_MANIFEST_BYTES {
            return Err("The media manifest is too large.".to_string());
        }
        let text = String::from_utf8_lossy(&bytes).to_string();
        Ok(crate::media_manifest::parse(&url, &text))
    });
    match result {
        Ok(parsed) => parsed,
        Err(message) => CommandResult::failure(codes::NETWORK_ERROR, message, true),
    }
}

#[tauri::command]
pub fn control_download(
    state: State<'_, Arc<AppEngine>>,
    req: ControlRequest,
) -> CommandResult<ControlResult> {
    let result = state.control(&req.task_id, &req.action, req.priority);

    // Explicitly starting a manual task makes it eligible for scheduler admission.
    if result.ok && matches!(req.action.as_str(), "start" | "resume" | "retry") {
        if let Err(error) = state.store().make_task_automatic(&req.task_id) {
            return CommandResult::failure(codes::STORE_ERROR, error.to_string(), true);
        }
    }
    if result.ok && req.action == "retry" {
        state.store().set_retry_count(&req.task_id, 0).ok();
    }
    result
}

#[tauri::command]
pub fn list_tasks(
    state: State<'_, Arc<AppEngine>>,
    limit: i64,
    cursor: Option<String>,
    status: Option<String>,
) -> CommandResult<TaskListResponse> {
    // Bound the page size: an unbounded limit from the UI (or a malformed IPC
    // call) must not turn into an unbounded allocation.
    let limit = limit.clamp(1, 500);
    // Cursor format: "{updated_at}|{id}". A malformed cursor is treated as a
    // request for the first page rather than failing the whole query.
    let cursor = cursor.as_deref().and_then(|c| c.split_once('|'));

    // Filtering and ordering happen in SQL before the limit. Fetching a raw
    // top-(limit+1) and filtering afterwards silently dropped matching rows
    // that happened to be older than non-matching ones.
    let rows = match state
        .store()
        .list_tasks_page(limit + 1, cursor, status.as_deref())
    {
        Ok(r) => r,
        Err(e) => return CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
    };

    let has_more = rows.len() > limit as usize;
    let tasks: Vec<DownloadProjection> = rows
        .into_iter()
        .take(limit as usize)
        .map(|r| project_task(state.store(), &r))
        .collect();

    CommandResult::success(TaskListResponse { tasks, has_more })
}

#[tauri::command]
pub fn get_task(
    state: State<'_, Arc<AppEngine>>,
    task_id: String,
) -> CommandResult<DownloadProjection> {
    match state.store().get_task(&task_id) {
        Ok(Some(row)) => CommandResult::success(project_task(state.store(), &row)),
        Ok(None) => CommandResult::failure(codes::TASK_NOT_FOUND, "No such download task", false),
        Err(e) => CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
    }
}

#[tauri::command]
pub fn get_task_segments(
    state: State<'_, Arc<AppEngine>>,
    task_id: String,
) -> CommandResult<Vec<SegmentDto>> {
    match state.store().list_segments(&task_id) {
        Ok(rows) => CommandResult::success(
            rows.into_iter()
                .map(|s| SegmentDto {
                    id: s.id,
                    download_id: s.download_id,
                    segment_index: s.segment_index,
                    start_offset: s.start_offset,
                    end_offset: s.end_offset,
                    received_bytes: s.received_bytes,
                    status: s.status,
                })
                .collect(),
        ),
        Err(e) => CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppEngine>>) -> CommandResult<Settings> {
    let raw = state
        .store()
        .get_setting("app_settings")
        .unwrap_or_else(|| Settings::default().to_json());
    CommandResult::success(Settings::from_json(&raw).normalized())
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, Arc<AppEngine>>,
    settings: Settings,
) -> CommandResult<()> {
    let normalized = settings.normalized();
    if normalized.scan_with_antivirus && normalized.scanner_command.trim().is_empty() {
        return CommandResult::failure(codes::INVALID_ACTION, "Configure a trusted scanner command before enabling scans", false);
    }
    let startup_changed = state.settings().start_on_login != normalized.start_on_login;
    let client = match crate::build_http_client(&normalized.proxy) {
        Ok(client) => client,
        Err(_) => return CommandResult::failure(codes::INVALID_ACTION, "Use an HTTP or HTTPS proxy URL without credentials, or leave it empty for direct connections", false),
    };
    if startup_changed {
        if let Err(message) = crate::engine::set_start_on_login(normalized.start_on_login) {
            return CommandResult::failure(codes::INVALID_ACTION, message, false);
        }
    }
    if let Err(e) = state
        .store()
        .set_setting("app_settings", &normalized.to_json())
    {
        return store_err(e);
    }
    state.inner().set_settings(normalized.clone());
    state.inner().set_http_client(client);
    // An unrelated save must not rewrite the current user's startup setting.
    CommandResult::success(())
}

/// Explicit, opt-in migration: enable at-rest protection of saved request
/// headers and rewrite existing rows in one transaction. `confirm` mirrors the
/// other recovery commands — nothing changes without an acknowledged intent.
/// Requires `commands::encrypt_saved_secrets` to be registered in
/// `lib.rs` -> `generate_handler!` (reported to the parent agent).
#[tauri::command]
pub fn encrypt_saved_secrets(
    state: State<'_, Arc<AppEngine>>,
    confirm: bool,
) -> CommandResult<SecretMigrationResult> {
    if !confirm {
        return CommandResult::failure(
            codes::INVALID_ACTION,
            "Confirm protecting saved request headers before continuing",
            false,
        );
    }
    let already_on = state.settings().encrypt_saved_headers;
    match state.store().encrypt_saved_secrets(true) {
        Ok(result) => {
            // Keep the in-memory copy in step with the persisted flag.
            if !already_on {
                let mut settings = state.settings().clone();
                settings.encrypt_saved_headers = true;
                state.inner().set_settings(settings);
            }
            CommandResult::success(result)
        }
        Err(error) => CommandResult::failure(codes::STORE_ERROR, error.to_string(), true),
    }
}

#[tauri::command]
pub fn list_categories(state: State<'_, Arc<AppEngine>>) -> CommandResult<Vec<CategoryDto>> {
    match state.store().list_categories() {
        Ok(rows) => CommandResult::success(rows.into_iter().map(category_to_dto).collect()),
        Err(e) => CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
    }
}

fn category_to_dto(c: Category) -> CategoryDto {
    CategoryDto {
        id: c.id,
        name: c.name,
        destination_template: c.destination_template,
        extension_rules_json: c.extension_rules_json,
    }
}

fn queue_to_dto(q: Queue) -> QueueDto {
    QueueDto {
        id: q.id,
        name: q.name,
        enabled: q.enabled,
        max_active_tasks: q.max_active_tasks,
    }
}

fn schedule_to_dto(s: Schedule) -> ScheduleDto {
    ScheduleDto {
        id: s.id,
        queue_id: s.queue_id,
        days_mask: s.days_mask,
        start_local_time: s.start_local_time,
        end_local_time: s.end_local_time,
        action_after: s.action_after,
        enabled: s.enabled,
    }
}

#[tauri::command]
pub fn save_category(
    state: State<'_, Arc<AppEngine>>,
    cat: CategoryDto,
) -> CommandResult<()> {
    let rules = serde_json::from_str::<Vec<String>>(&cat.extension_rules_json);
    if cat.id.trim().is_empty() || cat.id.len() > 128 || cat.name.trim().is_empty() || cat.name.len() > 512
        || cat.destination_template.len() > 2_048 || cat.extension_rules_json.len() > 8_192
        || !rules.is_ok_and(|rules| rules.len() <= 128 && rules.iter().all(|rule| {
            let rule = rule.trim_start_matches('.');
            !rule.is_empty() && rule.len() <= 32 && rule.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        }))
        || expand_category_template(&cat.destination_template, std::env::var("USERPROFILE").ok().as_deref(), &state.settings().destination_root)
            .is_none_or(|path| !std::path::Path::new(&path).is_absolute())
    {
        return CommandResult::failure(codes::INVALID_ACTION, "Use a category name, full destination template, and a JSON array of filename extensions", false);
    }
    if let Err(e) = state.store().upsert_category(
        &cat.id,
        &cat.name,
        &cat.destination_template,
        &cat.extension_rules_json,
    ) {
        return store_err(e);
    }
    CommandResult::success(())
}

#[tauri::command]
pub fn list_queues(state: State<'_, Arc<AppEngine>>) -> CommandResult<Vec<QueueDto>> {
    match state.store().list_queues() {
        Ok(rows) => CommandResult::success(rows.into_iter().map(queue_to_dto).collect()),
        Err(e) => CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
    }
}

#[tauri::command]
pub fn list_schedules(state: State<'_, Arc<AppEngine>>) -> CommandResult<Vec<ScheduleDto>> {
    match state.store().list_schedules() {
        Ok(rows) => CommandResult::success(rows.into_iter().map(schedule_to_dto).collect()),
        Err(e) => CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
    }
}

#[tauri::command]
pub fn save_schedule(
    state: State<'_, Arc<AppEngine>>,
    schedule: ScheduleDto,
) -> CommandResult<()> {
    let valid_time = |value: &str| {
        value.len() == 5 && value.as_bytes()[2] == b':'
            && chrono::NaiveTime::parse_from_str(value, "%H:%M").is_ok()
    };
    if schedule.id.trim().is_empty() || schedule.id.len() > 128 || !(1..=127).contains(&schedule.days_mask)
        || !valid_time(&schedule.start_local_time) || !valid_time(&schedule.end_local_time)
        || schedule.start_local_time == schedule.end_local_time
        || !matches!(schedule.action_after.as_str(), "nothing" | "pause" | "shutdown")
        || !state.store().list_queues().is_ok_and(|queues| queues.iter().any(|queue| queue.id == schedule.queue_id))
    {
        return CommandResult::failure(codes::INVALID_ACTION, "Choose a queue, weekdays, distinct HH:MM times, and a supported after-window action", false);
    }
    if let Err(e) = state.store().upsert_schedule(
        &schedule.id,
        &schedule.queue_id,
        schedule.days_mask,
        &schedule.start_local_time,
        &schedule.end_local_time,
        &schedule.action_after,
        schedule.enabled,
    ) {
        return store_err(e);
    }
    CommandResult::success(())
}

#[tauri::command]
pub fn delete_schedule(state: State<'_, Arc<AppEngine>>, id: String) -> CommandResult<()> {
    match state.store().delete_schedule(&id) {
        Ok(()) => CommandResult::success(()),
        Err(e) => CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
    }
}

#[tauri::command]
pub fn get_diagnostics(state: State<'_, Arc<AppEngine>>) -> CommandResult<Diagnostics> {
    let store = state.store();
    let task_count = store.count_tasks().unwrap_or(0);
    let active_count = store.count_active().unwrap_or(0);
    let database_size_bytes = std::fs::metadata(store.path())
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let events = store
        .list_recent_events(50)
        .unwrap_or_default()
        .into_iter()
        .map(|(event_type, created_at)| EventRow {
            event_type,
            created_at,
        })
        .collect();

    CommandResult::success(Diagnostics {
        version: env!("CARGO_PKG_VERSION").to_string(),
        database_path: store.path().to_string_lossy().to_string(),
        task_count,
        active_count,
        database_size_bytes,
        events,
    })
}

#[tauri::command]
pub fn clear_history(state: State<'_, Arc<AppEngine>>) -> CommandResult<()> {
    match state.store().clear_finished_tasks() {
        Ok(n) => {
            state.store().log_event(None, "history_cleared", &serde_json::json!({ "removed": n }).to_string()).ok();
            CommandResult::success(())
        }
        Err(e) => CommandResult::failure(codes::STORE_ERROR, e.to_string(), true),
    }
}

/// Build the UI projection for a task, deriving segment counts from the store.
fn project_task(store: &Store, row: &TaskRow) -> DownloadProjection {
    let mut projection = row.to_projection();
    // While a download is in flight the downloads row still carries the
    // automatic-name sentinel. Surface the server-provided name and the final
    // destination the engine will publish to. Once the task completes the
    // engine clears this marker and `complete_task` persists the real path, so
    // the projection falls back to the stored values (including Rename results).
    if let Some(name) = crate::engine::server_filename_for(store, &row.id) {
        if crate::engine::is_automatic_download_name(&row.display_name)
            && crate::engine::is_auto_name_opted_in(store, &row.id)
        {
            projection.display_name = name.clone();
            if let Some(path) = crate::engine::server_named_destination(&row.destination_path, Some(&name)) {
                projection.destination_path = path.to_string_lossy().to_string();
            }
        }
    }
    let (total, done) = store.segment_counts(&row.id);
    projection.segment_count = total;
    projection.completed_segments = done;
    projection
}

#[cfg(test)]
mod preparation_tests {
    use super::*;

    fn request() -> AddDownloadRequest {
        AddDownloadRequest {
            url: "https://example.com/release.ZIP".to_string(),
            suggested_name: None, destination_directory: None, queue_id: None,
            category_id: None, priority: None, start_policy: None, headers: Vec::new(),
            checksum: None, connections: None, speed_limit: Some(0),
        }
    }

    fn auto_named_row(id: &str, display_name: &str, destination: &str) -> TaskRow {
        let now = to_rfc3339(chrono::Utc::now());
        TaskRow {
            id: id.to_string(),
            queue_id: "default".to_string(),
            category_id: None,
            original_url: "https://example.com/".to_string(),
            final_url: None,
            headers_json: "[]".to_string(),
            display_name: display_name.to_string(),
            destination_path: destination.to_string(),
            temporary_path: None,
            status: DownloadStatus::Downloading.as_str().to_string(),
            total_bytes: None,
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

    #[test]
    fn projection_applies_server_name_only_to_auto_named_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("state.sqlite")).unwrap();
        let auto_id = "auto-task";
        let explicit_id = "explicit-task";

        // Automatic task: persisted opt-in + marker are reflected in the view.
        let mut auto = auto_named_row(auto_id, "download", "D:/Downloads/download");
        crate::engine::set_auto_name_opt_in(&store, auto_id);
        assert!(crate::engine::is_auto_name_opted_in(&store, auto_id));
        crate::engine::set_server_filename(&store, auto_id, "server-file.pdf");
        let view = project_task(&store, &auto);
        assert_eq!(view.display_name, "server-file.pdf");
        assert!(view.destination_path.ends_with("server-file.pdf"));

        // Explicit task: the same marker must never rewrite a user/URL name.
        let explicit = auto_named_row(explicit_id, "chosen.bin", "D:/Downloads/chosen.bin");
        crate::engine::set_server_filename(&store, explicit_id, "server-file.pdf");
        let view = project_task(&store, &explicit);
        assert_eq!(view.display_name, "chosen.bin");
        assert_eq!(view.destination_path, "D:/Downloads/chosen.bin");

        // Legacy row named `download` but with NO creation-time opt-in must not
        // be renamed by inference.
        let legacy_id = "legacy-task";
        let legacy = auto_named_row(legacy_id, "download", "D:/Downloads/download");
        crate::engine::set_server_filename(&store, legacy_id, "server-file.pdf");
        let view = project_task(&store, &legacy);
        assert_eq!(view.display_name, "download");
        assert_eq!(view.destination_path, "D:/Downloads/download");

        // Once the marker is cleared (on completion) the stored values win.
        crate::engine::set_server_filename(&store, auto_id, "");
        auto.status = DownloadStatus::Completed.as_str().to_string();
        auto.display_name = "server-file.pdf".to_string();
        auto.destination_path = "D:/Downloads/server-file.pdf".to_string();
        let view = project_task(&store, &auto);
        assert_eq!(view.display_name, "server-file.pdf");
        assert!(view.destination_path.ends_with("server-file.pdf"));
    }

    #[test]
    fn category_rules_choose_a_destination_and_allow_an_explicit_override() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::open(&directory.path().join("state.sqlite")).unwrap();
        store.ensure_default_queue().unwrap();
        store.upsert_category("archives", "Archives", "%DOWNLOADS%/archives", "[\"zip\"]").unwrap();
        let settings = Settings { destination_root: directory.path().to_string_lossy().to_string(), default_task_speed_limit: 1024, ..Default::default() };
        let task = prepare_download(&store, &settings, request()).unwrap();
        assert_eq!(task.category_id.as_deref(), Some("archives"));
        assert!(std::path::Path::new(&task.destination_path).ends_with("archives/release.ZIP"));
        assert_eq!(task.speed_limit, 0, "explicit unlimited speed must override the default");
        let mut explicit = request();
        explicit.destination_directory = Some(directory.path().join("explicit").to_string_lossy().to_string());
        let task = prepare_download(&store, &settings, explicit).unwrap();
        assert!(std::path::Path::new(&task.destination_path).ends_with("explicit/release.ZIP"));
        assert!(!directory.path().join("archives").exists(), "preparing a task should not create folders");
    }

    #[test]
    fn unknown_category_policy_and_invalid_checksum_are_rejected_before_queuing() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::open(&directory.path().join("state.sqlite")).unwrap();
        store.ensure_default_queue().unwrap();
        let settings = Settings { destination_root: directory.path().to_string_lossy().to_string(), ..Default::default() };
        let mut unknown = request();
        unknown.category_id = Some("missing".to_string());
        assert!(prepare_download(&store, &settings, unknown).is_err());
        let mut unknown = request();
        unknown.start_policy = Some("something-else".to_string());
        assert!(prepare_download(&store, &settings, unknown).is_err());
        let mut invalid = request();
        invalid.checksum = Some(crate::domain::Checksum { algorithm: "sha256".to_string(), value: Some("wrong".to_string()) });
        assert!(prepare_download(&store, &settings, invalid).is_err());
        assert_eq!(store.count_tasks().unwrap(), 0);
    }

    #[test]
    fn category_templates_expand_only_the_documented_variables() {
        assert_eq!(expand_category_template("%USERPROFILE%/Downloads", Some("C:/User"), "D:/Downloads").as_deref(), Some("C:/User/Downloads"));
        assert_eq!(expand_category_template("%DOWNLOADS%/Archives", None, "D:/Downloads").as_deref(), Some("D:/Downloads/Archives"));
        assert!(expand_category_template("%USERPROFILE%/Downloads", None, "D:/Downloads").is_none());
        assert!(expand_category_template("%OTHER%/Downloads", Some("C:/User"), "D:/Downloads").is_none());
    }
}
