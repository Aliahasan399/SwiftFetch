# Signed updates and rollback

The Windows app uses the pinned Tauri updater to verify Minisign signatures.
Signed package versions are required. An unconfigured build performs no release
network request and reports that the publisher has not configured updates.

## Publisher configuration

Set these at build time in the release environment:

| Variable | Value |
| --- | --- |
| `SWIFTFETCH_UPDATE_ENDPOINT` | HTTPS latest manifest endpoint |
| `SWIFTFETCH_UPDATE_ARCHIVE` | HTTPS immutable manifest endpoint containing `{{version}}` |
| `SWIFTFETCH_UPDATE_PUBLIC_KEY` | Tauri public signing key content |
| `TAURI_SIGNING_PRIVATE_KEY` or `TAURI_SIGNING_PRIVATE_KEY_PATH` | Release environment's update signing key |

The public channel values are embedded in the app. The private key is used only
by the build/signing tools. Keep it out of source and output logs. Windows
Authenticode signing uses the separate publisher certificate configured through
the Tauri signing overlay. Update signing alone does not establish an
Authenticode publisher identity.

Run `tools/build-windows.ps1` with the release owner's signing overlay. When the
channel is configured, it enables updater artifact signatures. Use
`tools/release-channel.ps1` to prepare each immutable version manifest. Publish
the manifest and signature beside the selected installer after release approval.
The archive endpoint must retain each supported version, including the initial
release, so clients can obtain the current release's rollback installer.

## Installation and recovery

The user explicitly checks and confirms updates. The app downloads and verifies
the currently installed release from its immutable endpoint before accepting an
upgrade. It caches that rollback package and its signed-version metadata. The
new package is downloaded with bounded memory and verified before transfers are
quiesced. Queue data remains separate from application installation files.

A recovery helper re-verifies both packages and acknowledges readiness before
the current app exits. It waits for the old process to release its queue, then
runs the signed installer to completion. Only after installation exits does it
launch the installed executable with normal arguments and begin a three-minute
health deadline. The new version must initialize SQLite and report that the
desktop UI is ready. A failed launch, early exit or missing health confirmation
uses the verified cached rollback installer. It does not execute unsigned or
modified cached bytes, and does not start competing installers during an upgrade.

Packages use content-addressed filenames so retrying an identical release reuses
the verified cache. Update staging has a 1 GiB bound; if reached, it asks the user
to review old backups instead of silently consuming unlimited space.

The UI also offers an explicit rollback action when a cached package exists.
Signature verification is repeated before installation; the existence of a cache
file is never treated as proof that its contents are trusted.

Local signature tests cover valid signed bytes, modified bytes, a replayed version,
and cached rollback verification. An actual signed installer upgrade and recovery
must also be exercised in a disposable Windows environment using the publisher's
release artifacts. No publisher credentials or update endpoint are invented by
the development build.
