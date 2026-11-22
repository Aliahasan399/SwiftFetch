# SWIFTFETCH_MEDIA_DETECTION_AUDIT.md

**Date:** 2026-11-22 · **Type:** strictly READ-ONLY source-code audit
**Method:** direct inspection of current source and test files only. `.bak-*` files, generated
artifacts, documentation, TODO comments and UI placeholders were **not** used as implementation
evidence. No build, test execution, source edit, configuration change, commit or release was performed.
**Verdict:** NO-GO unchanged; `ready_for_public_release: false` preserved.

## Verdict table

| # | Feature | Status |
|---|---|---|
| 1 | Automatic video detection on websites | **NOT IMPLEMENTED** |
| 2 | Automatic audio detection on websites | **NOT IMPLEMENTED** |
| 3 | Floating video download button | **NOT IMPLEMENTED** |
| 4 | Floating audio download button | **NOT IMPLEMENTED** |
| 5 | Media detection inside Chrome, Edge and Firefox | **PARTIALLY IMPLEMENTED** |
| 6 | Video quality and format selection | **NOT IMPLEMENTED** |
| 7 | HLS (.m3u8) detection and downloading | **NOT IMPLEMENTED** |
| 8 | DASH (.mpd) detection and downloading | **NOT IMPLEMENTED** |
| 9 | Communication of detected media URLs to the desktop application | **PARTIALLY IMPLEMENTED** |
| 10 | Media downloading through the existing download engine | **PARTIALLY IMPLEMENTED** |

"Test evidence" below means existing test **source** that covers the implementation; this audit did
not execute the tests.

---

## 1–2. Automatic video / audio detection on websites — NOT IMPLEMENTED

**Evidence (source):**
- `browser-extension/chrome/content.js` — the content script registers only two input listeners,
  `handleGesture` on `click` and `auxclick` (lines 101-102). `handleGesture` (lines 53-99) requires a
  trusted primary click with `Alt` or `Ctrl+Shift`, walks up from `event.target` with `anchorFromEvent`
  to find an `<a>` element (lines 25-34), and sends a `swiftfetch_intent` only then. The file header
  explicitly states the script never enumerates links, reads page content or observes network traffic
  (lines 4-10). There is no `querySelector('video')`, `querySelector('audio')`, `MutationObserver`,
  media event listener (`playing`, `canplay`), or periodic media scan in the file.
- The same `content.js` is used by Firefox (`browser-extension/firefox/manifest.json` →
  `content_scripts[0].js: ["../chrome/content.js"]`) and by Chrome/Edge
  (`browser-extension/chrome/manifest.json` → `content_scripts[0].js: ["content.js"]`).
- A workspace-wide source search found no media-detection API usage in `src/` or `src-tauri/src/`.

**Test evidence:** `browser-extension/test/test_release.js:49` — `synthetic page clicks cannot send a
browser download intent` executes the real `content.js` and asserts that an untrusted click sends zero
messages and defines no `__swiftfetchSend` hook. No test exists for automatic media discovery.

## 3–4. Floating video / audio download button — NOT IMPLEMENTED

**Evidence (source):**
- No DOM injection exists in `browser-extension/chrome/content.js` (no `createElement`, `appendChild`,
  CSS insertion, overlay or button code). A search for those APIs across `browser-extension/` found
  them only in `browser-extension/test/test_page.html` (test harness logging), not in extension code.
- `browser-extension/chrome/popup.html` and `popup.js` implement a toolbar popup, not an in-page
  floating control; the manifests declare `action.default_popup` / `browser_action.default_popup`.
- No floating-button component exists in `src/` (frontend) or `src-tauri/src/`.

**Test evidence:** none; no test asserts a floating control because none is implemented. The
content-script test above confirms the only page behaviour is gesture forwarding.

## 5. Media detection inside Chrome, Edge and Firefox — PARTIALLY IMPLEMENTED

**What exists:** explicit, user-initiated context-menu capture of media URLs — not automatic detection.

