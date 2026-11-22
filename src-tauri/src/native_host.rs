//! One-shot Native Messaging host. The browser starts the installed executable
//! and passes one framed message through stdin.

use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::domain::AddDownloadRequest;
use crate::store::Store;

const FIREFOX_ID: &str = "swiftfetch@localhost";
const MAX_FRAME: usize = 16 * 1024;
const MAX_URL: usize = 2_048;
const MAX_METADATA_LABEL: usize = 128;
const MAX_MEDIA_VARIANTS: usize = 32;
const MAX_SIZE_HINT: u64 = 100 * 1024 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    protocol: u8,
    #[serde(rename = "type")]
    kind: String,
    request_id: String,
    url: String,
    suggested_name: Option<String>,
    page_origin: String,
    content_type: String,
    user_initiated: bool,
    cookies: Vec<serde_json::Value>,
    expires_at: DateTime<Utc>,
    #[serde(default)]
    metadata: Option<IntentMetadata>,
}

/// Protocol v2 hints. These are strictly non-sensitive: no cookies, headers or
/// credentials can be represented here.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct IntentMetadata {
    #[serde(default)]
    size_hint: Option<u64>,
    #[serde(default)]
    mime: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    auto_accept: Option<bool>,
    #[serde(default)]
    site_rule: Option<String>,
    #[serde(default)]
    media: Option<MediaMetadata>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct MediaMetadata {
    url: String,
    #[serde(default)]
    mime: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    variants: Vec<MediaVariant>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct MediaVariant {
    url: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    mime: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Reply {
    Accepted { ok: bool, value: Receipt },
    Rejected { ok: bool, code: &'static str, message: &'static str },
}

#[derive(Debug, Serialize)]
struct Receipt {
    task_id: String,
}

impl Reply {
    fn reject(code: &'static str, message: &'static str) -> Self {
        Self::Rejected { ok: false, code, message }
    }

    fn invalid_intent() -> Self {
        Self::reject("invalid_intent", "Invalid or unexpected download intent fields.")
    }
}

// A recognized but malformed invocation still uses the framed response so
// the browser sees an error without inadvertently launching the desktop UI.
fn identify_caller(args: &[OsString]) -> Option<bool> {
    let first = args.first()?;
    if let Some(origin) = first.to_str().and_then(|value| value.strip_prefix("chrome-extension://")) {
        let id = origin.strip_suffix('/').unwrap_or(origin);
        return Some(id.len() == 32 && id.bytes().all(|byte| (b'a'..=b'p').contains(&byte)));
    }
    let manifest = Path::new(first);
    if manifest.file_name()?.to_str()?.eq_ignore_ascii_case("com.swiftfetch.app.json") {
        return Some(manifest.is_absolute() && args.get(1).is_some_and(|id| id == OsStr::new(FIREFOX_ID)));
    }
    None
}

fn read_intent(mut input: impl Read) -> Result<Intent, Reply> {
    let mut header = [0; 4];
    let malformed_frame = || Reply::reject("invalid_frame", "Invalid Native Messaging frame.");
    input.read_exact(&mut header).map_err(|_| malformed_frame())?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(malformed_frame());
    }
    let mut data = vec![0; length];
    input.read_exact(&mut data).map_err(|_| malformed_frame())?;
    serde_json::from_slice(&data).map_err(|_| Reply::invalid_intent())
}

fn http_url(value: &str) -> Option<Url> {
    if value.len() > MAX_URL || value.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return None;
    }
    let parsed = Url::parse(value).ok()?;
    (matches!(parsed.scheme(), "http" | "https")
        && parsed.has_host()
        && parsed.username().is_empty()
        && parsed.password().is_none())
    .then_some(parsed)
}

fn mime_type(value: &str) -> bool {
    let token = |part: &str| {
        (1..=127).contains(&part.len())
            && part.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&byte))
    };
    value.split_once('/').is_some_and(|(kind, subtype)| token(kind) && token(subtype))
}

fn valid_metadata_label(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|label| {
        !label.is_empty()
            && label.chars().count() <= MAX_METADATA_LABEL
            && !label.chars().any(|ch| ch.is_control())
    })
}

