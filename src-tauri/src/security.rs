//! URL, redirect and filesystem-safety policy.
//!
//! Everything here is a security boundary. The rules are deliberately strict:
//! reject first, ask later. See specification §10.2 and §10.3.

use std::path::{Component, Path, PathBuf};

use crate::domain::{codes, CommandResult, HeaderPair};

const MAX_URL_LENGTH: usize = 2048;
const MAX_FILENAME_LENGTH: usize = 200;
const MAX_PATH_LENGTH: usize = 260;

/// Schemes the engine will ever talk to.
const ALLOWED_SCHEMES: [&str; 2] = ["http", "https"];

/// Windows reserved device names (with or without an extension).
const RESERVED_NAMES: [&str; 12] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
];

/// Reserved names COM9/LPT1-9 are also blocked; kept separate for clarity.
const RESERVED_NAMES_EXTRA: [&str; 10] = [
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Characters that are invalid in a Windows filename component.
fn is_invalid_filename_char(c: char) -> bool {
    if (c as u32) < 0x20 {
        return true;
    }
    matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
}

/// Validate a user-supplied URL against the scheme and length policy.
///
/// Returns the normalized URL string on success.
pub fn validate_url(raw: &str) -> CommandResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return CommandResult::failure(codes::INVALID_URL, "URL is empty", false);
    }
    if trimmed.len() > MAX_URL_LENGTH {
        return CommandResult::failure(
            codes::URL_TOO_LONG,
            format!("URL exceeds {MAX_URL_LENGTH} characters"),
            false,
        );
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return CommandResult::failure(
            codes::INVALID_URL,
            "URL contains control characters",
            false,
        );
    }

    let parsed = match url::Url::parse(trimmed) {
        Ok(u) => u,
        Err(e) => {
            return CommandResult::failure(
                codes::INVALID_URL,
                format!("URL could not be parsed: {e}"),
                false,
            )
        }
    };

    let scheme = parsed.scheme().to_ascii_lowercase();
    if !ALLOWED_SCHEMES.contains(&scheme.as_str()) {
        return CommandResult::failure(
            codes::UNSUPPORTED_SCHEME,
            format!("Scheme '{scheme}' is not allowed (only http and https)"),
            false,
        );
    }
    if parsed.host_str().is_none() {
        return CommandResult::failure(codes::INVALID_URL, "URL has no host", false);
    }
    // Credentials in a URL leak into history, diagnostics and error messages.
    // Callers that need authentication must supply a header explicitly, with
    // the user's consent, instead of embedding userinfo here.
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return CommandResult::failure(
            codes::INVALID_URL,
            "Credentials must not be embedded in the URL",
            false,
        );
    }

    CommandResult::success(parsed.to_string())
}

/// Reject headers that would let a caller rewrite request framing or hop-by-hop
/// behaviour. Ordinary custom headers (including `Authorization` and `Cookie`,
/// which the user supplies explicitly) remain allowed.
pub fn validate_headers(headers: &[HeaderPair]) -> CommandResult<()> {
    const FORBIDDEN: [&str; 9] = [
        "host",
        "connection",
        "content-length",
        "transfer-encoding",
        "upgrade",
        "proxy-authorization",
        "proxy-authenticate",
        "te",
        "trailer",
    ];
    const TOKEN_EXTRA: &[u8] = b"!#$%&'*+-.^_`|~";
    for h in headers {
        let name = h.name.trim().to_ascii_lowercase();
        let name_ok = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || TOKEN_EXTRA.contains(&b));
        if !name_ok {
            return CommandResult::failure(codes::INVALID_ACTION, "Invalid header name", false);
        }
        if FORBIDDEN.contains(&name.as_str()) {
            return CommandResult::failure(
                codes::INVALID_ACTION,
                "This request header cannot be set",
                false,
            );
        }
        if h.value.bytes().any(|b| b == b'\r' || b == b'\n') {
            return CommandResult::failure(
                codes::INVALID_ACTION,
                "Header value contains a line break",
                false,
            );
        }
    }
    CommandResult::success(())
}

