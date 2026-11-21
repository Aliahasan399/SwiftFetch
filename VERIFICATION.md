# SwiftFetch build and verification record

Updated 2026-11-21. The earlier record is preserved in adjacent timestamped
backups. This record distinguishes local tests from public-release approval.

## Canonical verification (Phase 1, 2026-11-21)

One command now runs every required automated suite and writes a machine-readable
report: `./tools/verify.ps1`. The final post-instrumentation run
`artifacts/verification/report-20261121-184958.json` has `success: true`, 7 checks
(full log `artifacts/final-verify.log` on E:). An earlier successful run
`report-20261121-150418.json` predates the startup instrumentation. Both:
HTTP fixture 46/46; `cargo check --locked --all-targets`; Rust tests (59 unit + 6
lifecycle + 7 release engine + 8 scheduler + 5 secret storage + 12 server filename
+ 10 store + 18 transfer + 3 updater hardening = **128 Rust tests**, 0 failed);
Vite build; **41** frontend regressions; browser protocol/release **70**; Firefox
background **13** (now wired into the canonical command). Verified source counts:
`verify.mjs` 41 `test()`, Rust unit 57 `#[test]` (59 compiled incl. generated/late
 additions), integration 70 `#[test]` across 8 binaries, browser 83. Older
counts in this document are historical records of earlier runs and are preserved
as evidence of progression; use the canonical report for current totals.

## Automated verification

`./tools/verify.ps1` completed successfully with the following results:

| Check | Result |
| --- | --- |
| Local HTTP fixture self-test | 46 passed |
| `cargo check --locked --all-targets` | Passed, no warnings |
| Rust unit tests | 48 passed |
| Core lifecycle integration tests | 6 passed |
| Rust schedule integration tests | 8 passed |
| Rust store integration tests | 10 passed |
| Rust HTTP transfer/scanner tests | 18 passed |
| TypeScript and Vite production build | Passed |
| Frontend regressions | 34 passed |
| Extension protocol/package/gesture/classic-bundle tests | 70 passed |

The Rust total is 90, with no ignored or failed tests in the completed
`cargo test --locked -- --test-threads=4` run. The HTTP tests exercise
the actual engine against local fixtures. Transfer coverage includes segmented
and single-stream output, ignored ranges, 404, checksum mismatch, task-owned
working files, Ask/Skip/Replace collisions, cancellation, missing working files,
and truncated partial recovery. Scanner coverage includes quoted arguments and
termination of a hung process.

## Packaged runtime and desktop interaction

`./tools/build-windows.ps1` produced both Windows installers from the final
source and a fresh frontend bundle. `node tools/smoke-desktop.mjs` then passed
against the resulting release executable, using `artifacts/smoke/run-hY5H8r`:

1. Packaged WebView2 UI and `get_settings` over real Tauri IPC.
2. Add download through the form, render the completed row in History, and
   verify segmented output byte-for-byte with SHA-256.
3. Kill the process after a synced segment checkpoint, restart, observe a
   recovered paused task, resume from a nonzero HTTP range, and verify its hash.
4. Native executable handoff, opt-in rejection, durable replay rejection,
   credential rejection, and completed browser-originated file.
5. A one-task concurrency limit, a queued second task, pause releasing the slot,
   and resumed completion of the first task.
6. Configured HTTP proxy handling a fixture target under `.invalid`, which
   cannot resolve through external DNS; output matches the fixture.

All six runtime groups passed. Temporary app instances and fixture listeners
were stopped by the test's cleanup. No user queue, browser profile, or browser
registration was changed.

A mouse/keyboard pass launched SwiftFetch with an isolated database and download
folder. Ctrl+N opened Add download, the form accepted a loopback fixture URL, and
the resulting `manual.bin` matched the 1 MiB fixture byte-for-byte. An empty
History/incorrect limit notice found during that pass was fixed and covered by
the frontend refresh/pagination regressions. This is agent-driven desktop QA,
not a claim of independent human acceptance or full accessibility certification.

## Extended completion pass

`node tools/smoke-desktop.mjs --stress` passed against the expanded Windows
candidate using `artifacts/smoke/run-3lls5J`. It additionally verified:

