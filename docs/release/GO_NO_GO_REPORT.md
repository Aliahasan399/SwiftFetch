# SwiftFetch — Final GO / NO-GO Report

**Version:** 0.1.0 · **Date:** 2026-11-21 · **Repo:** `E:\CloneIdm\swiftfetch`.
**Correction (Phase 1 audit):** the original snapshot text said "not a git repo; source
snapshot hash used instead of commit". That was accurate when written but is now stale:
the workspace is a Git repository on `main` with a remote at
`https://github.com/Aliahasan399/SwiftFetch.git`. The verdict and all gates below are unchanged.

## Verdict: **NO-GO**

A trusted signing certificate, configured HTTPS update channel, approved license/publisher identity,
and clean-machine install/upgrade/rollback/uninstall acceptance are all absent. Per the master prompt,
any open critical gate forces NO-GO. This is an unsigned, privately verified release candidate, not a
safe public production release. Do not set `ready_for_public_release: true`.

## Evidence summary (fill final run paths after last verification)

- **Canonical verification (final, post-instrumentation):** `./tools/verify.ps1` exit 0;
  machine-readable `artifacts/verification/report-20261121-184958.json` (`success: true`,
  all 7 checks passed), full log `artifacts/final-verify.log`. Totals: HTTP fixture 46/46;
  Rust 128 (59 unit + 69 integration across 8 binaries), 0 failed; Vite build; 41 frontend;
  browser 83 (70 protocol/release + 13 Firefox background).
- **Browser real-run:** Edge `artifacts/browser-acceptance/run-EDt1Fy/result.json`;
  Firefox `artifacts/firefox-acceptance/run-ae163dec…/result.json`;
  Chrome `artifacts/browser-acceptance/run-7T7n8Y/result.json` — all passed real extension load,
  native-host spawn, queue intake and checksum-matched output under isolated profiles with
  temporary registrations removed.
- **Desktop real-run:** `artifacts/smoke/run-zIBKJy` (15/15 groups incl. full restore and corrupt-DB
  startup recovery); `artifacts/startup-probe/run-TDPTEn/results.json` (12/12 cold renders).
  Machine-readable smoke result now written by `tools/smoke-desktop.mjs` to `result.json` per run.
- **Startup (Phase 3):** instrumented phase traces at
  `artifacts/startup-probe/timing-*/timing.json` — Rust setup path completes in ~4.4–8.6 s
  (SQLite open 1–6 s, WebView2 window build 2.5–3.1 s, all other phases <40 ms). Full ~26–49 s
  cold-start figure therefore occurs **after** window creation (WebView2 first paint / React mount).
  No speculative performance change was made.

## Final artifact locations (fresh build, 2026-11-21)

Built binaries — all version `0.1.0` and all `NotSigned`:

| Artifact | SHA-256 |
|---|---|
| `src-tauri/target/release/swiftfetch.exe` | `eebb578832936b29dc73e97e65178a942d026c719bed1045379180bec596fceb` |
| `src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi` | `58df1a1410cd43fb7480e08bb708b5e9f71d7db17c224c1f481907a92d588f32` |
| `src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe` | `2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454` |

- Final candidate: `artifacts/SwiftFetch-0.1.0-candidate-e79ab68b63584b42ae77df98aa1e9e19`
  — `candidate-manifest.json` records 626 files, `ready_for_public_release=false`, 0 hash
  mismatches on independent recheck; `SHA256SUMS` has 626 lines.
- Final source snapshot:
  `artifacts/source/0.1.0-ab544367991d49518258baa9ffbbe97d/SwiftFetch-0.1.0-source.zip`
  — SHA-256 `0efc8ceb83bed247f9218688d69e28194dc79d8ef69576007af0eb49461b0c71`; 184 archive entries.
- Full build/repackage evidence and exit-code note: `docs/release/FINAL_BUILD_EVIDENCE.md`.
- Build log: `artifacts/final-build.log`. The reported build-shell `exit=1` was the PowerShell
  stderr-redirection false error (npm informational stderr under `$ErrorActionPreference='Stop'`),
  not a build failure; the fresh, hash-consistent artifacts above confirm success.

The `docs/release/` copies inside the candidate/source snapshots predate this section; the
working-tree files in `docs/release/` are authoritative. The verdict below is unchanged.

## Gate status

| Gate | Status | Evidence / blocker |
|---|---|---|
| Core §2.1 functionality | Implemented + automated-tested + real-app verified | gap register; verification report |
| Browser Edge/Firefox/Chrome handoff | Verified with real-run artifacts | three result.json runs |
| Evidence reliability (Phase 1) | Ready | one canonical command, all suites wired, JSON reports |
| Trusted Authenticode signatures | **Externally blocked** | all executables `NotSigned`; no certificate |
| HTTPS update channel + signed manifests | **Externally blocked** | `pubkey:""`, env unset; `OWNER_CONFIG_GUIDE.md` |
| Clean-machine install/upgrade/rollback/uninstall | **Externally blocked** | no clean VM; `CLEAN_MACHINE_ACCEPTANCE.md` |
| Real signed update + automatic rollback | **Externally blocked** | unit-level tests only |
| License + publisher identity/terms/privacy | **Externally blocked** | drafts/placeholders; 7 license-text gaps |
| Screen-reader / contrast certification | **Not verified** | `A11Y_CHECKLIST.md`; `[OWNER: SR personnel]` |
| 24-hour soak / permitted-site sampling | **Not verified** | `STABILITY_CHECKLIST.md` |
| Startup <1 s target | **Not met** (measured ~26–49 s cold) | phase traces; hypothesis: WebView2 first-paint; no fix applied |

## Unresolved risks
- Unsigned installers cannot establish publisher trust; SmartScreen/AV warnings likely.
- Unconfigured updater cannot deliver signed updates; rollback path only unit-tested.
- No clean-machine evidence for data preservation through install/upgrade/uninstall.
- Cold-start latency far above the architecture's <1 s target.
- Legal ownership (license/publisher terms) unresolved; cannot be decided by the agent.

## Owner decisions required
See `OWNER_CONFIG_GUIDE.md` and `RELEASE_PLAN.md` P0 list: signing certificate/key (local only),
update hosting + endpoints + public/signing keys, license choice, publisher identity/contact,
approved Terms/Privacy/Support, clean Windows VM, browser-store strategy, SR personnel/soak window.

## Conditions to flip to GO
1. Signed exe/MSI/NSIS with valid publisher Authenticode; hashes in manifest.
2. Configured HTTPS update channel with signed immutable manifests; real signed upgrade + rollback on a clean VM.
3. Documented clean-machine install/upgrade/rollback/uninstall preservation with evidence.
4. Approved license + publisher identity/contact + legal docs; 7 license-text gaps resolved.
5. Real screen-reader/contrast and 24-hour soak evidence; startup target accepted or fixed.
6. `ready_for_public_release: true` set only after all above are evidenced.
