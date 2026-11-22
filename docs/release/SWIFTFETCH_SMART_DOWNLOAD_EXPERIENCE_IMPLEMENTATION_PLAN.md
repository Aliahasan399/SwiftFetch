# SWIFTFETCH_SMART_DOWNLOAD_EXPERIENCE_IMPLEMENTATION_PLAN.md

**Date:** 2026-11-22 · **Type:** PLANNING ONLY — no source, config, workflow or dependency changes
**Status:** NO-GO preserved; `ready_for_public_release: false`. Owner approval required before any implementation.
**Design rule:** original SwiftFetch architecture, branding, UI and interaction patterns. No copying of
IDM's interface, visual design, source code or proprietary implementation. An IDM reference document
exists in `docs/reference/` but is **not** a design source to reproduce.

---

## 1. Executive summary

SwiftFetch already has a working, privacy-conservative browser-to-desktop handoff and a single
download engine. What is missing is the **interception-to-confirmation experience** and **media
detection**. The safest path is to extend the existing bridge, not to add a second engine or a second
transport.

Key conclusions from source inspection:

- Browser gesture forwarding is implemented and tested (Alt+Click / Ctrl+Shift+Click link capture,
  context menu, toolbar popup), but **no browser download interception** exists.
- A desktop "Add download" dialog already covers filename, destination, category, queue, start policy,
  checksum and overwrite consent — but it is opened from the app/clipboard, **not** from a browser
  download. This makes Smart Download Confirmation **PARTIALLY IMPLEMENTED**.
- The browser intent path is deliberately credential-free: protocol requires `cookies: []` and the
  Rust host rejects unknown fields; `AddDownloadRequest` supports headers but the browser path never
  supplies them. Header/cookie preservation therefore needs a new, explicit opt-in design.
- There is an unresolved transport split: a legacy Python host posts to a loopback HMAC endpoint that
  has no listener in the Rust app, while `register-host.ps1` registers the signed Rust `.exe` directly
  as a stdio native host. Implementation must standardise on the **Rust stdio host**.
- HTML5 media detection, floating controls, quality/format selection, HLS and DASH are absent from
  current source.

Recommended order: resolve the transport/protocol prerequisite, then Phase 1 (confirmation), then
direct media discovery, floating panel, quality selection, HLS/DASH, cross-browser testing, and
security/release verification.

---

## 2. Existing implementation findings (source evidence)

### 2.1 Browser capture and interception

| Capability | Evidence | Verdict |
|---|---|---|
| Link capture via trusted gesture | `browser-extension/chrome/content.js:21-102` — `isGestureEvent`, `anchorFromEvent`, `handleGesture`; only `click`/`auxclick` listeners | Implemented |
| Context menu on link/media | `chrome/background.js:151-187` `MENU_ITEMS` (`image`,`video`,`audio`); `firefox/background.js:21-28` | Implemented |
| Toolbar popup send | `chrome/popup.js:105-145` sends active page URL | Implemented |
| Download interception | `chrome/manifest.json` permissions are `contextMenus, storage, nativeMessaging, tabs`; no `downloads`, `webRequest` or `declarativeNetRequest`. No `downloads.onCreated` / `onDeterminingFilename` code exists | **Not implemented** |
| Automatic media detection | `content.js` has no media DOM query, `MutationObserver`, `playing`/`canplay` listener, or floating element injection | **Not implemented** |

Test evidence: `browser-extension/test/test_release.js:49` proves untrusted page clicks cannot create
intents; `test_firefox_background.test.mjs:180` and `:376` cover menu registration and context-menu
forwarding.

### 2.2 Native messaging and transport

- Browser → `chrome.runtime.sendNativeMessage('com.swiftfetch.app', intent)` in
  `chrome/background.js:69-83`.
- Rust host: `src-tauri/src/native_host.rs` — `Intent` (lines 20-34), `read_intent` (73), `validate`
  (106), `handle` (147), `run_if_requested` (168). It reads framed JSON from stdin and calls
  `commands::queue_browser_download` (line 155).
