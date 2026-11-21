# SwiftFetch privacy notice draft

SwiftFetch stores its queue, settings, filenames, destination paths, download
URLs and request metadata on the local Windows account. Downloads are stored
in the destination selected by the user. A URL or request header can contain
private access tokens; local database backups must therefore be kept private.

The application makes network requests to download sources, configured proxies,
and redirects accepted by its URL policy. A publisher-configured update check
contacts the release endpoint only when requested. No cloud account or telemetry
upload is required for downloading. Local diagnostic exports do not upload
themselves.

Clipboard monitoring is off by default. Enabling it reads clipboard text locally
to detect a permitted HTTP(S) URL and opens a confirmation form. Detection alone
does not start a transfer. The explicit Paste URL button reads the clipboard for
that action. Disable capture in Settings to stop monitoring.

The optional extension sends the selected URL through Windows Native Messaging
after a user action. It does not forward browser cookies or credentials.
Unpacked extension setup and native-host registration are optional.

History clearing and automatic retention remove queue metadata, not completed
files. Optional failed-partial cleanup is limited to the working file owned by
the failed task. An explicit Skip decision removes that task's new working file
and keeps the existing destination file unchanged.

A shareable queue inventory excludes request URLs and headers but includes
filenames and local paths. Review it before sharing. A full local SQLite backup
contains private request data; it is not a support attachment by default.

Before publication, the release owner must identify the publisher, provide a
privacy contact, choose support/retention procedures, and review this notice
against the actual release configuration. This draft does not claim regulatory
certification.