/// Derive a safe filename from a `Content-Disposition` header.
///
/// Only inline/attachment `filename` or RFC 5987 `filename*` are honoured, and
/// the result is passed through the same sanitizer as user-supplied names. A
/// server can never choose a directory or a device name. This is a *suggestion*
/// only: the caller decides whether the user opted into server-named files.
pub fn safe_filename_from_content_disposition(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    if !lower.contains("attachment") && !lower.contains("inline") {
        return None;
    }
    // Prefer filename*= (UTF-8) when present, then filename=.
    let starred = header_parameter(value, "filename*");
    let candidate = starred
        .and_then(|v| decode_extended_parameter(&v))
        .or_else(|| {
        header_parameter(value, "filename")
            .as_deref()
            .map(unquote_parameter)
    });
    candidate.and_then(|name| sanitize_filename(&name).ok())
}

fn header_parameter(value: &str, name: &str) -> Option<String> {
    for part in value.split(';').skip(1) {
        let (key, raw) = part.trim_start().split_once('=')?;
        if key.trim().eq_ignore_ascii_case(name) {
            return Some(raw.trim().to_string());
        }
    }
    None
}

fn unquote_parameter(raw: &str) -> String {
    let trimmed = raw.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(trimmed);
    unquoted.replace("\\\"", "\"")
}

fn decode_extended_parameter(raw: &str) -> Option<String> {
    let value = unquote_parameter(raw);
    let (charset, encoded) = value.split_once("''")?;
    if !charset.eq_ignore_ascii_case("utf-8") && !charset.eq_ignore_ascii_case("us-ascii") {
        return None;
    }
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Check whether a redirect target is acceptable.
///
/// Rules (spec §10.2): scheme must stay allowlisted, and we never silently
/// downgrade HTTPS to HTTP.
pub fn validate_redirect(from: &url::Url, to: &url::Url) -> CommandResult<()> {
    let scheme = to.scheme().to_ascii_lowercase();
    if !ALLOWED_SCHEMES.contains(&scheme.as_str()) {
        return CommandResult::failure(
            codes::UNSUPPORTED_SCHEME,
            format!("Redirect to scheme '{scheme}' is not allowed"),
            false,
        );
    }
    if from.scheme() == "https" && scheme == "http" {
        return CommandResult::failure(
            codes::UNSUPPORTED_SCHEME,
            "Refusing to downgrade HTTPS to HTTP on redirect",
            false,
        );
    }
    CommandResult::success(())
}

/// Sanitize an untrusted filename into a safe single path component.
///
/// Never returns an error for a merely ugly name — it cleans it. Only returns
/// `Err` when nothing usable remains.
fn sanitize_filename(raw: &str) -> Result<String, String> {
    let trimmed_raw = raw.trim();
    // Reduce to a single path component first: a suggested filename coming from
    // a browser or a URL must never address another directory.
    let component = trimmed_raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(trimmed_raw);

    let mut cleaned = String::with_capacity(component.len());
    for c in component.chars() {
        if is_invalid_filename_char(c) {
            cleaned.push('_');
        } else {
            cleaned.push(c);
        }
    }

    // Windows ignores trailing dots and spaces.
    let trimmed = cleaned.trim_end_matches(['.', ' ']).to_string();
    let mut name = trimmed;

    if name.is_empty() {
        name = "download".to_string();
    }

    // Reserved device name check on the stem.
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED_NAMES.contains(&stem.as_str()) || RESERVED_NAMES_EXTRA.contains(&stem.as_str()) {
        name = format!("_{name}");
    }

    if name.len() > MAX_FILENAME_LENGTH {
        let mut truncated: String = name.chars().take(MAX_FILENAME_LENGTH).collect();
        truncated = truncated.trim_end_matches(['.', ' ']).to_string();
        if truncated.is_empty() {
            truncated = "download".to_string();
        }
        name = truncated;
    }

    Ok(name)
}

/// Resolve a destination directory + filename into a canonical, contained path.
///
/// The returned path is guaranteed to live inside `root` after normalization.
pub fn resolve_destination(root: &Path, filename: &str) -> CommandResult<PathBuf> {
    let root = match normalize_root(root) {
        Ok(r) => r,
        Err(msg) => return CommandResult::failure(codes::INVALID_DESTINATION, msg, false),
    };

    // Reject absolute filenames outright — they must not escape the root.
    if filename.starts_with('/') || filename.starts_with('\\') || is_windows_absolute(filename) {
        return CommandResult::failure(
            codes::PATH_TRAVERSAL,
            "Filename must be a relative name, not an absolute path",
            false,
        );
    }
    if filename.split(['/', '\\']).any(|part| part == "..") {
        return CommandResult::failure(
            codes::PATH_TRAVERSAL,
            "Filename must not contain '..' components",
            false,
        );
    }

    let safe_name = match sanitize_filename(filename) {
        Ok(n) => n,
        Err(msg) => return CommandResult::failure(codes::INVALID_DESTINATION, msg, false),
    };
    let candidate = root.join(&safe_name);

    let normalized = match normalize_root(&candidate) {
        Ok(n) => n,
        Err(msg) => return CommandResult::failure(codes::INVALID_DESTINATION, msg, false),
    };
    if !normalized.starts_with(&root) {
        return CommandResult::failure(
            codes::PATH_TRAVERSAL,
            "Resolved path escapes the destination directory",
            false,
        );
    }
    if normalized.as_os_str().len() > MAX_PATH_LENGTH {
        return CommandResult::failure(
            codes::INVALID_DESTINATION,
            format!("Resolved path exceeds {MAX_PATH_LENGTH} characters"),
            false,
        );
    }

    CommandResult::success(normalized)
}

fn is_windows_absolute(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return true;
    }
    s.starts_with("\\\\")
}