- `browser-extension/tools/register-host.ps1:11-50` registers the **selected `.exe`** as the native
  host for Chrome/Edge/Firefox (with signed-build enforcement unless `-AllowUnsignedPreview`).
- Legacy path: `browser-extension/native-host/host.py:524-567` `forward()` HMAC-signs and POSTs to
  `http://127.0.0.1:27642/bridge/intent`; no such listener exists in `src-tauri/src`
  (confirmed by source search and `docs/release/PRODUCTION_SECURITY_AUDIT.md`). This path must not be
  extended.

**Architecture prerequisite (Phase 0):** treat the Rust stdio host as canonical. Keep the Python host
only as a legacy document or remove from planning; never build confirmation on the unimplemented
loopback listener unless the owner explicitly authorises adding one.

### 2.3 Download intake, queue and engine

- `src-tauri/src/native_host.rs:131-143` builds `AddDownloadRequest` with `headers: Vec::new()` and
  `category_id: None`.
- `src-tauri/src/commands.rs:328-356` `queue_browser_download` checks `browser_integration` opt-in,
  forces `OverwritePolicy::Rename`, calls `prepare_download` and `store.insert_browser_task`.
- `src-tauri/src/store.rs:711-751` `insert_browser_task` provides expiry, settings-change detection,
  replay protection and a transaction — a good basis for pending confirmations.
- `StartPolicy` (`src-tauri/src/domain.rs:111-137`) already has `Immediate`, `Manual`, `Scheduled` —
  these map directly to "Start Download" and "Download Later".
- `src-tauri/src/engine.rs:518,1556,1839` (`spawn_task`, `run_segment`, `probe`) is the only transfer
  engine. All new features must call `add_download`/`queue_browser_download`, never create a second
  engine.

### 2.4 Confirmation UI and categories

- `src/components/AddDownloadDialog.tsx:21-47,107-133` already has URL, filename, destination,
  category, queue, priority, connections, speed limit, start policy, checksum and overwrite consent;
  `Dialog.tsx` provides focus-trapped native modal semantics; `ConfirmDialog.tsx` provides explicit
  confirmations. These are reusable patterns, not a new visual language.
- `src/components/TaskDetailsDialog.tsx:77` already supports overwrite-conflict resolution
  (`rename`/`overwrite`/`skip`) after the fact.
- Categories are dynamic (`src/components/AddDownloadDialog.tsx:120`, `commands.rs:522-527`), but
  `store.rs:338-357` seeds only a single `default` / `Downloads` category. The six requested labels
  are not seeded.
- Theme support exists: `App.css:1-54` tokens with dark/light and `App.tsx:145-149` system preference;
  `SettingsForm.tsx:80` exposes the theme selector. New UI must use these tokens.

### 2.5 Security and privacy posture

- Protocol requires ten fields; `cookies` must be empty and `user_initiated` true
  (`src-tauri/src/native_host.rs:106-129`; `browser-extension/shared/protocol.js`).
- `security::validate_url` (`src-tauri/src/security.rs:38-84`) allows only `http`/`https`;
  `validate_headers` (`security.rs:94-134`) blocks framing/hop-by-hop headers but explicitly allows
  user-supplied `Authorization`/`Cookie` for the normal `add_download` path.
- `src-tauri/src/secrets.rs` and `store.rs:757-780` provide opt-in DPAPI encryption of saved headers.
  Any future header capture must be explicit, opt-in and stored through this path.
- `register-host.ps1` requires a signed executable unless preview override — currently blocked because
  NO-GO and unsigned artifacts are preserved.

### 2.6 Smart Download Confirmation audit

**Verdict: PARTIALLY IMPLEMENTED.**

Present: in-app confirmation form (filename/destination/category/queue/start policy/checksum/collision),
browser gesture forwarding, replay/expiry protection, Rename-by-default for browser-intake, and a
confirmed task read-back in `AddDownloadDialog.tsx:95-97`.

