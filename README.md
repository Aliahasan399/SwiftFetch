# SwiftFetch

SwiftFetch is a Windows desktop download manager built with Tauri, React,
TypeScript, and Rust. Add an HTTP or HTTPS link to queue a download, pause or
retry it, set transfer limits, or supply a trusted checksum.

## Distribution and license (owner decisions, 2026-11-21)

- **Publisher:** individual / personal publisher.
- **License:** GPL-3.0-or-later is **proposed** and pending the dependency
  compatibility audit in [LICENSE_AUDIT.md](docs/release/LICENSE_AUDIT.md).
  The final `LICENSE` file will be added only after compatibility is confirmed.
- **Distribution:** free download for everyone; zero-budget, open-source
  strategy. No account, hosted backend, telemetry, or paid service is required.
- **Current status:** unsigned private release candidate. **NO-GO** for public
  release until the gates in [GO_NO_GO_REPORT.md](docs/release/GO_NO_GO_REPORT.md)
  are satisfied. See [ZERO_BUDGET_DISTRIBUTION_PLAN.md](docs/release/ZERO_BUDGET_DISTRIBUTION_PLAN.md)
  for the free distribution plan and the exact owner steps.

- The transfer engine uses multiple connections for eligible files and falls
  back to one stream when a server ignores range requests.
- The SQLite queue keeps history and checkpoints across application restarts.
- The desktop app provides priorities, scheduling windows, and diagnostics.
- Task details offer explicit collision decisions, segment ranges, and checksum
  state. Diagnostics provides local backup and redacted inventory recovery.
- Optional clipboard capture and URL drag/drop open a confirmation form. Tray
  controls can pause work or exit safely. Shutdown requests have a cancel countdown.
- Optional browser handoff uses Windows Native Messaging and requires desktop
  opt-in and per-user setup. [Browser setup](browser-extension/README.md)

Use SwiftFetch for content you have permission to download. Respect DRM, paywalls,
authentication, and site access and rate limits.

## Build and test on Windows

Install the Microsoft C++ Build Tools with the Desktop development with C++
workload, the WebView2 Runtime, Rust 1.98.0, Node.js 24 or newer, and Python 3.12.
The Rust toolchain should target `x86_64-pc-windows-msvc`. Windows 11 x64 is the
local verification platform. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

```powershell
npm ci
npm run tauri dev
```

```powershell
./tools/verify.ps1
./tools/build-windows.ps1
node tools/smoke-desktop.mjs
```

The build produces a desktop binary and Windows installers in
`src-tauri/target/release/`. See [RELEASE.md](RELEASE.md) for signing and staging
and [VERIFICATION.md](VERIFICATION.md) for the current test results. This is a
release candidate; [RELEASE-READINESS.md](RELEASE-READINESS.md) lists the remaining
public-release requirements and architecture gaps.

## Data and privacy

SwiftFetch keeps queue state, settings, and download history in the current
user's `%APPDATA%\swiftfetch` directory by default. The database is local SQLite
and can contain full download URLs, filenames, and paths. Keep backups private.
Downloads go to the chosen destination. `SWIFTFETCH_DATA_DIR`,
`SWIFTFETCH_DB_PATH`, and `SWIFTFETCH_DOWNLOADS_DIR` can override these defaults.

The app sends requests to the download source and follows its redirect policy.
The extension sends the selected URL to the local app only after an explicit
user action. It does not forward cookies or credentials. SwiftFetch has no
required account or telemetry upload. Only open completed files you trust.

## Project layout

| Path | Contents |
| --- | --- |
| `src/` | React UI and typed Tauri IPC wrappers |
| `src-tauri/src/` | Transfer engine, scheduler, SQLite queue, and Native Messaging host |
| `src-tauri/tests/` | Storage, scheduler, and transfer tests |
| `browser-extension/` | Browser packages, protocol, registration tools, and tests |
| `tools/fixtures/` | Local HTTP test server |

Publisher-configured signed updates and rollback are described in
[docs/UPDATES.md](docs/UPDATES.md). Privacy, permitted-use and support drafts are
under `docs/`; licensing and publisher release identity still require owner input.
