# Changelog

All notable changes to this project are documented here. This project does not
yet have a public production release; 0.1.0 is a Windows release candidate.

## Unreleased — distribution preparation

- Documented owner decisions: individual publisher, proposed GPL-3.0-or-later,
  free download, zero budget.
- Added license/compatibility audit, third-party notices, public repository
  documents, free hosting/updater/SignPath guidance, and a GitHub Pages site.
- This is documentation-only preparation; no application behavior changed.

## 0.1.0 — Windows candidate

- Added checkpoint-safe HTTP transfers, task-owned working files, range
  validation, restart recovery, concurrency limits, categories and HTTP(S) proxy.
- Added explicit collision resolution without redownloading verified bytes.
- Added metadata retention, local queue backup, redacted inventory export/import,
  failed-partial ownership checks and task segment inspection.
- Added clipboard URL confirmation, URL drag-and-drop, tray controls,
  localized shell dictionaries, and cancellable shutdown requests.
- Added publisher-configured signed updates, signed-version verification,
  preflighted installer handoff and rollback health monitoring.
- Added self-contained browser packages, Firefox classic bundles, real Edge
  Native Messaging acceptance, Windows packaging and source/checksum tooling.
- Added validated full SQLite backup restore with a startup recovery page,
  safety copies, paused/manual restored tasks and consent resets.
- Added safe automatic server filenames from Content-Disposition, applied only
  to the final publish path with task-owned partial files preserved.
- Fixed browser host rejection reporting, update health acknowledgment, shutdown
  Cancel gating, and unsaved Add-dialog input preservation.
- Deferred WebView creation until native state is initialized and start workers
  only after the window builds; added a repeated cold-start probe.
- Verified live Firefox 160.0.1 add-on load and native messaging handoff with
  an isolated profile and a temporary registration that was removed afterward.
- Added opt-in Windows DPAPI encryption for saved request headers, transparent
  decryption of legacy rows and a transactional migration action.
- Added updater hardening: bound metadata snapshot with validated rollback,
  durable attempt phases with crash reconciliation, MSI INSTALLDIR preservation
  and a single recursive 1 GiB staging budget.
- Completed Bengali/English dictionary key parity and strengthened dialog focus,
  labelled controls and polite progress announcements.
- Moved tray registration after WebView creation so first paint is not blocked.

This candidate is not a statement that publisher signing, licensing, store
approval or clean-machine acceptance has been completed. See
`RELEASE-READINESS.md` and the verification evidence before distribution.
