# SWIFTFETCH_DEVELOPMENT_BUILD_REPORT.md

**Date:** 2026-11-22 · **Type:** fresh unsigned development build for second-laptop testing
**NO-GO preserved.** Not a production release; no publication, signing, updater or tag.

## 1. Build command and result

`./tools/build-installers.ps1` → exit 0. It ran `tools/build-windows.ps1` (`npm run tauri build --ci … -- --locked`),
then `tools/patch-nsis-cleanup.ps1`, then `tools/rebuild-nsis-installer.ps1`.
Build evidence: `artifacts/installer-builds/build-20261123-142728.json`;
NSIS rebuild evidence: `artifacts/nsis-rebuild/nsis-20261123-144008/rebuild-result.json`.
Final canonical verification after this build: `artifacts/verification/report-20261123-142607.json` (8/8).

Frontend build inside the installer: `index-CM-YCWGD.js` 352,728 B,
SHA `38cca5ebc12103a2430501b43db81d3783d442bb0ab4cb14d33a3afc716fa6dd`;
`index-FvquKpkj.css` 27,327 B, SHA `8000f8a0cb488b7be4d2b3ce972f936ab778c3f96b3936d6eb915279ca70b773`.

## 2. Development artifacts (exact copies)

All staged under `E:\CloneIdm\swiftfetch\artifacts\dev-build\`:

| Artifact | Bytes | SHA-256 | Version | Architecture | Signature |
|---|---:|---|---|---|---|
| `SwiftFetch_0.1.0_x64-setup.exe` (NSIS) | 3,755,344 | `0a8e8253e61a7eadb55e49be95d6242c646421692eb989dbddf173963446fac5` | FileVersion 0.1.0 | x64 payload (32-bit NSIS stub) | `NotSigned` |
| `SwiftFetch_0.1.0_x64_en-US.msi` | 6,352,896 | `53d449e53f6ccf67b930ee93f4a26d3478afb0651251d9082c9078f0f5eea096` | 0.1.0 | x64 | `NotSigned` |
| `swiftfetch.exe` | 10,529,792 | `345220921fd59c8b966146f358ec92c786c2d2cc87a4c0cd8de24f6ac7e23fe0` | 0.1.0 | AMD64 (x64) | `NotSigned` |
| `swiftfetch_lib.dll` | 2,032,128 | `363790e289b8aea8d18ffa2d0d59aa8c2f86fd316096dfd95d41268f21` | — | x64 | `NotSigned` |

Historical installer hashes were backed up to `artifacts/historical-installers-backup/` before this build
and remain documented in `INSTALLER_HASH_BASELINE.md`. Do not use those hashes for this development build.

## 3. Installer content verification

`tools/verify-installer.ps1 -Msi artifacts/dev-build/... -Nsis artifacts/dev-build/...` was attempted, but
the extracted WebView intermittently rendered a blank document (`title:""`, no scripts) on four consecutive
attempts. This is recorded as **BLOCKED/PARTIAL**, not a PASS. A direct smoke run of the staged
`artifacts/dev-build/swiftfetch.exe` succeeded on the third attempt with **14 pass, 1 skip**
(`artifacts/smoke/run-nyhKSs/result.json`), covering native handoff, replay guard, restart recovery and
update-health handshake. MSI-contained `swiftfetch_lib.dll` hash matches the staged DLL;
the extracted `swiftfetch.exe` differs from the staged standalone EXE only because Tauri patches
bundle metadata per bundle type, and its version is 0.1.0.

NSIS cleanup: `installer.nsi:880` contains `RmDir /r "$APPDATA\swiftfetch"`; no `LOCALAPPDATA`
recursive removal exists. The built installer was produced by `rebuild-nsis-installer.ps1` from that
patched script. Actual uninstall data behavior must be confirmed on the second laptop.

## 4. Install / uninstall check (previous development build)

This check was performed for the earlier development build, not repeated for the latest unsigned
artifacts. Controlled silent install into `artifacts/install-test/SwiftFetch`:
- `install_exit=0`; `swiftfetch.exe` and `uninstall.exe` present (NSIS installs only the standalone EXE).
- The installed standalone EXE passed the full smoke suite (14 pass, 1 skip) without the DLL.
- `uninstall_exit=0`; install directory removed.

## 5. Native Messaging development registration

`browser-extension/tools/register-host.ps1 -Browser Chrome -ExecutablePath <dev exe> -AllowUnsignedPreview`
succeeded with a temporary test extension ID and manifest directory. It wrote the expected per-user
registry key and manifest pointing at the development EXE. The test registration and manifest were
removed afterward; no host registration was left behind on this machine.

## 6. Development extension packages

Generated with `browser-extension/tools/package.mjs` into
`artifacts/dev-build/browser-extensions/{chromium,firefox}`. Key hashes:
`content.js` `e963fce891fca056ac797d0c6d10aeb2e979cc73597821c4635573b9a2748aa4`,
`chromium/background.js` `112a40bccc97e395017a5cfa1e87ea1a9a77d571ce20da12eee48268239233c8`,
`firefox/background.js` `38179f8fb33941fbee9e3f81de6a9e66850baa86675f0a577dd60d7ef0f0f41b`,
`protocol.bundle.js` `cb88d3e29e23c304513268bd890a81406e443fa2bfdf6819c0a46ef5e722288a`,
`media-indicator.bundle.js` `6146298cbb42e8b44598dd1d39141380e81f3153dcf6ccca1d734dc27fad22fe`.

## 7. Runtime prerequisites

Windows 10/11 x64; Microsoft Edge WebView2 runtime (NSIS downloads bootstrapper if absent); local
SQLite state under `%APPDATA%\swiftfetch`; no account or hosted service required.

## 8. Limitations

Unsigned build triggers SmartScreen/Unknown Publisher warnings. Firefox was unavailable for real-browser
testing. Real-browser extension/native-host matrix is the owner's second-laptop task. HLS/DASH support
is limited to static, unencrypted, non-live fixtures. NO-GO and `ready_for_public_release: false` remain.
