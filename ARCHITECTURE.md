# SwiftFetch implementation

SwiftFetch is a Windows x64 desktop application with a React/TypeScript
presentation layer in a Tauri WebView2 window and a Rust transfer engine.
The application works without a hosted account or queue service.

## Execution and data

The UI sends typed requests through the Tauri command bridge and subscribes to
status and progress events. Rust owns download status and completion decisions.
SQLite stores queues, task metadata, schedules, segment checkpoints and settings.
Downloaded bytes are written to task working files in the destination directory.

One desktop scheduler owns a queue database at a time. A filesystem lock prevents
another desktop process from starting a second scheduler for the same database.
An interrupted transfer is reconciled to a recoverable status on startup.

The transfer engine probes the source, chooses bounded range requests when
supported, and writes through a dedicated writer. Finalization checks the file's
size and any supplied checksum before publishing the result. Optional scanning
runs a user-configured local executable. Downloaded files open only after an
explicit user action.

The desktop shell owns tray, clipboard and cancellable power requests. Windows
sleep prevention runs on a dedicated OS thread so acquisition and release have
the same thread lifetime. The transfer engine emits shutdown intent; the shell
restores the window and offers a 60-second cancellation countdown before an OS
request. Unconfigured update builds make no release network requests.

Signed updates use publisher-embedded HTTPS endpoints and a public key. Both
the current signed rollback package and proposed update are verified before the
app drains workers and hands control to an isolated installer helper. The helper
waits for installation to finish before checking the new desktop's health, and
re-verifies cached rollback bytes when recovery is needed.

## Browser handoff

The optional extension uses Native Messaging with the installed executable.
The browser invokes a short-lived host process that validates one framed intent,
checks the local opt-in setting, and saves the task. The existing scheduler picks
up the task. When the desktop is closed, the task remains queued until it opens.

The host accepts an explicit user gesture, HTTP(S) URL, safe filename, originating
page origin, versioned protocol and short expiry. Cookies and unexpected fields
are rejected. A transaction records a replay receipt and the task together so a
repeat request cannot duplicate the operation across host processes. The
browser's per-user host manifest restricts which extension can launch the host.

## Main files

| File | Responsibility |
| --- | --- |
| `src/ipc.ts`, `src/types.ts` | UI wire contract |
| `src/store.ts` | Projections and event reconciliation |
| `src-tauri/src/lib.rs` | Application, client and storage lifetime |
| `src-tauri/src/commands.rs` | Validation and application commands |
| `src-tauri/src/engine.rs` | Transfers, scheduler, verification and finalization |
| `src-tauri/src/store.rs` | SQLite persistence |
| `src-tauri/src/security.rs` | URL, header and path policy |
| `src-tauri/src/native_host.rs` | Native Messaging framing and intake |
| `src-tauri/src/scanner.rs` | Local scanner process and timeout |
| `tools/smoke-desktop.mjs` | Packaged WebView2/IPC/recovery check |

## Release boundary

The initial delivery targets HTTP and HTTPS on Windows. Torrent, FTP, cloud sync,
media extraction, remote control and mandatory online licensing are outside the
initial product. Signing and distribution use the release owner's identity.
See [RELEASE.md](RELEASE.md) and [VERIFICATION.md](VERIFICATION.md).