/// Normalize a path without requiring it to exist yet.
///
/// Uses `canonicalize` when possible (existing dirs) and falls back to a
/// component-wise normalization that collapses `.` and `..`.
fn normalize_root(p: &Path) -> Result<PathBuf, String> {
    if let Ok(canon) = p.canonicalize() {
        return Ok(canon);
    }
    // The path may not exist yet; normalize lexically.
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                out.push(comp.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                // Only pop if there is something to pop and it isn't the root.
                if !out.as_os_str().is_empty() && out.file_name().is_some() {
                    out.pop();
                }
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err("Destination directory could not be resolved".to_string());
    }
    Ok(out)
}

/// Pick a final path according to the overwrite policy.
///
/// Only `Replace` is allowed to name an existing file. `Ask` rejects a
/// collision (there is no interactive prompt in this candidate), `Skip` returns
/// the existing path so the engine can leave it untouched, and `Rename` finds a
/// free sibling. The engine publishes all non-Replace paths with a no-clobber
/// hard link, so a race still fails safely instead of overwriting.
pub fn choose_final_path(
    desired: &Path,
    policy: crate::domain::OverwritePolicy,
) -> std::io::Result<std::path::PathBuf> {
    use crate::domain::OverwritePolicy;

    if !desired.exists() {
        return Ok(desired.to_path_buf());
    }
    match policy {
        OverwritePolicy::Skip => Ok(desired.to_path_buf()),
        OverwritePolicy::Ask => Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "destination already exists",
        )),
        OverwritePolicy::Replace => Ok(desired.to_path_buf()),
        OverwritePolicy::Rename => {
            let parent = desired.parent().unwrap_or_else(|| Path::new("."));
            let stem = desired
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("download");
            let ext = desired.extension().and_then(|s| s.to_str());
            for i in 1..10_000 {
                let candidate_name = match ext {
                    Some(e) => format!("{stem} ({i}).{e}"),
                    None => format!("{stem} ({i})"),
                };
                let candidate = parent.join(candidate_name);
                if !candidate.exists() {
                    return Ok(candidate);
                }
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "no free filename available",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{plan_segments, MIN_SEGMENT_SIZE};

    #[test]
    fn rejects_non_http_schemes() {
        assert!(validate_url("file:///C:/Windows/system32").is_err());
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("data:text/plain,hi").is_err());
        assert!(validate_url("ftp://example.com/a.zip").is_err());
        // Direct IPC must reject embedded credentials too, not just the UI.
        assert!(validate_url("https://user:secret@example.com/a.zip").is_err());
    }

    #[test]
    fn accepts_http_and_https() {
        assert!(validate_url("https://example.com/a.zip").is_ok());
        assert!(validate_url("http://example.com/a.zip").is_ok());
    }

    #[test]
    fn rejects_control_characters() {
        assert!(validate_url("https://example.com/a\u{7f}b").is_err());
    }

    #[test]
    fn content_disposition_names_are_sanitized_suggestions() {
        assert_eq!(
            safe_filename_from_content_disposition("attachment; filename=\"report.pdf\""),
            Some("report.pdf".to_string())
        );
        assert_eq!(
            safe_filename_from_content_disposition("attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.txt"),
            Some("résumé.txt".to_string())
        );
        assert_eq!(
            safe_filename_from_content_disposition("attachment; filename=\"../../etc/passwd\""),
            Some("passwd".to_string())
        );
        assert_eq!(
            safe_filename_from_content_disposition("attachment; filename=\"CON.txt\""),
            Some("_CON.txt".to_string())
        );
        assert_eq!(safe_filename_from_content_disposition("inline"), None);
    }

    #[test]
    fn headers_reject_framing_control_and_line_breaks() {
        let bad = vec![HeaderPair {
            name: "Content-Length".into(),
            value: "0".into(),
        }];
        assert!(validate_headers(&bad).is_err());

        let newline = vec![HeaderPair {
            name: "X-Note".into(),
            value: "a\r\nInjected: 1".into(),
        }];
        assert!(validate_headers(&newline).is_err());

        let ok = vec![
            HeaderPair {
                name: "Authorization".into(),
                value: "Bearer xyz".into(),
            },
            HeaderPair {
                name: "X-Custom".into(),
                value: "value".into(),
            },
        ];
        assert!(validate_headers(&ok).is_ok());
    }

    #[test]
    fn sanitize_strips_invalid_characters() {
        assert_eq!(sanitize_filename("a<b>c:d").unwrap(), "a_b_c_d");
    }

    #[test]
    fn sanitize_blocks_reserved_names() {
        let name = sanitize_filename("CON.txt").unwrap();
        assert!(!name.eq_ignore_ascii_case("CON.txt"));
        assert!(name.starts_with('_'));
    }

    #[test]
    fn sanitize_removes_trailing_dots() {
        assert_eq!(sanitize_filename("file.").unwrap(), "file");
    }

    #[test]
    fn sanitize_takes_last_component() {
        assert_eq!(sanitize_filename("../../etc/passwd").unwrap(), "passwd");
    }

    #[test]
    fn destination_rejects_absolute_filename() {
        let root = Path::new("C:/Users/Test/Downloads");
        assert!(resolve_destination(root, "C:/Windows/evil.exe").is_err());
    }

    #[test]
    fn destination_rejects_parent_traversal() {
        let root = Path::new("C:/Users/Test/Downloads");
        assert!(resolve_destination(root, "../evil.exe").is_err());
    }

    #[test]
    fn destination_accepts_plain_name() {
        let root = Path::new("C:/Users/Test/Downloads");
        let p = resolve_destination(root, "file.zip").unwrap();
        assert_eq!(p, PathBuf::from("C:/Users/Test/Downloads/file.zip"));
    }

    #[test]
    fn redirect_allows_same_or_upgrade() {
        let from = url::Url::parse("https://example.com/a").unwrap();
        let to = url::Url::parse("https://cdn.example.com/b").unwrap();
        assert!(validate_redirect(&from, &to).is_ok());
    }

    #[test]
    fn redirect_rejects_downgrade() {
        let from = url::Url::parse("https://example.com/a").unwrap();
        let to = url::Url::parse("http://example.com/b").unwrap();
        assert!(validate_redirect(&from, &to).is_err());
    }

    #[test]
    fn plan_segments_splits_evenly() {
        let plan = plan_segments(1_000_000, 4);
        assert_eq!(plan.len(), 4);
        assert_eq!(plan[0].start, 0);
        assert_eq!(plan[3].end, 999_999);
        // Contiguous and non-overlapping
        for w in plan.windows(2) {
            assert_eq!(w[0].end + 1, w[1].start);
        }
    }

    #[test]
    fn plan_segments_respects_min_size() {
        let plan = plan_segments(100_000, 8);
        assert!(plan.len() < 8);
        for s in &plan {
            assert!(s.len() >= MIN_SEGMENT_SIZE || plan.len() == 1);
        }
    }
}