Absent: browser download interception, browser-triggered confirmation, file-size/type display before
creation, an explicit "Download Later" action distinct from a select box, header/cookie/auth
preservation, persisted pending confirmations across app unavailability, and the six requested
category presets.

### 2.7 Smart Media Detection audit

**Verdict: NOT IMPLEMENTED** (no HTML5 element scan, no floating indicator, no quality/format panel,
no HLS/DASH tokens in `src/`, `src-tauri/` or `browser-extension/` source; see
`docs/release/SWIFTFETCH_MEDIA_DETECTION_AUDIT.md`).

---

## 3. Proposed architecture

### 3.1 Confirmation flow (Feature A)

```
browser download/gesture
  -> content script (trusted gesture only) or downloads API interception
  -> background validation (protocol v2, idempotency key)
  -> Rust native host (framed stdin; canonical)
  -> new command prepare_browser_download(request, metadata, id)
       -> validate URL/headers, probe availability (size/type), check destination collisions
       -> persist PendingConfirmation in SQLite with TTL
       -> emit event browser_confirmation_pending (and focus main window)
  -> frontend modal BrowserDownloadConfirmation
       -> Start now / Add to queue / Cancel / edit fields
  -> add_download (or queue_browser_download for v1 compatibility) -> existing engine
```

Key changes:
- **Protocol v2** adds optional fields (`size_hint`, `mime`, `suggested_name`, `page_origin`,
  `intent_metadata`, `media` descriptor, `header_consent_token`). v1 remains accepted.
- **PendingConfirmation store**: new table with `id`, `payload_json`, `expires_at`, `state`
  (`pending`/`accepted`/`cancelled`/`expired`), created via `prepare_browser_download`.
- **Idempotency**: reuse UUID v4 request id; extend `insert_browser_task` replay guard to pending state;
  a repeated intent with the same id returns the existing confirmation instead of a duplicate task.
- **Interruption safety**: if the app is closed, the extension stores a bounded, redacted draft in
  `chrome.storage.local` and retries on startup with the same id; the desktop app also lists pending
  confirmations on launch. No silent fallback to an unconfirmed download.
- **Credentials**: default off. A separate, explicit opt-in can capture permitted headers (excluding
  forbidden framing headers) into a consent token; store through existing DPAPI path. Never place
  cookies in protocol v2 directly.

### 3.2 Media detection flow (Feature B)

```
content script scans accessible media on document_idle (bounded, opt-in per site)
  -> detects <video>/<audio>, <source src>, direct media URLs, optional manifest hints
  -> injects small SwiftFetch indicator in closed shadow DOM near the player
  -> click opens original SwiftFetch media panel (in-page, not an IDM-style popup)
  -> user selects variant -> background validates -> same native host -> prepare_browser_download
  -> existing engine downloads the chosen direct URL
```

- No `webRequest`/network sniffing. No DRM/EME bypass. Skip protected/cross-origin/unreadable sources.
- Bounded observer: one initial scan plus a short, debounced `MutationObserver` limited to
  `video,audio,source`; disconnect after a cap; no polling intervals.
- Direct media URLs use the existing engine. HLS/DASH are planned as explicit, optional parsing modes
  gated behind owner approval for any new dependency.

### 3.3 Transport decision

Standardise on `register-host.ps1` + Rust stdio host. Do **not** add the missing loopback HMAC listener
unless separately authorised. Keep `host.py` out of the new design.

---

## 4. Original UI/UX concepts (all new, SwiftFetch-branded)

No IDM layout, icons, colours, terminology or screenshots are reused. Use existing `App.css` tokens
(`--surface`, `--raised`, `--border`, `--accent`, `--text`, etc.) so dark/light/system themes work.

