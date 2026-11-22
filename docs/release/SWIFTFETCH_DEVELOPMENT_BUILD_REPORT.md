# SWIFTFETCH_DEVELOPMENT_BUILD_REPORT.md

**Date:** 2026-11-22 · **Type:** fresh unsigned development build for second-laptop testing
**NO-GO preserved.** Not a production release; no publication, signing, updater or tag.

## 1. Build command and result

`./tools/build-installers.ps1` → exit 0. It ran `tools/build-windows.ps1` (`npm run tauri build --ci … -- --locked`),
then `tools/patch-nsis-cleanup.ps1`, then `tools/rebuild-nsis-installer.ps1`.
Build evidence: `artifacts/installer-builds/build-20261122-184015.json`;
NSIS rebuild evidence: `artifacts/nsis-rebuild/nsis-20261122-184628/rebuild-result.json`.
Final canonical verification after this build: `artifacts/verification/report-20261122-192556.json` (8/8).

Frontend build inside the installer: `index-CM-YCWGD.js` 352,728 B,
SHA `38cca5ebc12103a2430501b43db81d3783d442bb0ab4cb14d33a3afc716fa6dd`;
`index-FvquKpkj.css` 27,327 B, SHA `8000f8a0cb488b7be4d2b3ce972f936ab778c3f96b3936d6eb915279ca70b773`.

## 2. Development artifacts (exact copies)

All staged under `E:\CloneIdm\swiftfetch\artifacts\dev-build\`:

| Artifact | Bytes | SHA-256 | Version | Architecture | Signature |
|---|---:|---|---|---|---|
| `SwiftFetch_0.1.0_x64-setup.exe` (NSIS) | 3,752,846 | `4d05c3491df93657b21e39ba6735288c05f42f342c90dc24f9419e1d4a186e81` | FileVersion 0.1.0 | x64 payload (32-bit NSIS stub) | `NotSigned` |
| `SwiftFetch_0.1.0_x64_en-US.msi` | 6,352,896 | `ed08fd0a2693519cf8c3ab6adef6da6ebbe5ec952b87932e3454f61219968408` | 0.1.0 | x64 | `NotSigned` |
| `swiftfetch.exe` | 10,527,232 | `2693341a98952ee5ae14c2f5c9724fc2b4579a223b623ffa94b8524f3732913a` | 0.1.0 | AMD64 (x64) | `NotSigned` |
| `swiftfetch_lib.dll` | 2,032,128 | `199e75b13a1f9828505de063e8743f538fb33cf60681783051282d592ca94aad` | — | x64 | `NotSigned` |

Historical installer hashes were backed up to `artifacts/historical-installers-backup/` before this build
and remain documented in `INSTALLER_HASH_BASELINE.md`. Do not use those hashes for this development build.

## 3. Installer content verification

`tools/verify-installer.ps1 -Msi artifacts/dev-build/... -Nsis artifacts/dev-build/...` → exit 0.
Administrative MSI extraction at `artifacts/installer-ebc5d2ca485946d7b2cc84fe6d2a2a4d`; packaged
smoke result `artifacts/smoke/run-e1U7nY/result.json`: **14 pass, 1 skip**. The smoke includes native
handoff, replay guard, upgrades/restart recovery and update-health handshake, confirming the latest
backend and frontend are present. MSI-contained `swiftfetch_lib.dll` hash matches the staged DLL;
the extracted `swiftfetch.exe` differs from the staged standalone EXE only because Tauri patches
bundle metadata per bundle type, and its version is 0.1.0.

NSIS cleanup: `installer.nsi:880` contains `RmDir /r "$APPDATA\swiftfetch"`; no `LOCALAPPDATA`
recursive removal exists. The built installer was produced by `rebuild-nsis-installer.ps1` from that
patched script. Actual uninstall data behavior must be confirmed on the second laptop.

## 4. Install / uninstall check (this machine)

Controlled silent install into `artifacts/install-test/SwiftFetch`:
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