fn validate_metadata(metadata: &Option<IntentMetadata>) -> Result<(), Reply> {
    let Some(metadata) = metadata else { return Ok(()) };
    if metadata.size_hint.is_some_and(|size| size > MAX_SIZE_HINT) {
        return Err(Reply::invalid_intent());
    }
    if metadata.mime.as_deref().is_some_and(|mime| !mime_type(mime)) {
        return Err(Reply::invalid_intent());
    }
    if metadata.kind.as_deref().is_some_and(|kind| !matches!(kind, "ordinary" | "media")) {
        return Err(Reply::invalid_intent());
    }
    if metadata.site_rule.as_deref().is_some_and(|rule| !matches!(rule, "ask" | "allow" | "deny")) {
        return Err(Reply::invalid_intent());
    }
    if !valid_metadata_label(&metadata.mime) && metadata.mime.is_some() {
        return Err(Reply::invalid_intent());
    }
    if let Some(media) = &metadata.media {
        if http_url(&media.url).is_none() || !valid_metadata_label(&media.label) {
            return Err(Reply::invalid_intent());
        }
        if media.mime.as_deref().is_some_and(|mime| !mime_type(mime)) {
            return Err(Reply::invalid_intent());
        }
        if media.width.is_some_and(|w| w > 10_000) || media.height.is_some_and(|h| h > 10_000) {
            return Err(Reply::invalid_intent());
        }
        if media.variants.len() > MAX_MEDIA_VARIANTS {
            return Err(Reply::invalid_intent());
        }
        for variant in &media.variants {
            if http_url(&variant.url).is_none() || !valid_metadata_label(&variant.label) {
                return Err(Reply::invalid_intent());
            }
            if variant.mime.as_deref().is_some_and(|mime| !mime_type(mime)) {
                return Err(Reply::invalid_intent());
            }
        }
    }
    Ok(())
}

fn validate(intent: Intent, now: DateTime<Utc>) -> Result<(String, AddDownloadRequest, DateTime<Utc>), Reply> {
    if !matches!(intent.protocol, 1 | 2)
        || intent.kind != "download_intent"
        || !intent.user_initiated
        || !intent.cookies.is_empty()
    {
        return Err(Reply::invalid_intent());
    }
    validate_metadata(&intent.metadata)?;
    if intent.protocol == 1 && intent.metadata.is_some() {
        return Err(Reply::invalid_intent());
    }
    if intent.expires_at <= now || intent.expires_at > now + TimeDelta::minutes(5) {
        return Err(Reply::reject("expired_intent", "The download intent is outside its five-minute validity window."));
    }
    let id = Uuid::parse_str(&intent.request_id).map_err(|_| Reply::invalid_intent())?;
    if id.get_version_num() != 4 || id.get_variant() != uuid::Variant::RFC4122
        || !id.hyphenated().to_string().eq_ignore_ascii_case(&intent.request_id)
    {
        return Err(Reply::invalid_intent());
    }
    let mut url = http_url(&intent.url).ok_or_else(Reply::invalid_intent)?;
    url.set_fragment(None);
    let origin = http_url(&intent.page_origin).ok_or_else(Reply::invalid_intent)?;
    if origin.path() != "/" || origin.query().is_some() || origin.fragment().is_some() || !mime_type(&intent.content_type) {
        return Err(Reply::invalid_intent());
    }
    if intent.suggested_name.as_ref().is_some_and(|name| {
        name.trim().is_empty() || name == "." || name == ".." || name.chars().count() > 200
            || name.chars().any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':'))
    }) {
        return Err(Reply::invalid_intent());
    }
    let request = AddDownloadRequest {
        url: url.to_string(),
        suggested_name: intent.suggested_name,
        destination_directory: None,
        queue_id: None,
        category_id: None,
        priority: None,
        start_policy: None,
        headers: Vec::new(),
        checksum: None,
        connections: None,
        speed_limit: None,
    };
    Ok((id.to_string(), request, intent.expires_at))
}

