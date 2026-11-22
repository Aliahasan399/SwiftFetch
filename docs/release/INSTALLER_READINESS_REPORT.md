# SwiftFetch — Installer Readiness Report (Phase 2)

**Date:** 2026-11-22 · **Version:** 0.1.0 · **Verdict:** **NO-GO** (unchanged)
**Repository:** `https://github.com/Aliahasan399/SwiftFetch` · branch `main`

## 1. Scope

Phase 2 improved installer tooling, verification, documentation and upgrade/uninstall analysis.
It did **not** publish installers, create a release or tag, configure automatic updates, start
production code signing, or modify dependencies. The candidate manifest keeps
`ready_for_public_release: false`.

## 2. What changed

| Change | Path |
|---|---|
| Added reproducible-build recording script | `tools/build-installers.ps1` |
| Extended installer verification (NSIS metadata, pinned hashes) | `tools/verify-installer.ps1` |
| Added static installer behaviour auditor | `tools/audit-installer.ps1` |
| Added offline data-preservation simulation | `tools/installer-data-safety.ps1` |
| Expanded clean-machine acceptance instructions | `docs/release/CLEAN_MACHINE_ACCEPTANCE.md` |
| Documented installer architecture and analysis | `docs/release/INSTALLER_ENGINEERING.md` |
| This report | `docs/release/INSTALLER_READINESS_REPORT.md` |
| Post-Phase-2 NSIS cleanup fix | `tools/patch-nsis-cleanup.ps1`, `tools/rebuild-nsis-installer.ps1`, `docs/release/NSIS_CLEANUP_FIX.md` |
| Acceptance procedure | `docs/release/INSTALLER_ACCEPTANCE_PROCEDURE.md` |
| Hash baseline policy | `docs/release/INSTALLER_HASH_BASELINE.md`, `tools/generate-distribution-hashes.ps1` |
| Phase 3 signing research (read-only) | `docs/release/CODE_SIGNING_PROGRAM_COMPARISON.md`, `CODE_SIGNING_APPLICATION_PACKET.md`, `SIGNING_ARCHITECTURE.md`, `CLEAN_VM_OPTIONS.md` — no application or signing activated |

Existing files were backed up before editing:
`tools/verify-installer.ps1.bak-20261122-093603`,
`docs/release/CLEAN_MACHINE_ACCEPTANCE.md.bak-20261122-093603`.
No evidence file was overwritten.

## 3. Commands executed and observed outputs

All commands ran from `E:\CloneIdm\swiftfetch` in Windows PowerShell.

