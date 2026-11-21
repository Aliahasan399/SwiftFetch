# SwiftFetch — Release Gap Register (Phase 0 baseline)

**Status:** Read-only baseline audit. No code, config, schema, or DB changes were made to produce this register.
**Created:** 2026-11-21. **Version:** 0.1.0. **Repo root:** `E:\CloneIdm\swiftfetch`.

## Source-of-truth & repository confirmation

- Architecture canonical copy: `docs/reference/IDM-DOWNLOAD-MANAGER-ARCHITECTURE.md`.
  A second copy exists at `E:\CloneIdm\IDM-DOWNLOAD-MANAGER-ARCHITECTURE.md`; both are
  **byte-identical** (54,501 bytes; SHA-256 prefix `782191FD1B981E1F`). No conflict.
- Other copies found are inside `artifacts/source/**` and `artifacts/SwiftFetch-0.1.0-candidate-*`
  snapshots — historical, not authoritative.
- Repository root `E:\CloneIdm\swiftfetch`; **not a git repository** (no `.git` directory), so
  there is no branch or working-tree status to report. Version `0.1.0` in both `package.json`
  and `src-tauri/Cargo.toml`.
- Build/test commands: `package.json` scripts `dev`, `build` (`tsc && vite build`), `preview`,
  `tauri`. Canonical verification is `tools/verify.ps1` (HTTP fixture self-test,
  `cargo check --locked --all-targets`, `cargo test --locked`, Vite build, `src/testing/verify.mjs`,
  and `node --test` for `test_protocol.js` + `test_release.js`). Release: `tools/build-windows.ps1`,
  `tools/stage-release.ps1`, `tools/package-candidate.ps1`, `tools/package-source.ps1`.

## Status legend

- **Implemented** — source exists.
- **Automated-tested** — a relevant automated test exists and a recorded pass exists in evidence.
- **Application-verified** — a reproducible real-app artifact (result.json / results.json / real file) exists.
- **Externally blocked** — needs owner-supplied certificate, endpoint, legal decision, account, or clean test machine.
- **Not implemented** — missing behavior.
- **Deferred/not applicable** — outside approved scope (architecture §2.2/§2.3 or master prompt §2).

## Requirement-by-requirement gap summary (architecture §2.1, lines 53–66)

| # | Requirement | Status | Evidence / gap |
|---|---|---|---|
| 1 | HTTP/HTTPS downloads | Implemented; Automated-tested; Application-verified | `security.rs:15,38-92`, `engine.rs:1839`; `transfer_tests.rs:239`; smoke `run-hY5H8r` SHA-256 output |
| 2 | Optional FTP after review | Deferred/not applicable | Gated by `security.rs:15`, `domain.rs:421`; master prompt §2 out of scope |
| 3 | Segmented/range downloads | Implemented; Automated-tested; Application-verified | `domain.rs:208-253`, `engine.rs:1280,1556`; `transfer_tests.rs:320`, `release_engine_tests.rs:38,55`; smoke multi-segment `ready.bin` |
| 4 | Single-stream fallback | Implemented; Automated-tested | `engine.rs:1290-1303,1464`; `transfer_tests.rs:277,363,545` |
| 5 | Pause/resume/cancel/retry/restart recovery | Implemented; Automated-tested; Application-verified | `engine.rs:664-814,1113-1201`, `store.rs:579`, `recovery.rs:28`; `store_tests.rs:102`, `transfer_tests.rs:640,829`; smoke kill→restart→nonzero-range |
| 6 | Queue/priorities/categories/speed limits/concurrency | Implemented; Automated-tested; Application-verified | `store.rs:299,338,932`, `ratelimit.rs:16-81`; smoke queue-admission group |
| 7 | Scheduling + optional shutdown | Implemented; Automated-tested; Application-verified | `engine.rs:2114-2179,984-993`, `desktop.rs:72-113`; `scheduler_tests.rs:63-143`; smoke power-cancel. Note: `apply_action_after` shutdown arm is an intentional no-op; real shutdown driven by `shutdown_after_complete` |
| 8 | Browser handoff for permitted downloads | Implemented; Automated-tested; Application-verified (Edge/Firefox; **Chrome not confirmed**) | `native_host.rs:60-168`; 83 browser tests; `artifacts/browser-acceptance/run-EDt1Fy/result.json` (Edge), `artifacts/firefox-acceptance/run-ae163dec…/result.json` (Firefox). **No Chrome result artifact.** |
| 9 | Clipboard + drag-drop capture | Implemented; Automated-tested; Application-verified partial | `desktop.rs:139-161,220-233`, `App.tsx:174-185`; `desktop.rs:241-246`; drag-drop in smoke; **OS clipboard mutation skipped by default** (`smoke-desktop.mjs --allow-clipboard-test`) |
| 10 | Automatic filenames + category folders | Implemented; Automated-tested; **no real-run artifact** | `engine.rs:2490-2567`; `server_filename_tests.rs:22-230` (12). No artifact exercises a server-named download. |
| 11 | Checksum verification | Implemented; Automated-tested; Application-verified | `engine.rs:1932,2804`; `transfer_tests.rs:422`; smoke SHA-256 outputs |
| 12 | Optional antivirus scan | Implemented; Automated-tested; **fixture only** | `scanner.rs:1-70`; `transfer_tests.rs:494-525` uses fixture command, not a real installed AV product |
| 13 | Dark mode/keyboard/localization-ready/a11y status | Implemented; Automated-tested; partial application-verified | `App.tsx`, `i18n.tsx`, `Dialog.tsx`; `verify.mjs:331,348,358,386` (41 tests); smoke label check; **no screen-reader certification artifact**; view copy still largely hard-coded English |
| 14 | Signed installer/updates/rollback/diagnostics | Implemented; Automated-tested; **Externally blocked** for release | `updates.rs`, `commands.rs:641`, `DiagnosticsView.tsx:22-28`; `updater_hardening_tests.rs` (3) + signature tests; **installers unsigned, pubkey empty, update env unset** — see below |