fn handle(input: impl Read, database: &Path) -> Reply {
    let intent = match read_intent(input) {
        Ok(intent) => intent,
        Err(reply) => return reply,
    };
    let protocol = intent.protocol;
    let page_origin = intent.page_origin.clone();
    let content_type = intent.content_type.clone();
    let metadata = intent.metadata.clone();
    let (id, request, expires_at) = match validate(intent, Utc::now()) {
        Ok(validated) => validated,
        Err(reply) => return reply,
    };
    let Ok(store) = Store::open_existing(database) else {
        return Reply::reject("app_unavailable", "Open SwiftFetch and enable browser integration before sending downloads.");
    };
    let _ = store.ensure_pending_schema();

    let site_rule = metadata
        .as_ref()
        .and_then(|value| value.site_rule.as_deref())
        .unwrap_or("ask")
        .to_string();
    let auto_accept = metadata
        .as_ref()
        .and_then(|value| value.auto_accept)
        .unwrap_or(false);

    if site_rule == "deny" {
        return Reply::reject("site_blocked", "This site is blocked in the SwiftFetch popup.");
    }

    // Legacy v1 intents were explicit gesture handoffs and have always queued
    // directly. Preserve that contract; only v2 intents opt into confirmation.
    if protocol == 1 || (auto_accept && site_rule == "allow") {
        return match crate::commands::queue_browser_download(&store, request, &id, expires_at) {
            Ok(()) => Reply::Accepted { ok: true, value: Receipt { task_id: id } },
            Err((code, message)) => Reply::reject(code, message),
        };
    }

    let size_hint = metadata.as_ref().and_then(|value| value.size_hint);
    let mime = metadata.as_ref().and_then(|value| value.mime.clone());
    let kind = metadata
        .as_ref()
        .and_then(|value| value.kind.clone())
        .unwrap_or_else(|| "ordinary".to_string());
    let media_json = serde_json::to_string(&metadata.as_ref().and_then(|value| value.media.clone()))
        .unwrap_or_else(|_| "null".to_string());
    let confirmation = crate::commands::BrowserConfirmationInput {
        id: id.clone(),
        url: request.url.clone(),
        page_origin,
        suggested_name: request.suggested_name.clone(),
        content_type,
        size_hint,
        mime,
        kind,
        site_rule,
        media_json,
        expires_at,
    };
    match crate::commands::prepare_browser_confirmation(&store, confirmation) {
        Ok(()) => Reply::Accepted { ok: true, value: Receipt { task_id: id } },
        Err((code, message)) => Reply::reject(code, message),
    }
}

fn write_reply(mut output: impl Write, reply: &Reply) -> io::Result<()> {
    let body = serde_json::to_vec(reply)?;
    output.write_all(&(body.len() as u32).to_le_bytes())?;
    output.write_all(&body)?;
    output.flush()
}

