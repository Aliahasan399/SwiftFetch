# SWIFTFETCH_SECOND_LAPTOP_TEST_PREPARATION.md

**Date:** 2026-11-22 · **Scope:** READ-ONLY investigation and preparation for a separate Windows laptop
**Status:** NO-GO remains active; `ready_for_public_release: false`; installers are unsigned preview candidates.
No rebuild, signing, publication, tag, updater change, source/config change, or SignPath action was performed.

## 1. Latest successfully built Windows installer (.exe)

| Item | Value |
|---|---|
| Primary release-bundle installer | `E:\CloneIdm\swiftfetch\src-tauri\target\release\bundle\nsis\SwiftFetch_0.1.0_x64-setup.exe` |
| Size | 3,704,490 bytes |
| SHA-256 | `2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454` |
| Modified | 2026-11-21T19:00:00+06:00 |
| VersionInfo | FileVersion `0.1.0`, ProductVersion `0.1.0`, ProductName `SwiftFetch` |
| Authenticode | `NotSigned` |
| PE header | 32-bit NSIS stub (`I386`), GUI subsystem; installs the x64 application payload (`!define ARCH "x64"`) |

This is the only installer `.exe` in the normal release bundle. Newer compiled NSIS files exist under
`artifacts/nsis-patch-test/` (for example `patched-setup.exe`, SHA-256
`1ba33f1fb97862caee4b16828f2dbfbfa7f6c69384046f1a19b97f80416d70ba`); those are **test-only** build
outputs from the cleanup-path investigation and must **not** be treated as release artifacts.

Companion MSI (not an `.exe`, listed for completeness):
`src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi` — 6,283,264 bytes, SHA-256
`58df1a1410cd43fb7480e08bb708b5e9f71d7db17c224c1f481907a92d588f32`, `NotSigned`.

## 2. Standalone executable

| Item | Value |
|---|---|
| Path | `E:\CloneIdm\swiftfetch\src-tauri\target\release\swiftfetch.exe` |
| Size | 10,375,680 bytes |
| SHA-256 | `eebb578832936b29dc73e97e65178a942d026c719bed1045379180bec596fceb` |
| Modified | 2026-11-21T19:00:01+06:00 |
| VersionInfo / signature | `0.1.0` / `SwiftFetch`; `NotSigned` |
| PE header | AMD64 (x64), GUI subsystem |

Adjacent file: `src-tauri/target/release/swiftfetch_lib.dll` (2,032,128 bytes). A literal string
search found `api-ms-win-crt` in the EXE and did not find `VCRUNTIME140.dll`; the full import table
was **not** parsed (`dumpbin` unavailable). For a standalone-copy test, copy the EXE **and** the DLL
into the same directory and record whether the DLL is actually required.

## 3. Target Windows versions and architecture

- Application binary is **AMD64 / x64**; no x86 or ARM64 application build was found.
- NSIS file name and script define `x64`; the installer stub itself is a 32-bit NSIS executable, which
  is normal and runs on 64-bit Windows.
- Documented acceptance target: **Windows 10 or 11 x64**, fresh user profile
  (`docs/release/CLEAN_MACHINE_ACCEPTANCE.md`). Local build/verification platform: Windows 11 x64.
- No explicit minimum Windows version is declared in `src-tauri/tauri.conf.json`.

## 4. Runtime dependencies and prerequisites

1. **Microsoft Edge WebView2 Runtime** — required by Tauri. The NSIS script checks HKLM/HKCU registry
   keys; if absent it uses `INSTALLWEBVIEW2MODE "downloadBootstrapper"` and downloads
   `https://go.microsoft.com/fwlink/p/?LinkId=2124703` silently. Internet access is therefore needed
   on the laptop if WebView2 is not preinstalled.
2. **Windows x64** with a writable per-user profile; NSIS `INSTALLMODE` is `currentUser`, installing to
   `$LOCALAPPDATA\SwiftFetch`.
3. **Universal C runtime** — `api-ms-win-crt` strings present in the EXE; normally supplied by
   supported Windows 10/11 systems. Exact CRT/VC redist requirement not confirmed by import-table parsing.
4. Optional browser handoff requires separate extension/native-host setup on the laptop; it is not part
   of the installer and is opt-in.
5. For download tests: a reachable HTTP/HTTPS fixture or trusted source, and any configured proxy.
   No account or hosted queue service is required.

## 5. Administrator privileges

- **NSIS `.exe`:** generated with `!define INSTALLMODE "currentUser"` and
  `RequestExecutionLevel user` (installer.nsi lines 40, 104-106). It should install per user without
  UAC elevation; target directory is `$LOCALAPPDATA\SwiftFetch` (line 537). The WebView2 bootstrapper,
  if invoked because the runtime is missing, may request elevation depending on Microsoft's installer.
- **MSI (for comparison):** WiX `InstallScope="perMachine"` and a per-machine Program Files install
  directory, so administrator rights are expected for that installer.
- The NSIS `perMachine` branch exists in the generated script but is not the configured mode here.

## 6. Known limitations and cautions

- **Unsigned:** both installers and the EXE are `NotSigned`. Windows SmartScreen/Unknown Publisher
  warnings and local policy blocks are expected. Do not interpret them as a failed functional install.
