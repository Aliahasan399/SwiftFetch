# SwiftFetch — MSI and NSIS acceptance test procedure

**Version:** 0.1.0 · **Date:** 2026-11-22 · **Status:** PROCEDURE ONLY — no VM execution in this environment
**Verdict remains NO-GO.** This document specifies tests; it does not report that they passed.

## 1. Status vocabulary (use this in every result record)

| Label | Meaning |
|---|---|
| `SIMULATED` | Automated offline/static check already executed here. See §2. Not a VM test. |
| `NOT_EXECUTED` | Requires a real Windows VM. No result exists yet. Do not mark PASS. |
| `PASS` / `FAIL` | Reserved for an actually executed VM step with archived evidence. |
| `BLOCKED` | Cannot be performed until a named blocker is removed (certificate, VM, endpoint). |

## 2. What has actually been executed here (SIMULATED only)

| Check | Tool / evidence | Status |
|---|---|---|
| MSI administrative extraction + packaged runtime smoke | `tools/verify-installer.ps1` → `artifacts/installer-2059e517624b4f02b37bcfd83f88b4ed/result.json` | `SIMULATED` |
| NSIS metadata + hash | same script | `SIMULATED` |
| Static WiX/NSIS upgrade/uninstall audit | `tools/audit-installer.ps1` → `artifacts/installer-audit/audit-20261122-100440.json` | `SIMULATED` |
| Data preservation / opt-in deletion / unrelated-file guard | `tools/installer-data-safety.ps1` → `artifacts/installer-data-safety/data-safety-20261122-100440.json` | `SIMULATED` |
| NSIS cleanup patch compile | `docs/release/NSIS_CLEANUP_FIX.md` §4 | `SIMULATED` |
| Fresh install, upgrade, downgrade, startup, uninstall, rollback on a VM | — | `NOT_EXECUTED` |

None of the `SIMULATED` rows satisfies a `PASS` in the matrices below.

## 3. Environment prerequisites

- Two clean Windows 10/11 x64 VMs per installer (one per scenario group), fresh user profile.
- WebView2 runtime present, or the installer's bootstrapper permitted with recorded output.
- Isolated network; local HTTP fixture server allowed (`tools/fixtures/`). No external downloads
  except the explicit WebView2 bootstrap path, which must be recorded.
- Signed installers and their generated `SHA256SUMS` (see `INSTALLER_HASH_BASELINE.md`).
  Until signed artifacts exist, every install step is `BLOCKED`.
- A pre-agreed baseline data set: a completed download, a partial download, a queue entry, a
  history entry, a distinctive setting, and an encrypted-header fixture.

## 4. Shared evidence to record for every executed step

- VM identifier, Windows build, user account type (admin/standard), install scope.
- Installer path, SHA-256, `Get-AuthenticodeSignature` output (status, publisher, chain).
- Exact command line, start/end timestamps, exit code, screenshot, installer log path.
- Hashes of `%APPDATA%\swiftfetch\state.sqlite` and all fixture files before and after.
- Result row in `clean-machine-result.json` with `status`, `evidence_paths` and `notes`.

## 5. MSI matrix

| # | Scenario | Steps | Expected result | Status |
|---|---|---|---|---|
| M1 | Fresh install, per-machine | Run signed MSI with `/qn` and a verbose log; confirm exit 0/3010; inspect Program Files install dir, Start menu, desktop shortcut, Add/Remove entry, publisher | Installed; signed publisher shown; no unexpected files | `NOT_EXECUTED` |
| M2 | Fresh startup | Launch from Start menu; confirm UI renders, `get_settings` works over real IPC, WebView2 present; record cold launch time | UI and IPC healthy, no crash dialog | `NOT_EXECUTED` |
| M3 | Upgrade N → N+1 with data | Seed baseline data; run next signed MSI over install; re-check DB/settings/history/queue/partial hashes; confirm one Add/Remove entry | All data preserved; shortcuts intact; single product entry | `NOT_EXECUTED` |
| M4 | Downgrade N+1 → N | Install older signed MSI over newer; confirm older version actually installed; re-check data and schema compatibility | Either clean downgrade with data intact, or explicit refusal — no corrupt state | `NOT_EXECUTED` |
| M5 | Uninstall, default | Uninstall via Add/Remove; confirm install dir and shortcuts gone; confirm `%APPDATA%\swiftfetch` and downloads remain | Data preserved; no unrelated deletion | `NOT_EXECUTED` |
| M6 | Uninstall, repair/modify absent | Confirm Add/Remove offers no Repair/Modify (`ARPNOREPAIR`, `ARPNOMODIFY`) | Buttons absent | `NOT_EXECUTED` |
| M7 | Failed upgrade recovery | Start upgrade, force failure mid-install (e.g. terminate source), then re-run installer; verify previous version, shortcuts and data restored | Prior version and data restored; no duplicate entry | `NOT_EXECUTED` |
| M8 | WebView2 absence | On a VM without WebView2, install with network isolated; record bootstrapper behaviour | Clearly reported success or actionable failure; no silent hang | `NOT_EXECUTED` |
| M9 | Standard-user scope | Repeat M1/M2 as a standard user; confirm install requires elevation and data dir is writable | Correct elevation and writable data path | `NOT_EXECUTED` |

