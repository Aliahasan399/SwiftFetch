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

fn validate(intent: Intent, now: DateTime<Utc>) -> Result<(String, AddDownloadRequest, DateTime<Utc>), Reply> {
    if intent.protocol != 1 || intent.kind != "download_intent" || !intent.user_initiated || !intent.cookies.is_empty() {
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
    let (id, request, expires_at) = match read_intent(input).and_then(|intent| validate(intent, Utc::now())) {
        Ok(validated) => validated,
        Err(reply) => return reply,
    };
    let Ok(store) = Store::open_existing(database) else {
        return Reply::reject("app_unavailable", "Open SwiftFetch and enable browser integration before sending downloads.");
    };
    match crate::commands::queue_browser_download(&store, request, &id, expires_at) {
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
}
