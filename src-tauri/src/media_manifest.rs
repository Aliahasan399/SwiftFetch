//! Bounded HLS/DASH manifest inspection. No dependency, no decryption, no
//! second download engine. Only statically addressable variants are exposed.

use crate::domain::{codes, CommandResult};
use crate::security;

pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
pub const MAX_VARIANTS: usize = 32;
pub const MAX_SEGMENTS: usize = 5000;
pub const MAX_SEGMENT_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SegmentPlan {
    pub kind: String,
    pub segments: Vec<String>,
}

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
    // Strip the opening quote before locating the closing delimiter; otherwise
    // the quote itself is found at index 0 and the value becomes empty.
    let trimmed = rest.trim_start_matches('"');
    let end = trimmed
        .find(|ch: char| ch == '"' || ch == ' ' || ch == '>' || ch == '/')
        .unwrap_or(trimmed.len());
    let value = trimmed[..end].to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// Expand a selected media playlist/representation into bounded segment URLs.
/// Encryption, live streams and unsupported template variables are rejected.
pub fn expand(url: &str, text: &str) -> CommandResult<SegmentPlan> {
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
    let plan = if lower.contains(".m3u8") {
        expand_hls(&url, text)
    } else if lower.contains(".mpd") {
        expand_dash(&url, text)
    } else {
        return CommandResult::failure(codes::UNSUPPORTED_SCHEME, "Only HLS and DASH manifests are supported here.", false);
    };
    match plan {
        Ok(plan) => CommandResult::success(plan),
        Err((code, message)) => CommandResult::failure(code, message, false),
    }
}

fn push_segment(
    segments: &mut Vec<String>,
    url: &str,
    reference: &str,
) -> Result<(), (&'static str, String)> {
    if segments.len() >= MAX_SEGMENTS {
        return Err((codes::INVALID_ACTION, "The media has too many segments for this build.".to_string()));
    }
    let resolved = resolve(url, reference)
        .ok_or_else(|| (codes::INVALID_URL, "A resolved segment URL is invalid.".to_string()))?;
    segments.push(resolved);
    Ok(())
}

fn expand_hls(url: &str, text: &str) -> Result<SegmentPlan, (&'static str, String)> {
    if text.contains("#EXT-X-KEY") {
        return Err((codes::UNSUPPORTED_SCHEME, "Encrypted HLS streams are not supported; SwiftFetch does not bypass encryption or DRM.".to_string()));
    }
    if text.contains("#EXT-X-STREAM-INF") {
        return Err((codes::INVALID_ACTION, "This is a master playlist. Select a specific variant before downloading.".to_string()));
    }
    if text.contains("#EXT-X-BYTERANGE") || text.contains("#EXT-X-DISCONTINUITY") {
        return Err((codes::UNSUPPORTED_SCHEME, "This HLS playlist uses unsupported byte-range or discontinuity features.".to_string()));
    }
    if !text.contains("#EXT-X-ENDLIST") {
        return Err((codes::UNSUPPORTED_SCHEME, "Live HLS streams are not supported; the playlist has no end marker.".to_string()));
    }
    let mut segments = Vec::new();
    let mut expect_uri = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#EXTINF") {
            expect_uri = true;
            continue;
        }
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if expect_uri {
            push_segment(&mut segments, url, trimmed)?;
            expect_uri = false;
        }
    }
    if segments.is_empty() {
        return Err((codes::INVALID_URL, "No HLS segments were found in the selected playlist.".to_string()));
    }
    Ok(SegmentPlan { kind: "hls".to_string(), segments })
}