### A. Download confirmation window
- Native in-app modal built on `Dialog.tsx`, not a browser popup imitation.
- Header: SwiftFetch mark + "Confirm download" + small origin badge (`example.com`), never full URL.
- Body in two columns on wide screens, stacked on narrow:
  - Left: editable **File name**, read-only **From** (host only), **Type** badge (Video/Music/Document/
    Program/Compressed/Other), **Size** ("Checking…" until core probe returns), destination folder with
    Browse, and a duplicate-name notice showing the proposed safe name.
  - Right: **Category** select with the six requested presets plus existing custom categories,
    **Queue** select, **Start** choice.
- Footer: `Cancel` (text button), `Add to queue` (secondary), `Start download` (primary accent).
- Accessibility: labelled inputs, `aria-describedby` hints, focus trap from `Dialog`, Escape = Cancel.

### B. Floating media indicator
- Small rounded SwiftFetch chip anchored to the media element's bottom-right, using closed shadow DOM
  and theme tokens. Not an IDM floating bar; no green arrow or imitation icon.
- `pointer-events: auto` only on the chip; transparent overlay never covers player controls; repositions
  on scroll/resize; hides when the element leaves the viewport or site rule is `deny`.
- Keyboard reachable via `Tab`, labelled "SwiftFetch media options".

### C. Media quality/format panel
- Opens from the indicator as a compact in-page panel (not a desktop popup) with sections:
  "Direct file" (URL, type, size if known), "Available variants" (list of quality/format rows from
  detectable `<source>`/manifest metadata), and a footer with `Add to queue` / `Start download`.
- Each row shows a neutral format label and resolution when genuinely detectable; no fabricated
  quality claims. If no variants exist, show only "Original file".
- Respects `prefers-reduced-motion`, forced-colors and dark/light tokens.

---

## 5. Phased implementation plan

### Phase 0 — prerequisite: transport and protocol baseline (before Phase 1)
- **Files:** `browser-extension/shared/protocol.js` + regenerated `protocol.bundle.js` (via
  `browser-extension/tools/make-protocol-bundle.mjs`), `src-tauri/src/native_host.rs`,
  `src-tauri/src/commands.rs`, `browser-extension/tools/register-host.ps1`,
  `docs/release/PRODUCTION_SECURITY_AUDIT.md` (document resolution only).
- **Changes:** protocol v2 with v1 compatibility; canonicalise Rust stdio host; no loopback listener;
  document Python host as legacy.
- **Tests:** extend protocol suite with v1/v2 cases; native host unknown-field and cookie rejection.
- **Acceptance:** v1 intents still work; v2 accepted; no new remote listener; no credentials transmitted.
- **Regressions:** existing browser handoff and Firefox bundle loading.

### Phase 1 — Smart Download Confirmation
- **Files to modify:** `browser-extension/chrome/manifest.json`, `firefox/manifest.json`,
  `browser-extension/chrome/background.js`, `firefox/background.js`, `browser-extension/chrome/content.js`,
  `shared/protocol.js`, `src-tauri/src/native_host.rs`, `commands.rs`, `domain.rs`, `store.rs`,
  `lib.rs` (event registration), `src/ipc.ts`, `src/types.ts`, `src/App.tsx`, `src/i18n.tsx`.
- **New files:** `src/components/BrowserDownloadConfirmation.tsx`,
  `src/components/BrowserDownloadConfirmation.css`, `src-tauri/src/browser_confirmation.rs` (or module
  under `commands.rs`), migration for `pending_confirmations` table, tests in
  `browser-extension/test/` and `src-tauri/tests/`.
- **Architecture:** add `prepare_browser_download`, `list_pending_confirmations`,
  `resolve_browser_confirmation`; emit `browser_confirmation_pending`; use `downloads` permission with
  capability detection (fall back to existing gesture capture where unavailable).
- **UI:** confirmation modal above; six category presets seeded via `upsert_category`.
- **Dependencies:** none required; any download-API helper must use browser built-ins.
- **Security:** validate all messages; preserve idempotency; explicit header consent only; no cookies
  in protocol by default; `validate_url`/`validate_headers` reused; replay/expiry enforced.
