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
| `./tools/verify-installer.ps1 -ExpectedMsiSha256 … -ExpectedNsisSha256 …` | 0 | `artifacts/installer-2059e517624b4f02b37bcfd83f88b4ed/result.json`; MSI extraction + 14 smoke passes, 1 skip; hashes matched |

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
4. **Opt-in app-data deletion mismatch.** NSIS cleanup targets `com.swiftfetch.app`, not
   `swiftfetch`. Preserve-by-default holds, but the opt-in delete is ineffective. Documented,
   not fixed in this phase.
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
4. Decide and test the opt-in app-data deletion path mismatch.
5. Prove reproducible builds with two byte-identical independent builds, or formally accept
   non-reproducibility.
6. Configure and test a signed update channel only after the owner explicitly approves it.

Until all of the above are satisfied, keep `ready_for_public_release: false` and the NO-GO
verdict.