## 6. NSIS matrix

| # | Scenario | Steps | Expected result | Status |
|---|---|---|---|---|
| N1 | Fresh install (per-user and per-machine) | Run signed NSIS `/S` and GUI variants; confirm install dir, shortcuts, Add/Remove entry, publisher, elevation level per mode | Correct scope and signature | `NOT_EXECUTED` |
| N2 | Fresh startup | Launch from desktop and Start menu; confirm UI, IPC and cold launch time | Healthy | `NOT_EXECUTED` |
| N3 | Upgrade N → N+1 with data | Seed data; run next signed installer in update mode; confirm data hashes unchanged and shortcuts not removed | Data preserved, single entry, no app-data prompt | `NOT_EXECUTED` |
| N4 | Downgrade N+1 → N | Confirm downgrade choice in reinstall page and in silent/update mode; verify version, data and schema | No corrupt state | `NOT_EXECUTED` |
| N5 | Uninstall, checkbox unchecked | Uninstall with "Delete app data" unchecked; confirm install dir/shortcuts removed and `%APPDATA%\swiftfetch`, `com.swiftfetch.app`, `OtherApp`, `Local\swiftfetch` all intact | Data and unrelated files preserved | `NOT_EXECUTED` |
| N6 | Uninstall, checkbox checked | Uninstall with checkbox checked; confirm `%APPDATA%\swiftfetch` removed and unrelated trees above byte-identical | Only real SwiftFetch data removed | `NOT_EXECUTED` |
| N7 | Uninstall in update mode | Trigger uninstaller with `/UPDATE`; confirm app-data branch suppressed even if checkbox state is set | Data preserved | `NOT_EXECUTED` |
| N8 | Failed upgrade recovery | Interrupt update, re-run installer, verify previous binary, shortcuts, Add/Remove entry and data hash restored | Clean recovery | `NOT_EXECUTED` |
| N9 | Reinstall over existing | Run installer without update flag; exercise reinstall page choices; verify correct branch and no data loss | Correct branch chosen | `NOT_EXECUTED` |
| N10 | Silent/passive flags | Run `/S`, `/P`, `/R`, `/ARGS`; confirm shortcuts, auto-launch and argument forwarding | Documented flags behave as expected | `NOT_EXECUTED` |

## 7. Failure recovery and rollback criteria (both installers)

A rollback result is acceptable only if all of the following are evidenced:

1. Prior binary, Start menu/desktop shortcuts and Add/Remove entry are restored.
2. `%APPDATA%\swiftfetch\state.sqlite` and the settings/queue/history fixture hashes equal the
   pre-upgrade values.
3. Partial files created by the failed upgrade are removed; the user's downloads directory is
   untouched.
4. A second attempt at the same failing upgrade leaves the machine no worse than after step 3.
5. Logs, version strings, hashes and screenshots are archived.

## 8. Recording format

Create `docs/release/clean-machine-result.json` with:
`schema: swiftfetch.clean-machine/1`, `timestamp`, `installer` (path, sha256, authenticode),
`vm` (id, windows_build, scope), and a `results` array of
`{ id, status, command, exit_code, evidence_paths, notes }`.
Use only the status vocabulary in §1. Never copy a `SIMULATED` result into `results` as `PASS`.

## 9. Gate

Phase 4 clean-machine acceptance stays **externally blocked / NO-GO** until every `NOT_EXECUTED`
row above has a real executed result with archived evidence, and the signing/updater blockers in
`INSTALLER_READINESS_REPORT.md` are closed.
