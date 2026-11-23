# SWIFTFETCH_BROWSER_INTEGRATION_REGRESSION_REPORT.md

**Date:** 2026-11-23 · **Environment:** Windows 11 Pro build 10.0.26200; Chrome 158.0.8127.36;
Edge 158.0.4370.33 installed; Firefox NOT AVAILABLE.
**NO-GO preserved.**

## 1. Real-browser session

| Item | Result |
|---|---|
| Chrome DevTools session | available |
| Loaded unpacked SwiftFetch extension | **NO** — `list_extensions` returned "No unpacked extensions found" |
| QA-reported extension ID `hllhleedpmooaclncoolcinlfnkjkcjn` | observed only in owner prompt, not present in this browser |
| Native host registration in this browser | BLOCKED — no extension ID / no registration |
| Real browser download interception | BLOCKED |
| Repeated-click (1/2/8 links) | BLOCKED |
| Confirmation dialog Start / Queue / Cancel in browser | BLOCKED |
| Browser restart persistence | BLOCKED |

No browser interception result is claimed as PASS.

## 2. Real-browser media verification (BUG-07)

Fixture: `artifacts/browser-fixtures/media-indicator-test.html` with the real generated bundles.
Result: **PASS** — `SwiftFetchMediaDetect` and `SwiftFetchMediaIndicator` globals loaded; two accessible
video elements detected; no `Identifier 'api' has already been declared` console error. Only expected
DNS errors for `example.test` media URLs were logged. This verifies the packaging fix, not a full media
download in a real browser.

## 3. Automated browser-bridge evidence

| Test | Result |
|---|---|
| Browser protocol/validator suite | 76/76 pass (`node --test browser-extension/test/*`) |
| Firefox background harness (real file, callback APIs) | 15/15 pass |
| Generated bundle drift check | all bundles current |
| Synthetic page-click rejection | pass |
| Firefox classic bundle initialization | pass |
| Repeated-pending selection logic | pass (frontend regression suite) |

Harness tests are explicitly **not** real-browser results.

## 4. Native Messaging configuration work

- `register-host.ps1` now rejects temp/scratch manifest directories and temp executables, validates
  Chrome/Edge extension IDs, and retains the stable per-user manifest location.
- `unregister-host.ps1` provides safe, reversible cleanup of only SwiftFetch-managed keys/manifests.
- A controlled `-AllowUnsignedPreview` registration was previously verified and removed; no
  registration was left behind.

## 5. Console errors observed

Only expected `ERR_NAME_NOT_RESOLVED` for the fixture's non-resolvable example.test video sources.
No JavaScript syntax or uncaught extension-script errors were observed in the media fixture.

## 6. Verdict by browser

| Browser | Status | Reason |
|---|---|---|
| Chrome | **PARTIAL** | media bundle fix verified in real browser; interception BLOCKED (no extension loaded) |
| Edge | **NOT TESTED** | installed but no automation session or extension registration |
| Firefox | **NOT AVAILABLE** | not installed |

The fresh development extension package is staged at `artifacts/dev-build/browser-extensions/`
(chromium/ and firefox/) for owner/Hermes retesting.