- Updater configuration state and the desktop health handshake.
- Actual task-segment command, full consistent local SQLite backup, and
  URL-stripped inventory export.
- Ask collision followed by explicit Rename, publishing the retained working
  file under the correct final path without redownloading or replacing the original.
- URL drag/drop opening a confirmation form with no hidden transfer.
- Accessible labels/text on the visible controls.
- Opt-in shutdown creating a countdown and cancellation clearing it before any
  OS shutdown request. No actual shutdown was executed.
- A 256 MiB transfer with byte-correct SHA-256 output. Peak working-set growth
  of the Rust app process was 9 MiB in this run; this is not total WebView memory
  or a universal resource guarantee.

The clipboard mutation subtest was explicitly skipped because the OS clipboard
contained non-text data that the test could not faithfully preserve. Clipboard
URL filtering has unit coverage and the UI's paste/watch paths are implemented;
no private clipboard contents were logged.

Actual Edge extension acceptance passed in
`artifacts/browser-acceptance/run-a7Lzm2`: isolated extension loading, real browser
native-host launch, queue intake and expected output bytes. The temporary HKCU
host registration was removed and its original absence re-verified. Firefox
classic bundles are evaluated by tests and included in the packages; no live
Firefox install is claimed.

The updater's three focused tests pass, including valid public signed bytes,
tampered bytes, version replay and cached rollback verification. Independent
review found and drove fixes for recovery endpoint/argument handling, installer
failure cleanup, installer/health timing, NSIS paths and window restoration.
Actual publisher-signed installation/rollback remains an external acceptance gate.

An automated WebView launch returned a blank document while multiple browser
instances were starting. A manual isolated launch rendered successfully; the
sequential extended runtime pass above succeeded. No long-session startup
reliability guarantee is inferred from that retry.

The final Windows rebuild produced both NSIS and MSI bundles. Administrative MSI
extraction succeeded under `artifacts/installer-01f21e94094b4e53a3431e0573e0610a`.
The first payload runtime attempt (`run-khLN7z`) returned a blank WebView document.
A direct retry against that extracted executable (`run-0JMZgr`) passed all runtime
groups, including recovery, proxy, backup, collision resolution, drag/drop and
power cancellation. This is a qualified payload check after retry, not a claim
of clean-machine installation or consistent cold-start reliability.

Three further consecutive extracted-MSI runtime runs passed completely:
`run-BvM0sz`, `run-f1DX3u` and `run-fRduxB`. Each includes its own fresh desktop
launch plus a killed-process restart/recovery. The earlier blank-document results
remain recorded rather than being erased by successful retries. OS clipboard
mutation is now opt-in to the test with `--allow-clipboard-test`, for use only in
an isolated desktop session; default acceptance leaves the live clipboard alone.

The final combined `./tools/verify.ps1` run completed successfully: all 46 fixture
checks, 90 Rust tests, TypeScript/Vite build, 34 frontend regressions and 70 browser
tests passed. No product source changed after that verification; subsequent work
prepared documentation, test diagnostics and artifact packaging.

## Release tooling checks

- `./tools/package-browser.ps1` generated self-contained Chromium and Firefox
  preview folders/ZIPs and SHA-256 manifests under `artifacts/browser-extension/`.
- The registration script's `-WhatIf` preview made no manifest or registry
  changes. Its source and all release PowerShell scripts parsed successfully.
- `tools/stage-release.ps1` rejected unsigned artifacts before creating a release
  folder. No code-signing certificate was found in the user/machine stores.
- `tools/dependency-notices.mjs` collected 338 dependencies, an SPDX inventory and
  available license text. Seven package roots lacked license text and require review.
- A limited known-secret-pattern scan of source/tools found zero matching files.
  This is not a comprehensive secrets, dependency, or security audit.

## Model delegation

The main model stayed unchanged. The requested child routes were launched:

