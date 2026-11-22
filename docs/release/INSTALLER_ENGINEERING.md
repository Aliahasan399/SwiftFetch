# SwiftFetch — Installer Engineering (Phase 2)

**Version:** 0.1.0 · **Date:** 2026-11-22 · **Status:** NOT RELEASE READY (NO-GO preserved)

This document records the current installer architecture, the evidence collected during
Phase 2, and the exact boundary between what was verified here and what still requires a real
Windows VM. It does not publish, sign, or enable updates.

## 1. Installer technologies in use

| Item | Value | Source |
|---|---|---|
| Bundler | Tauri 2 built-in bundler | `src-tauri/tauri.conf.json:33-43` (`"targets": "all"`) |
| MSI | WiX-generated, per-machine | `src-tauri/target/release/wix/x64/main.wxs` |
| NSIS | Tauri NSIS script, install mode configurable | `src-tauri/target/release/nsis/x64/installer.nsi` |
| Identifier | `com.swiftfetch.app` (unchanged) | `tauri.conf.json:5` |
| UpgradeCode | `1b0348ae-0c8d-5327-8c38-b0e4d61e9ff1` (unchanged) | `main.wxs:18` |
| Update channel | `not_configured` (`pubkey` empty, `requireSignedVersion: true`) | `tauri.conf.json:31` |

No new installer framework, dependency, or paid service was introduced in Phase 2.
`Cargo.toml`, `Cargo.lock`, `package.json` and `package-lock.json` were not modified.

## 2. Current artifacts and hashes

| Artifact | Bytes | SHA-256 | Authenticode |
|---|---:|---|---|
| `src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi` | 6,283,264 | `58df1a1410cd43fb7480e08bb708b5e9f71d7db17c224c1f481907a92d588f32` | `NotSigned` |
| `src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe` | 3,704,490 | `2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454` | `NotSigned` |

These match `docs/release/FINAL_BUILD_EVIDENCE.md` and were re-measured by
`tools/build-installers.ps1 -InspectOnly` and `tools/verify-installer.ps1`.

## 3. Upgrade, downgrade and uninstall analysis

### 3.1 MSI (WiX)

- `MajorUpgrade Schedule="afterInstallInitialize"` with `AllowDowngrades="yes"` (`main.wxs:40`).
  On upgrade the previous product is removed during InstallInitialize, before new files are laid
  down. On downgrade the same path runs with a lower version.
- `REINSTALLMODE="amus"` (`main.wxs:33`) reinstalls files, registry entries and shortcuts.
- `INSTALLDIR` is resolved by registry search, NSIS key first and MSI `InstallDir` second
  (`main.wxs:55-65`), preserving the chosen install location across reinstalls.
- Install scope is `perMachine`; `INSTALLDIR` is under Program Files (`main.wxs:28,101-103`).
- Uninstall removes the INSTALLDIR tree and shortcuts (`main.wxs:43,128-129`). No app-data
  removal is authored, so `%APPDATA%\swiftfetch` should survive upgrade, downgrade and uninstall.
- Shortcut creation and an uninstall shortcut are present (`main.wxs:94-99,120-137,140-153`).
- WebView2 bootstrap is invoked only when no installed WebView2 version is detected
  (`main.wxs:191-204`). A clean-VM test must confirm this works on an offline-isolated image.

### 3.2 NSIS

- `ALLOWDOWNGRADES "true"` (`installer.nsi:54`); the reinstall page offers "don't uninstall"
  on a downgrade (`installer.nsi:262-265`). Downgrade behaviour is therefore permitted and
  must be tested, not assumed.
- In update mode the previous install is not removed (`installer.nsi:817-844` guards shortcut
  deletion with `$UpdateMode <> 1`).
- Per-machine installs use `$PROGRAMFILES\SwiftFetch`; per-user installs use
  `$LOCALAPPDATA\SwiftFetch` (`installer.nsi:530-538`). Elevation is requested only for the
  per-machine mode (`installer.nsi:101-105`).
- Uninstall deletes the install directory and shortcuts, then removes registry keys
  (`installer.nsi:799-861`).
