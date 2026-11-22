# SwiftFetch — zero-cost clean Windows test environment options (Phase 3 parallel work)

**Status:** PLANNING ONLY. No VM was created, no installer was run on a VM, no clean-machine result exists.
**Verdict remains NO-GO.** See `INSTALLER_ACCEPTANCE_PROCEDURE.md` for the test matrix.

## 1. Environment requirements (what the tests actually need)

| Requirement | Reason |
|---|---|
| Fresh Windows 10/11 x64 profile per scenario | No pre-existing SwiftFetch data, native-host key, or browser extension state |
| WebView2 present or bootstrapper exercised | Tauri runtime dependency; MSI authors a WebView2 bootstrap fallback |
| Hypervisor-backed isolation | Uninstall/upgrade/downgrade must not affect the host |
| Revertible snapshot or disposable instance | Each matrix row starts clean; failure recovery can be repeated |
| Ability to run silent and GUI installers | MSI `/qn`, NSIS `/S` and the interactive reinstall/uninstall checkbox paths |
| Isolated network with local fixture server | Download/resume/handoff tests without external traffic |
| Evidence capture | Logs, screenshots, version strings, hashes, exit codes |
| Signed installer artifacts | Prerequisite for realistic publisher/SmartScreen assessment (not available yet) |

## 2. Local host facts (observed 2026-11-22)

- OS: **Windows 11 Pro**, build `10.0.26200` (edition `Professional`).
- Windows Sandbox is supported on Pro/Enterprise/Education (Microsoft Learn), so this host is
  *edition-eligible*. Feature activation could not be read because the query requires elevation;
  it remains **unverified**.
- VirtualBox (`VBoxManage`) not found on PATH.
- No clean VM exists; no VM test has been executed.

## 3. Zero-cost options

| Option | Cost | Isolation | Snapshot / reset | GUI | Silent CLI | Fit for full matrix | Limits |
|---|---|---|---|---|---|---|---|
| **Windows Sandbox** on this host | Included with Windows Pro/Enterprise/Education | Hypervisor VM, disposable | Fresh each launch; data persists across in-sandbox restarts since 22H2 | Yes | Yes | Good for fresh install, startup, uninstall, opt-in delete; weak for long-lived upgrade chains | Single instance at a time; not supported on Home; requires Hyper-V enabled; networking on by default (can be disabled in `.wsb`); host-installed software unavailable |
| **GitHub Actions `windows-latest`** standard runner | Free for public repositories (GitHub billing docs) | Fresh hosted runner per job | Fresh by construction | Limited | Yes | Good for automated MSI/NSIS silent install, upgrade, hash and signature checks; not a substitute for GUI acceptance | 6-hour job limit; no interactive desktop by default; runner image drifts; cannot verify SmartScreen or visual dialogs reliably |
| **Microsoft Windows 11 Enterprise Evaluation VHD/ISO** | Free evaluation (90-day, Microsoft Evaluation Center) | Full VM via Hyper-V or VirtualBox | Checkpoint/revert | Yes | Yes | **Best fit** for the complete matrix including downgrade and failure recovery | Evaluation expiry; intended for IT professionals; must be re-armed/re-downloaded; needs host resources and a hypervisor |
| **Hyper-V** (built into Windows Pro/Enterprise) | Included with eligible host edition | Full VM | Checkpoints | Yes | Yes | Good host hypervisor for the eval VHD | Feature activation unverified here; nested-virtualization/host requirements |
| **VirtualBox** | Free (GPL) | Full VM | Snapshots | Yes | Yes | Alternative hypervisor if Hyper-V is unavailable | Not installed here; guest additions and eval image setup effort |
| Cloud free trials (Azure/AWS/etc.) | Not durably free | Full VM | Snapshots | Yes | Yes | Only short-term | Require payment details, expire, and conflict with the zero-budget rule; **not recommended** |
| Host machine directly | Free | None | None | Yes | Yes | Not acceptable | Would modify the working machine and invalidate "clean machine" evidence |

## 4. Recommended combination

1. **GitHub Actions `windows-latest`** for repeatable, free, silent-install smoke checks on every
   candidate build. Treat results as automated CI evidence, not clean-machine acceptance.
2. **Windows Sandbox on this Windows 11 Pro host** (once Hyper-V is confirmed enabled) for quick
   interactive checks: fresh install, UI startup, uninstall, and the NSIS opt-in/checkbox paths.
   Use a `.wsb` config with networking disabled and read-only mapped folders for installer input.
3. **Windows 11 Enterprise Evaluation VHD in Hyper-V or VirtualBox** for the full acceptance matrix
   requiring snapshots, multi-step upgrade/downgrade chains, and failed-upgrade rollback. Revert to a
   clean checkpoint between rows.

Each executed run must record the actual environment used; results from one option cannot be
relabelled as another.

## 5. Readiness checklist before any VM run

- ☐ Signed MSI and NSIS artifacts plus `SHA256SUMS` from `generate-distribution-hashes.ps1`.
- ☐ Hyper-V / Windows Sandbox feature confirmed enabled, or eval VHD downloaded and verified.
- ☐ Baseline fixture data set prepared (download, partial, queue entry, history, distinctive setting).
- ☐ Local fixture server starts and passes `tools/fixtures/selftest.py`.
- ☐ `clean-machine-result.json` schema initialized with `NOT_EXECUTED` rows from the acceptance procedure.
- ☐ Owner authorization for any external network access by the guest.

## 6. Evidence and honesty rules

- A CI run on `windows-latest` is `SIMULATED`-grade for GUI/visual items and `EXECUTED`-grade only
  for the specific silent-install steps it actually performs.
- Windows Sandbox results are real for that sandbox instance only; record that the host was
  Windows 11 Pro and the sandbox was disposable.
- Eval-VHD results are real VM results and may be marked `PASS`/`FAIL` in `clean-machine-result.json`.
- No result in this document or its companion documents has been executed. Until real evidence
  exists, every matrix row remains `NOT_EXECUTED` and the release verdict remains **NO-GO**.

## 7. Sources (fetched 2026-11-22)

| Topic | URL |
|---|---|
| Windows Sandbox features and edition requirements | https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/ |
| GitHub-hosted runners free for public repos | https://docs.github.com/en/billing/concepts/product-billing/github-actions |
| GitHub Actions limits (6h job, 35-day run) | https://docs.github.com/en/actions/reference/limits |
| Windows 11 Enterprise evaluation | https://www.microsoft.com/en-us/evalcenter/evaluate-windows-11-enterprise |
