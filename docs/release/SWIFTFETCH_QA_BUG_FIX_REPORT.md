# SWIFTFETCH_QA_BUG_FIX_REPORT.md

**Date:** 2026-11-23 · **Type:** implementation and verification report for independent real-world QA findings
**Starting state:** branch `main`, HEAD `c3d5c3196c7fe5a2ccb3cee7071276747c88cb3c` (commit 32), clean tree,
installed app `E:\SwiftFetch\swiftfetch.exe` v0.1.0 SHA `2693341a…3732913a`.
The named QA report file was **not present** in the workspace; the owner prompt findings were treated as authoritative.
**NO-GO preserved; `ready_for_public_release: false`.**

## 1. Executive summary

| Bug | Status | Evidence |
|---|---|---|
| BUG-07 media content-script `api` collision | **PASS** | real Chrome fixture: both bundles execute, 2 media elements detected, no syntax error |
| BUG-10 repeated browser intents disappear | **PARTIAL** | queue selection fix + regression test; repeated-click real-browser run BLOCKED (no extension loaded) |
| BUG-08 HTTP 200 protocol error marked Completed | **PASS (automated)** | new `PROTOCOL_ERROR` path + local fixture integration test; legitimate binary still completes |
| BUG-01 Native Messaging misconfiguration | **PARTIAL** | registration hardened + reversible unregister; no real extension ID/registration exercised |
| BUG-04 Diagnostics preview invisible | **PARTIAL** | always-visible dialog + redaction test; real UI click not exercised |
| BUG-03 stale URL validation | **PASS** | validation cleared when input becomes valid; source-level regression test |
| BUG-09 window disappears | **PARTIAL** | ErrorBoundary added; original failure not reproduced |
| BUG-02 browser integration productization | **PARTIAL** | clearer in-app guidance; guided status interface not implemented |

## 2. Defect-to-source mapping and fixes

### BUG-07 — `SyntaxError: Identifier 'api' has already been declared`
- Root cause: generated classic bundles for Firefox/Chrome content scripts each declared a top-level
  `const api` in the shared content-script world.
- Fix: `browser-extension/tools/make-protocol-bundle.mjs` now wraps every generated bundle in an IIFE;
  all four bundles regenerated.
- Test/evidence: real Chrome loaded `artifacts/browser-fixtures/media-indicator-test.html`;
  `SwiftFetchMediaDetect` and `SwiftFetchMediaIndicator` both `object`, `__fixture` reported 2
  descriptors, console had no `api` declaration error (only expected DNS errors for example.test video URLs).
- Status: **PASS**.

### BUG-10 — repeated browser download intents silently disappear
- Root cause addressed in UI orchestration: pending confirmations were polled but only one dialog could
  be selected, and a stale selected id could block advancing to the next request.
- Fix: `src/pending.ts` adds deterministic `selectPendingId(rows, currentId, dialogOpen)`; `src/App.tsx`
  uses it on every poll and shows a visible count banner when more than one confirmation is waiting.
- Regression test: `src/testing/verify.mjs` exercises 8 queued rows, dialog-open suppression, stale-id
  fallback and next-id selection.
- Real-browser repeated-click scenario: **BLOCKED** — the Chrome DevTools session has no unpacked
  SwiftFetch extension loaded (`list_extensions` returned none), so interception cannot be exercised.
- Status: **PARTIAL** (logic fixed and unit-tested; user-facing repeated-click not demonstrated here).

### BUG-08 — false Completed on HTTP 200 protocol-error payload
- Root cause: engine treated any HTTP 200 byte stream as a valid file; no response-body semantics check.
- Fix: `domain.rs` adds `codes::PROTOCOL_ERROR`; `engine.rs` adds narrow
  `protocol_error_payload()` for the observed `application/vnd.youtube-ump` + `sabr.malformed_config`
  schema and checks a bounded response prefix during probe. Legitimate binary content is unaffected.