- **Tests:** protocol v2; background interception mocked; Rust pending-store tests; frontend component
  test via `src/testing/verify.mjs`; `tools/smoke-desktop.mjs` for UI render.
- **Acceptance:** browser download opens confirmation; filename/destination/type/size/category editable;
  Start/Queue/Cancel produce correct core state; duplicate names safely renamed; pending survives app
  restart until TTL; no unconfirmed download created.
- **Regressions:** existing Alt+Click, context menu, popup and clipboard flows; overwrite policy.

### Phase 2 — Direct video and audio detection
- **Files:** `browser-extension/chrome/content.js` (shared by Firefox), both backgrounds, protocol,
  `manifest.json` for both (still no `webRequest`), `src-tauri/src/native_host.rs` (media metadata
  pass-through), UI types only.
- **New files:** `browser-extension/shared/media-detect.js` + generated bundle for Firefox;
  `browser-extension/test/test_media_detect.js`.
- **Architecture:** bounded one-shot scan + short debounced observer; per-site rule gate; emit only
  accessible elements (`currentSrc || src || <source src>`), skip `blob:`/`media source` protected
  content unless a direct URL is exposed; no DRM circumvention.
- **UI:** no floating UI yet; detection results emitted only to background/test hooks.
- **Security:** no page text/network capture; size/cap; no cross-origin secret access.
- **Tests:** HTML fixtures with video/audio/source; assert no intent without gesture; assert observer
  disconnects; Firefox bundle test.
- **Acceptance:** detectable media listed in tests; zero downloads without user click; no performance
  regression in smoke page.
- **Regressions:** content-script privacy contract and site rules.

### Phase 3 — Floating media download interface
- **Files:** `browser-extension/shared/media-detect.js`, both manifests (minimal CSS permission if
  needed), new `browser-extension/shared/media-indicator.js`, generated bundles, tests.
- **New files:** indicator module with closed shadow DOM; `test_media_indicator.js`.
- **UI:** original chip from §4B; no IDM resemblance; theme-aware; keyboard accessible.
- **Security:** no overlay interception of site controls; safe `aria-label`; avoid clickjacking by
  requiring a real user click on the chip.
- **Tests:** DOM fixture for positioning, accessibility label, no control obstruction, `deny` rule hides.
- **Acceptance:** indicator appears only near supported media, disappears on scroll/deny, click opens
  selection panel (Phase 4) or direct confirmation (Phase 1).
- **Regressions:** page layout, scrolling, keyboard focus, other browser extensions.

### Phase 4 — Media quality and format selection
- **Files:** media-detect module, protocol (optional `media` descriptor), confirmation component,
  `src/types.ts`, `src/i18n.tsx`; new `src/components/MediaSelectionPanel.tsx` + CSS.
- **Architecture:** map `<source type/res>` and direct URL extension to labels; optional `media` object
  with `variants[]` of `{url, mime, width, height, label}`; no invented quality.
- **UI:** panel from §4C, integrated with Phase 1 confirmation (Start/Queue/Cancel).
- **Security:** validate each variant URL with the same rules; no arbitrary remote command execution.
- **Tests:** variant fixture parsing; invalid variant rejection; frontend panel tests.
- **Acceptance:** honest options list; selected variant produces exactly one confirmed task.
- **Regressions:** direct media download path; protocol size limits.

### Phase 5 — HLS and DASH support (conditional)
- **Files:** new `src-tauri/src/media_manifest.rs`, protocol `media.manifest` field, engine task mode
  extension, new tests; possibly `Cargo.toml` — **dependency changes require separate owner approval**.
- **Architecture:** parse `.m3u8`/`.mpd` only when user selects; resolve variants; download segments
  through the existing engine primitives, writing through the existing file writer; HLS TS and simple
  DASH single-representation first; state limitations explicitly; no decryption or DRM bypass.