pub(super) fn run_if_requested(database: impl FnOnce() -> PathBuf) -> bool {
    let args: Vec<_> = std::env::args_os().skip(1).take(2).collect();
    let Some(allowed) = identify_caller(&args) else { return false };
    // The OS user's native host manifest holds the browser's caller allowlist.
    let reply = if allowed {
        handle(io::stdin().lock(), &database())
    } else {
        Reply::reject("invalid_caller", "This browser extension is unavailable for SwiftFetch.")
    };
    let _ = write_reply(io::stdout().lock(), &reply);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recognizes_windows_browser_arguments() {
        let parse = |args: &[&str]| identify_caller(&args.iter().map(OsString::from).collect::<Vec<_>>());
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&["chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/", "--parent-window=0"]), Some(true));
        assert_eq!(parse(&["chrome-extension://not-an-id/"]), Some(false));
        let file = std::env::current_dir().unwrap().join("com.swiftfetch.app.json");
        assert_eq!(parse(&[file.to_str().unwrap(), FIREFOX_ID]), Some(true));
        assert_eq!(parse(&[file.to_str().unwrap(), "another-extension"]), Some(false));
    }

    #[test]
    fn rejects_expired_frames_and_fields_that_might_carry_secrets() {
        let now = Utc::now();
        let make = |expiry: DateTime<Utc>| json!({
            "protocol": 1, "type": "download_intent", "request_id": Uuid::new_v4(),
            "url": "https://example.com/file.bin", "suggested_name": null,
            "page_origin": "https://example.com", "content_type": "application/octet-stream",
            "user_initiated": true, "cookies": [], "expires_at": expiry,
        });
        let frame = |value: serde_json::Value| {
            let body = serde_json::to_vec(&value).unwrap();
            let mut framed = (body.len() as u32).to_le_bytes().to_vec();
            framed.extend(body);
            framed
        };
        let valid = make(now + TimeDelta::minutes(1));
        assert!(validate(read_intent(frame(valid.clone()).as_slice()).unwrap(), now).is_ok());
        let expired = make(now - TimeDelta::seconds(1));
        assert!(validate(read_intent(frame(expired).as_slice()).unwrap(), now).is_err());
        let mut secret = valid.clone();
        secret["headers"] = json!([]);
        assert!(read_intent(frame(secret).as_slice()).is_err());
        let mut secret = valid;
        secret["cookies"] = json!([{}]);
        assert!(validate(read_intent(frame(secret).as_slice()).unwrap(), now).is_err());
        assert!(read_intent((u32::MAX).to_le_bytes().as_slice()).is_err());
    }

    #[test]
    fn queued_browser_intent_requires_opt_in_and_survives_history_clear() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let store = Store::open(&path).unwrap();
        store.ensure_default_queue().unwrap();
        let id = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + TimeDelta::minutes(2);
        let request = || AddDownloadRequest {
            url: "https://example.com/file.bin".to_string(),
            suggested_name: Some("file.bin".to_string()),
            destination_directory: None, queue_id: None, category_id: None,
            priority: None, start_policy: None, headers: Vec::new(),
            checksum: None, connections: None, speed_limit: None,
        };
        assert!(matches!(
            crate::commands::queue_browser_download(&store, request(), &id, expires_at),
            Err(("bridge_disabled", _))
        ));
        let settings = crate::settings::Settings {
            browser_integration: true,
            destination_root: directory.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        store.set_setting("app_settings", &settings.to_json()).unwrap();
        crate::commands::queue_browser_download(&store, request(), &id, expires_at).unwrap();
        store.insert_segment("segment", &id, 0, 0, 1023).unwrap();
        store.set_task_status(&id, crate::domain::DownloadStatus::Completed).unwrap();
        assert_eq!(store.clear_finished_tasks().unwrap(), 1);
        assert!(store.get_task(&id).unwrap().is_none());
        assert!(store.list_segments(&id).unwrap().is_empty());
        assert!(matches!(
            crate::commands::queue_browser_download(&store, request(), &id, expires_at),
            Err(("replayed_intent", _))
        ));
        let new_id = Uuid::new_v4().to_string();
        crate::commands::queue_browser_download(&store, request(), &new_id, expires_at).unwrap();
        store.delete_task(&new_id).unwrap();
        assert!(matches!(
            crate::commands::queue_browser_download(&store, request(), &new_id, expires_at),
            Err(("replayed_intent", _))
        ));
    }

    fn framed(value: serde_json::Value) -> Vec<u8> {
        let body = serde_json::to_vec(&value).unwrap();
        let mut bytes = (body.len() as u32).to_le_bytes().to_vec();
        bytes.extend(body);
        bytes
    }

    #[test]
    fn v2_intent_with_ask_rule_creates_pending_confirmation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let store = Store::open(&path).unwrap();
        store.ensure_default_queue().unwrap();
        store.ensure_default_category().unwrap();
        let settings = crate::settings::Settings {
            browser_integration: true,
            ..Default::default()
        };
        store.set_setting("app_settings", &settings.to_json()).unwrap();
        drop(store);

        let intent = json!({
            "protocol": 2,
            "type": "download_intent",
            "request_id": Uuid::new_v4().to_string(),
            "url": "https://example.com/video.mp4",
            "suggested_name": "video.mp4",
            "page_origin": "https://example.com",
            "content_type": "video/mp4",
            "user_initiated": true,
            "cookies": [],
            "expires_at": (Utc::now() + TimeDelta::minutes(2)).to_rfc3339(),
            "metadata": {
                "kind": "media",
                "site_rule": "ask",
                "size_hint": 4096,
                "mime": "video/mp4",
                "media": { "url": "https://example.com/video.mp4", "label": "Original" }
            }
        });
        let reply = handle(framed(intent).as_slice(), &path);
        assert!(matches!(reply, Reply::Accepted { .. }));

        let store = Store::open_existing(&path).unwrap();
        let pending = store.list_pending_confirmations().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].url, "https://example.com/video.mp4");
        assert_eq!(pending[0].size_hint, Some(4096));
        assert_eq!(store.count_tasks().unwrap(), 0);
    }

    #[test]
    fn v1_intent_keeps_legacy_direct_queue_behavior() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let store = Store::open(&path).unwrap();
        store.ensure_default_queue().unwrap();
        store.ensure_default_category().unwrap();
        let settings = crate::settings::Settings {
            browser_integration: true,
            ..Default::default()
        };
        store.set_setting("app_settings", &settings.to_json()).unwrap();
        drop(store);

        let intent = json!({
            "protocol": 1,
            "type": "download_intent",
            "request_id": Uuid::new_v4().to_string(),
            "url": "https://example.com/legacy.bin",
            "suggested_name": "legacy.bin",
            "page_origin": "https://example.com",
            "content_type": "application/octet-stream",
            "user_initiated": true,
            "cookies": [],
            "expires_at": (Utc::now() + TimeDelta::minutes(2)).to_rfc3339()
        });
        let reply = handle(framed(intent).as_slice(), &path);
        assert!(matches!(reply, Reply::Accepted { .. }));
        let store = Store::open_existing(&path).unwrap();
        assert_eq!(store.count_tasks().unwrap(), 1);
        assert_eq!(store.list_pending_confirmations().unwrap().len(), 0);
    }

    #[test]
    fn pending_confirmation_resolution_controls_task_creation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let store = Store::open(&path).unwrap();
        store.ensure_default_queue().unwrap();
        store.ensure_default_category().unwrap();
        let settings = crate::settings::Settings {
            browser_integration: true,
            ..Default::default()
        };
        let raw = settings.to_json();
        store.set_setting("app_settings", &raw).unwrap();

        let make = |id: String| crate::store::PendingConfirmationRow {
            id,
            url: "https://example.com/file.bin".to_string(),
            page_origin: "https://example.com".to_string(),
            suggested_name: Some("file.bin".to_string()),
            content_type: "application/octet-stream".to_string(),
            size_hint: Some(128),
            mime: None,
            kind: "ordinary".to_string(),
            site_rule: "ask".to_string(),
            media_json: "null".to_string(),
            settings_json: raw.clone(),
            created_at: Utc::now().to_rfc3339(),
            expires_at: (Utc::now() + TimeDelta::minutes(3)).to_rfc3339(),
        };

        let cancelled = Uuid::new_v4().to_string();
        store.insert_pending_confirmation(&make(cancelled.clone())).unwrap();
        assert_eq!(store.list_pending_confirmations().unwrap().len(), 1);
        store.resolve_pending_confirmation(&cancelled, false).unwrap();
        assert_eq!(store.list_pending_confirmations().unwrap().len(), 0);
        assert_eq!(store.count_tasks().unwrap(), 0);

        let accepted = Uuid::new_v4().to_string();
        store.insert_pending_confirmation(&make(accepted.clone())).unwrap();
        let pending = store.pending_confirmation(&accepted).unwrap().unwrap();
        store.resolve_pending_confirmation(&accepted, true).unwrap();
        let request = AddDownloadRequest {
            url: pending.url,
            suggested_name: pending.suggested_name,
            destination_directory: None,
            queue_id: None,
            category_id: None,
            priority: None,
            start_policy: Some("manual".to_string()),
            headers: Vec::new(),
            checksum: None,
            connections: None,
            speed_limit: None,
        };
        let expiry = Utc::now() + TimeDelta::minutes(2);
        crate::commands::queue_browser_download(&store, request, &accepted, expiry).unwrap();
        assert_eq!(store.count_tasks().unwrap(), 1);
        assert_eq!(store.list_pending_confirmations().unwrap().len(), 0);
    }

    #[test]
    fn v2_metadata_cannot_carry_headers() {
        let intent = json!({
            "protocol": 2,
            "type": "download_intent",
            "request_id": Uuid::new_v4().to_string(),
            "url": "https://example.com/file.zip",
            "suggested_name": "file.zip",
            "page_origin": "https://example.com",
            "content_type": "application/zip",
            "user_initiated": true,
            "cookies": [],
            "expires_at": (Utc::now() + TimeDelta::minutes(2)).to_rfc3339(),
            "metadata": { "headers": [{ "name": "Cookie", "value": "secret" }] }
        });
        let reply = handle(framed(intent).as_slice(), Path::new("missing.sqlite"));
        assert!(matches!(reply, Reply::Rejected { .. }));
    }
}