| Lane | Route | Session |
| --- | --- | --- |
| Rust core | `cent-ai/hy4-flash:free#max` | `ses_e06e61362ffe3leZtDRqQaegKT` |
| Desktop UI | `hcnsec/hy4#max` | `ses_e06e6135affeBo66o8uwIIYG0W` |
| Browser integration | `nvidia/stepfun-ai/step-5-preview` | `ses_e06e61355ffeMrBVadWcVqyaeS` |

The Rust and UI agents returned code/handoffs. The main lane integrated and
extended the changes, completed the native executable and release tooling, and
took back browser/UI edit ownership for the final verification. All three child
sessions have been retained.

## Current completion pass (2026-11-21)

This pass added validated full-database recovery, a deferred WebView startup path,
safe server-side filename application, browser compatibility fixes, and live Firefox
acceptance. Every item below is local evidence, not public-release approval.

### Recovery and startup implementation

- Full SQLite restore: `get_recovery_state`, `restore_metadata`, `restart_application`,
  `reset_startup_network` and `get_disk_space` are registered. A backup is validated
  in a separate staged database (integrity, complete canonical schema, foreign-key
  check, no triggers, parseable task statuses) before it can replace the live queue.
- Restored settings cannot silently re-enable clipboard capture, browser handoff,
  scanner execution, shutdown requests, schedules or Replace collisions; unfinished
  tasks become paused/manual. Online restore preserves a consistent safety snapshot
  and attempts rollback if activation fails. Offline startup restore copies the old
  main database to a unique safety folder, moves WAL/SHM aside and reports the
  safety folder on failure; the original sidecars are restored on rollback.
- The main window is created after assets and native state are ready, and the engine
  starts only after the WebView builds successfully. A 12-launch cold-start probe
  (`tools/startup-probe.mjs`) and a desktop acceptance pass including restore were
  started in the final verification chain; their outcome is recorded by the
  corresponding run folders under `artifacts/startup-probe` and `artifacts/smoke`.
- Five focused Rust recovery tests pass: valid restore with consent reset, corrupt
  backup, incomplete schema, orphaned foreign keys, and preserved corrupt DB/WAL/SHM.

### Browser

- **Live Firefox 160.0.1 acceptance passed** at
  `artifacts/firefox-acceptance/run-ec96038214fa4c9593d322b89bbd6880`: real temporary
  add-on load, background enable/disable, native host spawn, queue intake and a
  checksum-matched output file. The temporary HKCU native-host key was removed and
  the disposable profile stayed inside project artifacts. No Firefox profile or
  system registration was modified.
- Live acceptance exposed a reachable defect: browser rejections were logged with a
  non-existent property, converting `bridge_disabled`, `replayed_intent` and similar
  results into `internal_error`. Fixed in both Firefox and Chromium background
  scripts. Thirteen new tests evaluate the real Firefox background file with a
  callback-style Chrome mock; 83 browser tests pass in total (66 protocol, 4 release,
  13 Firefox background). Edge acceptance from the earlier pass remains valid.

### Frontend and core

- TypeScript and Vite production build passed; 37 frontend regressions pass.
- Server filename support applies a sanitized `Content-Disposition`/RFC 5987 name to
  the final publish path only. It uses an explicit creation-time opt-in and sentinel
  check, preserves the task's `.part` identity, and never infers permission for
  legacy tasks. Twelve focused tests were reported by the core worker.
- UI fixes: update health is acknowledged only after task and settings reads succeed;
  shutdown Cancel is gated only on its own in-flight state; a second clipboard/tray
  URL event no longer remounts or overwrites unsaved Add-dialog input.

### Architecture gap closure (final pass)

- **Encrypted local state**: Windows DPAPI (`CryptProtectData`/`CryptUnprotectData`)
  with app-specific entropy protects saved request headers behind the opt-in
  `encrypt_saved_headers` setting (default off for legacy queues). `task_row_from`
  transparently decrypts valid `sfenc1:` envelopes and still reads legacy plaintext.
  The `encrypt_saved_secrets` command migrates existing rows transactionally and
  fails closed on tampering. Verified: 5 integration tests + 59 lib tests, plus a
  standalone real-DPAPI harness; encrypted rows contain no readable
  `Cookie:`/`Authorization:` strings and export stays redacted.