- App data is deleted **only** when the "Delete app data" checkbox is selected AND the run is
  not an update (`installer.nsi:863-879`). By default all user data is preserved.
- Known defect: the cleanup targets `$APPDATA\com.swiftfetch.app` and
  `$LOCALAPPDATA\com.swiftfetch.app`, while runtime data lives at `%APPDATA%\swiftfetch`
  (`lib.rs:60-62`). The opt-in deletion does not remove the real data directory. It is
  documented here and not changed in Phase 2.

### 3.3 Data separation conclusion

The application database path resolution is: `SWIFTFETCH_DB_PATH` → `SWIFTFETCH_DATA_DIR` →
`%APPDATA%/swiftfetch/state.sqlite` → `%USERPROFILE%/.swiftfetch/state.sqlite`
(`lib.rs:44-64`). All of these are outside the installer-controlled Program Files / LocalAppData
install directories, so both installers **should** preserve user data. This is a design
conclusion from source inspection, not a clean-machine result.

## 4. Tools added or extended in Phase 2

| Script | Purpose | Execution mode |
|---|---|---|
| `tools/build-installers.ps1` | Single auditable build entry point; records toolchain, lockfiles, artifact hashes; refuses to build if update-channel env vars are set; `-InspectOnly` audits artifacts without rebuilding | build / inspect |
| `tools/verify-installer.ps1` | MSI administrative extraction + packaged runtime smoke test; NSIS metadata + hash check; optional pinned expected hashes | real extraction, no install |
| `tools/audit-installer.ps1` | Static WiX/NSIS/lib.rs audit against the accepted baseline, hash verification, finding list | static only |
| `tools/installer-data-safety.ps1` | Isolated synthetic `%APPDATA%` simulation of MSI upgrade/uninstall and NSIS update/uninstall scenarios | offline simulation |

Backups were taken before editing existing files:
`tools/verify-installer.ps1.bak-20261122-093603` and
`docs/release/CLEAN_MACHINE_ACCEPTANCE.md.bak-20261122-093603`.

## 5. Reproducibility status

Reproducible builds are **not claimed**. The build wrapper pins the Rust toolchain in CI
(1.98.0), uses `npm ci` plus `Cargo --locked`, and generates a fresh frontend directory per run,
but the following gaps remain:

- no `SOURCE_DATE_EPOCH` set by default (timestamps are embedded in artifacts);
- the frontend output directory name includes a GUID, which can be embedded in build output;
- GitHub runner image (`windows-2022`) and Node major (`24`) are floating;
- deterministic NSIS/WiX timestamp suppression is not configured;
- no two independent builds have produced byte-identical hashes.

`tools/build-installers.ps1` records the inputs (`rustc -V`, `cargo -V`, `node -v`, `npm -v`,
git HEAD, `SOURCE_DATE_EPOCH`, updater env state) needed for a future reproducibility attempt.
`SOURCE_DATE_EPOCH` can be supplied via `-SourceDateEpoch` or the environment.

## 6. Failed-upgrade rollback requirements

A release cannot be approved until a real failed-upgrade rollback test proves:

1. prior binary, shortcuts and Add/Remove Programs entry are restored;
2. the bound metadata snapshot (settings, queue, history, SQLite state) is restored to
   pre-upgrade bytes and verified by hash;
3. partial files from the failed upgrade are removed without touching the downloads directory;
4. repeating the failing upgrade is idempotent and does not degrade machine state;
5. commands, versions, hashes and screenshots are archived as `clean-machine-result.json`.

See `docs/release/CLEAN_MACHINE_ACCEPTANCE.md` for the full clean-VM sequence and blockers.

## 7. Verdict

**NO-GO.** Installers remain unsigned, the update channel is unconfigured, no clean-VM
install/upgrade/downgrade/uninstall/rollback evidence exists, and reproducible builds are not
proven. The Phase 2 tools improve auditability only. Do not set
`ready_for_public_release: true`, create a release/tag, publish installers, or configure
automatic updates.
