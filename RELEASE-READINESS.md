# SwiftFetch release readiness

This workspace contains a Windows release candidate. Public distribution is
blocked on the release-owner and verification items below. A successful build
does not establish that the entire architecture baseline is complete.

## Implemented in this pass

| Area | Result |
| --- | --- |
| Transfer state | `downloading` is emitted during transfer; cancellation remains cancellation |
| Range validation | Probe and segment ranges validate their offsets and total size |
| File ownership | Each task has its own working file; legacy partial files are preserved |
| Checkpoints | The writer syncs file bytes before acknowledging a durable checkpoint |
| Recovery | Missing or inconsistent working files restart safely; compatible checkpoints can resume |
| Finalization | Rename/Ask preserve existing files; only explicit Replace may overwrite; final paths are persisted |
| Queue control | UI and browser requests use scheduler admission; manual tasks wait for explicit release |
| Scheduling | Queue windows, overnight weekdays, completion/window-end requests, cancellable shutdown countdown |
| Categories | Explicit categories and automatic extension rules choose validated destination folders |
| Limits and proxy | Global/task limits and configured HTTP(S) proxy connections |
| Scanner | Quoted local scanner commands run asynchronously with a deadline |
| History | Cursor queries filter before limiting; refresh reconciles all loaded pages |
| Browser bridge | Executable Native Messaging host with opt-in, framing, expiry and durable replay receipts |
| Packaging | Windows build scripts, browser preview packages, source allowlist, and signing gate |
| Collision choices | Explicit overwrite/rename/skip from task details; no redownload for verified bytes |
| Lifecycle policies | Metadata retention, bounded logs, opt-in cleanup of task-owned failed partials |
| Desktop integration | Tray controls, clipboard URL confirmation, URL drag/drop, per-thread Windows power adapter |
| Recovery tools | Consistent local SQLite backup, redacted inventory export and safe paused import |
| Updates | Publisher-configured signed-version checks, verified rollback preflight, installer helper and health monitoring |
| Localization | Typed English/Bengali shell dictionaries and locale-aware formatting; English fallback for untranslated text |

## Public release blockers

- No code-signing certificate was found in the current user's or machine's
  code-signing certificate stores. The candidate installers are unsigned.
  The release owner must configure signing and run the signed staging procedure
  in [RELEASE.md](RELEASE.md).
- A project distribution license has not been selected. No open-source license
  has been assigned on the owner's behalf. Review third-party notices as well;
  the generated inventory identifies dependencies whose package lacks license
  text at its root.
- A clean Windows install, upgrade, rollback, and uninstall-preservation test
  remains required. Local WebView2/IPC checks use an isolated database and profile;
  they are not a clean-machine installer test.
- Cold-start reliability needs follow-up: some automated WebView launches returned
  a blank document. Manual launch and sequential runtime checks passed, followed
  by three consecutive complete extracted-MSI runtime passes. The initial
  failures remain recorded and require clean-machine follow-up.
- Actual Microsoft Edge extension installation and native handoff passed using
  an isolated profile. Live temporary Firefox 160.0.1 add-on load and native
  handoff also passed (`artifacts/firefox-acceptance/run-ec9603821...`); a permanent
  install and any required browser-store signing/review still need distribution
  validation.
- The publisher must provide the HTTPS update endpoints and public signing key
  at build time and retain immutable signed version manifests. A development
  build does not invent a publisher channel. An actual signed installer upgrade
  and automatic rollback still require release-environment acceptance.

## Scope still incomplete against the architecture baseline

Signed update and rollback code is implemented and signature tests cover valid,
modified and version-replayed packages. Actual publisher-signed installation and
rollback have not been exercised on a clean machine. See [docs/UPDATES.md](docs/UPDATES.md).

Clipboard capture is opt-in, and drag/drop opens a confirmation form rather than
starting a hidden transfer. Retention and partial policies are operative. Ask
collisions offer explicit choices in task details. Skip cancels the task and
removes only the new working file; it never labels another file as verified.

The application is localization-ready, with translated shell/navigation/action
keys and English fallback. It is not fully translated or independently certified
for accessibility. A 24-hour soak, broad site-compatibility sampling and clean
machine power/startup behavior are not established by the short local suites.

## Evidence and prepared artifacts

[VERIFICATION.md](VERIFICATION.md) records completed commands and their limits.
[ARCHITECTURE.md](ARCHITECTURE.md) describes the implemented local application.
[browser-extension/SETUP.md](browser-extension/SETUP.md) describes the executable
host and registration procedure. Build and test outputs are kept under
`artifacts/` and `src-tauri/target/`; local databases and working backups are
excluded from the source package.

Privacy, incident response, permitted-use and support drafts are in `docs/`.
They require the real publisher identity/contact and terms approval before
publication. They do not claim legal certification or assign a project license.