## Audit findings reconciliation (master prompt §5, verified against current repo)

| Audit finding | Verdict | Evidence |
|---|---|---|
| Chrome handoff not confirmed | **RESOLVED** | Real Chrome 158 handoff passed: `artifacts/browser-acceptance/run-7T7n8Y/result.json` (`browser: "Google Chrome"`, `passed: true`), via `SWIFTFETCH_TEST_BROWSER` + `SWIFTFETCH_TEST_HEADFUL` in `tools/browser-acceptance.mjs` and `-Browser Chrome` in `tools/test-browser-registration.ps1` (isolated profile, temporary registration removed). |
| Clipboard OS test skipped by default | **Confirmed** | `tools/smoke-desktop.mjs` `--allow-clipboard-test` gate; unit URL-filter test exists. |
| Server-filename no real-run artifact | **Confirmed** | 12 source tests; no artifact with a server-named download. |
| Antivirus uses fixture command | **Confirmed** | `transfer_tests.rs:494-525`. |
| Screen-reader certification absent | **Confirmed** | No SR artifact; docs concede this. |
| Installers unsigned / pubkey empty / env unset | **Confirmed** | `candidate-manifest.json` `ready_for_public_release:false`, all executables `NotSigned`; `tauri.conf.json:31` `pubkey:""`; update/signing env vars unset. |
| No clean-machine install/upgrade/rollback/uninstall run | **Confirmed** | Only isolated-profile/WebView/IPC runs in artifacts; no clean-VM result. |
| Startup 26–49 s, cause unconfirmed | **INSTRUMENTED (Phase 3)** | `artifacts/startup-probe/timing-*/timing.json`: Rust setup completes in ~4.4–8.6 s (SQLite open 1–6 s, WebView2 window build 2.5–3.1 s, all other phases <40 ms). Full cold render 26–49 s (`run-TDPTEn`, `run-fiWmSE`) is therefore post-window first-paint, not app logic. Instrumentation is env-gated (`SWIFTFETCH_STARTUP_TRACE=1`) and privacy-safe (phase labels + ms only). No performance change made; smallest safe fix + regression test to be proposed before implementation per prompt. |
| Smoke results not machine-readable | **Confirmed** | No summary/result JSON in any `artifacts/smoke/run-*` folder (only WebView `metadata.json` files). |
| 13 Firefox-background tests invoked separately | **Confirmed** | `tools/verify.ps1` runs only `test_protocol.js` + `test_release.js` (70); no `test_firefox_background` reference. |
| Documentation test counts drift | **Confirmed** | Current source: `verify.mjs` 41 `test()`, Rust unit 57 `#[test]`, Rust integration 70 `#[test]` (8 binaries), browser 66+4+13=83. Docs variously state 34/37/41, 48/55, "six binaries/61". |
| Seven third-party packages lack license text | **Confirmed (doc record)** | `VERIFICATION.md` records 338 packages, 7 roots without license text; notices at `artifacts/dependencies/notices-a1587a6b…`. |
| Terms/publisher details drafts | **Confirmed** | `docs/TERMS-DRAFT.md`; `RELEASE-READINESS.md:32-57` lists owner decisions. |

## Externally blocked items (need owner/external resources)

1. Trusted Authenticode certificate + private key → signed installers (zero-cost path: none for a publicly trusted cert).
2. HTTPS update endpoint + archive URL + public key + Tauri update-signing key; immutable signed manifests.
3. Distribution license, publisher identity/contact, approved Terms/Privacy/Support.
4. Clean Windows VM/Sandbox for install/upgrade/rollback/uninstall acceptance (Sandbox unavailable here).
5. Browser-store signing/review and permanent install validation.
6. Screen-reader (NVDA/JAWS) and contrast certification; 24-hour soak; real-site compatibility sampling.
7. Real installed AV product for antivirus adapter acceptance.

## Conflicts between prompt, architecture, and code

- None found on scope: master prompt §2 matches architecture §2.1/§2.2/§2.3. "Direct media capture" in the prompt is the explicit-gesture/direct-URL handoff already implemented; it does **not** authorize hidden page scanning (prompt §2 explicitly forbids it). HLS/DASH extraction is deferred in both.
- Implementation nuance to note: architecture §2.1 says "optional shutdown after completion"; the `shutdown` action-after arm is a deliberate no-op and shutdown is driven by `shutdown_after_complete` in `run_task` — functionally present but implemented through a different path than the literal schedule action.