| Command | Exit | Output / evidence |
|---|---|---|
| `./tools/build-installers.ps1 -InspectOnly` | 0 | `artifacts/installer-builds/build-20261122-093740.json`; recorded MSI/NSIS size, hash and `NotSigned`; no build run |
| `./tools/audit-installer.ps1` | 0 | `artifacts/installer-audit/audit-20261122-093740.json`; 18 findings — 0 FAIL, 6 WARN, 2 BLOCKER |
| `./tools/installer-data-safety.ps1` | 0 | `artifacts/installer-data-safety/data-safety-20261122-093816.json`; 5/5 scenarios preserve `swiftfetch\` data |
| `./tools/verify-installer.ps1 -ExpectedMsiSha256 … -ExpectedNsisSha256 …` | 0 | `artifacts/installer-52dbf174a2664359861db8d5c776dcaa/result.json`; MSI extraction + 14 smoke passes, 1 skip; hashes matched. One earlier run hit a transient `Desktop launch 1 did not render` timeout immediately after the heavy canonical suite; an isolated rerun passed with no code change. |
| `./tools/verify.ps1` (canonical suite) | 0 | `artifacts/verification/report-20261122-100931.json`; `success: true`, 7/7 checks (46 fixture, 128 Rust, 41 frontend, 83 browser assertions) |
| `./tools/patch-nsis-cleanup.ps1` | 0 | patched then idempotent; `artifacts/installer-nsis-patch/patch-20261122-100208.json` |
| `./tools/rebuild-nsis-installer.ps1 -OutputPath …` | 0 | patched NSIS compiled by makensis v3.13; `artifacts/nsis-rebuild/nsis-20261122-100234/rebuild-result.json` |
| `./tools/installer-data-safety.ps1` (updated) | 0 | `artifacts/installer-data-safety/data-safety-20261122-101257.json`; 5/5 expectations met, unrelated files intact |
| `./tools/audit-installer.ps1` (updated) | 0 | `artifacts/installer-audit/audit-20261122-101257.json`; 20 findings, 0 FAIL, path alignment PASS |
| `./tools/build-installers.ps1 -InspectOnly` (final) | 0 | `artifacts/installer-builds/build-20261122-101256.json`; both historical hashes unchanged |
| `./tools/patch-nsis-cleanup.ps1 -CheckOnly` (final) | 0 | `artifacts/installer-nsis-patch/patch-20261122-101257.json`; already-patched |
| `./tools/generate-distribution-hashes.ps1` | 1 (expected refusal) | refused unsigned artifacts; no `SHA256SUMS` generated |

The NSIS path in `verify-installer.ps1` is verified statically (metadata + hash) only; it was
not installed on this machine.

## 4. Hash verification

| Artifact | SHA-256 | Status |
|---|---|---|
| MSI | `58df1a1410cd43fb7480e08bb708b5e9f71d7db17c224c1f481907a92d588f32` | matches `FINAL_BUILD_EVIDENCE.md` baseline |
| NSIS | `2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454` | matches `FINAL_BUILD_EVIDENCE.md` baseline |

Both artifacts are `NotSigned`.

## 5. Principal findings

1. **Upgrade identity stable.** UpgradeCode `1b0348ae-0c8d-5327-8c38-b0e4d61e9ff1` and Tauri
   identifier `com.swiftfetch.app` are unchanged.
2. **Data separation by design.** Runtime data resolves to `%APPDATA%\swiftfetch\state.sqlite`,
   outside both installers' install directories. MSI authors no app-data removal; NSIS removes
   app data only on explicit opt-in and not during updates.
3. **Downgrades allowed in both installers** (`AllowDowngrades="yes"`, `ALLOWDOWNGRADES "true"`).
   This is a permitted-but-untested path and must be validated on a clean VM, including SQLite
   schema compatibility with an older binary.
4. **Opt-in app-data deletion corrected in source.** NSIS cleanup now targets only
   `$APPDATA\swiftfetch`; no `LOCALAPPDATA` tree is removed. Offline preserve/delete/unrelated-file
   scenarios passed. Real VM confirmation is still pending
   (`docs/release/NSIS_CLEANUP_FIX.md`, `docs/release/INSTALLER_ACCEPTANCE_PROCEDURE.md`).
5. **Reproducibility not proven.** No `SOURCE_DATE_EPOCH`, GUID-named frontend output path,
   floating Node/runner versions, and no byte-identical independent rebuild.
6. **Signature and updater blockers unchanged.** No trusted Authenticode certificate; updater
   `pubkey` empty and `not_configured`.

## 6. Limitations — explicit boundary

- All installer-behaviour conclusions in this report are static inspection or offline
  simulation. No installer was installed, upgraded, downgraded or uninstalled on a real
  Windows machine.
- No clean Windows VM, no signed artifacts, no signed update channel, and no 24-hour soak were
  available in this environment.
- The simulation modelled app-data operations only; it cannot detect OS-level behaviour such
  as file locking, shortcut repair, elevation prompts, or WebView2 bootstrap failure.
- No claim of reproducible builds is made. A real rebuild comparison is still required.

## 7. Required before GO

1. Obtain a trusted Authenticode signing certificate and sign both installers.
2. Execute the full `CLEAN_MACHINE_ACCEPTANCE.md` sequence on clean VMs for MSI and NSIS,
   including upgrade, downgrade, uninstall, per-user/per-machine and failed-upgrade rollback.
3. Produce a `clean-machine-result.json` with real signatures, hashes, commands and screenshots.
4. Confirm the corrected opt-in app-data deletion on a clean VM (procedure N5/N6/N7).
5. Prove reproducible builds with two byte-identical independent builds, or formally accept
   non-reproducibility.
6. Configure and test a signed update channel only after the owner explicitly approves it.

Until all of the above are satisfied, keep `ready_for_public_release: false` and the NO-GO
verdict.
