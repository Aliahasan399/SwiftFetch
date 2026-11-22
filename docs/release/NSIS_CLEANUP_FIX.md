# SwiftFetch — NSIS opt-in app-data cleanup fix (Task 1)

**Date:** 2026-11-22 · **Status:** source corrected and offline-tested; real VM test still required
**NO-GO preserved.** No installer was published, signed, or released.

## 1. Problem

Tauri's generated NSIS uninstaller offered an opt-in "Delete app data" checkbox, but its
recursive removals targeted `$APPDATA\com.swiftfetch.app` and `$LOCALAPPDATA\com.swiftfetch.app`
(`installer.nsi:877-878`). SwiftFetch's runtime data is resolved as
`%APPDATA%\swiftfetch\state.sqlite` (`src-tauri/src/lib.rs:60-62`), so the opt-in delete did not
remove the real data directory. The second removal also had the potential to delete an unrelated
`LOCALAPPDATA\com.swiftfetch.app` tree belonging to something else.

## 2. Correction applied

| Item | Before | After |
|---|---|---|
| Roaming removal | `RmDir /r "$APPDATA\${BUNDLEID}"` | `RmDir /r "$APPDATA\swiftfetch"` |
| Local removal | `RmDir /r "$LOCALAPPDATA\${BUNDLEID}"` | removed entirely |
| Recursive removals in script | 2 | 1 |
| Guard | checkbox `= 1` AND `$UpdateMode <> 1` | unchanged |

Data is still preserved by default. Deletion requires the user to tick the checkbox and must not
be an update. No `LOCALAPPDATA` tree is removed, so unrelated user files are not targeted.
Only the known SwiftFetch roaming data directory is removed on explicit consent.

## 3. Files changed / added

- Patched generated source: `src-tauri/target/release/nsis/x64/installer.nsi`
  (SHA-256 before `aa0e5568dfec5b6f706c9335ae71cca82d6752e2ae9c4aa7477823d7fc652687`,
  after `7f1d4063c22d33852eb19276cfa943f8bb1484f753b8dffee50b063c9878b2fe`).
- Backup: `artifacts/evidence-backups/installer.nsi.bak-20261122-100208`.
- Added `tools/patch-nsis-cleanup.ps1` — idempotent post-build patch with backup and guards.
- Added `tools/rebuild-nsis-installer.ps1` — recompiles NSIS from patched source.
- Updated `tools/build-installers.ps1` — runs patch + recompile after a real build, so the fix
  survives Tauri's regeneration of `installer.nsi` on every build.
- Updated `tools/audit-installer.ps1` — now expects the corrected path and fails on any
  `LOCALAPPDATA` recursive removal.
- Rewrote `tools/installer-data-safety.ps1` — asserts both preserve-by-default and explicit
  deletion, plus unrelated-file integrity.

## 4. Exact test results

### 4.1 Patch tool
- `./tools/patch-nsis-cleanup.ps1` → `patched`;
  `artifacts/installer-nsis-patch/patch-20261122-100208.json`.
- Re-run → `already-patched` (idempotent);
  `artifacts/installer-nsis-patch/patch-20261122-100219.json`,
  `artifacts/installer-nsis-patch/patch-20261122-100234.json`.
- Post-conditions verified by the tool: exactly one `RmDir /r`, target `$APPDATA\swiftfetch`,
  no old path, no `LOCALAPPDATA` removal, both opt-in and not-update guards present.

### 4.2 Recompile with makensis v3.13
| Build | Bytes | SHA-256 |
|---|---:|---|
| Control (unpatched script), compressed | 3,703,402 | `fac1934cb95c7abe092b3c2d9bfcae64044394c02ea6f5d1b1d3bcb6dc796e96` |
| Patched script, compressed | 3,703,351 | `1ba33f1fb97862caee4b16828f2dbfbfa7f6c69384046f1a19b97f80416d70ba` |
| Control, uncompressed test build | 10,709,646 | `572aa7103db770b0730cbfbc8bddd4ad8b737a45878c5fb61efec45d6e1bdf70` |
| Patched, uncompressed test build | 10,709,558 | `d3eb0ca5d6330f826e43d983b44adf4e2b71df7ec1793f1f96e823b611f09656` |

Both scripts compiled with exit code 0 and the same single pre-existing warning
(`LangString "choowHowToInstall" is not set`, NSIS warning 6040). Rebuild evidence:
`artifacts/nsis-rebuild/nsis-20261122-100234/rebuild-result.json`.
The test installers were written only under `artifacts/nsis-patch-test/`; the historical
release artifact was not overwritten.

Plain string inspection of the compiled installer is inconclusive because the NSIS payload is
LZMA-compressed and `com.swiftfetch.app` also appears in the embedded application resources.
The behavioural assertion therefore rests on the static script check plus the simulation below.

### 4.3 Offline data simulation
`./tools/installer-data-safety.ps1` →
`artifacts/installer-data-safety/data-safety-20261122-100440.json`.

| Scenario | Expectation | Result | Unrelated files intact |
|---|---|---|---|
| MSI major upgrade | preserve | PASS | PASS |
| MSI uninstall | preserve | PASS | PASS |
| NSIS update mode | preserve | PASS | PASS |
| NSIS uninstall, checkbox unchecked | preserve | PASS | PASS |
| NSIS uninstall, checkbox checked | delete only `Roaming\swiftfetch` | PASS | PASS |

The synthetic tree also included `Roaming\com.swiftfetch.app`, `Roaming\OtherApp` and
`Local\swiftfetch` as unrelated data; all three were byte-identical after every scenario.

### 4.4 Static audit after fix
`./tools/audit-installer.ps1` → `artifacts/installer-audit/audit-20261122-100440.json`:
20 findings, 0 FAIL, 2 BLOCKER (clean VM, rollback), 5 WARN.
`NSIS-DATA-DELETE-PATH`, `NSIS-LOCALAPPDATA-REMOVAL`, `NSIS-DATA-DELETE-GUARD` and
`DATA-PATH-ALIGNED` are all PASS. A WARN records that the historical unsigned NSIS artifact
predates this fix.

## 5. Limitation

None of the above executed a real install or uninstall. Whether Windows actually permits
removal of `%APPDATA%\swiftfetch` under the uninstaller's integrity context, and whether any
file is locked by a running process, remains a clean-VM acceptance item. The existing unsigned
NSIS installer is historical evidence only; a rebuilt installer is required before release.
