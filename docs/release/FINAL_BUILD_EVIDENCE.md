# SwiftFetch — Final Build & Repackage Evidence

**Version:** 0.1.0 · **Date:** 2026-11-21 · **Repo:** `E:\CloneIdm\swiftfetch` (not a git repo; source snapshot hash used instead of a commit).

Scope of this file: evidence for the two operations that were interrupted and then finished —
(1) the final Windows release build and (2) the final candidate/source repackaging.
Verification was already complete before this work and was **not** re-run. No source, config,
schema, or signing/updater changes were made. The GO / NO-GO verdict remains **NO-GO**.

## 1. Build (fresh, completed)

- Driver: `tools/build-windows.ps1`, launched in background by the parent session.
- Log: `E:\CloneIdm\swiftfetch\artifacts\final-build.log` (captured from the build wrapper).
- Effective Tauri command recorded in the log:
  `npm run tauri build -- --ci --config E:\CloneIdm\swiftfetch\artifacts\candidate-5a9647650deb4d1a979a095280fcf04e\tauri.json -- --locked`
- Build overlay config (`artifacts/candidate-5a9647650deb4d1a979a095280fcf04e\tauri.json`) only redirects
  the frontend output to a fresh candidate directory; it does **not** change product name or version.
  No update channel or updater artifacts were configured.

**Exit-code note (do not treat as build failure):** the parent session reported build-shell `exit=1`.
This is the known PowerShell stderr-redirection false error: `npm.cmd` writes an informational line
("Looking up installed tauri packages") to stderr, and under `$ErrorActionPreference = 'Stop'` with
`*>` redirection PowerShell treats it as a terminating error. The build itself completed — all three
fresh outputs exist below with post-build timestamps and consistent metadata/hashes. A fix for the
wrapper script is owned by the parent session and is outside this evidence lane's edit scope.

### Built artifacts (measured directly from disk)

| Artifact | Bytes | Modified (local) | SHA-256 | Authenticode |
|---|---:|---|---|---|
| `src-tauri/target/release/swiftfetch.exe` | 10,375,680 | 2026-11-21T19:00:01+06:00 | `eebb578832936b29dc73e97e65178a942d026c719bed1045379180bec596fceb` | `NotSigned` |
| `src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi` | 6,283,264 | 2026-11-21T18:59:12+06:00 | `58df1a1410cd43fb7480e08bb708b5e9f71d7db17c224c1f481907a92d588f32` | `NotSigned` |
| `src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe` | 3,704,490 | 2026-11-21T19:00:00+06:00 | `2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454` | `NotSigned` |

Only these two bundle files are present under `src-tauri/target/release/bundle` (MSI and NSIS).
No updater/signature `.sig` artifacts were produced.

### Version consistency

| Source | Value |
|---|---|
| `package.json` → `version` | `0.1.0` |
| `src-tauri/Cargo.toml` → `version` | `0.1.0` |
| `src-tauri/tauri.conf.json` → `version` / `productName` / `identifier` | `0.1.0` / `SwiftFetch` / `com.swiftfetch.app` |
| Built `swiftfetch.exe` VersionInfo | `FileVersion=0.1.0`, `ProductVersion=0.1.0`, `ProductName=SwiftFetch` |
| Built NSIS installer VersionInfo | `FileVersion=0.1.0`, `ProductVersion=0.1.0`, `ProductName=SwiftFetch` |
| Bundle filenames | `SwiftFetch_0.1.0_x64_en-US.msi`, `SwiftFetch_0.1.0_x64-setup.exe` |

All version sources agree at `0.1.0`.

## 2. Final candidate repackage

Command:
`tools/package-candidate.ps1 -BrowserPackageDirectory artifacts\browser-extension\fb64b5c3-88e9-4057-a47b-857a6438c56d -NoticesDirectory artifacts\dependencies\notices-a1587a6b-6e23-4a51-bf9b-493b0294a934`

The browser package directory and the dependency-notices directory were selected as the newest of
their respective kinds (browser package dir modified 2026-11-21 09:01:09; notices dir modified
2026-11-21 09:02:11, containing `dependencies.json`, `sbom.spdx.json`, `THIRD-PARTY-NOTICES.md`).

- Output: `E:\CloneIdm\swiftfetch\artifacts\SwiftFetch-0.1.0-candidate-e79ab68b63584b42ae77df98aa1e9e19`
- `candidate-manifest.json`: `product=SwiftFetch`, `version=0.1.0`, `kind=private-release-candidate`,
  **`ready_for_public_release=false`** (reported by PowerShell as `False`).
- File count: **626** manifest entries; `SHA256SUMS` has **626** lines.
- Independent recheck: re-hashed every one of the 626 packaged files — **0 missing, 0 hash mismatches**.
- Copied binaries retain their original hashes and `NotSigned` status (see section 1).
- Browser previews included: `SwiftFetch-chromium-0.1.0-preview.zip`
  (`7559c81eac2ddb9bbb8aa3ae0349cd8e70f0d8a9453b23fa6b588d66560e6b09`, 26,864 bytes) and
  `SwiftFetch-firefox-0.1.0-preview.zip`
  (`7f150fb919de1a5b3eca5543d053cc4b0aa4cd6ab5b59860038d89762877cb14`, 26,924 bytes).
- Third-party notices included under `notices/`: `dependencies.json`, `sbom.spdx.json`,
  `THIRD-PARTY-NOTICES.md`, plus the per-package license files.

## 3. Final source repackage

Command: `tools/package-source.ps1`

- Output: `E:\CloneIdm\swiftfetch\artifacts\source\0.1.0-ab544367991d49518258baa9ffbbe97d\SwiftFetch-0.1.0-source.zip`
- Size: 632,052 bytes · Modified: 2026-11-21T19:04:32+06:00
- SHA-256: `0efc8ceb83bed247f9218688d69e28194dc79d8ef69576007af0eb49461b0c71`
- `SHA256SUMS` in the same directory matches the measured hash exactly.
- Archive contains 184 entries under `SwiftFetch-source/`, including `package.json`,
  `src-tauri/Cargo.toml`, `tools/build-windows.ps1`, and `docs/release/GO_NO_GO_REPORT.md`.
- The script's allowlist excludes databases, downloads, keys, previous bundles and `*.bak-*` files.

## 4. Verification status (not re-run)

Canonical verification had already completed before this build:
`tools/verify.ps1` exit 0; `artifacts/verification/report-20261121-184958.json` with
`success: true` and all 7 checks; full log `artifacts/final-verify.log`.
Those artifacts were left untouched. This lane did not claim any new test run, signing,
updater configuration, or clean-machine/external testing.

## 5. Ordering note

The candidate and source snapshots above were created **before** this evidence file and the
final artifact-location update to `docs/release/GO_NO_GO_REPORT.md` were written. The copies of
`docs/release/` inside those snapshots therefore reflect the pre-update state; the authoritative,
up-to-date release decision and paths are the files in the working tree under `docs/release/`.
The `GO_NO_GO_REPORT.md` edit was preceded by the timestamped backup
`docs/release/GO_NO_GO_REPORT.md.bak-20261121-190606`.

## 6. Verdict

Unchanged: **NO-GO**. The binaries remain unsigned (`NotSigned`), no HTTPS update channel or signed
updater manifests are configured, and no clean-machine install/upgrade/rollback/uninstall or
screen-reader/soak evidence exists. Do not set `ready_for_public_release: true`.
