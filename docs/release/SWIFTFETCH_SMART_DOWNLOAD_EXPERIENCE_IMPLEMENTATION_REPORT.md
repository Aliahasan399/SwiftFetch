# SWIFTFETCH_SMART_DOWNLOAD_EXPERIENCE_IMPLEMENTATION_REPORT.md

**Date:** 2026-11-22 · **Authorization:** owner master authorization for Phases 0–7
**Status:** NO-GO preserved; `ready_for_public_release: false`; no release, tag, signing, updater,
SignPath submission or publication performed.

## 1. Executive summary

Implemented an original SwiftFetch Smart Download Experience on top of the existing browser bridge
and single download engine:

- **Phase 0 — PASS:** protocol v2 with strict, bounded, non-sensitive metadata and v1 compatibility.
  Rust stdio native host remains canonical; legacy Python loopback host not extended.
- **Phase 1 — PASS (backend/UI implemented):** ordinary browser download interception, persisted
  pending confirmations, original confirmation modal, explicit Start / Add to queue / Cancel,
  six category presets, duplicate-safe Rename policy, TTL and replay protection.
- **Phase 2 — PASS:** bounded accessible video/audio detection with per-site deny handling.
- **Phase 3 — PARTIAL:** original closed-shadow-DOM floating indicator and in-page selection panel
  implemented with positioning, keyboard and theme support; real-browser visual verification not
  executed.
- **Phase 4 — PASS:** variant selection with genuine labels, variant URL revalidation and integration
  with the confirmation workflow.
- **Phase 5 — PARTIAL:** dependency-free HLS/DASH manifest inspection and direct-variant selection;
  segment playlist assembly and DASH segment templates are explicitly rejected with clear messages.
- **Phase 6 — PARTIAL:** automated cross-browser harness and package tests pass; real Chrome/Edge/Firefox
  manual testing not executed in this environment.
- **Phase 7 — PASS (automated):** canonical `./tools/verify.ps1` 8/8 green, including 76 browser
  tests and 134 Rust tests; real-browser/clean-VM items remain environment-blocked.

## 2. Changed-file inventory

Modified: `browser-extension/chrome/background.js`, `chrome/content.js`, both manifests,
`firefox/background.js`, `shared/protocol.js` + generated `protocol.bundle.js`,
`test/test_protocol.js`, `test/test_release.js`, `test/test_firefox_background.test.mjs`,
`tools/make-protocol-bundle.mjs`, `tools/package.mjs`, `tools/verify.ps1`,
`src-tauri/src/{commands,domain,engine,lib,native_host,store}.rs`, `src/{App,ipc,types}.ts(x)`.

New: `browser-extension/shared/media-detect.js` + bundle, `media-indicator.js` + bundle,
`browser-extension/test/test_media.js`, `src-tauri/src/media_manifest.rs`,
`src/components/BrowserDownloadConfirmation.tsx` + CSS, and both release reports.

Diff stat at inventory time: 22 modified files, 1,362 insertions, 37 deletions; 10 new files.

## 3. Database migration summary

New table `pending_confirmations` with `CREATE TABLE IF NOT EXISTS` in both the normal migration
(`store.rs:167`) and a short-lived native-host path (`store.rs:334`). Columns: id (request id), URL,
page origin, suggested name, content type, size hint, MIME, kind, site rule, media JSON, settings JSON
snapshot, state, created/expires timestamps. No existing table or column was altered or dropped;
migration is backward compatible.

Lifecycle: `pending → accepted | cancelled | expired`; replay guarded through `event_log`
(`browser_confirmation` events). Expired rows are hidden and marked on list.

## 4. Dependency and license summary

**No new dependencies.** `src-tauri/Cargo.toml`, `Cargo.lock`, `package.json` and `package-lock.json`
are unchanged. HLS/DASH inspection uses existing `reqwest`, `serde` and `url` crates with a 1 MiB
manifest cap, 32-variant cap and 15-second timeout. Browser code uses built-in Web APIs only.
Licensing remains GPL-3.0-or-later with documented CDLA residual uncertainty; no licensing decision changed.

## 5. Security review findings

- v2 metadata is an allowlist (size hint, MIME, kind, site rule, media variants); cookies/headers/
  credentials are rejected by both JS and Rust validators.
