//! Bounded HLS/DASH manifest inspection. No dependency, no decryption, no
//! second download engine. Only statically addressable variants are exposed.

use crate::domain::{codes, CommandResult};
use crate::security;

pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
pub const MAX_VARIANTS: usize = 32;

#[derive(Debug, Clone, serde::Serialize)]
pub struct MediaVariant {
    pub url: String,
    pub label: String,
    pub mime: Option<String>,
}

fn resolve(base: &str, reference: &str) -> Option<String> {
    let base = url::Url::parse(base).ok()?;
    let resolved = base.join(reference).ok()?;
    if !matches!(resolved.scheme(), "http" | "https") || resolved.host_str().is_none() {
        return None;
    }
    if resolved.username() != "" || resolved.password().is_some() {
        return None;
    }
    Some(resolved.to_string())
}

fn attribute(line: &str, name: &str) -> Option<String> {
    for part in line.split(',') {
        let mut pair = part.splitn(2, '=');
        if pair.next().is_some_and(|key| key.trim() == name) {
            return pair.next().map(|value| value.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn label(prefix: &str, resolution: Option<String>, bandwidth: Option<String>) -> String {
    match (resolution, bandwidth) {
        (Some(res), Some(bw)) => format!("{prefix} · {res} · {bw} bps"),
        (Some(res), None) => format!("{prefix} · {res}"),
        (None, Some(bw)) => format!("{prefix} · {bw} bps"),
        (None, None) => prefix.to_string(),
    }
}

fn parse_hls(url: &str, text: &str) -> Result<Vec<MediaVariant>, (&'static str, String)> {
    let mut variants = Vec::new();
    let mut pending: Option<(Option<String>, Option<String>)> = None;
    let mut has_segment_playlist = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#EXT-X-STREAM-INF") {
            pending = Some((
                attribute(trimmed, "RESOLUTION"),
                attribute(trimmed, "BANDWIDTH"),
            ));
            continue;
        }
        if trimmed.starts_with("#EXTINF") {
            has_segment_playlist = true;
            continue;
        }
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if let Some((resolution, bandwidth)) = pending.take() {
            let resolved = resolve(url, trimmed)
                .ok_or_else(|| (codes::INVALID_URL, "A playlist variant URL is invalid.".to_string()))?;
            variants.push(MediaVariant {
                url: resolved,
                label: label("HLS", resolution, bandwidth),
                mime: Some("application/vnd.apple.mpegurl".to_string()),
            });
            if variants.len() > MAX_VARIANTS {
                return Err((codes::INVALID_ACTION, "The playlist has too many variants.".to_string()));
            }
        }
    }

    if variants.is_empty() && has_segment_playlist {
        return Err((
            codes::UNSUPPORTED_SCHEME,
            "This HLS URL is a segment playlist. SwiftFetch cannot safely merge its segments yet; use a direct media file instead.".to_string(),
        ));
    }
    if variants.is_empty() {
        return Err((codes::INVALID_URL, "No usable HLS variants were found.".to_string()));
    }
    Ok(variants)
}

fn parse_dash(url: &str, text: &str) -> Result<Vec<MediaVariant>, (&'static str, String)> {
    if !text.contains("<Representation") {
        return Err((codes::INVALID_URL, "No usable DASH representations were found.".to_string()));
    }
    if text.contains("SegmentTemplate") || text.contains("SegmentList") {
        return Err((
            codes::UNSUPPORTED_SCHEME,
            "This DASH manifest uses segment templates. SwiftFetch cannot safely assemble them yet.".to_string(),
        ));
    }
    let mut variants = Vec::new();
    let chunks: Vec<&str> = text.split("<Representation").skip(1).collect();
    for chunk in chunks {
        let end = chunk.find("</Representation>").unwrap_or(chunk.len());
        let block = &chunk[..end];
        let base = block
            .split("<BaseURL")
            .nth(1)
            .and_then(|part| part.split('>').nth(1))
            .and_then(|part| part.split('<').next())
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| (codes::UNSUPPORTED_SCHEME, "A DASH representation has no static BaseURL.".to_string()))?;
        let resolved = resolve(url, base)
            .ok_or_else(|| (codes::INVALID_URL, "A DASH BaseURL is invalid.".to_string()))?;
        let width = attribute_fallback(block, "width");
        let height = attribute_fallback(block, "height");
        let bandwidth = attribute_fallback(block, "bandwidth");
        let resolution = match (width, height) {
            (Some(w), Some(h)) => Some(format!("{w}x{h}")),
            _ => None,
        };
        variants.push(MediaVariant {
            url: resolved,
            label: label("DASH", resolution, bandwidth),
            mime: attribute_fallback(block, "mimeType"),
        });
        if variants.len() > MAX_VARIANTS {
            return Err((codes::INVALID_ACTION, "The manifest has too many representations.".to_string()));
        }
    }
    if variants.is_empty() {
        return Err((codes::INVALID_URL, "No usable DASH variants were found.".to_string()));
    }
    Ok(variants)
}

fn attribute_fallback(block: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let start = block.find(&needle)? + needle.len();
    let rest = &block[start..];
    let end = rest.find(|ch: char| ch == '"' || ch == ' ' || ch == '>' || ch == '/')?;
    let value = rest.trim_start_matches('"')[..end].to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// Parse a bounded manifest into direct, user-selectable variants.
pub fn parse(url: &str, text: &str) -> CommandResult<Vec<MediaVariant>> {
    if text.len() > MAX_MANIFEST_BYTES {
        return CommandResult::failure(codes::INVALID_ACTION, "The media manifest is too large.", false);
    }
    let validated = security::validate_url(url);
    if !validated.ok {
        let error = validated.error.unwrap();
        return CommandResult::failure(&error.code, error.message, error.retryable);
    }
    let url = validated.data.unwrap_or_default();
    let lower = url.to_ascii_lowercase();
    let parsed = if lower.contains(".m3u8") {
        parse_hls(&url, text)
    } else if lower.contains(".mpd") {
        parse_dash(&url, text)
    } else {
        return CommandResult::failure(codes::UNSUPPORTED_SCHEME, "Only HLS and DASH manifests are supported here.", false);
    };
    match parsed {
        Ok(variants) => CommandResult::success(variants),
        Err((code, message)) => CommandResult::failure(code, message, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hls_master_playlist() {
        let text = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\nlow.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2400000,RESOLUTION=1280x720\nhigh.m3u8\n";
        let result = parse("https://example.test/master.m3u8", text);
        assert!(result.ok);
        let variants = result.data.unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].url, "https://example.test/low.m3u8");
        assert!(variants[1].label.contains("1280x720"));
    }

    #[test]
    fn rejects_hls_segment_playlist_with_clear_message() {
        let text = "#EXTM3U\n#EXTINF:10,\nseg1.ts\n#EXTINF:10,\nseg2.ts\n";
        let result = parse("https://example.test/media.m3u8", text);
        assert!(!result.ok);
        assert!(result.error.unwrap().message.contains("segment playlist"));
    }

    #[test]
    fn rejects_dash_segment_templates() {
        let text = "<MPD><Representation><SegmentTemplate media=\"seg-$Number$.m4s\"/></Representation></MPD>";
        let result = parse("https://example.test/manifest.mpd", text);
        assert!(!result.ok);
        assert!(result.error.unwrap().message.contains("segment templates"));
    }
}