- Tests: `tools/fixtures/server.py` adds `/status/200-error` and `/status/200-binary`;
  `src-tauri/tests/transfer_tests.rs` adds `protocol_error_payload_is_not_marked_completed` and
  `legitimate_binary_response_still_completes` (25/25 transfer tests pass).
- Status: **PASS** for the tested signatures. Other unspecified error schemas remain intentionally unsupported.

### BUG-01 — Native Messaging host misconfiguration
- Root cause: QA scratch manifest used a temporary directory, stale executable path and wrong extension ID.
- Fix: `browser-extension/tools/register-host.ps1` now refuses temp/scratch manifest directories and
  executables, requires a 32-character lowercase a–p Chrome/Edge extension ID, and keeps the stable
  `%LOCALAPPDATA%\SwiftFetch\NativeHost` default. Added `browser-extension/tools/unregister-host.ps1`
  that removes only the SwiftFetch-managed key/manifest and refuses unrelated registrations.
- Verification: earlier controlled registration with `-AllowUnsignedPreview` succeeded and was cleaned up;
  no real browser extension ID was available in this environment.
- Status: **PARTIAL**.

### BUG-04 — Diagnostics preview no visible result
- Fix: `src/diagnosticsBundle.ts` redacts sensitive keys; `src/components/DiagnosticsBundleDialog.tsx`
  is a visible modal with Copy and Save actions; `src/views/DiagnosticsView.tsx` opens it explicitly.
- Test: `src/testing/verify.mjs` verifies redaction (no session cookie or AppData path) and dialog wiring.
- Real UI click: **NOT TESTED**.
- Status: **PARTIAL**.

### BUG-03 — stale URL validation
- Fix: `src/components/AddDownloadDialog.tsx` clears `validation` when the URL becomes valid on input change.
- Test: source-level regression assertion in `verify.mjs` (44/44 frontend checks pass).
- Status: **PASS**.

### BUG-09 — main window disappears
- Fix: `src/components/ErrorBoundary.tsx` wraps the app, logs render exceptions and offers a visible
  reload path without terminating the process.
- Original trigger: **NOT REPRODUCED** in this environment.
- Status: **PARTIAL**.

### BUG-02 — browser integration productization
- Fix: `src/components/SettingsForm.tsx` now states what is supported, what must be registered, how to
  recover, and that failed handoffs leave the browser download intact.
- Full guided setup/status UI: **NOT IMPLEMENTED**.
- Status: **PARTIAL**.

## 3. Regression coverage

Preserved working areas were exercised by the full suite: launch/navigation, queue/history/scheduler/
categories/settings/diagnostics, small HTTP downloads, custom filenames, invalid URL validation,
checksums, pause/resume/cancel, category rules, restart reconciliation and history persistence.
`./tools/verify.ps1` → exit 0, report `artifacts/verification/report-20261123-142607.json`, 8/8 checks;
frontend suite with the added tests: 44/44; transfer engine: 25/25.

## 4. Changed files

`src/pending.ts` (new), `src/App.tsx`, `src/components/AddDownloadDialog.tsx`,
`src/diagnosticsBundle.ts` (new), `src/components/DiagnosticsBundleDialog.tsx` (new),
`src/views/DiagnosticsView.tsx`, `src/components/ErrorBoundary.tsx` (new),
`src/components/SettingsForm.tsx`, `src/testing/verify.mjs`,
`src-tauri/src/domain.rs`, `src-tauri/src/engine.rs`,
`src-tauri/tests/transfer_tests.rs`, `tools/fixtures/server.py`,
`browser-extension/tools/register-host.ps1`, `browser-extension/tools/unregister-host.ps1` (new).

No new dependencies; no migration changes; no release-gate changes.

## 5. Remaining limitations / owner actions

- Load the development extension in real Chrome and repeat the 8-click interception scenario.
- Register the native host with the actual installed extension ID and verify reconnect after browser restart.
- Exercise the diagnostics modal and window lifecycle in the owner's QA environment.
- Firefox testing NOT AVAILABLE (not installed).
- No production recommendation: release posture remains NO-GO.