- **Historical build:** the release-bundle NSIS `.exe` was built **before** the Phase 2 NSIS cleanup-path
  fix. The generated script under `target/` is patched, but the shipped `.exe` still contains the old
  opt-in deletion target (`com.swiftfetch.app`). Do **not** use this binary to validate the corrected
  opt-in deletion behaviour; rebuild is explicitly out of scope here.
- **No clean-machine results:** all VM acceptance rows in `INSTALLER_ACCEPTANCE_PROCEDURE.md` are
  `NOT_EXECUTED`; only static/offline simulations exist. This laptop exercise would be the first real
  non-build-machine execution and must be recorded as such.
- **No updater:** update channel is `not_configured`; no automatic update or signed rollback can be tested.
- **Reproducibility:** byte-identical rebuilds are not proven; hashes above are historical evidence only.
- **WebView2 bootstrap:** needs network access if the runtime is absent; offline laptops must have it preinstalled.
- **Download behaviour:** depends on source server range support, redirects, proxy settings and checksum
  state; no guarantee of speed or success against arbitrary servers. Website terms prohibit circumvention
  claims. Checksum/antivirus results have limited meaning; only open files the user trusts.
- **Data preservation:** default install/uninstall preserves `%APPDATA%\swiftfetch`; this is designed and
  simulated, not yet confirmed on a second machine.

## 7. Safe transfer and verification instructions

1. Transfer only the primary installer from the exact path in §1. Prefer a trusted local medium or a
   controlled network share; do not fetch an installer from an unverified source.
2. On the source machine, record expected values:
   ```powershell
   $f = 'E:\CloneIdm\swiftfetch\src-tauri\target\release\bundle\nsis\SwiftFetch_0.1.0_x64-setup.exe'
   (Get-Item -LiteralPath $f).Length
   (Get-FileHash -LiteralPath $f -Algorithm SHA256).Hash.ToLowerInvariant()
   (Get-AuthenticodeSignature -LiteralPath $f).Status
   ```
3. Copy the file to the laptop, for example `C:\SwiftFetch-Test\SwiftFetch_0.1.0_x64-setup.exe`.
   Do not rename it in a way that hides its version or origin.
4. On the laptop, verify before running:
   ```powershell
   $f = 'C:\SwiftFetch-Test\SwiftFetch_0.1.0_x64-setup.exe'
   $expected = '2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454'
   $actual = (Get-FileHash -LiteralPath $f -Algorithm SHA256).Hash.ToLowerInvariant()
   if ($actual -ne $expected) { throw "Hash mismatch: $actual" }
   (Get-Item -LiteralPath $f).Length   # expect 3704490
   (Get-Item -LiteralPath $f).VersionInfo.ProductVersion  # expect 0.1.0
   (Get-AuthenticodeSignature -LiteralPath $f).Status     # expect NotSigned for this candidate
   ```
5. Keep the expected hash in a separate channel/file from the copied installer so a corrupted or
   replaced copy cannot validate itself. If any check fails, stop; do not run the file.
6. Do not disable SmartScreen or antivirus globally. If an isolated test requires a policy exception,
   the owner must authorize it explicitly and record the exact scope and duration.
7. Do not transfer or run the `artifacts/nsis-patch-test/*.exe` files; they are investigation outputs.

## 8. Second-laptop acceptance testing checklist

Record each row with date, Windows build/user type, commands, exit codes, hashes and screenshots. Use
`NOT_EXECUTED` until actually performed; do not mark `PASS` from this document.

| # | Check | Expected / action | Status |
|---|---|---|---|
| A1 | Laptop readiness | Windows 10/11 x64, fresh local user, WebView2 present or controlled network for bootstrap | `NOT_EXECUTED` |
| A2 | Pre-transfer hash and size | Source values match §1/§2 | `NOT_EXECUTED` |
| A3 | Post-transfer hash and size | Laptop values match §7; signature `NotSigned` recorded, not treated as pass | `NOT_EXECUTED` |
| A4 | Fresh install | Run NSIS installer; expect user-level elevation, `$LOCALAPPDATA\SwiftFetch`, shortcuts, Add/Remove entry | `NOT_EXECUTED` |
| A5 | Startup smoke | Launch UI; confirm render, `get_settings` over IPC, no crash dialog; record cold launch time | `NOT_EXECUTED` |
| A6 | Download fixture | Create one download against local fixture; verify completion and checksum | `NOT_EXECUTED` |
| A7 | Data location | Confirm `%APPDATA%\swiftfetch` state file exists and user data is outside install dir | `NOT_EXECUTED` |
| A8 | Queue/history/settings | Confirm settings and history persist across an app restart | `NOT_EXECUTED` |
| A9 | Uninstall preservation | Uninstall with data checkbox unchecked; confirm app removed and user data remains | `NOT_EXECUTED` |
| A10 | Known-build caveat | Do **not** validate corrected opt-in deletion with this binary; note historical build | `NOT_EXECUTED` |
| A11 | Evidence archive | Archive hashes, logs, screenshots and notes; keep NO-GO unchanged | `NOT_EXECUTED` |
| A12 | Stop conditions | Any hash mismatch, unexpected publisher, crash, or data loss stops testing and is reported | `NOT_EXECUTED` |

## 9. Boundaries preserved

No rebuild, source/config change, release, tag, signing, updater activation, SignPath submission,
dependency change, or website modification was made. NO-GO and `ready_for_public_release: false` remain
in force. This report is preparation only; it does not authorise installation on any machine.
