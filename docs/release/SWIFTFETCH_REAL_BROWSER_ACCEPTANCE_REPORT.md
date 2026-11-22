# SWIFTFETCH_REAL_BROWSER_ACCEPTANCE_REPORT.md

**Date:** 2026-11-22 · **Environment:** Windows 11 Pro, build 10.0.26200 · Chrome 158.0.8127.36
**NO-GO preserved.** Automated harness results are not presented as real-browser results.

## 1. Method

A local HTML fixture (`artifacts/browser-fixtures/media-indicator-test.html`) loaded the real generated
classic bundles (`media-detect.bundle.js`, `media-indicator.bundle.js`) in the installed Chrome browser
through Chrome DevTools. This is a **real-browser DOM/CSS/accessibility test** of the indicator and
panel only. It does not include the unpacked extension, Native Messaging, or actual downloads.

## 2. Real-browser results — Chrome

| Test case | Result | Evidence |
|---|---|---|
| Detection of two accessible `<video>` elements | **PASS** | `descriptors=2`; URLs `https://example.test/video-one.mp4`, `video-two.mp4`; each with one variant |
| Indicator positioned near player | **PASS** | host style `left=34px`, `top=119.875px`, 30×30, visible for first player |
| Multiple players | **PASS** | two hosts created; second hidden while off-screen |
| Scroll behavior | **PASS** | after `scrollTo(0,600)`: first `display:none`, second `display:block`, `top=482.875px` |
| Resize behavior | **PASS** | viewport resized to 900×500; `update()` kept geometry within viewport |
| Light theme | **PASS** | `emulate colorScheme=light`; screenshot showed light page and light indicator chip |
| Dark theme | **PARTIAL** | CSS declares dark defaults; not visually captured in this run |
| Keyboard accessibility | **PASS** | accessibility tree exposes `button "SwiftFetch media options"` |
| Panel opens without download | **PASS** | panel snapshot: heading "Media found", variant button "Video 640×360", "Add to queue", "Start download"; `chosen=false` until explicit button |
| Media removal | **PASS (after fix)** | removing both `<video>` elements and calling `update()` produced `display:none` for both hosts |
| No interference with host controls | **PASS** | native video controls remained in accessibility tree; indicator was a separate 30×30 chip |

### Real-browser bugs found and fixed
1. Both generated classic bundles declared `const api` in the same content-script world, producing
   `SyntaxError: Identifier 'api' has already been declared`. Fixed by wrapping generated bundles in
   IIFEs (`tools/make-protocol-bundle.mjs`) and regenerating all four bundles. Browser tests pass after fix.
2. An indicator remained visible when its media element was removed. Fixed in
   `browser-extension/shared/media-indicator.js` by hiding the host when `element.isConnected` is false;
   real-browser retest confirmed both hosts hidden.

## 3. Real-browser results — Edge

| Item | Status |
|---|---|
| Edge 158.0.4370.33 installed at `C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe` | observed |
| Real-browser test session | **BLOCKED** — no Edge DevTools session used in this run |
| Extension/native-host tests | **BLOCKED** — pending unsigned development installer and host registration |

## 4. Real-browser results — Firefox

| Item | Status |
|---|---|
| Firefox installed | **NO** — executable not present |
| Real-browser tests | **BLOCKED** |
| Evidence | harness tests only (see test report) |

## 5. Extension and Native Messaging flows (all browsers)

| Test case | Status |
|---|---|
| Native Messaging connection | **BLOCKED** — requires registered development host |
| Alt+Click / Ctrl+Shift+Click | **PARTIAL** — harness/unit evidence only |
| Context-menu integration | **PARTIAL** — Firefox harness evidence only |
| Ordinary download interception | **PARTIAL** — Firefox harness evidence only |
| Confirmation modal interactions | **PARTIAL** — component source + Rust tests only |
| Pending confirmation after restart | **PARTIAL** — Rust persistence tests only |
| Media detection / panel / quality selection | **PASS in Chrome fixture**; other browsers BLOCKED |
| HLS/DASH fixtures | **PARTIAL** — engine integration tests; not real browser |
| Site deny rules | **PARTIAL** — harness tests only |
| Browser download fallback when native host fails | **PARTIAL** — harness failure-path tests only |

## 6. Exact remaining owner/browser actions
- Install the development build and register the unsigned native host (guide to follow).
- Load unpacked Chrome/Edge extension and Firefox development extension, then repeat the full matrix.
- Record browser version, Windows build, PASS/FAIL and evidence per case.

No result in this report should be read as production release approval.