**Evidence (source):**
- `browser-extension/chrome/background.js` lines 151-154 — `MENU_ITEMS` defines
  `swiftfetch-link` for `['link']` and `swiftfetch-media` for `['image', 'video', 'audio']`.
  `installMenus()` (lines 156-173) creates them; the `chrome.contextMenus.onClicked` listener
  (lines 175-187) reads `info.linkUrl || info.srcUrl` and calls `handleDownloadIntent(..., 'context_menu')`.
  This MV3 package is the Chrome/Edge build (file header: "Chrome / Edge (Manifest V3)";
  `chrome/manifest.json` has `manifest_version: 3`).
- `browser-extension/firefox/background.js` lines 21-28 — equivalent `MENU_ITEMS` with
  `contexts: ['image', 'video', 'audio']`; Firefox manifest is MV2 with callback-style APIs.
- `handleDownloadIntent` in both backgrounds calls `buildIntent` from `browser-extension/shared/protocol.js`
  and forwards through `forwardToNativeHost` (Chrome) / native messaging (Firefox). Content type for
  context-menu gestures is hard-coded to `application/octet-stream` (`chrome/background.js:183`), so
  no media MIME-specific handling occurs.
- No browser-specific detector exists; all three use the same explicit-gesture design.

**Test evidence:**
- `browser-extension/test/test_firefox_background.test.mjs:180` — `bootstraps menus and listeners
  without any dynamic import` asserts menu ids `['swiftfetch-link', 'swiftfetch-media']` and that a
  `contextMenus.onClicked` listener is registered.
- Same file, line ~376 — `forwards a context-menu gesture with the tab origin` asserts one native-host
  call with the tab origin for a context-menu gesture.
- No dedicated Chrome/Edge background test file was found; the Chrome context-menu path is
  implemented but not directly unit-tested in this repository.

## 6. Video quality and format selection — NOT IMPLEMENTED

**Evidence (source):**
- `src/components/AddDownloadDialog.tsx` state fields (lines 21-47) are URL, name, destination, queue,
  category, priority, connections, speed limit, start policy, checksum algorithm/value and overwrite
  consent. There is no quality, resolution, codec, container, format or variant selector.
- `src-tauri/src/domain.rs` `AddDownloadRequest` (used by `native_host.rs:131-143` and the dialog)
  contains no media-quality or format fields.
- `browser-extension/shared/protocol.js` `buildIntent` carries a single generic `content_type` field;
  the context-menu path always supplies `application/octet-stream` (`chrome/background.js:183`).
- No `video`, `audio`, `quality`, `format`, `resolution`, `codec`, `m3u8` or `mpd` media-selection code
  appears in `src/` or `src-tauri/src/` (searches returned only formatting/UI unrelated matches).

**Test evidence:** `src/testing/verify.mjs` checks that all Settings fields have form controls and
that DTO/command names match Rust, but no test covers media quality or format selection.

## 7–8. HLS (.m3u8) / DASH (.mpd) — NOT IMPLEMENTED

**Evidence (source):** case-insensitive source searches for `m3u8`, `.mpd`, `mpegurl`, `mpeg-dash`
and `hls` across `src/` (frontend), `src-tauri/` (Rust) and `browser-extension/` (all current JS)
returned **zero matches**. `src-tauri/src/security.rs:38` `validate_url` and
`src-tauri/src/native_host.rs:86` `http_url` accept only `http`/`https` URLs; there is no playlist
parser, manifest fetch, variant selection, segment muxer or media pipeline. The engine's
`src-tauri/src/engine.rs` transfers byte ranges of a single resource (`run_segment` line 1556,
`probe` line 1839), not HLS/DASH segment sequences.

**Test evidence:** none. No fixture, unit test or integration test references HLS or DASH.

## 9. Communication of detected media URLs to the desktop application — PARTIALLY IMPLEMENTED

**What exists:** a verified bridge for **explicitly supplied** URLs. No detector exists, so no
"detected" URL is ever produced or communicated.