fn template_value(block: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = block.find(&needle)? + needle.len();
    let rest = &block[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn expand_template(pattern: &str, number: u64, representation: &str, bandwidth: &str) -> String {
    pattern
        .replace("$Number$", &number.to_string())
        .replace("$RepresentationID$", representation)
        .replace("$Bandwidth$", bandwidth)
}

fn expand_dash(url: &str, text: &str) -> Result<SegmentPlan, (&'static str, String)> {
    if text.contains("$Time$") {
        return Err((codes::UNSUPPORTED_SCHEME, "DASH time-based segment templates are not supported.".to_string()));
    }
    let mut segments = Vec::new();

    if let Some(list_index) = text.find("<SegmentList") {
        let end = text[list_index..].find("</SegmentList>").map(|v| list_index + v).unwrap_or(text.len());
        let block = &text[list_index..end];
        for part in block.split("<SegmentURL").skip(1) {
            let media = template_value(part, "media")
                .ok_or_else(|| (codes::UNSUPPORTED_SCHEME, "A DASH SegmentURL has no media attribute.".to_string()))?;
            push_segment(&mut segments, url, &media)?;
        }
        if !segments.is_empty() {
            return Ok(SegmentPlan { kind: "dash".to_string(), segments });
        }
    }

    let template_index = text.find("<SegmentTemplate")
        .ok_or_else(|| (codes::UNSUPPORTED_SCHEME, "Only static DASH BaseURL or supported segment templates are handled.".to_string()))?;
    let template_end = text[template_index..].find('>').map(|v| template_index + v).unwrap_or(text.len());
    let template = &text[template_index..=template_end];
    let initialization = template_value(template, "initialization")
        .ok_or_else(|| (codes::UNSUPPORTED_SCHEME, "The DASH template has no initialization segment.".to_string()))?;
    let media_pattern = template_value(template, "media")
        .ok_or_else(|| (codes::UNSUPPORTED_SCHEME, "The DASH template has no media segment pattern.".to_string()))?;
    let representation = template_value(template, "id").or_else(|| attribute_fallback(text, "id")).unwrap_or_default();
    let bandwidth = attribute_fallback(text, "bandwidth").unwrap_or_default();
    let start_number = template_value(template, "startNumber")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);

    let timeline_index = text.find("<SegmentTimeline")
        .ok_or_else(|| (codes::UNSUPPORTED_SCHEME, "DASH templates without a SegmentTimeline are not supported.".to_string()))?;
    let timeline_end = text[timeline_index..].find("</SegmentTimeline>").map(|v| timeline_index + v).unwrap_or(text.len());
    let timeline = &text[timeline_index..timeline_end];
    let mut count: u64 = 0;
    // Split on "<S " so the enclosing "<SegmentTimeline>" tag is not mistaken
    // for an <S> timeline entry.
    for part in timeline.split("<S ").skip(1) {
        let repeat = template_value(part, "r").and_then(|value| value.parse::<u64>().ok()).unwrap_or(0);
        count = count.saturating_add(repeat.saturating_add(1));
        if count as usize > MAX_SEGMENTS {
            return Err((codes::INVALID_ACTION, "The DASH timeline has too many segments.".to_string()));
        }
    }
    if count == 0 {
        return Err((codes::INVALID_URL, "The DASH timeline contains no segments.".to_string()));
    }

    let init = expand_template(&initialization, start_number, &representation, &bandwidth);
    push_segment(&mut segments, url, &init)?;
    for offset in 0..count {
        let media = expand_template(&media_pattern, start_number + offset, &representation, &bandwidth);
        push_segment(&mut segments, url, &media)?;
    }
    Ok(SegmentPlan { kind: "dash".to_string(), segments })
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

    #[test]
    fn expands_hls_media_playlist_segments() {
        let text = "#EXTM3U\n#EXT-X-ENDLIST\n#EXTINF:10,\nseg1.ts\n#EXTINF:10,\n/seg2.ts\n";
        let result = expand("https://example.test/v/media.m3u8", text);
        assert!(result.ok);
        let plan = result.data.unwrap();
        assert_eq!(plan.kind, "hls");
        assert_eq!(plan.segments, vec![
            "https://example.test/v/seg1.ts".to_string(),
            "https://example.test/seg2.ts".to_string(),
        ]);
    }

    #[test]
    fn rejects_encrypted_and_live_hls() {
        let encrypted = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128\n#EXT-X-ENDLIST\n#EXTINF:10,\nseg.ts\n";
        assert!(!expand("https://example.test/a.m3u8", encrypted).ok);
        let live = "#EXTM3U\n#EXTINF:10,\nseg.ts\n";
        let result = expand("https://example.test/a.m3u8", live);
        assert!(!result.ok);
        assert!(result.error.unwrap().message.contains("Live"));
    }

    #[test]
    fn expands_dash_template_with_initialization_and_media_segments() {
        let text = "<MPD><Representation id=\"720p\" bandwidth=\"2000000\">\
            <SegmentTemplate initialization=\"init-$RepresentationID$.m4s\" media=\"seg-$Number$-$RepresentationID$.m4s\" startNumber=\"1\">\
            <SegmentTimeline><S d=\"4\" r=\"1\"/></SegmentTimeline></SegmentTemplate>\
            </Representation></MPD>";
        let result = expand("https://example.test/dash/manifest.mpd", text);
        assert!(result.ok);
        let plan = result.data.unwrap();
        assert_eq!(plan.kind, "dash");
        assert_eq!(plan.segments.len(), 3);
        assert!(plan.segments[0].ends_with("init-720p.m4s"));
        assert!(plan.segments[1].ends_with("seg-1-720p.m4s"));
        assert!(plan.segments[2].ends_with("seg-2-720p.m4s"));
    }

    #[test]
    fn rejects_invalid_segment_url_scheme() {
        let text = "#EXTM3U\n#EXT-X-ENDLIST\n#EXTINF:10,\ndata:video/mp4;base64,AAAA\n";
        assert!(!expand("https://example.test/a.m3u8", text).ok);
    }

    #[test]
    fn enforces_segment_count_limit() {
        let mut text = String::from("#EXTM3U\n#EXT-X-ENDLIST\n");
        for i in 0..(MAX_SEGMENTS + 1) {
            text.push_str("#EXTINF:1,\n");
            text.push_str(&format!("seg{i}.ts\n"));
        }
        let result = expand("https://example.test/a.m3u8", &text);
        assert!(!result.ok);
        assert!(result.error.unwrap().message.contains("too many segments"));
    }
}