- Protocol v1 intents remain valid; v2 may not be smuggled through v1.
- All pending state is local SQLite; settings snapshot prevents stale/altered settings from being used
  at resolution time.
- Ordinary browser downloads are cancelled **only after** the native host accepts the intent, so a
  bridge failure does not lose the user's download.
- Media detection skips `blob:`, `data:`, `filesystem:` and MediaStream sources, caps elements and
  variants, and stops observing after a bounded number of mutations or 15 seconds.
- Manifest parser rejects segment playlists and DASH segment templates with explicit messages; no
  EME/DRM bypass or decryption exists.
- No remote command execution, no new listener, no credential logging; redaction retained.
- Legacy `browser-extension/native-host/host.py` (HMAC loopback POST) remains a historical, unused
  reference only. It was not modified or extended; the registered host is the Rust stdio executable.

## 6. Browser compatibility matrix (from source/harness evidence)

| Browser | Manifest | Interception | Confirmation | Media detection | Evidence |
|---|---|---|---|---|---|
| Chrome | MV3, `downloads` permission | implemented | implemented | implemented | source + package test |
| Edge | same MV3 package | implemented | implemented | implemented | source + package test |
| Firefox | MV2, `downloads` permission | implemented | implemented | implemented | VM harness tests (15 pass) |

Real-browser execution: **not performed**; see test report.

## 7. Remaining limitations and known bugs

- HLS segment playlists and DASH segment templates are detected but not assembled; users see a clear
  unsupported message. This is a deliberate safety boundary, not a silent failure.
- Floating indicator positioning and obstruction avoidance are implemented but not visually verified
  on real media sites.
- Real cross-browser manual testing and signed/native-host registration on a clean VM remain blocked
  by environment and NO-GO.
- Pending confirmations survive restarts until TTL, but the browser-side retry draft is not persisted
  across a browser restart if the app was closed at the moment of interception.

## 8. Local development build artifacts

Frontend production build succeeded:
- `dist/assets/index-CM-YCWGD.js` — 352,728 bytes, SHA-256 `38cca5ebc12103a2430501b43db81d3783d442bb0ab4cb14d33a3afc716fa6dd`
- `dist/assets/index-FvquKpkj.css` — 27,327 bytes, SHA-256 `8000f8a0cb488b7be4d2b3ce972f936ab778c3f96b3936d6eb915279ca70b773`

Development browser-extension package generated at
`artifacts/smart-download-dev-package/` with separate `chromium/` and `firefox/` trees. Key file hashes:
`chromium/content.js` `e963fce891fca056ac797d0c6d10aeb2e979cc73597821c4635573b9a2748aa4`,
`chromium/background.js` `112a40bccc97e395017a5cfa1e87ea1a9a77d571ce20da12eee48268239233c8`,
`firefox/background.js` `38179f8fb33941fbee9e3f81de6a9e66850baa86675f0a577dd60d7ef0f0f41b`,
`shared/protocol.bundle.js` `bc06fe816ce6b6b53fcd68d1fb705abbe0a5a161e18ac04229078a22bee2f5f2`.

No installer rebuild was performed, so canonical unsigned installer hashes remain historical evidence.

## 9. Second-laptop development build instructions

1. Use the existing second-laptop preparation report: `docs/release/SWIFTFETCH_SECOND_LAPTOP_TEST_PREPARATION.md`.
2. Copy only the current unsigned NSIS candidate from `src-tauri/target/release/bundle/nsis/` and verify
   its SHA-256 against `docs/release/INSTALLER_HASH_BASELINE.md` (historical baseline).
3. This implementation is source-level; the shipped unsigned candidate does **not** include these new
   features. Testing the new features requires a local development build, which was not produced in
   this phase. Do not present the old candidate as the Smart Download Experience build.
4. Enable browser integration in Settings, register the native host with the development executable
   using `browser-extension/tools/register-host.ps1 -AllowUnsignedPreview`, load the unpacked extension
   from `browser-extension/chrome/` or `firefox/`, and use the local fixture server for downloads.
5. Record PASS/FAIL per browser; keep NO-GO and `ready_for_public_release: false`.

NO-GO and release gates are unchanged. No release authorization is implied by this report.