- **Dependencies:** prefer no new crate; if a parser is needed, request approval before adding.
- **Security:** manifest URL validation, no unbounded playlist growth, no remote code, no credentials.
- **Tests:** local fixture manifests, variant selection, interrupted segment recovery, failure cleanup.
- **Acceptance:** selected supported manifest completes via existing engine; DRM/protected content is
  skipped with a clear message; no duplicate engine.
- **Regressions:** engine segment logic, disk-space and cancellation paths.

### Phase 6 — Cross-browser testing
- **Files:** `browser-extension/tools/package.mjs`, both manifests, `tools/firefox-acceptance.py`,
  new acceptance scripts; no production release artifacts.
- **Matrix:** Chrome, Edge (MV3 `chrome/` package), Firefox (MV2 `firefox/` package) on Windows 10/11
  x64; gesture, context menu, interception fallback, pending confirmation, media detection, indicator,
  panel, deny rule, signed-host registration (blocked while NO-GO).
- **Acceptance:** each browser records explicit PASS/FAIL with evidence; no assumed parity.
- **Regressions:** packaging, Firefox classic bundle loading, extension permissions.

### Phase 7 — Security, regression and release verification
- Run `./tools/verify.ps1`, `node --test browser-extension/test/*`, `tools/smoke-desktop.mjs`,
  security review of message schemas and stored headers, and clean-VM acceptance when available.
- Keep NO-GO and `ready_for_public_release: false` until signing, clean-VM evidence and owner release
  authorization are complete. No signing, updater activation, release or tag in this phase.

---

## 6. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Browser `downloads` API differs across MV3/MV2 | Capability detection; keep gesture path as fallback; Phase 6 matrix |
| Native host transport split (Python vs Rust) | Phase 0 canonical Rust stdio; no loopback listener without approval |
| Credential leakage | Default no cookies; explicit opt-in; DPAPI at rest; redacted logs |
| Duplicate/expired intents | UUID v4 + replay guard + TTL + persisted pending state |
| DRM/access-control violation | Only accessible elements; no EME/decryption; site deny rules |
| Performance from media observer | One-shot scan + bounded debounced observer + disconnect cap |
| Dependency approval for HLS/DASH | Phase 5 gated; prefer no new crates |
| UI resemblance to IDM | Original tokens, layout, labels and icons; no copied assets |
| Regressions to existing downloads | Keep one engine; reuse existing dialog and store patterns; full regression suite |

---

## 7. Acceptance criteria summary

- Every browser-originated download requires an original SwiftFetch confirmation before any task is
  created, except an explicit per-site `allow` rule the owner/user configured.
- Confirmation shows editable filename, destination, detected type, size when known, six categories,
  Start / Queue / Cancel, and safe duplicate handling.
- Media indicator and panel appear only for accessible media, never obscure site controls, and never
  bypass access controls.
- All intents are validated, idempotent, bounded and optionally header-consented; no cookies are sent
  by default; no remote command execution.
- No second download engine; all downloads use existing `engine.rs` paths.
- Full cross-browser and regression evidence is archived; NO-GO remains until owner authorises release.

## 8. Recommendation — safest implementation order

1. **Phase 0 first** (transport/protocol baseline) — otherwise confirmation would be built on an
   ambiguous or dead path.
2. **Phase 1** (Smart Download Confirmation) — highest user value, reuses most existing code.
3. **Phase 2** (direct media detection) — establishes trustworthy detection before any UI.
4. **Phase 3** (floating indicator) — adds UI only after detection is proven safe.
5. **Phase 4** (quality/format panel) — depends on reliable variant metadata.
6. **Phase 5** (HLS/DASH) — last and conditional on owner dependency approval; highest complexity and
   access-control risk.
7. **Phases 6–7** as gates, not afterthoughts: cross-browser evidence and security/regression
   verification before any release decision.

**Wait for explicit owner approval before implementing any phase.** This document makes no source,
configuration, workflow, dependency, installer, signing, submission, release or updater change.
