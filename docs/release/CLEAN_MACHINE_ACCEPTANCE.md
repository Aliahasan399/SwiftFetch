# SwiftFetch — Clean-machine acceptance instructions

**Status:** externally blocked (no clean Windows VM available in this environment).
Do not simulate or claim these results. Record real evidence in a result file per run.

## Environment
- Clean Windows 10/11 x64 VM, fresh user profile, WebView2 runtime present or installer provided.
- No SwiftFetch data, no browser extension registration, no native-host key.
- Isolated network where needed; fixture URLs permitted.

## Sequence (record pass/fail + artifact paths for each)
1. **Install** signed NSIS and MSI builds separately on fresh VMs.
   - Verify publisher signature (`Get-AuthenticodeSignature` = Valid, correct publisher).
   - Verify installed executable hash equals `SHA256SUMS` in the candidate folder.
   - Launch; confirm UI renders and `get_settings` works over real IPC.
2. **Data preservation on upgrade**
   - Add a download, create a SQLite backup, set a distinctive setting.
   - Install the next signed version over the existing one.
   - Confirm queue, history, partial files, settings, and encrypted headers survive.
   - Confirm no unrelated files are deleted.
3. **Signed update + automatic rollback**
   - Configure the real HTTPS update channel per `OWNER_CONFIG_GUIDE.md`.
   - Trigger a signed update to a healthy build; confirm health check passes.
   - Force a failed health check; confirm signed rollback restores the prior version **and** the bound metadata snapshot.
   - Test bad signature/hash, interrupted update, incompatible version, and unavailable endpoint.
4. **Uninstall preservation**
   - Uninstall; confirm user data (downloads, backups, settings) remains unless the user explicitly chose removal.
   - Confirm browser native-host registration and extension removal behavior.
5. **Browser distribution**
   - Install the extension per supported browser; confirm permissions rationale, native host handoff, site-rule consent, and removal.
   - Do not claim store approval without a store receipt.
6. **Startup/soak**
   - Record warm/cold launch timings from the instrumented build (Phase 3).
   - Run a 24-hour soak; record crashes, memory, and completion rates.

## Phase 2 additions — what can and cannot be concluded offline

The following checks were executed in this environment. They are **static inspection** or
**offline simulation** only and do not satisfy any item in the sequence above:

| Check | Tool / evidence | Result |
|---|---|---|
| MSI payload extraction + packaged runtime | `tools/verify-installer.ps1` → `artifacts/installer-2059e517624b4f02b37bcfd83f88b4ed/result.json` | Pass (14 smoke pass, 1 skip) |
| MSI/NSIS hash pinning | same, with `-ExpectedMsiSha256` / `-ExpectedNsisSha256` | Match baseline in `FINAL_BUILD_EVIDENCE.md` |
| Static upgrade/uninstall audit | `tools/audit-installer.ps1` → `artifacts/installer-audit/audit-20261122-093740.json` | 0 FAIL; 2 blockers; 6 warnings |
| Data-preservation simulation | `tools/installer-data-safety.ps1` → `artifacts/installer-data-safety/data-safety-20261122-093816.json` | 5/5 modelled scenarios preserve `swiftfetch\` data |
| Build input/hash recording | `tools/build-installers.ps1 -InspectOnly` → `artifacts/installer-builds/build-20261122-093740.json` | Recorded; no rebuild run |

These results are evidence that the installer sources are *consistent with* data preservation;
they are **not** proof that a real Windows machine behaves that way.

## Downgrade behaviour (must be tested on a clean VM)

Both installers currently permit downgrades, and this is a deliberate part of the existing
architecture rather than an accepted release risk:

- MSI: `main.wxs:40` — `<MajorUpgrade Schedule="afterInstallInitialize" AllowDowngrades="yes" />`
- NSIS: `installer.nsi:54` — `!define ALLOWDOWNGRADES "true"`

Required clean-VM checks before release:

1. Install version N, then install an older version over it with both MSI and NSIS.
2. Confirm the older version is actually installed (not silently refused).
3. Confirm downloads, settings, history, queues and partial files survive the downgrade.
4. Confirm a downgrade does not create two Add/Remove Programs entries or a broken shortcut.
5. Record whether the app's SQLite schema is compatible with the older binary; if it is not,
   this is a release blocker and downgrades must be disabled in both installers.

## Per-user / per-machine assumption (must be verified)

- MSI is authored `InstallScope="perMachine"` (`main.wxs:28`) and installs under Program Files.
- NSIS is generated for the configured install mode; the source contains per-machine
  (`$PROGRAMFILES`) and per-user (`$LOCALAPPDATA`) branches (`installer.nsi:536-537`), and
  elevates only in the per-machine branch (`installer.nsi:101-105`).
- Runtime user data resolves to `%APPDATA%\swiftfetch\state.sqlite` (`lib.rs:60-62`).

Required clean-VM checks: verify both scopes separately, verify that `AppData\Roaming\swiftfetch`
is writable for a standard (non-elevated) user, and confirm uninstall in one scope does not
affect data written by the other.

## Opt-in app-data deletion mismatch (known defect, documented not fixed)

NSIS deletes app data only when the user ticks "Delete app data" and the install is not in
update mode (`installer.nsi:863-879`). It removes `$APPDATA\com.swiftfetch.app` and
`$LOCALAPPDATA\com.swiftfetch.app`, but the app writes to `%APPDATA%\swiftfetch`. The opt-in
cleanup therefore does **not** remove the real data directory. Preserve-by-default still holds,
and MSI authors no app-data removal at all, but the opt-in behaviour must be corrected or
explicitly accepted before release. This change was out of scope for Phase 2 installer tooling.

## Failed-upgrade rollback requirements

A release cannot be approved until a real failed-upgrade rollback test demonstrates all of:

1. The previous application version is restored, not just the binary but its start menu,
   desktop shortcuts and Add/Remove Programs entry.
2. The bound metadata snapshot (settings, queue, history, SQLite state) is restored to the
   pre-upgrade bytes, verified by hash.
3. Any partially written files from the failed upgrade are cleaned up without touching the
   user's downloads directory.
4. A second attempt to install the same failing upgrade does not leave the machine in a worse
   state (idempotent failure handling).
5. The exact commands, version numbers, hashes and screenshots are archived as a
   `clean-machine-result.json` companion to this document.

## Blocking status
Until this document has a companion `clean-machine-result.json` with real signatures and hashes,
Phase 4 clean-machine acceptance remains **externally blocked / NO-GO**. The offline checks above
reduce uncertainty about installer intent; they do not change that verdict.