**Evidence (source) — the working path:**
1. `browser-extension/chrome/content.js:85-94` — `chrome.runtime.sendMessage({type:'swiftfetch_intent', ...})`.
2. `browser-extension/chrome/background.js:95-145` — `handleDownloadIntent` validates via
   `buildIntent` and calls `forwardToNativeHost`; `forwardToNativeHost` (lines 69-83) uses
   `chrome.runtime.sendNativeMessage('com.swiftfetch.app', intent)`.
3. `browser-extension/native-host/host.py` — `validate_intent` (line 338), `forward` (line 546)
   HMAC-signs the intent and POSTs it to `app_url()` (`http://127.0.0.1:27642/bridge/intent`) after
   `assert_loopback` (line 528) checks the target.
4. `src-tauri/src/native_host.rs` — `Intent` struct (lines 20-34), `validate` (line 106),
   `handle` (line 147) and `run_if_requested` (line 168); `handle` calls
   `crate::commands::queue_browser_download` (line 155).
5. `src-tauri/src/commands.rs:328` — `queue_browser_download` checks browser-integration opt-in,
   forces `OverwritePolicy::Rename`, prepares the task and inserts it with replay/expiry protection.

**Test evidence:**
- `browser-extension/test/test_firefox_background.test.mjs` — `forwards a context-menu gesture with
  the tab origin`; `forwards exactly the ten protocol fields and never cookies, even from a hostile
  payload`; `gives every gesture a fresh request id`; `honours a per-site deny rule`.
- `src-tauri/src/native_host.rs` tests — `recognizes_windows_browser_arguments` (line 187),
  `rejects_expired_frames_and_fields_that_might_carry_secrets` (line 198),
  `queued_browser_intent_requires_opt_in_and_survives_history_clear` (line 226).
- `browser-extension/test/test_release.js:49` confirms the content script cannot emit an intent
  without a trusted gesture.

## 10. Media downloading through the existing download engine — PARTIALLY IMPLEMENTED

**What exists:** the generic HTTP(S) engine can download any directly addressable resource as bytes,
including a direct media file URL. There is no media-specific download path.

**Evidence (source):**
- `src-tauri/src/native_host.rs:131-143` and `src-tauri/src/commands.rs:341` convert a browser intent
  into the same `AddDownloadRequest` used by the normal Add Download dialog.
- `src-tauri/src/engine.rs` — `spawn_task` (line 518), `run_segment` (line 1556) and `probe` (line 1839)
  implement segmented/range-based HTTP transfers with resume, checksum and single-stream fallback.
  Content-Type is stored as metadata only (e.g. `engine.rs:1877,1891`), not used to select a media pipeline.
- `src-tauri/src/security.rs:69-75` restricts URLs to `http`/`https`, so a direct `.mp4`/`.mp3` URL is
  acceptable, but playlist/manifest parsing is absent.
- No function, struct or module in `src-tauri/src/` handles video/audio decoding, muxing, variant
  selection or media-specific post-processing.

**Test evidence:** `src-tauri/tests/transfer_tests.rs` contains generic end-to-end transfer tests
(`downloads_a_single_stream_file_end_to_end` line 239, `downloads_a_segmented_file_end_to_end` line
320, `verifies_a_checksum_and_fails_on_mismatch` line 422, plus fallback/resume/collision cases).
These prove byte transfer and integrity for ordinary files; **no test uses actual video or audio
content**, and no HLS/DASH test exists.

---

## Summary of limitations of this audit

- Static source inspection only; no tests were executed and no browser install/clean-machine run was
  performed. "Verified" above refers to existing test source, not to a fresh test result in this session.
- Absence of code is established for the searched source directories; a future dependency or generated
  artifact was not reverse-engineered.
- Documentation and UI text were deliberately excluded as evidence.

NO-GO and `ready_for_public_release: false` remain in force. This audit does not authorize any build,
release, publication, signing or updater action.
