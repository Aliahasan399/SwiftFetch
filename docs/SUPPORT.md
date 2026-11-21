# SwiftFetch support and recovery

Start with the Diagnostics view. Record the application version, error code,
task state and the action that failed. Use the local HTTP fixture for a safe
reproduction whenever possible. Do not include private download URLs or full
database backups in a public report.

## Queue and file recovery

- Pause a transfer before changing its destination or investigating partials.
- A compatible saved checkpoint can resume after restart. If the source changed
  or a working file is inconsistent, SwiftFetch restarts the affected bytes
  rather than combining representations.
- For an existing destination under Ask, choose Rename, Replace after explicit
  confirmation, or Skip. Rename keeps both files; Skip keeps the original.
- On disk-full or permission errors, preserve partials, free space or choose a
  writable destination, then retry. Never delete unrelated `.part` files by a
  filename pattern.
- Create a consistent local backup from Diagnostics. Close the app before
  restoring a full SQLite backup using an offline recovery procedure. Keep the
  original database and its backup; do not overwrite user downloads.
- A shareable inventory is for catalog recovery. Its private request URLs are
  omitted, so imported unfinished tasks remain paused until reviewed.

## Updates and installation

See [UPDATES.md](UPDATES.md) for signed installation and rollback. When updates
are not configured, install a reviewed signed release manually after closing
SwiftFetch. Keep data backups and the prior signed installer. Installer removal
must preserve queue state and user-owned downloads; verify that behavior on a
disposable Windows machine for each public release.

The release owner must publish supported versions, response expectations, an
issue-reporting location and a private security contact before public release.
