# SWIFTFETCH_SMART_DOWNLOAD_EXPERIENCE_TEST_REPORT.md

**Date:** 2026-11-22 · **Scope:** Phase 0–7 verification for the Smart Download Experience
**NO-GO preserved.** No publication, signing, release tag or updater activation.

## 1. Commands executed and observed results

| Command | Result | Evidence |
|---|---|---|
| `node browser-extension/tools/make-protocol-bundle.mjs` | exit 0; four bundles generated | `shared/*.bundle.js` |
| `node --test browser-extension/test/test_protocol.js` | 69 pass, 0 fail | protocol v1/v2, metadata credential rejection |
| `node --test browser-extension/test/test_release.js` | 4 pass, 0 fail | packaged manifests, downloads permission, synthetic-click rejection |
| `node --test browser-extension/test/test_media.js` | 3 pass, 0 fail | URL scheme filter, media/source detection, stream skip |
| `node --test browser-extension/test/test_firefox_background.test.mjs` | 15 pass, 0 fail | menus, download interception, media selection, replay/deny rules |
| `npm run build` | exit 0; TypeScript + Vite build | `dist/assets/index-CM-YCWGD.js`, `index-FvquKpkj.css` |
| `cargo check --locked --all-targets --manifest-path src-tauri/Cargo.toml` | exit 0 | Rust compile across all targets |
| `cargo test --locked --no-run --manifest-path src-tauri/Cargo.toml` | exit 0 | all test binaries built |
| `./tools/verify.ps1` | see §2 | canonical report |

## 2. Canonical verification

`./tools/verify.ps1` → exit 0; report
`artifacts/verification/report-20261122-132627.json`; all 8 checks passed:
HTTP fixture self-test (46/46), Rust compilation, Rust tests (65 unit + 6 + 7 + 8 + 5 + 12 + 10 + 18 + 3
integration = 134 tests, 0 failed), Desktop UI build, Desktop UI regressions (41/41), Browser protocol/
media/package tests (76/76), generated extension bundles current, Firefox background behavior tests
(15/15).

One earlier run failed at `store_tests::seeds_default_queue_and_category` because the test expected 1
category while Phase 1 intentionally seeds 7 (default + six presets). The assertion was updated to
verify all seven IDs; the rerun passed. No production code was weakened to hide a failure.

## 3. Phase results

| Phase | Classification | Evidence |
|---|---|---|
| 0 | PASS | protocol 69/69; v1 compatibility; Rust v2 validation tests |
| 1 | PASS | pending confirmation table, native-host tests, confirmation component, browser interception test |
| 2 | PASS | media detection 3/3; bounded observer source |
| 3 | PARTIAL | indicator/panel source implemented; no real-browser visual evidence |
| 4 | PASS | variant validation and confirmation integration; Firefox harness media-selection test |
| 5 | PARTIAL | manifest parser unit tests (3) and clear unsupported messages; segment assembly not implemented |
| 6 | PARTIAL | package/harness tests pass; real Chrome/Edge/Firefox runs not executed |
| 7 | PASS (automated) | canonical suite 8/8; real-browser and clean-VM items remain PARTIAL/BLOCKED |

## 4. Security checks performed

- v2 metadata allowlist and credential rejection verified by protocol tests.
- Native host rejects metadata with headers and rejects v1 messages carrying metadata.
- Browser download interception test confirms cancel happens only after native acceptance.
- Media URL filter test confirms `blob:`, `data:` and `javascript:` are rejected.
- Manifest parser tests confirm segment playlists/templates produce explicit unsupported errors.
- No dependency changes; no new network listener.

## 5. Limitations

- No real browser, clean VM, signed host registration or second-laptop execution.
- No production installer rebuild; local frontend build only.
- SmartScreen/signature behavior untested (unsigned candidate remains).
- HLS/DASH segment assembly intentionally unsupported.

## 6. Verdict

Implementation is substantially complete with two deliberate PARTIAL phases (floating UI visual
verification and HLS/DASH segment assembly) and one environment-blocked phase (real cross-browser
testing). Release status remains **NO-GO** with `ready_for_public_release: false`.