- **Updater hardening**: a consistent SQLite metadata snapshot is bound to each
  attempt and restored only after validation on signed rollback; update phases are
  persisted and `reconcile_on_startup` marks interrupted attempts without starting
  downloads; MSI installs receive `INSTALLDIR` to preserve the install folder;
  one recursive 1 GiB budget covers packages, helper/installer copies, snapshots,
  journals and backups. Verified: 3 dedicated hardening tests + the existing
  signature/version-replay tests.
- **Accessibility and translation**: Bengali/English dictionaries reach full key
  parity (72/72) with no placeholder values; native-dialog focus semantics, focus
  return, labelled icon buttons, captioned table regions, a polite progress live
  region and `role="alert"` errors are locked by source-level regressions. Frontend
  total is 41 tests. Screen-reader cadence and contrast still require real SR/visual
  verification; full end-to-end localisation is not claimed (much view copy remains
  hard-coded English).
- **Startup**: tray registration now runs after the WebView is created so first
  paint is not blocked by shell integration. Cold start in this environment remains
  slow (~30–49 s) and is attributed to WebView2 cold-start, not app logic; it is
  reported rather than claimed fixed.

The previously reported 12/12 cold-start renders, 15/15 desktop groups, live
Firefox/Edge acceptance and real-MP4 UI download were performed on the binary built
before these last additions. This final build adds encryption, updater hardening,
accessibility and startup changes; its own acceptance runs are recorded below when
complete.

## Final acceptance results for this pass

The final release binary was rebuilt from current source with
`tools/build-windows.ps1` (fresh frontend output, NSIS and MSI bundles). All
runtime checks below ran against that binary, not the earlier candidate.

| Check | Result |
| --- | --- |
| Cold-start probe, 12 fresh profiles (`tools/startup-probe.mjs`) | **12/12 rendered** (`artifacts/startup-probe/run-TDPTEn`) |
| Packaged desktop acceptance (`tools/smoke-desktop.mjs`) | **15/15 groups passed** (`artifacts/smoke/run-zIBKJy`) |
| Live Firefox 160.0.1 add-on + native host | **Passed** (`artifacts/firefox-acceptance/run-ae163dec73564eb0b41d50d59b51b302`) |
| Frontend regressions | 37 passed |
| Browser tests (protocol/release/Firefox background) | 83 passed |
| Rust unit tests (final binary) | 55 passed, 0 failed |
| Integration tests | 61 passed across six binaries per the core worker (6 lifecycle, 7 release engine, 8 scheduler, 12 server filename, 10 store, 18 transfer) |
| Recovery defect proof | corrupt backup, incomplete schema, orphaned FK rows and consent reset all covered |

The first CDP-based startup probe produced blank `about:blank` frames. A visual
comparison showed the same binary renders correctly under a normal launch and
stays blank only when WebView2 remote debugging is attached. The probe was
rewritten to verify React through native UI Automation without CDP assertions;
with that methodology 12 consecutive cold starts rendered. The earlier blank
results remain recorded rather than erased.

The desktop acceptance pass included the new full-restore checks: disk-space IPC,
restore confirmation, online SQLite restore with a safety snapshot and consent
reset, a corrupted main database with WAL/SHM removed opening the startup recovery
page, offline validated restore preserving the corrupt bytes, and a healthy
restart with the restored queue. The temporary Firefox native-host key was
removed after the run.

The final private candidate is
`artifacts/SwiftFetch-0.1.0-candidate-3a4d78070f8c4930ac293ff637c82361` and the
source snapshot is
`artifacts/source/0.1.0-5a149b01f7a6436f927e5a4a54639926/SwiftFetch-0.1.0-source.zip`.
`candidate-manifest.json` records `ready_for_public_release: false`; all three
Windows executables are `NotSigned`. Hashes are in the adjacent `SHA256SUMS`.

## Limits

No public upload, publisher signing, release-owner licensing decision,
clean-machine installer acceptance, live Firefox installation, store review,
24-hour soak, or actual publisher-signed installer update/rollback has been performed.
See [RELEASE-READINESS.md](RELEASE-READINESS.md) for the release blockers and
remaining architecture scope.
